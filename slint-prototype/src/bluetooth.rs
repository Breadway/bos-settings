//! Bluetooth adapter and device actions over bluetoothctl.

use std::process::Command;

#[derive(Clone, Debug, PartialEq)]
pub enum State {
    NoAdapter,
    ServiceOff,
    Ready { powered: bool },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Device {
    pub address: String,
    pub name: String,
    pub connected: bool,
    pub battery: Option<u32>,
    pub paired: bool,
}

fn text(program: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(program).args(args).output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).to_string())
}

fn service_active() -> bool {
    Command::new("systemctl")
        .args(["is-active", "--quiet", "bluetooth.service"])
        .status()
        .is_ok_and(|status| status.success())
}

pub fn state() -> State {
    if !service_active() {
        return State::ServiceOff;
    }
    let show = text("bluetoothctl", &["show"]).unwrap_or_default();
    if !show.trim_start().starts_with("Controller") {
        return State::NoAdapter;
    }
    State::Ready {
        powered: show.lines().any(|line| line.trim() == "Powered: yes"),
    }
}

pub fn start_service() -> Result<String, String> {
    let out = Command::new("busctl")
        .args([
            "call",
            "org.freedesktop.systemd1",
            "/org/freedesktop/systemd1",
            "org.freedesktop.systemd1.Manager",
            "StartUnit",
            "ss",
            "bluetooth.service",
            "replace",
        ])
        .output()
        .map_err(|error| error.to_string())?;
    if out.status.success() {
        Ok("Bluetooth started.".into())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

pub fn set_power(on: bool) -> Result<(), String> {
    let out = Command::new("bluetoothctl")
        .args(["power", if on { "on" } else { "off" }])
        .output()
        .map_err(|error| error.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

pub fn devices() -> Vec<Device> {
    let list = text("bluetoothctl", &["devices"]).unwrap_or_default();
    list.lines()
        .filter_map(|line| {
            let rest = line.strip_prefix("Device ")?;
            let (address, name) = rest.split_once(' ')?;
            let info = text("bluetoothctl", &["info", address]).unwrap_or_default();
            let field = |key: &str| {
                info.lines().find_map(|line| {
                    let (name, value) = line.trim().split_once(':')?;
                    (name == key).then(|| value.trim().to_string())
                })
            };
            Some(Device {
                address: address.to_string(),
                name: name.trim().to_string(),
                connected: field("Connected").as_deref() == Some("yes"),
                battery: field("Battery Percentage").and_then(|value| {
                    value
                        .trim_matches(|c| c == '(' || c == ')')
                        .rsplit_once(' ')
                        .and_then(|(_, percent)| percent.parse().ok())
                }),
                paired: field("Paired").as_deref() == Some("yes"),
            })
        })
        .collect()
}

pub fn connect(address: &str, up: bool) -> Result<(), String> {
    let out = Command::new("bluetoothctl")
        .args([if up { "connect" } else { "disconnect" }, address])
        .output()
        .map_err(|error| error.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

pub fn forget(address: &str) -> Result<(), String> {
    let out = Command::new("bluetoothctl")
        .args(["remove", address])
        .output()
        .map_err(|error| error.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

pub fn scan() -> Vec<Device> {
    let _ = Command::new("bluetoothctl")
        .args(["--timeout", "8", "scan", "on"])
        .output();
    devices()
}

pub fn pair(address: &str) -> Result<(), String> {
    let out = Command::new("bluetoothctl")
        .args(["pair", address])
        .output()
        .map_err(|error| error.to_string())?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    let _ = Command::new("bluetoothctl")
        .args(["trust", address])
        .output();
    let _ = connect(address, true);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_lines_are_addresses_and_names() {
        let text = "Device AC:80:0A:C9:E6 WH-1000XM4\nDevice 00:11:22:33:44:55 Keyboard\n";
        let rows: Vec<_> = text
            .lines()
            .filter_map(|line| {
                let rest = line.strip_prefix("Device ")?;
                let (address, name) = rest.split_once(' ')?;
                Some((address, name))
            })
            .collect();
        assert_eq!(rows[0], ("AC:80:0A:C9:E6", "WH-1000XM4"));
        assert_eq!(rows.len(), 2);
    }
}
