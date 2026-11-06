//! CUPS printers via lpstat / lpadmin. Adding a printer through the full
//! device wizard is `system-config-printer`; a simple IPP Everywhere queue
//! can be created here when the user has a URI.

use std::time::Duration;

use serde::Serialize;
use tokio::process::Command;
use tokio::time::timeout;

use super::util::{fail_output, valid_printer_name};

/// lpstat exits non-zero with this message when CUPS is healthy but no
/// destinations are configured. That is an empty list, not a broken service.
const NO_DESTINATIONS: &str = "no destinations added";
const LPSTAT_TIMEOUT: Duration = Duration::from_secs(5);
const LPSTAT_RETRY_DELAY: Duration = Duration::from_millis(300);

#[derive(Debug, Serialize, Clone)]
pub struct Printer {
    name: String,
    status: String,
    enabled: bool,
    is_default: bool,
}

#[derive(Debug, Serialize)]
pub struct PrintingStatus {
    printers: Vec<Printer>,
    default: Option<String>,
    cups_ok: bool,
    error: Option<String>,
}

impl PrintingStatus {
    fn empty_ok() -> Self {
        Self {
            printers: Vec::new(),
            default: None,
            cups_ok: true,
            error: None,
        }
    }

    fn failure(message: impl Into<String>) -> Self {
        Self {
            printers: Vec::new(),
            default: None,
            cups_ok: false,
            error: Some(message.into()),
        }
    }
}

#[derive(Debug)]
struct LpstatFailure {
    message: String,
    /// True when the first call probably raced socket activation, so one
    /// retry is worth making before calling CUPS unreachable.
    retryable: bool,
}

impl LpstatFailure {
    fn new(message: impl Into<String>, retryable: bool) -> Self {
        Self {
            message: message.into(),
            retryable,
        }
    }
}

#[tauri::command]
pub async fn get_printers() -> PrintingStatus {
    collect_printers(run_lpstat).await
}

/// Run one lpstat attempt, retrying once only for failures that look like
/// the cups.service socket-activation race. Everything else is reported
/// immediately; a real outage must not turn into an endless loop.
async fn collect_printers<F, Fut>(mut attempt: F) -> PrintingStatus
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<PrintingStatus, LpstatFailure>>,
{
    match attempt().await {
        Ok(status) => status,
        Err(first) if first.retryable => {
            tokio::time::sleep(LPSTAT_RETRY_DELAY).await;
            match attempt().await {
                Ok(status) => status,
                Err(second) => PrintingStatus::failure(second.message),
            }
        }
        Err(first) => PrintingStatus::failure(first.message),
    }
}

async fn run_lpstat() -> Result<PrintingStatus, LpstatFailure> {
    run_lpstat_program("lpstat").await
}

async fn run_lpstat_program(program: &str) -> Result<PrintingStatus, LpstatFailure> {
    let mut command = Command::new(program);
    command.args(["-p", "-d"]).kill_on_drop(true);
    let output = match timeout(LPSTAT_TIMEOUT, command.output()).await {
        Ok(Ok(output)) => output,
        Ok(Err(e)) => {
            return Err(LpstatFailure::new(
                format!("couldn't run {program}: {e}"),
                false,
            ));
        }
        Err(_) => {
            return Err(LpstatFailure::new(
                format!(
                    "{program} didn't answer within {} seconds",
                    LPSTAT_TIMEOUT.as_secs()
                ),
                true,
            ));
        }
    };
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    classify_lpstat(output.status.success(), &stdout, &stderr)
}

/// Map one lpstat result to either a usable printer list or a failure.
/// `success` is the process exit status; fake outputs make this testable.
fn classify_lpstat(
    success: bool,
    stdout: &str,
    stderr: &str,
) -> Result<PrintingStatus, LpstatFailure> {
    if success {
        return Ok(parse_lpstat(stdout));
    }
    if is_no_destinations(stdout, stderr) {
        return Ok(PrintingStatus::empty_ok());
    }
    let message = output_message(stdout, stderr).unwrap_or_else(|| "lpstat failed".to_string());
    Err(LpstatFailure::new(
        message,
        is_activation_failure(stdout, stderr),
    ))
}

fn is_no_destinations(stdout: &str, stderr: &str) -> bool {
    [stdout, stderr]
        .iter()
        .any(|text| text.to_ascii_lowercase().contains(NO_DESTINATIONS))
}

fn is_activation_failure(stdout: &str, stderr: &str) -> bool {
    let text = format!("{stdout}\n{stderr}").to_ascii_lowercase();
    [
        "connection refused",
        "unable to connect",
        "cannot connect",
        "connection reset",
        "scheduler is not running",
        "temporarily unavailable",
    ]
    .iter()
    .any(|needle| text.contains(needle))
}

fn output_message(stdout: &str, stderr: &str) -> Option<String> {
    for text in [stderr, stdout] {
        let message = text.trim();
        if !message.is_empty() {
            return Some(message.to_string());
        }
    }
    None
}

fn parse_lpstat(text: &str) -> PrintingStatus {
    let mut printers = Vec::new();
    let mut default = None;
    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("printer ") {
            let mut parts = rest.splitn(2, ' ');
            let name = parts.next().unwrap_or("").to_string();
            let rest = parts.next().unwrap_or("");
            if name.is_empty() {
                continue;
            }
            let enabled = !rest.contains("disabled");
            let status = rest
                .strip_prefix("is ")
                .unwrap_or(rest)
                .split(".  ")
                .next()
                .unwrap_or(rest)
                .trim()
                .to_string();
            printers.push(Printer {
                name,
                status,
                enabled,
                is_default: false,
            });
        } else if let Some(name) = line.strip_prefix("system default destination: ") {
            default = Some(name.trim().to_string());
        } else if line == "no system default destination" {
            default = None;
        }
    }
    if let Some(def) = default.as_deref() {
        for p in &mut printers {
            p.is_default = p.name == def;
        }
    }
    PrintingStatus {
        printers,
        default,
        cups_ok: true,
        error: None,
    }
}

#[tauri::command]
pub async fn set_default_printer(name: String) -> Result<(), String> {
    if !valid_printer_name(&name) {
        return Err(format!("invalid printer name '{name}'"));
    }
    let output = Command::new("lpadmin")
        .args(["-d", &name])
        .output()
        .await
        .map_err(|e| e.to_string())?;
    if output.status.success() {
        return Ok(());
    }
    let output = Command::new("pkexec")
        .args(["lpadmin", "-d", &name])
        .output()
        .await
        .map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(fail_output(&output, "lpadmin"))
    }
}

#[tauri::command]
pub async fn add_ipp_printer(name: String, uri: String) -> Result<(), String> {
    if !valid_printer_name(&name) {
        return Err(format!("invalid printer name '{name}'"));
    }
    if !valid_printer_uri(&uri) {
        return Err("URI must be ipp://, ipps://, socket://, usb://, or dnssd://".into());
    }
    let args_owned = [
        "-p".into(),
        name.clone(),
        "-E".into(),
        "-v".into(),
        uri,
        "-m".into(),
        "everywhere".into(),
    ];
    let output = Command::new("lpadmin")
        .args(&args_owned)
        .output()
        .await
        .map_err(|e| e.to_string())?;
    if output.status.success() {
        return Ok(());
    }
    let mut pk = vec!["lpadmin".to_string()];
    pk.extend(args_owned);
    let output = Command::new("pkexec")
        .args(&pk)
        .output()
        .await
        .map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(fail_output(&output, "lpadmin"))
    }
}

fn valid_printer_uri(uri: &str) -> bool {
    let u = uri.trim();
    !u.is_empty()
        && u.len() <= 512
        && !u.contains(char::is_whitespace)
        && (u.starts_with("ipp://")
            || u.starts_with("ipps://")
            || u.starts_with("socket://")
            || u.starts_with("usb://")
            || u.starts_with("dnssd://"))
}

#[tauri::command]
pub fn open_printer_settings() {
    let _ = std::process::Command::new("system-config-printer").spawn();
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    #[test]
    fn lpstat_parses_idle_and_default() {
        let text = "\
printer Canon-TS6360a is idle.  enabled since Mon 10 Aug 2026
printer Hall is disabled since yesterday
system default destination: Canon-TS6360a
";
        let st = parse_lpstat(text);
        assert_eq!(st.printers.len(), 2);
        assert!(st.printers[0].is_default);
        assert!(st.printers[0].enabled);
        assert!(!st.printers[1].enabled);
        assert_eq!(st.default.as_deref(), Some("Canon-TS6360a"));
    }

    #[test]
    fn uri_schemes() {
        assert!(valid_printer_uri("ipp://192.168.1.5/ipp/print"));
        assert!(valid_printer_uri("ipps://printer.local/ipp"));
        assert!(!valid_printer_uri("http://evil"));
        assert!(!valid_printer_uri("ipp://x y"));
    }

    #[test]
    fn no_destinations_on_stderr_is_an_empty_success() {
        let st = classify_lpstat(false, "", "lpstat: No destinations added.\n")
            .expect("no destinations is a healthy empty list");
        assert!(st.cups_ok);
        assert!(st.printers.is_empty());
        assert!(st.default.is_none());
        assert!(st.error.is_none());
    }

    #[test]
    fn no_destinations_on_stdout_is_an_empty_success() {
        let st = classify_lpstat(false, "No destinations added.\n", "")
            .expect("no destinations is a healthy empty list");
        assert!(st.cups_ok);
        assert!(st.printers.is_empty());
        assert!(st.error.is_none());
    }

    #[test]
    fn normal_printer_list_parses() {
        let stdout = "\
printer Office is idle.  enabled since Mon 10 Aug 2026
system default destination: Office
";
        let st = classify_lpstat(true, stdout, "").expect("a normal list is ok");
        assert!(st.cups_ok);
        assert_eq!(st.printers.len(), 1);
        assert_eq!(st.printers[0].name, "Office");
        assert!(st.printers[0].is_default);
        assert!(st.error.is_none());
    }

    #[test]
    fn connection_refused_is_a_real_retryable_failure() {
        let failure = classify_lpstat(
            false,
            "",
            "lpstat: Unable to connect to server: Connection refused\n",
        )
        .expect_err("a refused socket is not an empty list");
        assert!(failure.retryable);
        assert!(failure.message.contains("Connection refused"));
    }

    #[test]
    fn unexpected_output_is_a_real_non_retryable_failure() {
        let failure = classify_lpstat(false, "", "lpstat: Invalid destination name\n")
            .expect_err("unexpected output must not become an empty list");
        assert!(!failure.retryable);
        assert_eq!(failure.message, "lpstat: Invalid destination name");
    }

    #[tokio::test]
    async fn missing_lpstat_binary_is_not_ok() {
        let st =
            collect_printers(|| run_lpstat_program("bos-settings-no-such-lpstat-binary")).await;
        assert!(!st.cups_ok);
        assert!(st.error.unwrap().contains("couldn't run"));
    }

    #[tokio::test]
    async fn activation_retry_replaces_the_first_connection_error() {
        let calls = AtomicUsize::new(0);
        let st = collect_printers(|| {
            let attempt = calls.fetch_add(1, Ordering::SeqCst);
            async move {
                if attempt == 0 {
                    classify_lpstat(
                        false,
                        "",
                        "lpstat: Unable to connect to server: Connection refused\n",
                    )
                } else {
                    classify_lpstat(true, "printer Office is idle.  enabled since today\n", "")
                }
            }
        })
        .await;
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert!(st.cups_ok);
        assert_eq!(st.printers.len(), 1);
        assert!(st.error.is_none());
    }

    #[tokio::test]
    async fn repeated_connection_failure_stops_after_one_retry() {
        let calls = AtomicUsize::new(0);
        let st = collect_printers(|| {
            calls.fetch_add(1, Ordering::SeqCst);
            async {
                classify_lpstat(
                    false,
                    "",
                    "lpstat: Unable to connect to server: Connection refused\n",
                )
            }
        })
        .await;
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert!(!st.cups_ok);
        assert!(st.error.unwrap().contains("Connection refused"));
    }
}
