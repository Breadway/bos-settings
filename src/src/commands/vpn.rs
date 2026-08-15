//! NetworkManager VPN / WireGuard connections. breadcrumbs stays Wi-Fi
//! profiles; this panel only lists `vpn` and `wireguard` connection types.

use serde::Serialize;
use tokio::process::Command;

use super::util::{fail_output, valid_nm_id};

#[derive(Serialize, Clone)]
pub struct VpnConnection {
    name: String,
    kind: String,
    active: bool,
    autoconnect: bool,
}

#[derive(Serialize)]
pub struct VpnStatus {
    connections: Vec<VpnConnection>,
    error: Option<String>,
}

#[tauri::command]
pub async fn get_vpn_connections() -> VpnStatus {
    let output = match Command::new("nmcli")
        .args([
            "-t",
            "-f",
            "NAME,TYPE,STATE,AUTOCONNECT",
            "connection",
            "show",
        ])
        .output()
        .await
    {
        Ok(o) => o,
        Err(e) => {
            return VpnStatus {
                connections: Vec::new(),
                error: Some(format!("couldn't run nmcli: {e}")),
            };
        }
    };
    if !output.status.success() {
        return VpnStatus {
            connections: Vec::new(),
            error: Some(fail_output(&output, "nmcli")),
        };
    }
    let text = String::from_utf8_lossy(&output.stdout);
    VpnStatus {
        connections: parse_nm_connections(&text),
        error: None,
    }
}

fn parse_nm_connections(text: &str) -> Vec<VpnConnection> {
    text.lines()
        .filter_map(|line| {
            // nmcli -t escapes ":" in names as "\:".
            let cols = split_nmcli(line);
            if cols.len() < 3 {
                return None;
            }
            let kind = cols[1].as_str();
            if kind != "vpn" && kind != "wireguard" {
                return None;
            }
            let state = cols[2].as_str();
            let autoconnect = cols.get(3).map(|s| s == "yes").unwrap_or(false);
            Some(VpnConnection {
                name: cols[0].clone(),
                kind: kind.to_string(),
                active: state == "activated" || state == "activating",
                autoconnect,
            })
        })
        .collect()
}

fn split_nmcli(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(n) = chars.next() {
                cur.push(n);
            }
        } else if c == ':' {
            out.push(std::mem::take(&mut cur));
        } else {
            cur.push(c);
        }
    }
    out.push(cur);
    out
}

#[tauri::command]
pub async fn vpn_connect(name: String) -> Result<(), String> {
    nmcli_con(&["connection", "up", "id", &checked_id(&name)?]).await
}

#[tauri::command]
pub async fn vpn_disconnect(name: String) -> Result<(), String> {
    nmcli_con(&["connection", "down", "id", &checked_id(&name)?]).await
}

fn checked_id(name: &str) -> Result<String, String> {
    if !valid_nm_id(name) {
        return Err(format!("invalid connection name '{name}'"));
    }
    Ok(name.trim().to_string())
}

async fn nmcli_con(args: &[&str]) -> Result<(), String> {
    let output = Command::new("nmcli")
        .args(args)
        .output()
        .await
        .map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(fail_output(&output, "nmcli"))
    }
}

#[tauri::command]
pub async fn vpn_import(path: String) -> Result<(), String> {
    let path = path.trim();
    if path.is_empty() || path.contains('\0') || path.contains('\n') {
        return Err("invalid path".into());
    }
    let p = std::path::Path::new(path);
    if !p.is_absolute() || !p.is_file() {
        return Err("pick an existing .conf or .ovpn file".into());
    }
    let kind = match p
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase())
        .as_deref()
    {
        Some("ovpn") => "openvpn",
        Some("conf") => "wireguard",
        _ => return Err("import a WireGuard .conf or OpenVPN .ovpn file".into()),
    };
    nmcli_con(&["connection", "import", "type", kind, "file", path]).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_wireguard_and_skips_wifi() {
        let text = "\
Home WG:wireguard:activated:yes
Office:vpn:
NetComm:802-11-wireless:activated
tailscale0:tun:activated:yes
";
        let v = parse_nm_connections(text);
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].name, "Home WG");
        assert!(v[0].active);
        assert_eq!(v[1].kind, "vpn");
        assert!(!v[1].active);
    }

    #[test]
    fn unescapes_colon_in_name() {
        let text = r"Work\:VPN:vpn:activated:no";
        let v = parse_nm_connections(text);
        assert_eq!(v[0].name, "Work:VPN");
    }
}
