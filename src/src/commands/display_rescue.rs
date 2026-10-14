//! Reverting a pending display change when the app itself is gone (BSET-67).
//!
//! The keep-or-revert deadline normally lives in a thread of the app, so a
//! SIGKILL or crash during the countdown would leave an unconfirmed layout
//! live. Before applying, the guard writes the previous layout, the deadline
//! (wall clock) and its own pid to a file in the runtime dir and starts a
//! detached `bos-settings --display-watchdog`. Keep, Revert and every revert
//! delete the file. If the file is still there at the deadline and its owner
//! is no longer running, the watchdog restores the previous layout. App start
//! performs the same check for a file left behind by an earlier run.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use super::display_guard::{plan_statements, Compositor, HyprCompositor};
use super::hyprland::LiveMonitor;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Record {
    pub id: u64,
    pub pid: u32,
    /// Milliseconds since the Unix epoch.
    pub deadline_ms: u64,
    pub before: Vec<LiveMonitor>,
}

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// `$XDG_RUNTIME_DIR/bos-settings/display-pending.json`. The runtime dir is
/// per-user, mode 0700 and cleared at logout, which is the right lifetime:
/// a stale record must never apply to a later session.
pub fn record_path() -> Option<PathBuf> {
    let dir = std::env::var_os("XDG_RUNTIME_DIR")?;
    Some(PathBuf::from(dir).join("bos-settings/display-pending.json"))
}

pub fn write_record(path: &Path, rec: &Record) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension("json.tmp");
    let text = serde_json::to_string(rec).map_err(|e| e.to_string())?;
    std::fs::write(&tmp, text).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())
}

pub fn read_record(path: &Path) -> Option<Record> {
    serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
}

/// Remove the record only if it still belongs to change `id`, so a later
/// change's record is never deleted by an earlier one finishing.
pub fn clear_record(path: &Path, id: u64) {
    if read_record(path).is_some_and(|r| r.id == id) {
        let _ = std::fs::remove_file(path);
    }
}

/// True when `pid` is a live bos-settings process. A recycled pid that now
/// belongs to something else counts as gone.
pub fn owner_alive(pid: u32) -> bool {
    let Ok(exe) = std::fs::read_link(format!("/proc/{pid}/exe")) else {
        return false;
    };
    let me = std::env::current_exe().ok();
    match me {
        Some(me) => exe == me || exe.file_name() == me.file_name(),
        None => true,
    }
}

#[derive(Debug, PartialEq)]
pub enum Decision {
    /// Nothing to do: no record, a different change, or the owner will handle it.
    Leave,
    /// Not yet due; check again after this long.
    Wait(Duration),
    /// The owner is gone and the deadline passed: put the old layout back.
    Restore,
}

pub fn decide(rec: Option<&Record>, id: Option<u64>, now_ms: u64, alive: bool) -> Decision {
    let Some(rec) = rec else {
        return Decision::Leave;
    };
    if id.is_some_and(|id| id != rec.id) {
        return Decision::Leave;
    }
    if now_ms < rec.deadline_ms {
        return Decision::Wait(Duration::from_millis(rec.deadline_ms - now_ms));
    }
    if alive {
        // The app's own timer reverts on time; give it a moment before
        // deciding it is stuck.
        if now_ms < rec.deadline_ms + GRACE_MS {
            return Decision::Wait(Duration::from_millis(rec.deadline_ms + GRACE_MS - now_ms));
        }
    }
    Decision::Restore
}

/// How long past the deadline a still-running owner gets to revert itself.
const GRACE_MS: u64 = 10_000;

pub fn restore(comp: &dyn Compositor, rec: &Record) -> Result<(), String> {
    let stmts = plan_statements(&rec.before)?;
    comp.eval(&stmts.join("; "))
}

/// One pass: restore if due. Returns how long to wait before the next pass,
/// or None when there is nothing left to watch.
pub fn check_once(
    comp: &dyn Compositor,
    path: &Path,
    id: Option<u64>,
    alive: impl Fn(u32) -> bool,
) -> Option<Duration> {
    let rec = read_record(path);
    let owner_alive = rec.as_ref().is_some_and(|r| alive(r.pid));
    match decide(rec.as_ref(), id, now_ms(), owner_alive) {
        Decision::Leave => None,
        Decision::Wait(d) => Some(d),
        Decision::Restore => {
            let rec = rec.expect("decide restores only with a record");
            match restore(comp, &rec) {
                Ok(()) => eprintln!("bos-settings: restored the previous display layout"),
                Err(e) => {
                    eprintln!("bos-settings: restoring the previous display layout failed: {e}")
                }
            }
            clear_record(path, rec.id);
            None
        }
    }
}

/// `bos-settings --display-watchdog <id>`: started detached by the guard.
pub fn run_watchdog(args: &[String]) {
    let id = args
        .iter()
        .position(|a| a == "--display-watchdog")
        .and_then(|i| args.get(i + 1))
        .and_then(|v| v.parse::<u64>().ok());
    let Some(path) = record_path() else { return };
    let comp = HyprCompositor;
    while let Some(wait) = check_once(&comp, &path, id, owner_alive) {
        std::thread::sleep(wait.min(Duration::from_secs(5)));
    }
}

/// At app start: a record left by a run that died during its countdown.
pub fn recover_at_start() {
    let Some(path) = record_path() else { return };
    let Some(rec) = read_record(&path) else {
        return;
    };
    if rec.pid == std::process::id() || owner_alive(rec.pid) {
        return;
    }
    if now_ms() < rec.deadline_ms {
        // Its watchdog will act at the deadline; nothing to do yet.
        return;
    }
    let _ = check_once(&HyprCompositor, &path, Some(rec.id), |_| false);
}

/// Start the detached watchdog for change `id`. It gets its own session, so
/// killing the app or its process group does not take it down.
pub fn spawn_watchdog(id: u64) -> Result<(), String> {
    use std::os::unix::process::CommandExt;
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let mut cmd = std::process::Command::new(exe);
    cmd.arg("--display-watchdog")
        .arg(id.to_string())
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    // SAFETY: setsid is async-signal-safe and only detaches the child from
    // the app's session before exec.
    unsafe {
        cmd.pre_exec(|| {
            libc_setsid();
            Ok(())
        });
    }
    cmd.spawn().map(|_| ()).map_err(|e| e.to_string())
}

extern "C" {
    fn setsid() -> i32;
}

fn libc_setsid() {
    // SAFETY: plain libc call with no arguments.
    unsafe {
        setsid();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    fn rec(id: u64, deadline_ms: u64) -> Record {
        Record {
            id,
            pid: 4_000_000,
            deadline_ms,
            before: Vec::new(),
        }
    }

    #[test]
    fn decide_waits_restores_and_leaves() {
        let r = rec(7, 10_000);
        assert_eq!(decide(None, Some(7), 0, false), Decision::Leave);
        assert_eq!(
            decide(Some(&r), Some(8), 20_000, false),
            Decision::Leave,
            "another change"
        );
        assert_eq!(
            decide(Some(&r), Some(7), 4_000, false),
            Decision::Wait(Duration::from_millis(6_000))
        );
        assert_eq!(
            decide(Some(&r), Some(7), 10_000, false),
            Decision::Restore,
            "owner killed"
        );
        assert!(
            matches!(decide(Some(&r), Some(7), 11_000, true), Decision::Wait(_)),
            "a live owner gets a grace period to revert itself"
        );
        assert_eq!(
            decide(Some(&r), None, 10_000 + GRACE_MS, true),
            Decision::Restore,
            "owner stuck"
        );
    }

    #[test]
    fn record_round_trip_and_clear_only_own_id() {
        let dir = std::env::temp_dir().join(format!("bset67-rt-{}", std::process::id()));
        let path = dir.join("display-pending.json");
        let r = rec(3, 123);
        write_record(&path, &r).unwrap();
        assert_eq!(read_record(&path), Some(r));
        clear_record(&path, 4);
        assert!(
            path.exists(),
            "a different change must not clear this record"
        );
        clear_record(&path, 3);
        assert!(!path.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    struct Rec(Mutex<Vec<String>>);
    impl Compositor for Rec {
        fn read(&self) -> Result<Vec<LiveMonitor>, String> {
            Ok(Vec::new())
        }
        fn eval(&self, lua: &str) -> Result<(), String> {
            self.0.lock().unwrap().push(lua.to_string());
            Ok(())
        }
        fn persist(&self, _: &[LiveMonitor]) -> Result<(), String> {
            Ok(())
        }
    }

    #[test]
    fn check_once_restores_after_owner_dies() {
        let dir = std::env::temp_dir().join(format!("bset67-co-{}", std::process::id()));
        let path = dir.join("display-pending.json");
        let comp = Rec(Mutex::new(Vec::new()));
        // Not due yet: wait, nothing applied.
        write_record(&path, &rec(1, now_ms() + 60_000)).unwrap();
        assert!(check_once(&comp, &path, Some(1), |_| false).is_some());
        assert!(comp.0.lock().unwrap().is_empty());
        // Past the deadline and the owner is gone: restore once and clear.
        write_record(&path, &rec(1, now_ms().saturating_sub(1))).unwrap();
        assert_eq!(check_once(&comp, &path, Some(1), |_| false), None);
        assert_eq!(comp.0.lock().unwrap().len(), 1);
        assert!(!path.exists());
        // Nothing left: nothing more to do.
        assert_eq!(check_once(&comp, &path, Some(1), |_| false), None);
        assert_eq!(comp.0.lock().unwrap().len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn dead_pid_is_not_alive() {
        assert!(!owner_alive(4_000_000));
        assert!(owner_alive(std::process::id()));
    }
}
