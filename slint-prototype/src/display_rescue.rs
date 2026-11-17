//! Reverting a pending display change when the app itself is gone.
//!
//! The keep-or-revert deadline normally lives in a thread of the app, so a
//! SIGKILL or crash during the countdown would leave an unconfirmed layout
//! live. Before applying, the guard writes the previous layout, the deadline
//! and its own pid to a file in the runtime dir. Keep, Revert and every revert
//! delete the file. If the file is still there at the deadline and its owner
//! is no longer running, restore puts the previous layout back.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::displays::{self, Monitor};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Record {
    pub id: u64,
    pub pid: u32,
    /// Milliseconds since the Unix epoch.
    pub deadline_ms: u64,
    pub before: Vec<Monitor>,
}

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

/// `$XDG_RUNTIME_DIR/bos-settings/display-pending.json`. The runtime dir is
/// per-user and cleared at logout, which is the right lifetime for a record.
pub fn record_path() -> Option<PathBuf> {
    let dir = std::env::var_os("XDG_RUNTIME_DIR")?;
    Some(PathBuf::from(dir).join("bos-settings/display-pending.json"))
}

pub fn write_record(path: &Path, record: &Record) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let temp = path.with_extension("json.tmp");
    let text = serde_json::to_string(record).map_err(|error| error.to_string())?;
    std::fs::write(&temp, text).map_err(|error| error.to_string())?;
    std::fs::rename(&temp, path).map_err(|error| error.to_string())
}

pub fn read_record(path: &Path) -> Option<Record> {
    serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
}

/// Remove the record only if it still belongs to change `id`.
pub fn clear_record(path: &Path, id: u64) {
    if read_record(path).is_some_and(|record| record.id == id) {
        let _ = std::fs::remove_file(path);
    }
}

/// True when `pid` is a live instance of this program. A recycled pid that now
/// belongs to something else counts as gone.
pub fn owner_alive(pid: u32) -> bool {
    let Ok(exe) = std::fs::read_link(format!("/proc/{pid}/exe")) else {
        return false;
    };
    match std::env::current_exe().ok() {
        Some(me) => same_program(&exe, &me),
        None => true,
    }
}

fn same_program(exe: &Path, me: &Path) -> bool {
    let strip = |path: &Path| {
        let text = path.to_string_lossy();
        text.strip_suffix(" (deleted)").unwrap_or(&text).to_string()
    };
    let (a, b) = (strip(exe), strip(me));
    a == b || Path::new(&a).file_name() == Path::new(&b).file_name()
}

#[derive(Debug, PartialEq)]
pub enum Decision {
    Leave,
    Wait(Duration),
    Restore,
}

const GRACE_MS: u64 = 10_000;

pub fn decide(record: Option<&Record>, id: Option<u64>, now: u64, alive: bool) -> Decision {
    let Some(record) = record else {
        return Decision::Leave;
    };
    if id.is_some_and(|id| id != record.id) {
        return Decision::Leave;
    }
    if now < record.deadline_ms {
        return Decision::Wait(Duration::from_millis(record.deadline_ms - now));
    }
    if alive && now < record.deadline_ms + GRACE_MS {
        return Decision::Wait(Duration::from_millis(
            record.deadline_ms + GRACE_MS - now,
        ));
    }
    Decision::Restore
}

pub fn restore(record: &Record) -> Result<(), String> {
    displays::write_layout(&record.before)
}

/// One pass: restore if due. Returns how long to wait before the next pass, or
/// None when there is nothing left to watch.
pub fn check_once(path: &Path, id: Option<u64>, alive: impl Fn(u32) -> bool) -> Option<Duration> {
    let record = read_record(path);
    let owner = record.as_ref().is_some_and(|record| alive(record.pid));
    match decide(record.as_ref(), id, now_ms(), owner) {
        Decision::Leave => None,
        Decision::Wait(wait) => Some(wait),
        Decision::Restore => {
            let record = record.expect("decide restores only with a record");
            let _ = restore(&record);
            clear_record(path, record.id);
            None
        }
    }
}

/// `bos-settings-slint --display-watchdog <id>`: started detached by the
/// guard and watches until the change is kept, reverted or restored.
pub fn run_watchdog(args: &[String]) {
    let id = args
        .iter()
        .position(|arg| arg == "--display-watchdog")
        .and_then(|index| args.get(index + 1))
        .and_then(|value| value.parse::<u64>().ok());
    let Some(path) = record_path() else {
        return;
    };
    while let Some(wait) = check_once(&path, id, owner_alive) {
        std::thread::sleep(wait.min(Duration::from_secs(5)));
    }
}

/// At app start: a record left by a run that died during its countdown.
pub fn recover_at_start() {
    let Some(path) = record_path() else {
        return;
    };
    let Some(record) = read_record(&path) else {
        return;
    };
    if record.pid == std::process::id() || owner_alive(record.pid) {
        return;
    }
    if now_ms() < record.deadline_ms {
        return;
    }
    let _ = check_once(&path, Some(record.id), |_| false);
}

/// Start the detached watchdog for change `id`. It gets its own session, so
/// killing the app or its process group does not take it down.
pub fn spawn_watchdog(id: u64) -> Result<(), String> {
    use std::os::unix::process::CommandExt;
    let exe = std::env::current_exe().map_err(|error| error.to_string())?;
    let mut command = std::process::Command::new(exe);
    command
        .arg("--display-watchdog")
        .arg(id.to_string())
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    unsafe {
        command.pre_exec(|| {
            setsid();
            Ok(())
        });
    }
    let mut child = command.spawn().map_err(|error| error.to_string())?;
    std::thread::Builder::new()
        .name("display-watchdog-reaper".into())
        .spawn(move || {
            let _ = child.wait();
        })
        .map(|_| ())
        .map_err(|error| error.to_string())
}

extern "C" {
    fn setsid() -> i32;
    fn signal(signum: i32, handler: usize) -> usize;
}

static SIGNALLED: AtomicBool = AtomicBool::new(false);

extern "C" fn on_signal(_: i32) {
    SIGNALLED.store(true, Ordering::SeqCst);
}

/// Undo an unconfirmed change when the app is asked to quit with SIGTERM,
/// SIGINT or SIGHUP.
pub fn revert_on_signal() {
    const SIGHUP: i32 = 1;
    const SIGINT: i32 = 2;
    const SIGTERM: i32 = 15;
    let handler = on_signal as extern "C" fn(i32) as usize;
    for number in [SIGHUP, SIGINT, SIGTERM] {
        unsafe {
            signal(number, handler);
        }
    }
    std::thread::spawn(|| loop {
        std::thread::sleep(Duration::from_millis(200));
        if SIGNALLED.load(Ordering::SeqCst) {
            displays::revert_on_exit();
            std::process::exit(0);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(id: u64, deadline_ms: u64) -> Record {
        Record {
            id,
            pid: 4_000_000,
            deadline_ms,
            before: Vec::new(),
        }
    }

    #[test]
    fn decide_waits_restores_and_leaves() {
        let held = record(7, 10_000);
        assert_eq!(decide(None, Some(7), 0, false), Decision::Leave);
        assert_eq!(decide(Some(&held), Some(8), 20_000, false), Decision::Leave);
        assert_eq!(
            decide(Some(&held), Some(7), 4_000, false),
            Decision::Wait(Duration::from_millis(6_000))
        );
        assert_eq!(decide(Some(&held), Some(7), 10_000, false), Decision::Restore);
        assert!(matches!(
            decide(Some(&held), Some(7), 11_000, true),
            Decision::Wait(_)
        ));
    }

    #[test]
    fn clear_only_removes_its_own_record() {
        let dir = std::env::temp_dir().join(format!("rescue-rt-{}", std::process::id()));
        let path = dir.join("display-pending.json");
        let held = record(3, 123);
        write_record(&path, &held).unwrap();
        assert_eq!(read_record(&path), Some(held));
        clear_record(&path, 4);
        assert!(path.exists());
        clear_record(&path, 3);
        assert!(!path.exists());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn replaced_binary_is_the_same_program() {
        let me = Path::new("/usr/local/bin/bos-settings-slint");
        assert!(same_program(
            Path::new("/usr/local/bin/bos-settings-slint (deleted)"),
            me
        ));
        assert!(!same_program(Path::new("/usr/bin/sleep"), me));
    }
}
