//! Read-only system info, plus the one thing worth making writable: hostname.
//! BOS is a rolling release (no fixed version number to show; `os-release`
//! ships `BUILD_ID=rolling` on purpose), so there's no "BOS 1.2.3" readout
//! here the way a point-release distro's About panel would have one.

use serde::Serialize;
use std::fs;
use tokio::process::Command;

#[derive(Serialize)]
pub struct SystemInfo {
    os: String,
    version: Option<String>,
    kernel: String,
    cpu: String,
    gpu: String,
    memory: String,
    disk: String,
    uptime: String,
    hostname: String,
}

fn os_pretty_name() -> String {
    fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|s| {
            s.lines().find_map(|l| {
                l.strip_prefix("PRETTY_NAME=")
                    .map(|v| v.trim_matches('"').to_string())
            })
        })
        .unwrap_or_else(|| "BOS".to_string())
}

pub(crate) fn hostname() -> String {
    fs::read_to_string("/etc/hostname")
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "unknown".to_string())
}

async fn kernel() -> String {
    Command::new("uname")
        .arg("-r")
        .output()
        .await
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

fn cpu() -> String {
    let model = fs::read_to_string("/proc/cpuinfo")
        .ok()
        .and_then(|s| {
            s.lines().find_map(|l| {
                l.strip_prefix("model name")
                    .map(|v| v.trim_start_matches([':', ' ', '\t']).to_string())
            })
        })
        .unwrap_or_else(|| "unknown".to_string());
    let cores = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(0);
    if cores > 0 {
        format!("{model} ({cores} threads)")
    } else {
        model
    }
}

fn memory() -> String {
    let kb = fs::read_to_string("/proc/meminfo").ok().and_then(|s| {
        s.lines()
            .find(|l| l.starts_with("MemTotal:"))
            .and_then(|l| l.split_whitespace().nth(1))
            .and_then(|v| v.parse::<u64>().ok())
    });
    match kb {
        Some(kb) => format!("{:.1} GiB", kb as f64 / 1024.0 / 1024.0),
        None => "unknown".to_string(),
    }
}

async fn gpu() -> String {
    let Ok(output) = Command::new("lspci").output().await else {
        return "unknown".to_string();
    };
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines()
        // "Display controller" covers integrated GPUs some laptop chipsets
        // (this dev laptop's AMD Radeon 860M included) report under instead
        // of "VGA compatible controller"; without it those show "unknown".
        .find(|l| {
            l.contains("VGA compatible controller")
                || l.contains("3D controller")
                || l.contains("Display controller")
        })
        .and_then(|l| l.split(": ").nth(1))
        .unwrap_or("unknown")
        .to_string()
}

async fn disk_usage() -> String {
    let Ok(output) = Command::new("df")
        .args(["-h", "--output=used,size,pcent", "/"])
        .output()
        .await
    else {
        return "unknown".to_string();
    };
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines()
        .nth(1)
        .map(|l| {
            let cols: Vec<&str> = l.split_whitespace().collect();
            match cols.as_slice() {
                [used, size, pcent] => format!("{used} of {size} used ({pcent})"),
                _ => l.trim().to_string(),
            }
        })
        .unwrap_or_else(|| "unknown".to_string())
}

async fn uptime() -> String {
    Command::new("uptime")
        .arg("-p")
        .output()
        .await
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

#[tauri::command]
pub async fn get_system_info() -> SystemInfo {
    SystemInfo {
        os: os_pretty_name(),
        version: fs::read_to_string("/etc/os-release").ok().and_then(|s| {
            s.lines().find_map(|line| {
                line.strip_prefix("VERSION_ID=")
                    .map(|v| v.trim_matches('"').to_string())
            })
        }),
        kernel: kernel().await,
        cpu: cpu(),
        gpu: gpu().await,
        memory: memory(),
        disk: disk_usage().await,
        uptime: uptime().await,
        hostname: hostname(),
    }
}

/// RFC 1123 labels (digit start allowed), no leading `-`. Linux static
/// hostnames are also capped at `HOST_NAME_MAX` (64).
fn valid_hostname(name: &str) -> bool {
    let name = name.trim();
    if name.is_empty() || name.len() > 64 || name.starts_with('-') {
        return false;
    }
    if name.contains('\n') || name.contains('\r') || name.contains('\0') {
        return false;
    }
    name.split('.').all(valid_dns_label)
}

fn valid_dns_label(label: &str) -> bool {
    let b = label.as_bytes();
    if b.is_empty() || b.len() > 63 {
        return false;
    }
    if !b[0].is_ascii_alphanumeric() || !b[b.len() - 1].is_ascii_alphanumeric() {
        return false;
    }
    b.iter().all(|c| c.is_ascii_alphanumeric() || *c == b'-')
}

#[tauri::command]
pub async fn set_hostname(name: String) -> Result<(), String> {
    let name = name.trim();
    if !valid_hostname(name) {
        return Err("invalid hostname".into());
    }
    let output = Command::new("pkexec")
        .args(["hostnamectl", "set-hostname", name])
        .output()
        .await
        .map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hostname_rfc1123() {
        assert!(valid_hostname("bos"));
        assert!(valid_hostname("bos.local"));
        assert!(valid_hostname("a1-b"));
        assert!(valid_hostname("1host"));
        assert!(!valid_hostname(""));
        assert!(!valid_hostname("-bos"));
        assert!(!valid_hostname("bos-"));
        assert!(!valid_hostname("-foo.bar"));
        assert!(!valid_hostname("foo_bar"));
        assert!(!valid_hostname("bos\n-set-hostname evil"));
        assert!(!valid_hostname("--help"));
        assert!(!valid_hostname(&"a".repeat(65)));
    }

    fn raw(ok: bool, stdout: &str) -> RawOutput {
        RawOutput {
            ok,
            stdout: stdout.to_string(),
            stderr: String::new(),
            error: None,
        }
    }

    fn unit_block(
        id: &str,
        load: &str,
        active: &str,
        sub: &str,
        restarts: u32,
        file_state: &str,
    ) -> String {
        format!(
            "Id={id}\nLoadState={load}\nActiveState={active}\nSubState={sub}\n\
             NRestarts={restarts}\nUnitFileState={file_state}\nResult=success\n\n"
        )
    }

    fn healthy_units() -> String {
        [
            unit_block(
                "breadd.service",
                "loaded",
                "active",
                "running",
                0,
                "enabled",
            ),
            unit_block(
                "breadcrumbs.service",
                "loaded",
                "active",
                "running",
                0,
                "enabled",
            ),
            unit_block(
                "breadclipd.service",
                "loaded",
                "active",
                "running",
                0,
                "enabled",
            ),
            unit_block(
                "breadmill.service",
                "loaded",
                "active",
                "running",
                0,
                "enabled",
            ),
            unit_block(
                "breadbox-sync.service",
                "loaded",
                "active",
                "running",
                0,
                "enabled",
            ),
        ]
        .concat()
    }

    fn healthy_doctor() -> RawOutput {
        raw(
            true,
            "bread doctor\n  daemon     ✓ running (pid 42)\n  version    0.9.0\n\n\
             adapters\n  udev                 connected\n  hyprland             connected\n  podman               connected\n",
        )
    }

    #[test]
    fn healthy_system_reports_ok() {
        let doctor = doctor_check(&healthy_doctor());
        let services = services_check(&raw(true, &healthy_units()), &raw(true, ""));
        let packages = packages_check(&raw(true, "all good\n"));

        assert_eq!(doctor.status, HealthState::Ok);
        assert_eq!(services.status, HealthState::Ok);
        assert_eq!(packages.status, HealthState::Ok);
        assert!(doctor.ok && services.ok && packages.ok);
    }

    #[test]
    fn one_failed_unit_is_not_ok() {
        let units = healthy_units().replace(
            "Id=breadcrumbs.service\nLoadState=loaded\nActiveState=active\nSubState=running\nNRestarts=0",
            "Id=breadcrumbs.service\nLoadState=loaded\nActiveState=failed\nSubState=failed\nNRestarts=0",
        );
        let check = services_check(&raw(true, &units), &raw(true, ""));
        assert_ne!(check.status, HealthState::Ok);
        assert_eq!(check.status, HealthState::Failed);
        assert!(check.summary.contains("breadcrumbs.service"));
    }

    #[test]
    fn restart_loop_is_not_ok_even_though_systemctl_exited_zero() {
        let units = healthy_units().replace(
            "Id=breadcrumbs.service\nLoadState=loaded\nActiveState=active\nSubState=running\nNRestarts=0",
            "Id=breadcrumbs.service\nLoadState=loaded\nActiveState=activating\nSubState=auto-restart\nNRestarts=185",
        );
        let systemctl = raw(true, &units);
        let check = services_check(&systemctl, &raw(true, ""));

        // This is exactly the old bug: the command exited 0, so the old check
        // printed OK while the watcher was restart-looping.
        assert!(systemctl.ok, "fixture must model a successful systemctl");
        assert_ne!(check.status, HealthState::Ok);
        assert_eq!(check.status, HealthState::Failed);
        assert!(check.summary.contains("restart-looping"));
        assert!(check.summary.contains("185"));
    }

    #[test]
    fn missing_required_watcher_is_not_ok() {
        let units = healthy_units().replace(
            "Id=breadcrumbs.service\nLoadState=loaded\nActiveState=active\nSubState=running\nNRestarts=0\nUnitFileState=enabled\nResult=success\n\n",
            "",
        );
        let check = services_check(&raw(true, &units), &raw(true, ""));
        assert_eq!(check.status, HealthState::Failed);
        assert!(check.summary.contains("breadcrumbs.service is missing"));
    }

    #[test]
    fn extra_failed_bread_unit_is_reported() {
        let failed = "breadmon.service loaded failed failed Bread monitor\n";
        let check = services_check(&raw(true, &healthy_units()), &raw(true, failed));
        assert_eq!(check.status, HealthState::Failed);
        assert!(check.summary.contains("breadmon.service failed"));
    }

    #[test]
    fn daemon_down_is_not_ok_even_though_bread_doctor_exited_zero() {
        let doctor = raw(
            true,
            "bread doctor\n  daemon     ✗ not running\n  socket     /run/user/1000/bread/breadd.sock  (not found)\n",
        );
        let check = doctor_check(&doctor);
        assert!(doctor.ok, "fixture must model a successful bread doctor");
        assert_eq!(check.status, HealthState::Failed);
        assert!(check.summary.contains("not running"));
    }

    #[test]
    fn disconnected_adapter_is_degraded() {
        let doctor = raw(
            true,
            "bread doctor\n  daemon     ✓ running (pid 42)\n\n\
             adapters\n  udev                 connected\n  podman               disconnected\n",
        );
        let check = doctor_check(&doctor);
        assert_eq!(check.status, HealthState::Degraded);
        assert!(!check.ok);
        assert!(check.summary.contains("podman adapter is disconnected"));
    }

    #[test]
    fn parse_doctor_reads_daemon_and_adapters() {
        let report = parse_doctor(&healthy_doctor().stdout);
        assert_eq!(report.daemon_running, Some(true));
        assert_eq!(
            report.adapters,
            vec![
                ("udev".to_string(), "connected".to_string()),
                ("hyprland".to_string(), "connected".to_string()),
                ("podman".to_string(), "connected".to_string()),
            ]
        );
    }

    #[test]
    fn parse_units_handles_multiple_blocks_and_restarts() {
        let text = format!(
            "{}{}",
            unit_block(
                "breadd.service",
                "loaded",
                "active",
                "running",
                2,
                "enabled"
            ),
            unit_block(
                "breadcrumbs.service",
                "loaded",
                "active",
                "running",
                185,
                "enabled"
            )
        );
        let units = parse_unit_show(&text);
        assert_eq!(units.len(), 2);
        assert_eq!(units[0].id, "breadd.service");
        assert_eq!(units[0].restarts, 2);
        assert_eq!(units[1].id, "breadcrumbs.service");
        assert_eq!(units[1].restarts, 185);
    }

    #[test]
    fn output_is_capped_instead_of_flooding() {
        let flood = (0..700)
            .map(|index| format!("warning line {index}"))
            .collect::<Vec<_>>()
            .join("\n");
        let capped = capped_text(&flood);
        assert_eq!(capped.lines().count(), MAX_HEALTH_LINES);
    }
}

/// Coarse state for one line of the About health check. The old check only
/// looked at an exit status, so a healthy command covered a failed unit or a
/// disconnected adapter. `ok` is kept for the existing frontend contract, but
/// the label distinguishes a degraded result (still running, needs attention)
/// from a hard failure.
#[derive(Serialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum HealthState {
    Ok,
    Degraded,
    Failed,
}

#[derive(Serialize)]
pub struct HealthCheck {
    name: String,
    status: HealthState,
    /// Backwards-compatible: true only when `status` is `Ok`.
    ok: bool,
    summary: String,
    output: String,
}

impl HealthCheck {
    fn new(
        name: &str,
        status: HealthState,
        summary: impl Into<String>,
        output: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            status,
            ok: status == HealthState::Ok,
            summary: summary.into(),
            output: output.into(),
        }
    }
}

struct ExpectedUnit {
    unit: &'static str,
    required: bool,
}

/// The units the service panels already expose. `breadd` is the runtime and
/// `breadcrumbs` is the Wi-Fi recovery watcher; both are required for a BOS
/// desktop. The rest belong to optional apps, so "not installed" is only a
/// note for them.
const EXPECTED_UNITS: &[ExpectedUnit] = &[
    ExpectedUnit {
        unit: "breadd.service",
        required: true,
    },
    ExpectedUnit {
        unit: "breadcrumbs.service",
        required: true,
    },
    ExpectedUnit {
        unit: "breadclipd.service",
        required: false,
    },
    ExpectedUnit {
        unit: "breadmill.service",
        required: false,
    },
    ExpectedUnit {
        unit: "breadbox-sync.service",
        required: false,
    },
];

/// Any unit that has already restarted this many times is crash-looping, not
/// merely recovering from one transient failure.
const RESTART_FAILED: u32 = 5;
const MAX_HEALTH_LINES: usize = 20;
const MAX_HEALTH_LINE_CHARS: usize = 300;

struct RawOutput {
    ok: bool,
    stdout: String,
    stderr: String,
    error: Option<String>,
}

async fn run_output(program: &str, args: &[&str]) -> RawOutput {
    match Command::new(program).args(args).output().await {
        Ok(out) => RawOutput {
            ok: out.status.success(),
            stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
            error: None,
        },
        Err(e) => RawOutput {
            ok: false,
            stdout: String::new(),
            stderr: String::new(),
            error: Some(e.to_string()),
        },
    }
}

#[tauri::command]
pub async fn run_health_checks() -> Vec<HealthCheck> {
    let doctor = run_output("bread", &["doctor"]).await;

    let mut show_args = vec![
        "--user",
        "show",
        "--property=Id",
        "--property=LoadState",
        "--property=ActiveState",
        "--property=SubState",
        "--property=NRestarts",
        "--property=UnitFileState",
        "--property=Result",
    ];
    show_args.extend(EXPECTED_UNITS.iter().map(|unit| unit.unit));
    let units = run_output("systemctl", &show_args).await;
    let failed_units = run_output(
        "systemctl",
        &["--user", "list-units", "--failed", "--no-legend", "--plain"],
    )
    .await;

    let packages = run_output("bakery", &["doctor"]).await;

    vec![
        doctor_check(&doctor),
        services_check(&units, &failed_units),
        packages_check(&packages),
    ]
}

#[derive(Default, Debug)]
struct DoctorReport {
    daemon_running: Option<bool>,
    adapters: Vec<(String, String)>,
}

fn parse_doctor(text: &str) -> DoctorReport {
    let mut report = DoctorReport::default();
    let mut in_adapters = false;
    for raw in text.lines() {
        let line = raw.trim_end();
        let trimmed = line.trim();
        if trimmed.eq_ignore_ascii_case("adapters") {
            in_adapters = true;
            continue;
        }
        if in_adapters {
            if trimmed.is_empty() {
                continue;
            }
            // A section heading is unindented; adapter rows are indented.
            if !line.starts_with(' ') && !line.starts_with('\t') {
                in_adapters = false;
            } else {
                let cols: Vec<&str> = trimmed.split_whitespace().collect();
                if cols.len() >= 2 {
                    let status = cols[cols.len() - 1];
                    if status == "connected" || status == "disconnected" {
                        report
                            .adapters
                            .push((cols[0].to_string(), status.to_string()));
                    }
                }
                continue;
            }
        }
        if let Some(rest) = trimmed.strip_prefix("daemon") {
            let rest = rest.trim();
            if rest.starts_with('✓') {
                report.daemon_running = Some(true);
            } else if rest.starts_with('✗') {
                report.daemon_running = Some(false);
            }
        }
    }
    report
}

fn doctor_check(out: &RawOutput) -> HealthCheck {
    const NAME: &str = "Bread daemon and desktop";
    if let Some(error) = &out.error {
        return HealthCheck::new(
            NAME,
            HealthState::Failed,
            format!("bread doctor could not run: {error}"),
            error.clone(),
        );
    }

    let report = parse_doctor(&format!("{}\n{}", out.stdout, out.stderr));
    let mut problems = Vec::new();
    let mut warnings = Vec::new();
    let mut lines = Vec::new();

    match report.daemon_running {
        Some(true) => lines.push("daemon: running".to_string()),
        Some(false) => {
            problems.push("the bread daemon is not running".to_string());
            lines.push("daemon: not running".to_string());
        }
        None => {
            warnings.push("bread doctor did not report daemon status".to_string());
            lines.push("daemon: status unavailable".to_string());
        }
    }

    for (adapter, status) in &report.adapters {
        lines.push(format!("adapter {adapter}: {status}"));
        if status == "disconnected" {
            warnings.push(format!("the {adapter} adapter is disconnected"));
        }
    }
    if report.daemon_running == Some(true) && report.adapters.is_empty() {
        warnings.push("bread doctor did not report any adapters".to_string());
    }
    if !out.ok && report.daemon_running != Some(false) {
        warnings.push("bread doctor exited with an error".to_string());
    }

    let status = health_status(&problems, &warnings);
    let summary = health_summary(
        status,
        "Bread daemon is running and every adapter is connected.",
        &problems,
        &warnings,
    );
    HealthCheck::new(NAME, status, summary, capped_lines(lines))
}

#[derive(Default, Debug, Clone)]
struct UnitState {
    id: String,
    load: String,
    active: String,
    sub: String,
    restarts: u32,
    file_state: String,
}

fn parse_unit_show(text: &str) -> Vec<UnitState> {
    let mut units = Vec::new();
    let mut current = UnitState::default();
    for raw in text.lines() {
        let line = raw.trim_end();
        if line.is_empty() {
            if !current.id.is_empty() {
                units.push(std::mem::take(&mut current));
            }
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            match key {
                "Id" => current.id = value.to_string(),
                "LoadState" => current.load = value.to_string(),
                "ActiveState" => current.active = value.to_string(),
                "SubState" => current.sub = value.to_string(),
                "NRestarts" => current.restarts = value.parse().unwrap_or(0),
                "UnitFileState" => current.file_state = value.to_string(),
                _ => {}
            }
        }
    }
    if !current.id.is_empty() {
        units.push(current);
    }
    units
}

fn failed_bread_units(text: &str) -> Vec<String> {
    let mut units: Vec<String> = text
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        .filter(|unit| unit.starts_with("bread") && unit.ends_with(".service"))
        .map(|unit| unit.to_string())
        .collect();
    units.sort();
    units.dedup();
    units
}

fn services_check(units: &RawOutput, failed: &RawOutput) -> HealthCheck {
    const NAME: &str = "BOS services";
    if let Some(error) = &units.error {
        return HealthCheck::new(
            NAME,
            HealthState::Failed,
            format!("could not read systemd user units: {error}"),
            error.clone(),
        );
    }
    if !units.ok && units.stdout.trim().is_empty() {
        return HealthCheck::new(
            NAME,
            HealthState::Failed,
            "could not read systemd user units",
            capped_text(&format!("{}\n{}", units.stdout, units.stderr)),
        );
    }

    let states = parse_unit_show(&units.stdout);
    let mut problems = Vec::new();
    let mut warnings = Vec::new();
    let mut lines = Vec::new();

    for expected in EXPECTED_UNITS {
        let Some(state) = states.iter().find(|state| state.id == expected.unit) else {
            lines.push(format!("{}: not installed", expected.unit));
            if expected.required {
                problems.push(format!("{} is missing", expected.unit));
            }
            continue;
        };

        lines.push(format!(
            "{}: {}/{} restarts {} ({})",
            state.id, state.active, state.sub, state.restarts, state.file_state
        ));

        if state.load == "not-found" {
            if expected.required {
                problems.push(format!("{} is missing", state.id));
            }
        } else if state.active == "failed" {
            problems.push(format!(
                "{} failed{}",
                state.id,
                restart_note(state.restarts)
            ));
        } else if state.sub == "auto-restart" {
            problems.push(format!(
                "{} is restart-looping{}",
                state.id,
                restart_note(state.restarts)
            ));
        } else if state.restarts >= RESTART_FAILED {
            problems.push(format!(
                "{} has restarted {} times",
                state.id, state.restarts
            ));
        } else if state.restarts > 0 {
            warnings.push(format!(
                "{} has restarted {} time(s)",
                state.id, state.restarts
            ));
        } else if state.active != "active" && state.active != "activating" {
            warnings.push(format!("{} is {}", state.id, state.active));
        }
    }

    for unit in failed_bread_units(&failed.stdout) {
        if !EXPECTED_UNITS.iter().any(|expected| expected.unit == unit) {
            problems.push(format!("{unit} failed"));
            lines.push(format!("{unit}: failed"));
        }
    }

    let status = health_status(&problems, &warnings);
    let summary = health_summary(
        status,
        "All expected BOS services are healthy.",
        &problems,
        &warnings,
    );
    HealthCheck::new(NAME, status, summary, capped_lines(lines))
}

fn packages_check(out: &RawOutput) -> HealthCheck {
    const NAME: &str = "Packages";
    if let Some(error) = &out.error {
        return HealthCheck::new(
            NAME,
            HealthState::Failed,
            format!("bakery doctor could not run: {error}"),
            error.clone(),
        );
    }
    if out.ok {
        return HealthCheck::new(
            NAME,
            HealthState::Ok,
            "Package tools report healthy.",
            capped_text(&out.stdout),
        );
    }
    HealthCheck::new(
        NAME,
        HealthState::Failed,
        "bakery doctor reported a problem.",
        capped_text(&format!("{}\n{}", out.stdout, out.stderr)),
    )
}

fn restart_note(restarts: u32) -> String {
    if restarts == 0 {
        String::new()
    } else if restarts == 1 {
        ", 1 restart".to_string()
    } else {
        format!(", {restarts} restarts")
    }
}

fn health_status(problems: &[String], warnings: &[String]) -> HealthState {
    if !problems.is_empty() {
        HealthState::Failed
    } else if !warnings.is_empty() {
        HealthState::Degraded
    } else {
        HealthState::Ok
    }
}

fn health_summary(
    status: HealthState,
    healthy: &str,
    problems: &[String],
    warnings: &[String],
) -> String {
    if status == HealthState::Ok {
        healthy.to_string()
    } else {
        problems
            .iter()
            .chain(warnings.iter())
            .cloned()
            .collect::<Vec<_>>()
            .join("; ")
    }
}

/// Keep details useful without dumping a journal-sized log into the About
/// panel. Each line is also bounded so one long exec line cannot flood it.
fn capped_lines(lines: impl IntoIterator<Item = String>) -> String {
    let mut out = String::new();
    for (index, line) in lines.into_iter().take(MAX_HEALTH_LINES).enumerate() {
        if index > 0 {
            out.push('\n');
        }
        let mut line = line.trim_end().to_string();
        if line.chars().count() > MAX_HEALTH_LINE_CHARS {
            line = line.chars().take(MAX_HEALTH_LINE_CHARS).collect();
            line.push_str("...");
        }
        out.push_str(&line);
    }
    out
}

fn capped_text(text: &str) -> String {
    capped_lines(text.lines().map(|line| line.to_string()))
}
