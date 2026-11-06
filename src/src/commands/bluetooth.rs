//! Bluetooth over `bluetoothctl`'s non-interactive mode, no D-Bus
//! dependency needed, same "shell out to the standard CLI" choice as
//! Network (nmcli). None of this needs `pkexec`: BlueZ's D-Bus policy
//! already allows the active session user.
//!
//! Pairing only covers "Just Works" Simple Secure Pairing; bluetoothd's
//! own built-in default agent auto-accepts that for most audio/HID
//! devices. A device that requires PIN/passkey confirmation isn't
//! supported (would need this app to register its own bluetoothd agent);
//! pairing such a device just fails, surfaced as an error.

use serde::Serialize;
use std::collections::HashSet;
use std::path::PathBuf;
use std::process::Output;
use std::time::Duration;
use tokio::process::Command;

/// bluetoothctl waits forever for bluetoothd when the service is not running
/// (BSET-65), so every call is bounded and the child killed on timeout.
const QUICK: Duration = Duration::from_secs(5);
const SCAN: Duration = Duration::from_secs(15);
const PAIR: Duration = Duration::from_secs(30);
/// bluetoothd needs a moment after `systemctl start` before it owns the
/// BlueZ name on the bus; the Start button waits this long for an answer.
const START_WAIT: Duration = Duration::from_secs(10);
const POLL: Duration = Duration::from_millis(250);

async fn btctl(args: &[&str], limit: Duration) -> Result<Output, String> {
    bounded("bluetoothctl", args, limit).await
}

async fn bounded(program: &str, args: &[&str], limit: Duration) -> Result<Output, String> {
    bounded_with(program, &[], args, limit).await
}

async fn bounded_with(
    program: &str,
    prefix: &[String],
    args: &[&str],
    limit: Duration,
) -> Result<Output, String> {
    let child = Command::new(program)
        .args(prefix)
        .args(args)
        .kill_on_drop(true)
        .output();
    match tokio::time::timeout(limit, child).await {
        Ok(out) => out.map_err(|e| e.to_string()),
        Err(_) => Err("Bluetooth isn't responding. The Bluetooth service may be stopped.".into()),
    }
}

fn has_adapter_in(sysfs: &std::path::Path) -> bool {
    std::fs::read_dir(sysfs)
        .map(|mut d| {
            d.any(|e| {
                e.map(|e| e.file_name().to_string_lossy().starts_with("hci"))
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false)
}

#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum BtState {
    /// No controller in /sys/class/bluetooth.
    NoAdapter,
    /// A controller exists but bluetoothd is not running.
    ServiceOff,
    /// bluetoothd answers; `powered` is the controller's power.
    Ready { powered: bool },
}

/// A command to run. Production uses the plain program name; tests point
/// it at a script run through `sh`, so a script that was just written is
/// only ever read, not exec'd (a fresh exec can hit ETXTBSY while a sibling
/// test thread still has the file open on some filesystems).
#[derive(Clone)]
struct Cmd {
    program: String,
    prefix: Vec<String>,
}

impl Cmd {
    fn plain(program: &str) -> Self {
        Self {
            program: program.into(),
            prefix: Vec::new(),
        }
    }

    #[cfg(test)]
    fn script(path: &std::path::Path) -> Self {
        Self {
            program: "sh".into(),
            prefix: vec![path.to_string_lossy().into_owned()],
        }
    }

    fn command(&self) -> Command {
        let mut command = Command::new(&self.program);
        command.args(&self.prefix);
        command
    }
}

/// The commands and sysfs root the probe uses. Kept together so tests can
/// point them at fake systemctl/bluetoothctl scripts instead of the host.
struct BtHost {
    systemctl: Cmd,
    bluetoothctl: Cmd,
    launcher: Cmd,
    sysfs: PathBuf,
}

impl BtHost {
    fn system() -> Self {
        Self {
            systemctl: Cmd::plain("systemctl"),
            bluetoothctl: Cmd::plain("bluetoothctl"),
            launcher: Cmd::plain("pkexec"),
            sysfs: PathBuf::from("/sys/class/bluetooth"),
        }
    }

    #[cfg(test)]
    fn with_programs(
        systemctl: &std::path::Path,
        bluetoothctl: &std::path::Path,
        launcher: &std::path::Path,
        sysfs: PathBuf,
    ) -> Self {
        Self {
            systemctl: Cmd::script(systemctl),
            bluetoothctl: Cmd::script(bluetoothctl),
            launcher: Cmd::script(launcher),
            sysfs,
        }
    }

    fn adapter_present(&self) -> bool {
        has_adapter_in(&self.sysfs)
    }

    async fn service_active(&self) -> bool {
        self.systemctl
            .command()
            .args(["is-active", "--quiet", "bluetooth.service"])
            .status()
            .await
            .map(|s| s.success())
            .unwrap_or(false)
    }

    async fn btctl(&self, args: &[&str], limit: Duration) -> Result<Output, String> {
        bounded_with(
            &self.bluetoothctl.program,
            &self.bluetoothctl.prefix,
            args,
            limit,
        )
        .await
    }

    async fn adapter_powered(&self) -> Option<bool> {
        let output = self.btctl(&["show"], QUICK).await.ok()?;
        if !output.status.success() {
            return None;
        }
        parse_controller_show(&String::from_utf8_lossy(&output.stdout))
    }

    /// Probe systemd and BlueZ together so the page and Home agree.
    async fn probe(&self) -> Probe {
        let service_active = self.service_active().await;
        let powered = if service_active {
            self.adapter_powered().await
        } else {
            None
        };
        Probe {
            state: classify(powered, self.adapter_present()),
            service_active,
        }
    }

    /// Poll until bluetoothd actually answers, not just until systemd says
    /// the unit is active. bluetoothctl can miss the controller for a
    /// moment while the daemon claims its bus name (BSET-92).
    async fn wait_for_started(&self, limit: Duration) -> Probe {
        let start = std::time::Instant::now();
        loop {
            let probe = self.probe().await;
            // A just-started daemon with an adapter present can still
            // answer nothing at first: keep polling for a real answer.
            if probe.service_active && probe.state != BtState::ServiceOff {
                return probe;
            }
            if start.elapsed() >= limit {
                return probe;
            }
            tokio::time::sleep(POLL).await;
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Probe {
    state: BtState,
    service_active: bool,
}

/// BlueZ answering `show` wins over systemd's view: if it reports a
/// controller, Bluetooth is usable. Otherwise a controller in sysfs means
/// the service is the missing piece, and no controller means no adapter.
fn classify(powered: Option<bool>, adapter: bool) -> BtState {
    match powered {
        Some(powered) => BtState::Ready { powered },
        None if adapter => BtState::ServiceOff,
        None => BtState::NoAdapter,
    }
}

/// A `bluetoothctl show` answer: a controller block means BlueZ is awake,
/// and `Powered: yes` is its radio state. Empty output (daemon not up yet)
/// is not an answer.
fn parse_controller_show(text: &str) -> Option<bool> {
    if !text.trim_start().starts_with("Controller") {
        return None;
    }
    Some(text.lines().any(|l| l.trim() == "Powered: yes"))
}

#[tauri::command]
pub async fn get_bluetooth_state() -> BtState {
    BtHost::system().probe().await.state
}

#[derive(Serialize, Debug)]
pub struct BluetoothStart {
    state: BtState,
    message: String,
}

/// Starts bluetoothd after asking for the admin password. Only this fixed
/// unit, never a caller-supplied one. The result carries a re-probed state
/// and a message the page shows, so a successful start never leaves the
/// old "service is off" screen behind.
#[tauri::command]
pub async fn start_bluetooth_service() -> Result<BluetoothStart, String> {
    start_bluetooth_with(&BtHost::system(), START_WAIT).await
}

async fn start_bluetooth_with(host: &BtHost, limit: Duration) -> Result<BluetoothStart, String> {
    let out = host
        .launcher
        .command()
        .args(["systemctl", "start", "bluetooth.service"])
        .output()
        .await
        .map_err(|e| format!("couldn't ask for the admin password: {e}"))?;
    if !out.status.success() {
        return Err(super::util::fail_output(
            &out,
            "Starting the Bluetooth service",
        ));
    }
    let probe = host.wait_for_started(limit).await;
    let message = match probe.state {
        BtState::Ready { .. } => "Bluetooth service started.".to_string(),
        BtState::NoAdapter => {
            "The Bluetooth service started, but this computer has no Bluetooth adapter."
                .to_string()
        }
        BtState::ServiceOff if probe.service_active => {
            "The Bluetooth service started, but Bluetooth isn't answering yet. Check again in a moment."
                .to_string()
        }
        BtState::ServiceOff => {
            "The start command finished, but the Bluetooth service isn't running yet. Check again in a moment."
                .to_string()
        }
    };
    Ok(BluetoothStart {
        state: probe.state,
        message,
    })
}

#[derive(Serialize, Clone)]
pub struct BtDevice {
    address: String,
    name: String,
    connected: bool,
    battery: Option<u32>,
    /// BlueZ device class icon ("audio-headset", "input-gaming", ...), for a type icon.
    #[serde(default)]
    icon: String,
    /// False when the device only broadcasts its address: BlueZ then uses
    /// the address itself ("7D-70-3F-22-2D-62") as the name. These are
    /// mostly beacons and phones nearby, not something you'd pair.
    named: bool,
}

/// Whether `name` is a real name rather than BlueZ's address stand-in.
fn has_real_name(name: &str, address: &str) -> bool {
    let stand_in = name.replace('-', ":");
    !name.is_empty() && !stand_in.eq_ignore_ascii_case(address)
}

#[tauri::command]
pub async fn get_adapter_powered() -> Option<bool> {
    BtHost::system().adapter_powered().await
}

#[tauri::command]
pub async fn set_adapter_powered(on: bool) -> Result<(), String> {
    let host = BtHost::system();
    let val = if on { "on" } else { "off" };
    let out = btctl(&["power", val], QUICK).await?;
    if !out.status.success() || host.adapter_powered().await != Some(on) {
        return Err(super::util::fail_output(&out, "Changing Bluetooth power"));
    }
    Ok(())
}

fn parse_device_lines(text: &str) -> Vec<BtDevice> {
    text.lines()
        .filter_map(|l| {
            let rest = l.strip_prefix("Device ")?;
            let (addr, name) = rest.split_once(' ')?;
            let (addr, name) = (addr.trim(), name.trim());
            Some(BtDevice {
                address: addr.to_string(),
                name: name.to_string(),
                connected: false,
                battery: None,
                icon: String::new(),
                named: has_real_name(name, addr),
            })
        })
        .collect()
}

async fn run_devices(filter: Option<&str>) -> Vec<BtDevice> {
    let mut args = vec!["devices"];
    if let Some(f) = filter {
        args.push(f);
    }
    let Ok(out) = btctl(&args, QUICK).await else {
        return Vec::new();
    };
    parse_device_lines(&String::from_utf8_lossy(&out.stdout))
}

#[tauri::command]
pub async fn get_paired_devices() -> Vec<BtDevice> {
    let mut devices = run_devices(Some("Paired")).await;
    let connected: HashSet<String> = run_devices(Some("Connected"))
        .await
        .into_iter()
        .map(|d| d.address)
        .collect();
    for d in &mut devices {
        d.connected = connected.contains(&d.address);
        if let Ok(out) = btctl(&["info", &d.address], QUICK).await {
            let info = String::from_utf8_lossy(&out.stdout);
            d.icon = info_field(&info, "Icon:").unwrap_or_default();
            if d.connected {
                d.battery = info_field(&info, "Battery Percentage:")
                    .and_then(|v| {
                        v.split_once('(')
                            .map(|(_, v)| v.trim_end_matches(')').to_string())
                    })
                    .and_then(|v| v.parse::<u32>().ok())
                    .filter(|v| *v <= 100);
            }
        }
    }
    devices
}

fn info_field(info: &str, key: &str) -> Option<String> {
    info.lines()
        .find_map(|line| line.trim().strip_prefix(key))
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

/// Scans for a few seconds and returns every device BlueZ has seen that
/// isn't already paired.
#[tauri::command]
pub async fn scan_bluetooth() -> Vec<BtDevice> {
    let _ = btctl(&["--timeout", "5", "scan", "on"], SCAN).await;
    let paired: HashSet<String> = run_devices(Some("Paired"))
        .await
        .into_iter()
        .map(|d| d.address)
        .collect();
    let mut found: Vec<BtDevice> = run_devices(None)
        .await
        .into_iter()
        .filter(|d| !paired.contains(&d.address))
        .collect();
    // Type icons for the named ones only: there are usually a handful,
    // while unnamed beacons can number in the dozens.
    for d in found.iter_mut().filter(|d| d.named) {
        if let Ok(out) = btctl(&["info", &d.address], QUICK).await {
            d.icon = info_field(&String::from_utf8_lossy(&out.stdout), "Icon:").unwrap_or_default();
        }
    }
    found.sort_by_key(|d| !d.named);
    found
}

#[tauri::command]
pub async fn bt_connect(address: String) -> Result<(), String> {
    let out = btctl(&["connect", &address], PAIR).await?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stdout).trim().to_string())
    }
}

#[tauri::command]
pub async fn bt_disconnect(address: String) -> Result<(), String> {
    let out = btctl(&["disconnect", &address], QUICK).await?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stdout).trim().to_string())
    }
}

#[tauri::command]
pub async fn bt_forget(address: String) -> Result<(), String> {
    let out = btctl(&["remove", &address], QUICK).await?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stdout).trim().to_string())
    }
}

/// Pairs, then confirms it actually landed in the paired-device cache:
/// bluetoothctl's exit code alone isn't a reliable signal for pairing.
#[tauri::command]
pub async fn bt_pair(address: String) -> Result<(), String> {
    let out = btctl(&["pair", &address], PAIR).await?;
    let now_paired = run_devices(Some("Paired"))
        .await
        .iter()
        .any(|d| d.address == address);
    if now_paired {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stdout).trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;

    fn fake_script(dir: &Path, name: &str, body: &str) -> PathBuf {
        use std::io::Write;
        let path = dir.join(name);
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(format!("#!/bin/sh\n{body}\n").as_bytes())
            .unwrap();
        file.sync_all().unwrap();
        drop(file);
        let mut perms = std::fs::metadata(&path).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&path, perms).unwrap();
        path
    }

    fn fake_tree(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("bset92-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("sys/class/bluetooth")).unwrap();
        dir
    }

    fn add_adapter(root: &Path) {
        std::fs::create_dir_all(root.join("sys/class/bluetooth/hci0")).unwrap();
    }

    /// A host whose systemctl answers `active`, whose bluetoothctl prints
    /// `show`, and whose pkexec launcher always succeeds.
    fn fake_host(root: &Path, active: bool, show: &str) -> BtHost {
        let systemctl = fake_script(root, "systemctl", if active { "exit 0" } else { "exit 3" });
        let bluetoothctl = fake_script(root, "bluetoothctl", &format!("printf '%s\\n' '{show}'"));
        let launcher = fake_script(root, "pkexec", "exit 0");
        BtHost::with_programs(
            &systemctl,
            &bluetoothctl,
            &launcher,
            root.join("sys/class/bluetooth"),
        )
    }

    #[test]
    fn adapter_detected_from_sysfs() {
        let dir = std::env::temp_dir().join(format!("bset65-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        assert!(!has_adapter_in(&dir));
        std::fs::create_dir_all(dir.join("hci0")).unwrap();
        assert!(has_adapter_in(&dir));
        assert!(!has_adapter_in(&dir.join("missing")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn controller_show_needs_a_controller_block() {
        assert_eq!(parse_controller_show(""), None);
        assert_eq!(
            parse_controller_show("Failed to connect to bluetoothd"),
            None
        );
        assert_eq!(
            parse_controller_show("Controller AA:BB:CC:DD:EE:FF\n\tPowered: yes\n"),
            Some(true)
        );
        assert_eq!(
            parse_controller_show("Controller AA:BB:CC:DD:EE:FF\n\tPowered: no\n"),
            Some(false)
        );
    }

    #[test]
    fn state_off_with_adapter_is_service_off() {
        assert_eq!(classify(None, true), BtState::ServiceOff);
        assert_eq!(classify(None, false), BtState::NoAdapter);
        assert_eq!(classify(Some(true), true), BtState::Ready { powered: true });
        assert_eq!(
            classify(Some(false), false),
            BtState::Ready { powered: false }
        );
    }

    #[tokio::test]
    async fn service_off_with_adapter_is_service_off_not_no_adapter() {
        let root = fake_tree("off-adapter");
        add_adapter(&root);
        let host = fake_host(&root, false, "");
        let probe = host.probe().await;
        assert_eq!(probe.state, BtState::ServiceOff);
        assert!(!probe.service_active);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn service_off_without_adapter_is_no_adapter() {
        let root = fake_tree("off-none");
        let host = fake_host(&root, false, "");
        let probe = host.probe().await;
        assert_eq!(probe.state, BtState::NoAdapter);
        assert!(!probe.service_active);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn service_on_reports_adapter_power() {
        let root = fake_tree("on-powered");
        add_adapter(&root);
        let host = fake_host(&root, true, "Controller AA:BB:CC:DD:EE:FF\n\tPowered: yes");
        assert_eq!(host.probe().await.state, BtState::Ready { powered: true });

        let root2 = fake_tree("on-unpowered");
        add_adapter(&root2);
        let host2 = fake_host(&root2, true, "Controller AA:BB:CC:DD:EE:FF\n\tPowered: no");
        assert_eq!(host2.probe().await.state, BtState::Ready { powered: false });
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&root2);
    }

    #[tokio::test]
    async fn start_success_reprobes_until_bluez_answers() {
        let root = fake_tree("start-ok");
        add_adapter(&root);
        let marker = root.join("started");
        let systemctl = fake_script(
            &root,
            "systemctl",
            &format!("[ -f '{}' ] && exit 0 || exit 3", marker.to_string_lossy()),
        );
        let bluetoothctl = fake_script(
            &root,
            "bluetoothctl",
            &format!(
                "[ -f '{}' ] || exit 1\nprintf 'Controller AA:BB:CC:DD:EE:FF\\n\\tPowered: yes\\n'",
                marker.to_string_lossy()
            ),
        );
        let launcher = fake_script(
            &root,
            "pkexec",
            &format!(": > '{}'", marker.to_string_lossy()),
        );
        let host = BtHost::with_programs(
            &systemctl,
            &bluetoothctl,
            &launcher,
            root.join("sys/class/bluetooth"),
        );
        let res = start_bluetooth_with(&host, Duration::from_secs(2))
            .await
            .unwrap();
        assert_eq!(res.state, BtState::Ready { powered: true });
        assert!(
            res.message
                .to_lowercase()
                .contains("bluetooth service started"),
            "unexpected message: {}",
            res.message
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn start_failure_surfaces_the_error_instead_of_success() {
        let root = fake_tree("start-fail");
        add_adapter(&root);
        let systemctl = fake_script(&root, "systemctl", "exit 3");
        let bluetoothctl = fake_script(&root, "bluetoothctl", "exit 1");
        let launcher = fake_script(
            &root,
            "pkexec",
            "echo 'Error executing command as another user: Request dismissed' >&2\nexit 126",
        );
        let host = BtHost::with_programs(
            &systemctl,
            &bluetoothctl,
            &launcher,
            root.join("sys/class/bluetooth"),
        );
        let err = start_bluetooth_with(&host, Duration::from_millis(100))
            .await
            .unwrap_err();
        assert!(err.contains("Request dismissed"), "unexpected error: {err}");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn start_without_a_bluetoothd_answer_says_so() {
        let root = fake_tree("start-silent");
        add_adapter(&root);
        let systemctl = fake_script(&root, "systemctl", "exit 0");
        let bluetoothctl = fake_script(&root, "bluetoothctl", "exit 1");
        let launcher = fake_script(&root, "pkexec", "exit 0");
        let host = BtHost::with_programs(
            &systemctl,
            &bluetoothctl,
            &launcher,
            root.join("sys/class/bluetooth"),
        );
        let res = start_bluetooth_with(&host, Duration::from_millis(100))
            .await
            .unwrap();
        assert_eq!(res.state, BtState::ServiceOff);
        assert!(
            res.message.contains("isn't answering"),
            "unexpected message: {}",
            res.message
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn hung_command_times_out_and_is_killed() {
        let start = std::time::Instant::now();
        let r = bounded("sleep", &["30"], Duration::from_millis(200)).await;
        assert!(r.is_err());
        assert!(start.elapsed() < Duration::from_secs(5));
    }

    #[tokio::test]
    async fn quick_command_returns_output() {
        let out = bounded("echo", &["Controller"], Duration::from_secs(5))
            .await
            .unwrap();
        assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "Controller");
    }

    #[test]
    fn info_fields_parse() {
        let info =
            "Device AA\n\tName: Buds\n\tIcon: audio-headset\n\tBattery Percentage: 0x50 (80)\n";
        assert_eq!(info_field(info, "Icon:").as_deref(), Some("audio-headset"));
        assert_eq!(
            info_field(info, "Battery Percentage:").as_deref(),
            Some("0x50 (80)")
        );
        assert_eq!(info_field(info, "Alias:"), None);
    }

    #[test]
    fn address_stand_in_is_not_a_name() {
        let devs = parse_device_lines(
            "Device 7D:70:3F:22:2D:62 7D-70-3F-22-2D-62\nDevice E8:C9:13:88:AE:3F Galaxy Buds3\n",
        );
        assert!(!devs[0].named);
        assert!(devs[1].named);
        assert!(!has_real_name("7d-70-3f-22-2d-62", "7D:70:3F:22:2D:62"));
    }
}
