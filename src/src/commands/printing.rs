//! CUPS printers via lpstat / lpadmin. Adding a printer through the full
//! device wizard is `system-config-printer`; a simple IPP Everywhere queue
//! can be created here when the user has a URI.

use serde::Serialize;
use tokio::process::Command;

use super::util::{fail_output, valid_printer_name};

#[derive(Serialize, Clone)]
pub struct Printer {
    name: String,
    status: String,
    enabled: bool,
    is_default: bool,
}

#[derive(Serialize)]
pub struct PrintingStatus {
    printers: Vec<Printer>,
    default: Option<String>,
    cups_ok: bool,
    error: Option<String>,
}

#[tauri::command]
pub async fn get_printers() -> PrintingStatus {
    let output = match Command::new("lpstat").args(["-p", "-d"]).output().await {
        Ok(o) => o,
        Err(e) => {
            return PrintingStatus {
                printers: Vec::new(),
                default: None,
                cups_ok: false,
                error: Some(format!("couldn't run lpstat: {e}")),
            };
        }
    };
    if !output.status.success() {
        return PrintingStatus {
            printers: Vec::new(),
            default: None,
            cups_ok: false,
            error: Some(fail_output(&output, "lpstat")),
        };
    }
    let text = String::from_utf8_lossy(&output.stdout);
    parse_lpstat(&text)
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
}
