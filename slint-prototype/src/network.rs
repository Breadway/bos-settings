//! Wi-Fi, VPN and breadcrumbs settings used by the Network page.

use std::collections::HashSet;
use std::path::PathBuf;
use std::process::Command;

#[derive(Clone, Debug, PartialEq)]
pub struct Wifi {
    pub ssid: String,
    pub signal: i32,
    pub secured: bool,
    pub active: bool,
    pub known: bool,
}

#[derive(Clone, Debug, Default)]
pub struct Info {
    pub radio: bool,
    pub ethernet: Option<String>,
    pub ssid: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Vpn {
    pub name: String,
    pub kind: String,
    pub active: bool,
    pub autoconnect: bool,
}

#[derive(Clone, Debug, Default)]
pub struct Place {
    pub name: String,
    pub networks: Vec<String>,
    pub detect: Vec<String>,
    pub tailscale: bool,
    pub include_all: bool,
    pub exit_node: String,
    pub bootstrap: String,
}

#[derive(Clone, Debug, Default)]
pub struct Advanced {
    pub dns: String,
    pub exit_node: String,
    pub connectivity_url: String,
    pub ping_host: String,
    pub connect_wait: i64,
    pub watch_interval: i64,
    pub default_profile: String,
}

fn config_dir() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .unwrap_or_else(|| PathBuf::from(".config"))
}

fn text(program: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(program).args(args).output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

pub fn set_radio(enabled: bool) -> Result<(), String> {
    let out = Command::new("nmcli")
        .args(["radio", "wifi", if enabled { "on" } else { "off" }])
        .output()
        .map_err(|error| error.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

pub fn info() -> Info {
    let radio = text("nmcli", &["radio", "wifi"]).is_some_and(|value| value == "enabled");
    let ethernet = text("nmcli", &["-t", "-f", "DEVICE,TYPE,STATE", "dev"]).and_then(|value| {
        value.lines().find_map(|line| {
            let mut cols = line.splitn(3, ':');
            let (device, kind, state) = (cols.next()?, cols.next()?, cols.next()?);
            (kind == "ethernet").then(|| format!("{device}: {state}"))
        })
    });
    let ssid = text(
        "nmcli",
        &[
            "-t", "--escape", "no", "-f", "ACTIVE,SSID", "device", "wifi", "list", "--rescan",
            "no",
        ],
    )
    .and_then(|value| {
        value
            .lines()
            .find_map(|line| line.strip_prefix("yes:").map(str::to_string))
    });
    Info {
        radio,
        ethernet,
        ssid,
    }
}

fn saved_names() -> HashSet<String> {
    text("nmcli", &["-t", "-f", "NAME", "connection", "show"])
        .map(|value| value.lines().map(str::to_string).collect())
        .unwrap_or_default()
}

fn networks_path() -> PathBuf {
    config_dir().join("breadcrumbs/networks.toml")
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SavedNetwork {
    pub ssid: String,
    pub hidden: bool,
    pub has_password: bool,
}

pub fn breadcrumbs_networks() -> Vec<SavedNetwork> {
    let text = std::fs::read_to_string(networks_path()).unwrap_or_default();
    parse_networks(&text)
}

pub fn parse_networks(text: &str) -> Vec<SavedNetwork> {
    let Ok(doc) = text.parse::<toml_edit::DocumentMut>() else {
        return Vec::new();
    };
    let Some(entries) = doc.get("networks").and_then(|item| item.as_array_of_tables()) else {
        return Vec::new();
    };
    entries
        .iter()
        .filter_map(|table| {
            let ssid = table.get("ssid").and_then(|item| item.as_str())?.trim();
            (!ssid.is_empty()).then(|| SavedNetwork {
                ssid: ssid.to_string(),
                hidden: table
                    .get("hidden")
                    .and_then(|item| item.as_bool())
                    .unwrap_or(false),
                has_password: table
                    .get("password")
                    .and_then(|item| item.as_str())
                    .is_some_and(|value| !value.trim().is_empty()),
            })
        })
        .collect()
}

fn write_networks(networks: &[SavedNetwork], passwords: &[(String, String)]) -> Result<(), String> {
    let path = networks_path();
    let mut doc = std::fs::read_to_string(&path)
        .unwrap_or_default()
        .parse::<toml_edit::DocumentMut>()
        .unwrap_or_default();
    let existing: Vec<(String, String)> = doc
        .get("networks")
        .and_then(|item| item.as_array_of_tables())
        .map(|entries| {
            entries
                .iter()
                .filter_map(|table| {
                    Some((
                        table.get("ssid")?.as_str()?.to_string(),
                        table.get("password")?.as_str()?.to_string(),
                    ))
                })
                .collect()
        })
        .unwrap_or_default();
    let mut tables = toml_edit::ArrayOfTables::new();
    for network in networks {
        let mut table = toml_edit::Table::new();
        table.insert("ssid", toml_edit::value(&network.ssid));
        let password = passwords
            .iter()
            .find(|(ssid, _)| ssid == &network.ssid)
            .map(|(_, password)| password.clone())
            .or_else(|| {
                existing
                    .iter()
                    .find(|(ssid, _)| ssid == &network.ssid)
                    .map(|(_, password)| password.clone())
            });
        if let Some(password) = password.filter(|value| !value.trim().is_empty()) {
            table.insert("password", toml_edit::value(password));
        }
        table.insert("hidden", toml_edit::value(network.hidden));
        tables.push(table);
    }
    doc["networks"] = toml_edit::Item::ArrayOfTables(tables);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    std::fs::write(path, doc.to_string()).map_err(|error| error.to_string())
}

pub fn save_network(
    ssid: &str,
    password: Option<&str>,
    hidden: bool,
    rename_from: Option<&str>,
) -> Result<(), String> {
    let ssid = ssid.trim();
    if ssid.is_empty() || ssid.contains(['\n', '\0']) {
        return Err("enter a network name".into());
    }
    let mut networks = breadcrumbs_networks();
    if let Some(old) = rename_from.filter(|old| *old != ssid) {
        networks.retain(|network| network.ssid != old);
    }
    let mut passwords = Vec::new();
    if let Some(password) = password {
        passwords.push((ssid.to_string(), password.to_string()));
    }
    match networks.iter_mut().find(|network| network.ssid == ssid) {
        Some(network) => {
            network.hidden = hidden;
            if password.is_some_and(|value| !value.is_empty()) {
                network.has_password = true;
            }
        }
        None => networks.push(SavedNetwork {
            ssid: ssid.to_string(),
            hidden,
            has_password: password.is_some_and(|value| !value.is_empty()),
        }),
    }
    write_networks(&networks, &passwords)
}

pub fn forget_network(ssid: &str) -> Result<(), String> {
    let mut networks = breadcrumbs_networks();
    networks.retain(|network| network.ssid != ssid);
    write_networks(&networks, &[])
}

pub fn scan() -> Vec<Wifi> {
    let _ = Command::new("nmcli")
        .args(["dev", "wifi", "rescan"])
        .output();
    std::thread::sleep(std::time::Duration::from_millis(1200));
    let known = saved_names();
    let list = text(
        "nmcli",
        &[
            "-t", "--escape", "no", "-f", "IN-USE,SIGNAL,SECURITY,SSID", "dev", "wifi", "list",
        ],
    )
    .unwrap_or_default();
    let mut networks = parse_wifi(&list, &known);
    if let Some(active) = info().ssid {
        for net in &mut networks {
            if net.ssid == active {
                net.active = true;
            }
        }
    }
    networks
}

pub fn parse_wifi(text: &str, known: &HashSet<String>) -> Vec<Wifi> {
    use std::collections::HashMap;
    let mut by_ssid: HashMap<String, Wifi> = HashMap::new();
    for line in text.lines() {
        let mut cols = line.splitn(4, ':');
        let (Some(active), Some(signal), Some(security), Some(ssid)) =
            (cols.next(), cols.next(), cols.next(), cols.next())
        else {
            continue;
        };
        if ssid.is_empty() {
            continue;
        }
        let net = Wifi {
            ssid: ssid.to_string(),
            signal: signal.parse().unwrap_or(0),
            secured: !security.is_empty(),
            active: active == "*",
            known: known.contains(ssid),
        };
        match by_ssid.entry(net.ssid.clone()) {
            std::collections::hash_map::Entry::Vacant(slot) => {
                slot.insert(net);
            }
            std::collections::hash_map::Entry::Occupied(mut slot) => {
                let existing = slot.get();
                let active = existing.active || net.active;
                if net.active || (!existing.active && net.signal > existing.signal) {
                    *slot.get_mut() = net;
                }
                slot.get_mut().active = active;
            }
        }
    }
    let mut list: Vec<_> = by_ssid.into_values().collect();
    list.sort_by_key(|net| std::cmp::Reverse(net.signal));
    list
}

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty()
        && !value.contains('\n')
        && !value.contains('\0')
        && !value.starts_with('-')
}

pub fn connect(ssid: &str, password: Option<&str>) -> Result<(), String> {
    if !valid_id(ssid) {
        return Err(format!("invalid network name '{ssid}'"));
    }
    let known = saved_names().contains(ssid);
    let mut command = Command::new("nmcli");
    if let Some(password) = password.filter(|value| !value.is_empty()) {
        command.args(["dev", "wifi", "connect", ssid, "password", password]);
    } else if known {
        command.args(["connection", "up", "id", ssid]);
    } else {
        command.args(["dev", "wifi", "connect", ssid]);
    }
    let out = command.output().map_err(|error| error.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        let message = String::from_utf8_lossy(&out.stderr).trim().to_string();
        Err(if message.is_empty() {
            "NetworkManager refused the connection".into()
        } else {
            message
        })
    }
}

pub fn forget(ssid: &str) -> Result<(), String> {
    if !valid_id(ssid) {
        return Err("invalid network name".into());
    }
    let out = Command::new("nmcli")
        .args(["connection", "delete", "id", ssid])
        .output()
        .map_err(|error| error.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

pub fn vpns() -> Vec<Vpn> {
    let Some(list) = text(
        "nmcli",
        &[
            "-t",
            "-f",
            "NAME,TYPE,STATE,AUTOCONNECT",
            "connection",
            "show",
        ],
    ) else {
        return Vec::new();
    };
    parse_vpns(&list)
}

fn nmcli_columns(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut chars = line.chars();
    while let Some(ch) = chars.next() {
        match ch {
            '\\' => {
                if let Some(next) = chars.next() {
                    current.push(next);
                }
            }
            ':' => out.push(std::mem::take(&mut current)),
            _ => current.push(ch),
        }
    }
    out.push(current);
    out
}

pub fn parse_vpns(text: &str) -> Vec<Vpn> {
    text.lines()
        .filter_map(|line| {
            let cols = nmcli_columns(line);
            if cols.len() < 3 || !matches!(cols[1].as_str(), "vpn" | "wireguard") {
                return None;
            }
            Some(Vpn {
                name: cols[0].clone(),
                kind: cols[1].clone(),
                active: matches!(cols[2].as_str(), "activated" | "activating"),
                autoconnect: cols.get(3).is_some_and(|value| value == "yes"),
            })
        })
        .collect()
}

pub fn vpn_toggle(name: &str, up: bool) -> Result<(), String> {
    if !valid_id(name) {
        return Err("invalid connection name".into());
    }
    let out = Command::new("nmcli")
        .args([
            "connection",
            if up { "up" } else { "down" },
            "id",
            name,
        ])
        .output()
        .map_err(|error| error.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

pub fn import_vpn_dialog() -> Result<String, String> {
    let out = Command::new("zenity")
        .args([
            "--file-selection",
            "--title=Add a VPN",
            "--file-filter=WireGuard or OpenVPN | *.conf *.ovpn",
        ])
        .output()
        .map_err(|error| error.to_string())?;
    if !out.status.success() {
        return Err("cancelled".into());
    }
    let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
    import_vpn(&path)?;
    Ok(path)
}

pub fn import_vpn(path: &str) -> Result<(), String> {
    let path = path.trim();
    let file = std::path::Path::new(path);
    if !file.is_absolute() || !file.is_file() {
        return Err("pick an existing .conf or .ovpn file".into());
    }
    let kind = match file
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("ovpn") => "openvpn",
        Some("conf") => "wireguard",
        _ => return Err("import a WireGuard .conf or OpenVPN .ovpn file".into()),
    };
    let out = Command::new("nmcli")
        .args(["connection", "import", "type", kind, "file", path])
        .output()
        .map_err(|error| error.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

fn breadcrumbs_path() -> PathBuf {
    config_dir().join("breadcrumbs/breadcrumbs.toml")
}

pub fn places() -> (Vec<Place>, Option<String>) {
    let config = std::fs::read_to_string(breadcrumbs_path()).unwrap_or_default();
    let current = text("breadcrumbs", &["profile", "get"]).filter(|value| !value.is_empty());
    (parse_places(&config), current)
}

pub fn parse_places(text: &str) -> Vec<Place> {
    let Ok(doc) = text.parse::<toml_edit::DocumentMut>() else {
        return Vec::new();
    };
    let Some(profiles) = doc.get("profiles").and_then(|item| item.as_table_like()) else {
        return Vec::new();
    };
    let string_list = |item: Option<&toml_edit::Item>| {
        item.and_then(|value| value.as_array())
            .map(|array| {
                array
                    .iter()
                    .filter_map(|value| value.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default()
    };
    profiles
        .iter()
        .map(|(name, item)| Place {
            name: name.to_string(),
            networks: string_list(item.get("networks")),
            detect: string_list(item.get("detect_ssids")),
            tailscale: item.get("tailscale").and_then(|v| v.as_bool()).unwrap_or(false),
            include_all: item
                .get("include_all_known")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            exit_node: item
                .get("exit_node")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            bootstrap: item
                .get("bootstrap")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        })
        .collect()
}

pub fn add_place() -> Result<String, String> {
    let path = breadcrumbs_path();
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let mut doc = text.parse::<toml_edit::DocumentMut>().map_err(|error| error.to_string())?;
    if !doc.contains_key("profiles") {
        doc["profiles"] = toml_edit::Item::Table(toml_edit::Table::new());
    }
    let existing: Vec<String> = parse_places(&doc.to_string())
        .into_iter()
        .map(|place| place.name)
        .collect();
    let profiles = doc["profiles"]
        .as_table_like_mut()
        .ok_or("profiles is not a table")?;
    let mut name = "New place".to_string();
    let mut index = 2;
    while existing.contains(&name) {
        name = format!("New place {index}");
        index += 1;
    }
    let mut table = toml_edit::Table::new();
    table.insert("networks", toml_edit::value(toml_edit::Array::new()));
    table.insert("detect_ssids", toml_edit::value(toml_edit::Array::new()));
    table.insert("bootstrap", toml_edit::value(""));
    table.insert("exit_node", toml_edit::value(""));
    table.insert("tailscale", toml_edit::value(false));
    table.insert("include_all_known", toml_edit::value(false));
    profiles.insert(&name, toml_edit::Item::Table(table));
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    std::fs::write(&path, doc.to_string()).map_err(|error| error.to_string())?;
    Ok(name)
}

fn mutate_place(
    name: &str,
    change: impl FnOnce(&mut dyn toml_edit::TableLike) -> Result<(), String>,
) -> Result<(), String> {
    let path = breadcrumbs_path();
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let mut doc = text.parse::<toml_edit::DocumentMut>().map_err(|error| error.to_string())?;
    let profiles = doc
        .get_mut("profiles")
        .and_then(|item| item.as_table_like_mut())
        .ok_or("profiles is not a table")?;
    let profile = profiles
        .get_mut(name)
        .and_then(|item| item.as_table_like_mut())
        .ok_or_else(|| format!("there is no place called {name}"))?;
    change(profile)?;
    std::fs::write(path, doc.to_string()).map_err(|error| error.to_string())
}

pub fn set_place_bool(name: &str, key: &str, value: bool) -> Result<(), String> {
    if !matches!(key, "tailscale" | "include_all_known") {
        return Err("unknown place setting".into());
    }
    mutate_place(name, |profile| {
        profile.insert(key, toml_edit::value(value));
        Ok(())
    })
}

pub fn set_place_string(name: &str, key: &str, value: &str) -> Result<(), String> {
    if !matches!(key, "exit_node" | "bootstrap") {
        return Err("unknown place setting".into());
    }
    mutate_place(name, |profile| {
        profile.insert(key, toml_edit::value(value));
        Ok(())
    })
}

fn list_key(key: &str) -> Result<&'static str, String> {
    match key {
        "networks" => Ok("networks"),
        "detect" => Ok("detect_ssids"),
        _ => Err("unknown place list".into()),
    }
}

pub fn add_place_list(name: &str, key: &str, value: &str) -> Result<(), String> {
    let key = list_key(key)?;
    let value = value.trim();
    if value.is_empty() {
        return Ok(());
    }
    mutate_place(name, |profile| {
        let array = profile
            .entry(key)
            .or_insert(toml_edit::value(toml_edit::Array::new()))
            .as_array_mut()
            .ok_or("place list is not an array")?;
        if !array.iter().any(|item| item.as_str() == Some(value)) {
            array.push(value);
        }
        Ok(())
    })
}

pub fn remove_place_list(name: &str, key: &str, index: usize) -> Result<(), String> {
    let key = list_key(key)?;
    mutate_place(name, |profile| {
        let array = profile
            .get_mut(key)
            .and_then(|item| item.as_array_mut())
            .ok_or("place list is not an array")?;
        if index < array.len() {
            array.remove(index);
        }
        Ok(())
    })
}

pub fn remove_place(name: &str) -> Result<(), String> {
    let path = breadcrumbs_path();
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let mut doc = text.parse::<toml_edit::DocumentMut>().map_err(|error| error.to_string())?;
    if let Some(profiles) = doc
        .get_mut("profiles")
        .and_then(|item| item.as_table_like_mut())
    {
        profiles.remove(name);
    }
    if let Some(settings) = doc
        .get_mut("settings")
        .and_then(|item| item.as_table_like_mut())
    {
        if settings
            .get("default_profile")
            .and_then(|item| item.as_str())
            == Some(name)
        {
            settings.insert("default_profile", toml_edit::value(""));
        }
    }
    std::fs::write(path, doc.to_string()).map_err(|error| error.to_string())
}

pub fn rename_place(old: &str, new: &str) -> Result<(), String> {
    let new = new.trim();
    if !valid_id(new) || old == new {
        return Ok(());
    }
    let path = breadcrumbs_path();
    let mut doc = std::fs::read_to_string(&path)
        .unwrap_or_default()
        .parse::<toml_edit::DocumentMut>()
        .map_err(|error| error.to_string())?;
    let existing: Vec<String> = parse_places(&doc.to_string())
        .into_iter()
        .map(|place| place.name)
        .collect();
    if existing.iter().any(|name| name == new) {
        return Err(format!("there is already a place called {new}"));
    }
    {
        let profiles = doc
            .get_mut("profiles")
            .and_then(|item| item.as_table_like_mut())
            .ok_or("profiles is not a table")?;
        let item = profiles.remove(old).ok_or("place not found")?;
        profiles.insert(new, item);
    }
    if let Some(settings) = doc
        .get_mut("settings")
        .and_then(|item| item.as_table_like_mut())
    {
        if settings
            .get("default_profile")
            .and_then(|item| item.as_str())
            == Some(old)
        {
            settings.insert("default_profile", toml_edit::value(new));
        }
    }
    std::fs::write(path, doc.to_string()).map_err(|error| error.to_string())
}

pub fn set_default_place(name: &str) -> Result<(), String> {
    let path = breadcrumbs_path();
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let mut doc = text.parse::<toml_edit::DocumentMut>().map_err(|error| error.to_string())?;
    if !doc.contains_key("settings") {
        doc["settings"] = toml_edit::Item::Table(toml_edit::Table::new());
    }
    let settings = doc["settings"]
        .as_table_like_mut()
        .ok_or("settings is not a table")?;
    settings.insert("default_profile", toml_edit::value(name));
    std::fs::write(path, doc.to_string()).map_err(|error| error.to_string())
}

pub fn switch_place(name: &str) -> Result<(), String> {
    if !valid_id(name) {
        return Err("invalid place name".into());
    }
    let out = Command::new("breadcrumbs")
        .args(["profile", "set", name])
        .output()
        .map_err(|error| format!("Couldn't run breadcrumbs: {error}"))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

pub fn advanced() -> Advanced {
    let text = std::fs::read_to_string(breadcrumbs_path()).unwrap_or_default();
    parse_advanced(&text)
}

pub fn parse_advanced(text: &str) -> Advanced {
    let mut out = Advanced {
        connect_wait: 8,
        watch_interval: 30,
        ..Advanced::default()
    };
    let Ok(doc) = text.parse::<toml_edit::DocumentMut>() else {
        return out;
    };
    let Some(settings) = doc.get("settings").and_then(|item| item.as_table_like()) else {
        return out;
    };
    let string = |key: &str| {
        settings
            .get(key)
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .to_string()
    };
    out.dns = string("dns");
    out.exit_node = string("exit_node");
    out.connectivity_url = string("connectivity_url");
    out.ping_host = string("ping_host");
    out.default_profile = string("default_profile");
    out.connect_wait = settings
        .get("connect_wait")
        .or_else(|| settings.get("nmcli_wait"))
        .and_then(|value| value.as_integer())
        .unwrap_or(8);
    out.watch_interval = settings
        .get("watch_interval")
        .and_then(|value| value.as_integer())
        .unwrap_or(30);
    out
}

pub fn save_setting(key: &str, value: &str) -> Result<(), String> {
    let allowed = [
        "dns",
        "exit_node",
        "connectivity_url",
        "ping_host",
        "connect_wait",
        "watch_interval",
    ];
    if !allowed.contains(&key) {
        return Err("unknown setting".into());
    }
    let path = breadcrumbs_path();
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let mut doc = text.parse::<toml_edit::DocumentMut>().map_err(|error| error.to_string())?;
    if !doc.contains_key("settings") {
        doc["settings"] = toml_edit::Item::Table(toml_edit::Table::new());
    }
    let settings = doc["settings"]
        .as_table_like_mut()
        .ok_or("settings is not a table")?;
    if matches!(key, "connect_wait" | "watch_interval") {
        settings.insert(key, toml_edit::value(value.parse::<i64>().map_err(|_| "not a number")?));
    } else {
        settings.insert(key, toml_edit::value(value));
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    std::fs::write(path, doc.to_string()).map_err(|error| error.to_string())
}

#[derive(Clone, Debug, Default)]
pub struct Service {
    pub active: bool,
    pub enabled: bool,
}

pub fn breadcrumbs_service() -> Service {
    let active = Command::new("systemctl")
        .args(["is-active", "--quiet", "breadcrumbs.service"])
        .status()
        .is_ok_and(|status| status.success());
    let enabled = Command::new("systemctl")
        .args(["is-enabled", "--quiet", "breadcrumbs.service"])
        .status()
        .is_ok_and(|status| status.success());
    Service { active, enabled }
}

pub fn service_action(action: &str) -> Result<(), String> {
    let method = match action {
        "start" => "StartUnit",
        "stop" => "StopUnit",
        "restart" => "RestartUnit",
        _ => return Err("unknown service action".into()),
    };
    let out = Command::new("busctl")
        .args([
            "call",
            "org.freedesktop.systemd1",
            "/org/freedesktop/systemd1",
            "org.freedesktop.systemd1.Manager",
            method,
            "ss",
            "breadcrumbs.service",
            "replace",
        ])
        .output()
        .map_err(|error| error.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

pub fn open_editor() {
    let _ = Command::new("nm-connection-editor").spawn();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wifi_rows_merge_bands_and_keep_the_active_ap() {
        let known: HashSet<String> = ["NCC".to_string()].into_iter().collect();
        let list = parse_wifi(" :90:WPA2:Totally NCC\n*:70:WPA2:NCC\n :82:WPA2:NCC\n", &known);
        assert_eq!(list.len(), 2);
        let ncc = list.iter().find(|net| net.ssid == "NCC").unwrap();
        assert!(ncc.active);
        assert!(ncc.known);
        assert_eq!(ncc.signal, 70);
    }

    #[test]
    fn ssid_colon_is_kept() {
        let list = parse_wifi("*:80:WPA2:Lab: guest\n", &HashSet::new());
        assert_eq!(list[0].ssid, "Lab: guest");
    }

    #[test]
    fn vpns_skip_wifi_and_unescape_names() {
        let list = parse_vpns("Work\\:VPN:vpn:activated:yes\nHome:802-11-wireless:activated\nWG:wireguard:\n");
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].name, "Work:VPN");
        assert!(list[0].active);
    }

    #[test]
    fn places_and_advanced_read_the_real_schema() {
        let text = "[settings]\ndns = \"1.1.1.1\"\nconnect_wait = 12\n[profiles.home]\nnetworks = [\"A\", \"B\"]\ndetect_ssids = [\"A\"]\ntailscale = true\n";
        let places = parse_places(text);
        assert_eq!(places.len(), 1);
        assert_eq!(places[0].name, "home");
        assert_eq!(places[0].networks, ["A", "B"]);
        assert!(places[0].tailscale);
        let advanced = parse_advanced(text);
        assert_eq!(advanced.dns, "1.1.1.1");
        assert_eq!(advanced.connect_wait, 12);
    }
}
