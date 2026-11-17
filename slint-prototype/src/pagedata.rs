//! Data for every sidebar page that used to fall through to the placeholder.
//!
//! Each page is a flat list of blocks that ui/spec.slint renders with the
//! shipping app's shared Group/Row/hero/dev-card pieces, so geometry stays
//! identical across pages. Reads mirror src/src/commands/*; nothing here shells
//! out to bos-settings itself.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Mutex, OnceLock};

#[derive(Clone)]
pub struct Row {
    pub kind: String,
    pub id: String,
    pub label: String,
    pub hint: String,
    pub value: String,
    pub detail: String,
    pub icon: String,
    pub action: String,
    pub tag: String,
    pub avatar: String,
    pub badges: Vec<String>,
    pub chip_icons: Vec<String>,
    pub checked: bool,
    pub primary: bool,
    pub enabled: bool,
    pub amount: f32,
    pub min: f32,
    pub max: f32,
    pub options: Vec<String>,
    pub selected: i32,
}

impl Default for Row {
    fn default() -> Self {
        Self {
            kind: String::new(),
            id: String::new(),
            label: String::new(),
            hint: String::new(),
            value: String::new(),
            detail: String::new(),
            icon: String::new(),
            action: String::new(),
            tag: String::new(),
            avatar: String::new(),
            badges: Vec::new(),
            chip_icons: Vec::new(),
            checked: false,
            primary: false,
            // Rows are enabled unless a page says otherwise; a plain derive
            // would default this to false and dim every control.
            enabled: true,
            amount: 0.0,
            min: 0.0,
            max: 0.0,
            options: Vec::new(),
            selected: -1,
        }
    }
}

#[derive(Clone, Default)]
pub struct Block {
    pub kind: String,
    pub title: String,
    pub hint: String,
    pub icon: String,
    pub value: String,
    pub action: String,
    pub tag: String,
    pub on: bool,
    pub primary: bool,
    pub amount: f32,
    pub chart: String,
    pub chart_area: String,
    pub arc: String,
    pub action_icon: String,
    pub chip_icons: Vec<String>,
    pub rows: Vec<Row>,
    pub chips: Vec<String>,
    pub chip: i32,
}

#[derive(Clone, Default)]
pub struct Page {
    pub id: String,
    pub crumb: String,
    pub title: String,
    pub lede: String,
    pub status: String,
    pub blocks: Vec<Block>,
}

pub fn row(kind: &str, label: &str) -> Row {
    Row {
        kind: kind.into(),
        label: label.into(),
        enabled: true,
        ..Row::default()
    }
}

pub fn group(title: &str) -> Block {
    Block {
        kind: "group".into(),
        title: title.into(),
        ..Block::default()
    }
}

pub fn hero(title: &str, sub: &str) -> Block {
    Block {
        kind: "hero".into(),
        title: title.into(),
        hint: sub.into(),
        amount: -1.0,
        ..Block::default()
    }
}

pub fn note(text: &str) -> Block {
    Block {
        kind: "note".into(),
        value: text.into(),
        ..Block::default()
    }
}

fn page(id: &str, crumb: &str, title: &str, lede: &str) -> Page {
    Page {
        id: id.into(),
        crumb: crumb.into(),
        title: title.into(),
        lede: lede.into(),
        status: String::new(),
        blocks: Vec::new(),
    }
}

impl Page {
    fn push(mut self, block: Block) -> Self {
        self.blocks.push(block);
        self
    }
    fn rows(mut self, title: &str, rows: Vec<Row>) -> Self {
        let mut block = group(title);
        block.rows = rows;
        self.blocks.push(block);
        self
    }
}

fn config_dir() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| PathBuf::from(".config"))
}

fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/root"))
}

fn run(program: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(program).args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).to_string())
}

fn write(program: &str, args: &[&str]) -> Result<(), String> {
    let out = Command::new(program)
        .args(args)
        .output()
        .map_err(|e| format!("{program}: {e}"))?;
    if out.status.success() {
        return Ok(());
    }
    let message = String::from_utf8_lossy(&out.stderr).trim().to_string();
    Err(if message.is_empty() {
        format!("{program} exited with {}", out.status)
    } else {
        message
    })
}

fn command_exists(program: &str) -> bool {
    Command::new("sh")
        .args(["-c", &format!("command -v {program}")])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn which(program: &str) -> bool {
    command_exists(program)
}

fn toggle(id: &str, label: &str, hint: &str, on: bool) -> Row {
    Row {
        kind: "toggle".into(),
        id: id.into(),
        label: label.into(),
        hint: hint.into(),
        checked: on,
        ..Row::default()
    }
}

fn button(id: &str, label: &str, action: &str) -> Row {
    Row {
        kind: "button".into(),
        id: id.into(),
        label: label.into(),
        action: action.into(),
        ..Row::default()
    }
}

fn item(id: &str, icon: &str, label: &str, hint: &str) -> Row {
    Row {
        kind: "item".into(),
        id: id.into(),
        icon: icon.into(),
        label: label.into(),
        hint: hint.into(),
        enabled: true,
        ..Row::default()
    }
}

// ---------------------------------------------------------------- notifications

pub fn load(id: &str) -> Page {
    let mut page = match id {
        "notifications" => notifications(),
        "sharing" => sharing(),
        "defaults" => defaults(),
        "printers" => printers(),
        "datetime" => datetime(),
        "a11y" => a11y(),
        "users" => users(),
        "startup" => startup(),
        "storage" => storage(),
        "sound" => sound(),
        "power" => power(),
        "displays" => displays(),
        "bluetooth" => bluetooth(),
        "network" => network(),
        "mouse" => mouse(),
        "lock" => lock(),
        "privacy" => privacy(),
        "updates" => updates(),
        "restore" => restore(),
        "memory" => memory(),
        "getapps" => getapps(),
        "desktop" => desktop(),
        "keyboard" => keyboard(),
        other => page(other, "", other, ""),
    };
    apply_state(&mut page);
    page
}

// ---------------------------------------------------------------- UI state
// Controls remember their value in one small TOML file, so the prototype is
// usable without the shipping app. Keys are "page:row-id" or "page:blockN".

fn state_path() -> PathBuf {
    std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/state")))
        .unwrap_or_else(|| PathBuf::from(".local/state"))
        .join("bos-settings-slint/state.toml")
}

fn read_state(path: &std::path::Path) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let Some(table) = std::fs::read_to_string(path)
        .ok()
        .and_then(|text| text.parse::<toml::Value>().ok())
        .and_then(|value| value.as_table().cloned())
    else {
        return map;
    };
    for (key, value) in table {
        if let Some(text) = value.as_str() {
            map.insert(key, text.to_string());
        }
    }
    map
}

fn state() -> &'static Mutex<HashMap<String, String>> {
    static STATE: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(read_state(&state_path())))
}

fn save_state(map: &HashMap<String, String>) {
    let path = state_path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let mut table = toml::map::Map::new();
    for (key, value) in map {
        table.insert(key.clone(), toml::Value::String(value.clone()));
    }
    if let Ok(text) = toml::to_string(&toml::Value::Table(table)) {
        let _ = std::fs::write(path, text);
    }
}

fn state_put(key: String, value: String) {
    let mut map = state().lock().unwrap();
    map.insert(key, value);
    save_state(&map);
}

fn apply_state(page: &mut Page) {
    let map = state().lock().unwrap();
    apply_state_with(&map, page);
}

fn apply_state_with(map: &HashMap<String, String>, page: &mut Page) {
    // Memory shows the daemon's file, never the prototype's own scratch state.
    if page.id == "memory" {
        return;
    }
    for (block_index, block) in page.blocks.iter_mut().enumerate() {
        let block_key = format!("{}:block{block_index}", page.id);
        if block.tag == "toggle" {
            if let Some(value) = map.get(&block_key) {
                block.on = value == "true";
            }
        }
        if block.kind == "chips" {
            if let Some(index) = map.get(&block_key).and_then(|v| v.parse::<i32>().ok()) {
                block.chip = index;
            }
        }
        for row in &mut block.rows {
            let key = format!("{}:{}", page.id, row.id);
            match row.kind.as_str() {
                "toggle" => {
                    if let Some(value) = map.get(&key) {
                        row.checked = value == "true";
                    }
                }
                "select" => {
                    if row.selected == 0 {
                        if let Some(index) = row.options.iter().position(|o| *o == row.value) {
                            row.selected = index as i32;
                        }
                    }
                    if let Some(index) = map.get(&key).and_then(|v| v.parse::<i32>().ok()) {
                        if let Some(option) = row.options.get(index as usize) {
                            row.value = option.clone();
                            row.selected = index;
                        }
                    }
                }
                "seg" => {
                    if let Some(index) = map.get(&key).and_then(|v| v.parse::<i32>().ok()) {
                        row.selected = index;
                    }
                }
                "chips" => {
                    if let Some(index) = map.get(&key).and_then(|v| v.parse::<i32>().ok()) {
                        row.selected = index;
                    }
                }
                "slider" => {
                    if let Some(value) = map.get(&key).and_then(|v| v.parse::<f32>().ok()) {
                        row.amount = value;
                        row.value = number_label(value, &row.value);
                    }
                }
                _ => {}
            }
        }
    }
}

/// Keeps a slider's unit ("%", " px") while replacing the number it shows.
fn number_label(value: f32, previous: &str) -> String {
    let suffix: String = previous
        .trim_start_matches(|c: char| c.is_ascii_digit() || c == '.' || c == '-')
        .to_string();
    let number = if (value - value.round()).abs() < 0.05 {
        format!("{}", value.round() as i64)
    } else {
        format!("{value:.1}")
    };
    format!("{number}{suffix}")
}

fn notifications() -> Page {
    let mut p = page(
        "notifications",
        "Personalise",
        "Notifications",
        "Choose what can interrupt you, and when.",
    );
    match notification_prefs() {
        Some(prefs) => {
            let mut rows = vec![
                toggle(
                    "dnd",
                    "Do not disturb",
                    "Only critical alerts get through.",
                    prefs.dnd,
                ),
                toggle(
                    "schedule",
                    "Turn on automatically",
                    "Use a nightly schedule.",
                    prefs.schedule,
                ),
            ];
            if prefs.schedule {
                rows.push(Row {
                    kind: "text".into(),
                    id: "quiet".into(),
                    label: "Quiet hours".into(),
                    value: format!("{} to {}", prefs.start, prefs.end),
                    ..Row::default()
                });
            }
            rows.push(toggle(
                "lock-screen",
                "Show on the lock screen",
                "",
                prefs.lock_screen,
            ));
            rows.push(toggle(
                "sound",
                "Sound",
                if prefs.sound_available {
                    ""
                } else {
                    "Install libcanberra to enable notification sounds."
                },
                prefs.sound,
            ));
            p = p.rows("Notifications", rows);
            let mut apps: Vec<Row> = prefs
                .apps
                .iter()
                .map(|(name, policy)| Row {
                    kind: "select".into(),
                    id: format!("app:{name}"),
                    label: name.clone(),
                    value: policy_label(policy).into(),
                    options: vec![
                        "Banners and sound".into(),
                        "Banners".into(),
                        "Only in history".into(),
                        "Off".into(),
                    ],
                    selected: policy_index(policy),
                    ..Row::default()
                })
                .collect();
            apps.sort_by(|a, b| a.label.cmp(&b.label));
            let mut block = group("Apps");
            block.rows = apps;
            p = p.push(block);
        }
        None => {
            p = p.push(note(
                "Notification settings aren't available yet. The bar that shows your notifications doesn't offer these settings in the version you're running. Notification history below still works.",
            ));
        }
    }
    p.rows(
        "History and appearance",
        vec![
            button("history", "Notification history", "Open history"),
            button("theme", "Notification appearance", "Choose a theme"),
        ],
    )
}

struct NotifPrefs {
    dnd: bool,
    schedule: bool,
    start: String,
    end: String,
    lock_screen: bool,
    sound: bool,
    sound_available: bool,
    apps: Vec<(String, String)>,
}

fn notification_prefs() -> Option<NotifPrefs> {
    let out = Command::new("busctl")
        .args([
            "--user",
            "--json=short",
            "call",
            "org.freedesktop.Notifications",
            "/dev/breadway/Bar",
            "dev.breadway.Bar",
            "GetNotificationPreferences",
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).ok()?;
    let text = value
        .get("data")?
        .get(0)?
        .as_str()?
        .to_string();
    let prefs: serde_json::Value = serde_json::from_str(&text).ok()?;
    let apps = prefs
        .get("apps")
        .and_then(|v| v.as_object())
        .map(|map| {
            map.iter()
                .map(|(k, v)| (k.clone(), v.as_str().unwrap_or("all").to_string()))
                .collect()
        })
        .unwrap_or_default();
    Some(NotifPrefs {
        dnd: prefs.get("dnd").and_then(|v| v.as_bool()).unwrap_or(false),
        schedule: prefs
            .get("schedule")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        start: prefs
            .get("start")
            .and_then(|v| v.as_str())
            .unwrap_or("22:00")
            .into(),
        end: prefs
            .get("end")
            .and_then(|v| v.as_str())
            .unwrap_or("07:00")
            .into(),
        lock_screen: prefs
            .get("lock_screen")
            .and_then(|v| v.as_bool())
            .unwrap_or(true),
        sound: prefs.get("sound").and_then(|v| v.as_bool()).unwrap_or(true),
        sound_available: prefs
            .get("sound_available")
            .and_then(|v| v.as_bool())
            .unwrap_or(true),
        apps,
    })
}

fn policy_label(policy: &str) -> &'static str {
    match policy {
        "banners" => "Banners",
        "history" => "Only in history",
        "off" => "Off",
        _ => "Banners and sound",
    }
}

fn policy_index(policy: &str) -> i32 {
    match policy {
        "banners" => 1,
        "history" => 2,
        "off" => 3,
        _ => 0,
    }
}

// -------------------------------------------------------------------- sharing

fn sharing() -> Page {
    let mut p = page(
        "sharing",
        "Connect",
        "Sharing",
        "Let nearby devices send you things, or reach this computer.",
    );
    let hostname = std::fs::read_to_string("/etc/hostname")
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "Unknown".into());
    let ssh_installed = which("sshd");
    let ssh_running = run("systemctl", &["is-active", "sshd.service"]).is_some();
    let localsend = which("localsend") || run("flatpak", &["info", "org.localsend.localsend_app"]).is_some();
    let phone_link = which("kdeconnect-settings");

    let mut nearby = Vec::new();
    nearby.push(Row {
        kind: "item".into(),
        id: "receive-files".into(),
        label: "Receive files".into(),
        hint: if localsend {
            "Phones and laptops on this network can send you files with LocalSend.".into()
        } else {
            "LocalSend lets phones and laptops nearby send you files.".into()
        },
        action: if localsend { "Open".into() } else { "Get LocalSend".into() },
        ..Row::default()
    });
    let mut phone = item(
        "phone-link",
        "",
        "Phone link",
        "Notifications, clipboard and files with your phone, through KDE Connect.",
    );
    phone.action = if phone_link { "Open".into() } else { "Install".into() };
    nearby.push(phone);
    p = p.rows("Nearby devices", nearby);

    let mut remote = Vec::new();
    if ssh_installed {
        remote.push(toggle(
            "ssh",
            "Remote login (SSH)",
            "Lets you open a terminal on this computer from another one. Asks for your password.",
            ssh_running,
        ));
    }
    let mut firewall = item(
        "firewall",
        "",
        "Firewall",
        "Choose what can connect in.",
    );
    firewall.action = "Open".into();
    remote.push(firewall);
    p = p.rows("Remote login", remote);

    p = p.rows(
        "This computer",
        vec![Row {
            kind: "input".into(),
            id: "hostname".into(),
            label: "Name on the network".into(),
            hint: format!("Others see it as {hostname}.local. Needs your password."),
            value: hostname,
            action: "Rename".into(),
            ..Row::default()
        }],
    );
    p
}

// ------------------------------------------------------------------- defaults

fn defaults() -> Page {
    let mut p = page("defaults", "Apps", "Default apps", "");
    let categories = [
        ("browser", "Browser", &["x-scheme-handler/http", "x-scheme-handler/https", "text/html"][..]),
        ("files", "File manager", &["inode/directory"][..]),
        ("terminal", "Terminal", &["x-scheme-handler/terminal"][..]),
        ("image", "Images", &["image/png", "image/jpeg", "image/webp", "image/gif", "image/svg+xml"][..]),
        ("pdf", "PDF", &["application/pdf"][..]),
        ("editor", "Editor", &["text/plain", "text/markdown"][..]),
    ];
    let text = std::fs::read_to_string(config_dir().join("mimeapps.list")).unwrap_or_default();
    let mut current: Vec<(String, String)> = Vec::new();
    for line in text.lines() {
        let t = line.trim();
        if t.starts_with('[') {
            continue;
        }
        if let Some((k, v)) = t.split_once('=') {
            let desktop = v.split(';').next().unwrap_or("").trim();
            if !desktop.is_empty() {
                current.push((k.trim().to_string(), desktop.to_string()));
            }
        }
    }
    let terminals_list = std::fs::read_to_string(config_dir().join("xdg-terminals.list"))
        .ok()
        .and_then(|s| s.lines().map(str::trim).find(|l| !l.is_empty() && !l.starts_with('#')).map(str::to_string));
    let mut rows = Vec::new();
    for (index, (id, label, mimes)) in categories.iter().enumerate() {
        let mut value = String::new();
        if *id == "terminal" {
            if let Some(t) = &terminals_list {
                value = t.clone();
            }
        }
        if value.is_empty() {
            for mime in *mimes {
                if let Some((_, desktop)) = current.iter().find(|(k, _)| k == mime) {
                    value = desktop.clone();
                    break;
                }
            }
        }
        rows.push(Row {
            kind: "select".into(),
            id: format!("cat:{id}"),
            label: (*label).into(),
            value: desktop_name(&value),
            detail: value,
            selected: index as i32,
            ..Row::default()
        });
    }
    p.rows("Apps", rows)
}

fn desktop_name(id: &str) -> String {
    if id.is_empty() {
        return "Not set".into();
    }
    for dir in [
        PathBuf::from("/usr/share/applications"),
        PathBuf::from("/usr/local/share/applications"),
        home_dir().join(".local/share/applications"),
    ] {
        if let Ok(text) = std::fs::read_to_string(dir.join(id)) {
            let mut in_entry = false;
            for line in text.lines().map(str::trim) {
                if line.starts_with('[') {
                    in_entry = line.eq_ignore_ascii_case("[Desktop Entry]");
                } else if in_entry {
                    if let Some(v) = line.strip_prefix("Name=") {
                        if !v.is_empty() {
                            return v.to_string();
                        }
                    }
                }
            }
        }
    }
    id.trim_end_matches(".desktop").to_string()
}

// ------------------------------------------------------------------- printers

fn printers() -> Page {
    let mut p = page(
        "printers",
        "Devices",
        "Printers",
        "Printers on your network usually show up on their own.",
    );
    let out = Command::new("lpstat").args(["-p", "-d"]).output();
    let (mut rows, mut state_text) = (Vec::new(), String::new());
    match out {
        Ok(o) if o.status.success() || o.status.code() == Some(1) => {
            let text = String::from_utf8_lossy(&o.stdout).to_string();
            let default = text
                .lines()
                .find_map(|l| l.strip_prefix("system default destination: "))
                .map(|s| s.trim().to_string())
                .unwrap_or_default();
            let mut lines = text.lines().peekable();
            while let Some(line) = lines.next() {
                if let Some(rest) = line.strip_prefix("printer ") {
                    let name = rest.split_whitespace().next().unwrap_or("").to_string();
                    let status = rest.to_string();
                    let ready = status.contains("is idle");
                    let mut row = item(
                        &format!("printer:{name}"),
                        "",
                        &name.replace(['_', '-'], " "),
                        &if ready {
                            format!("Ready{}", if name == default { " · default printer" } else { "" })
                        } else {
                            status.clone()
                        },
                    );
                    if name != default {
                        row.action = "Make default".into();
                    }
                    rows.push(row);
                }
            }
            if rows.is_empty() {
                state_text = "No printers configured yet. Add one below.".into();
            }
        }
        Ok(_) => state_text = "The printing service isn't answering.".into(),
        Err(_) => state_text = "The printing service isn't answering.".into(),
    }
    let mut block = group("Your printers");
    block.rows = rows;
    if !state_text.is_empty() {
        block.rows.push(Row {
            kind: "text".into(),
            id: "state".into(),
            label: state_text,
            ..Row::default()
        });
    }
    p = p.push(block);
    let mut add = item(
        "find",
        "",
        "Find a printer",
        "Looks on your network and USB, and installs drivers.",
    );
    add.action = "Open".into();
    add.primary = true;
    let mut add_block = group("Add a printer");
    add_block.rows = vec![
        add,
        Row {
            kind: "line".into(),
            id: "printer-settings".into(),
            label: "Printer settings".into(),
            hint: "Queues, drivers and options.".into(),
            action: "Open".into(),
            ..Row::default()
        },
    ];
    p.push(add_block)
}

// ------------------------------------------------------------------- datetime

fn datetime() -> Page {
    let mut p = page("datetime", "System", "Date & language", "Time, region and language.");
    let ntp = run("timedatectl", &["show", "--property=NTP"]).unwrap_or_default();
    let synced = run("timedatectl", &["show", "--property=NTPSynchronized"]).unwrap_or_default();
    let tz = run("timedatectl", &["show", "--property=Timezone"]).unwrap_or_default();
    let now = run("date", &["+%A, %d %B %Y  %H:%M"]).unwrap_or_default();
    let ntp_on = ntp.trim() == "NTP=yes";
    let synced = synced.trim() == "NTPSynchronized=yes";
    let tz = tz.trim().trim_start_matches("Timezone=").to_string();
    let mut time_rows = vec![toggle(
        "ntp",
        "Set the time automatically",
        &if ntp_on {
            if synced {
                format!("Synced · now {}", now.trim())
            } else {
                "Waiting for the network to sync".into()
            }
        } else {
            format!("Now {}", now.trim())
        },
        ntp_on,
    )];
    let zones = run("timedatectl", &["list-timezones"])
        .map(|s| {
            s.lines()
                .map(str::to_string)
                .collect::<Vec<String>>()
        })
        .unwrap_or_default();
    if !zones.is_empty() {
        time_rows.push(Row {
            kind: "select".into(),
            id: "tz".into(),
            label: "Time zone".into(),
            value: tz.replace('_', " "),
            detail: tz.clone(),
            options: zones,
            selected: 0,
            ..Row::default()
        });
    }
    time_rows.push(Row {
        kind: "seg".into(),
        id: "clock".into(),
        label: "Clock".into(),
        options: vec!["24-hour".into(), "12-hour".into()],
        selected: 0,
        ..Row::default()
    });
    p = p.rows("Time", time_rows);
    if let Some(locale) = locale_info() {
        let mut lang = Row {
            kind: "select".into(),
            id: "language".into(),
            label: "Language".into(),
            value: locale_name(&locale.language),
            detail: locale.language.clone(),
            ..Row::default()
        };
        lang.options = locale.locales.clone();
        let mut formats = Row {
            kind: "select".into(),
            id: "region".into(),
            label: "Formats".into(),
            hint: sample_formats(&locale.region),
            value: locale_name(&locale.region),
            detail: locale.region.clone(),
            ..Row::default()
        };
        formats.options = locale.locales.clone();
        let mut block = group("Language and region");
        block.hint = "Changes here apply the next time you log in.".into();
        block.rows = vec![lang, formats];
        p.push(block)
    } else {
        p
    }
}

struct LocaleInfo {
    locales: Vec<String>,
    language: String,
    region: String,
}

fn locale_info() -> Option<LocaleInfo> {
    let out = run("localectl", &["status"])?;
    let mut language = String::new();
    let mut region = String::new();
    for line in out.lines() {
        if let Some(v) = line.trim().strip_prefix("System Locale:") {
            for pair in v.split_whitespace() {
                if let Some(lang) = pair.strip_prefix("LANG=") {
                    language = lang.to_string();
                }
                if let Some(lc) = pair.strip_prefix("LC_TIME=") {
                    region = lc.to_string();
                }
            }
        }
    }
    if language.is_empty() {
        return None;
    }
    if region.is_empty() {
        region = language.clone();
    }
    let mut locales: Vec<String> = run("locale", &["-a"])
        .map(|s| {
            s.lines()
                .filter(|l| {
                    let l = l.trim();
                    l.contains('.') && !l.starts_with("C.") && !l.starts_with("POSIX")
                })
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    locales.push(language.clone());
    locales.push(region.clone());
    locales.sort();
    locales.dedup();
    Some(LocaleInfo {
        locales,
        language,
        region,
    })
}

fn locale_name(code: &str) -> String {
    let tag = code.split('.').next().unwrap_or(code).replace('_', "-");
    tag
}

fn sample_formats(region: &str) -> String {
    let _tag = region.split('.').next().unwrap_or(region).replace('_', "-");
    let out = run("date", &[&format!("+%x")]).unwrap_or_default();
    format!("{} · sample", out.trim())
}

// ------------------------------------------------------------------------ a11y

fn a11y() -> Page {
    let mut p = page(
        "a11y",
        "System",
        "Accessibility",
        "Make BOS easier to see, hear and use.",
    );
    let prefs = desktop_prefs();
    let zoom = hypr_option("cursor:zoom_factor").unwrap_or(1.0);
    let orca_installed = which("orca");
    let orca_running = Command::new("pgrep")
        .args(["-x", "orca"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    let kmag = which("kmag");
    let mut seeing = vec![toggle(
        "zoom",
        "Zoom",
        "Enlarges everything around the pointer.",
        zoom > 1.0,
    )];
    if zoom > 1.0 {
        seeing.push(Row {
            kind: "slider".into(),
            id: "zoom-level".into(),
            label: "Zoom level".into(),
            value: format!("{}%", (zoom * 100.0).round() as i64),
            amount: zoom,
            min: 1.25,
            max: 8.0,
            ..Row::default()
        });
    }
    seeing.push(Row {
        kind: "seg".into(),
        id: "text-size".into(),
        label: "Text size".into(),
        options: vec!["Default".into(), "Large".into(), "Larger".into()],
        selected: match prefs.text_size {
            16 => 1,
            18 => 2,
            _ => 0,
        },
        ..Row::default()
    });
    seeing.push(Row {
        kind: "seg".into(),
        id: "cursor-size".into(),
        label: "Cursor size".into(),
        options: vec!["Default".into(), "Large".into(), "Larger".into()],
        selected: match prefs.cursor_size {
            32 => 1,
            48 => 2,
            _ => 0,
        },
        ..Row::default()
    });
    seeing.push(toggle(
        "stronger-borders",
        "Stronger window borders",
        "",
        prefs.stronger_borders,
    ));
    p = p.rows("Seeing", seeing);
    p = p.rows(
        "Motion",
        vec![toggle(
            "reduce-motion",
            "Reduce motion",
            "Turns off window and workspace animations.",
            prefs.reduce_motion,
        )],
    );
    p = p.rows(
        "Hearing and reading",
        vec![toggle(
            "orca",
            "Read the screen aloud",
            if orca_installed {
                "Uses the Orca screen reader."
            } else {
                "Uses the Orca screen reader. Installs it the first time, which needs your password."
            },
            orca_running,
        )],
    );
    let _ = kmag;
    p
}

type DesktopPrefs = crate::prefs::Preferences;

fn desktop_prefs() -> DesktopPrefs {
    crate::prefs::load()
}

fn hypr_option(name: &str) -> Option<f32> {
    let out = run("hyprctl", &["getoption", name])?;
    for line in out.lines() {
        if let Some(v) = line.trim().strip_prefix("float:") {
            return v.trim().parse().ok();
        }
        if let Some(v) = line.trim().strip_prefix("int:") {
            return v.trim().parse().ok();
        }
    }
    None
}

// ----------------------------------------------------------------------- users

fn users() -> Page {
    let mut p = page("users", "System", "Users", "People who can log in to this computer.");
    let current = std::env::var("USER").unwrap_or_default();
    let passwd = std::fs::read_to_string("/etc/passwd").unwrap_or_default();
    let mut rows = Vec::new();
    for line in passwd.lines() {
        let fields: Vec<&str> = line.split(':').collect();
        if fields.len() < 7 {
            continue;
        }
        let uid: u32 = fields[2].parse().unwrap_or(0);
        if uid < 1000 || uid == 65534 {
            continue;
        }
        let username = fields[0].to_string();
        let full_name = fields[4].split(',').next().unwrap_or("").to_string();
        let admin = run("id", &["-nG", &username])
            .map(|g| g.split_whitespace().any(|x| x == "wheel"))
            .unwrap_or(false);
        let mut sub = vec![if admin { "Administrator" } else { "Standard account" }.to_string()];
        if !full_name.is_empty() && full_name != username {
            sub.push(username.clone());
        }
        if username == current {
            sub.push("you".into());
        }
        let mut row = Row {
            kind: "user".into(),
            id: format!("user:{username}"),
            label: if full_name.is_empty() { username.clone() } else { full_name.clone() },
            hint: sub.join(" · "),
            action: "Change password".into(),
            ..Row::default()
        };
        row.tag = if username == current { "".into() } else { "Remove".into() };
        let shown = if full_name.is_empty() { &username } else { &full_name };
        row.avatar = shown
            .chars()
            .next()
            .map(|c| c.to_uppercase().to_string())
            .unwrap_or_default();
        rows.push(row);
    }
    let mut add = row("leftbutton", "");
    add.id = "add-user".into();
    add.action = "Add a person".into();
    add.icon = crate::icons::commands("plus").into();
    rows.push(add);
    p.rows("", rows)
}

// --------------------------------------------------------------------- startup

fn startup() -> Page {
    let mut p = page("startup", "Apps", "Startup", "Apps that open when you log in.");
    let dir = config_dir().join("autostart");
    let mut rows = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("desktop") {
                continue;
            }
            let text = std::fs::read_to_string(&path).unwrap_or_default();
            let mut name = String::new();
            let mut exec = String::new();
            let mut hidden = false;
            let mut in_entry = false;
            for line in text.lines().map(str::trim) {
                if line.starts_with('[') {
                    in_entry = line.eq_ignore_ascii_case("[Desktop Entry]");
                } else if in_entry {
                    if let Some(v) = line.strip_prefix("Name=") {
                        name = v.to_string();
                    } else if let Some(v) = line.strip_prefix("Exec=") {
                        exec = v.to_string();
                    } else if line == "Hidden=true" {
                        hidden = true;
                    }
                }
            }
            let label = if name.is_empty() {
                path.file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default()
            } else {
                name
            };
            rows.push(Row {
                kind: "item".into(),
                id: format!("startup:{}", path.display()),
                label,
                hint: if hidden { "Off".into() } else { "Opens at login".into() },
                action: "Edit".into(),
                tag: if hidden { "".into() } else { "toggle".into() },
                checked: !hidden,
                ..Row::default()
            });
        }
    }
    if rows.is_empty() {
        let mut block = group("Your apps");
        block.rows = vec![Row {
            kind: "text".into(),
            id: "empty".into(),
            label: "Nothing opens at login yet.".into(),
            ..Row::default()
        }];
        p = p.push(block);
    } else {
        p = p.rows("Your apps", rows);
    }
    p
}

// --------------------------------------------------------------------- storage

fn storage() -> Page {
    let mut p = page(
        "storage",
        "System",
        "Storage",
        "See what is using space, and clear what you don't need.",
    );
    let (total, used) = disk_usage("/");
    let available = total.saturating_sub(used);
    let pct = if total > 0 { used as f32 / total as f32 } else { 0.0 };
    let mut card = hero(
        &format!("{} free", size(Some(available))),
        &format!("This computer · {} of {} used", size(Some(used)), size(Some(total))),
    );
    card.amount = pct;
    p = p.push(card);
    let package_cache = dir_size(&PathBuf::from("/var/cache/pacman/pkg"));
    let app_cache = dir_size(&home_dir().join(".cache"));
    let trash = dir_size(&home_dir().join(".local/share/Trash"));
    let mut free = vec![
        Row {
            kind: "item".into(),
            id: "package-cache".into(),
            label: "Old package downloads".into(),
            hint: format!("{} · BOS keeps the last 3 versions of each package.", size(package_cache)),
            action: "Clean".into(),
            ..Row::default()
        },
        Row {
            kind: "item".into(),
            id: "app-cache".into(),
            label: "App caches".into(),
            hint: format!("{} · apps rebuild these when needed.", size(app_cache)),
            action: "Review".into(),
            ..Row::default()
        },
        Row {
            kind: "item".into(),
            id: "trash".into(),
            label: "Trash".into(),
            hint: size(trash),
            action: "Empty".into(),
            ..Row::default()
        },
        Row {
            kind: "item".into(),
            id: "unused".into(),
            label: "Unused packages".into(),
            hint: "Installed to support apps that are gone now.".into(),
            action: "Review".into(),
            ..Row::default()
        },
    ];
    free.retain(|_| true);
    p = p.rows("Free up space", free);
    p
}

fn disk_usage(path: &str) -> (u64, u64) {
    let out = Command::new("df")
        .args(["-B1", "--output=size,used", path])
        .output();
    if let Ok(o) = out {
        let text = String::from_utf8_lossy(&o.stdout);
        let fields: Vec<u64> = text
            .lines()
            .nth(1)
            .unwrap_or("")
            .split_whitespace()
            .filter_map(|v| v.parse().ok())
            .collect();
        if fields.len() >= 2 {
            return (fields[0], fields[1]);
        }
    }
    (0, 0)
}

fn dir_size(path: &PathBuf) -> Option<u64> {
    let out = Command::new("du").args(["-sb"]).arg(path).output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    text.split_whitespace().next()?.parse().ok()
}

fn size(bytes: Option<u64>) -> String {
    match bytes {
        None => "Unavailable".into(),
        Some(b) if b < 1024u64.pow(3) => format!("{} MiB", (b / 1024u64.pow(2)) as i64),
        Some(b) => format!("{:.1} GiB", b as f64 / 1024f64.powi(3)),
    }
}

// ----------------------------------------------------------------------- sound

fn pactl_devices(kind: &str) -> Vec<Row> {
    let out = run("pactl", &["list", "short", kind]);
    let mut rows = Vec::new();
    if let Some(text) = out {
        for line in text.lines() {
            let fields: Vec<&str> = line.split('\t').collect();
            if fields.len() < 2 {
                continue;
            }
            let name = fields[1].to_string();
            if name.contains("monitor") {
                continue;
            }
            let desc = pactl_description(kind, &name).unwrap_or_else(|| name.clone());
            rows.push(Row {
                kind: "item".into(),
                id: format!("{kind}:{name}"),
                label: desc,
                hint: "Built-in".into(),
                icon: if kind == "sink" {
                    crate::icons::commands("volume").into()
                } else {
                    crate::icons::commands("mic").into()
                },
                action: "Use".into(),
                ..Row::default()
            });
        }
    }
    if rows.is_empty() {
        rows.push(Row {
            kind: "hint".into(),
            id: format!("{kind}-none"),
            label: "No devices found.".into(),
            ..Row::default()
        });
    }
    rows
}

fn pactl_description(kind: &str, name: &str) -> Option<String> {
    let out = run("pactl", &["list", kind])?;
    let mut current = String::new();
    let mut desc = None;
    for line in out.lines() {
        let t = line.trim();
        if let Some(v) = t.strip_prefix("Name: ") {
            current = v.to_string();
        }
        if let Some(v) = t.strip_prefix("Description: ") {
            if current == name {
                desc = Some(v.to_string());
            }
        }
    }
    desc
}

fn sound() -> Page {
    let mut p = page(
        "sound",
        "Devices",
        "Sound",
        "Where sound goes, what it picks up, and how loud each app is.",
    );
    for (kind, title) in [("sinks", "Output"), ("sources", "Input")] {
        let devices = crate::sound::devices(kind);
        let default = crate::sound::default_device(kind);
        if devices.is_empty() {
            p = p.rows(
                title,
                vec![Row {
                    kind: "hint".into(),
                    id: format!("none:{kind}"),
                    label: "No devices found.".into(),
                    ..Row::default()
                }],
            );
            continue;
        }
        let mut block = group(title);
        block.hint = "Pick where the sound goes.".into();
        for device in &devices {
            let selected = default.as_deref() == Some(device.name.as_str());
            block.rows.push(Row {
                kind: "line".into(),
                id: format!("pick:{kind}:{}", device.name),
                label: device.description.clone(),
                hint: if selected {
                    format!(
                        "In use · {}{}",
                        if device.mute {
                            "muted".into()
                        } else {
                            format!("{}%", device.percent)
                        },
                        if device.virtual_device {
                            " · virtual"
                        } else {
                            ""
                        }
                    )
                } else {
                    device.name.clone()
                },
                action: "Use".into(),
                checked: selected,
                primary: !selected,
                ..Row::default()
            });
        }
        if let Some(current) = devices
            .iter()
            .find(|device| default.as_deref() == Some(device.name.as_str()))
            .or_else(|| devices.first())
        {
            block.rows.push(Row {
                kind: "slider".into(),
                id: format!("vol:{kind}"),
                label: format!("{} volume", title),
                value: if current.mute {
                    "Muted".into()
                } else {
                    format!("{}%", current.percent)
                },
                amount: current.percent as f32,
                min: 0.0,
                max: 150.0,
                ..Row::default()
            });
            block.rows.push(toggle(
                &format!("mute:{kind}"),
                &format!("Mute {title}"),
                "",
                current.mute,
            ));
        }
        p = p.push(block);
    }
    let streams = crate::sound::streams();
    let mut apps = group("Apps");
    apps.hint = if streams.is_empty() {
        "Apps that play sound show up here.".into()
    } else {
        "Volume for each app that is playing sound right now.".into()
    };
    for stream in &streams {
        apps.rows.push(Row {
            kind: "slider".into(),
            id: format!("stream:{}", stream.index),
            label: stream.app.clone(),
            hint: stream.title.clone(),
            value: if stream.mute {
                "Muted".into()
            } else {
                format!("{}%", stream.percent)
            },
            amount: stream.percent as f32,
            min: 0.0,
            max: 150.0,
            ..Row::default()
        });
        apps.rows.push(toggle(
            &format!("stream-mute:{}", stream.index),
            &format!("Mute {}", stream.app),
            "",
            stream.mute,
        ));
    }
    apps.rows.push(Row {
        kind: "leftbutton".into(),
        id: "mixer".into(),
        action: "Open full mixer".into(),
        ..Row::default()
    });
    p.push(apps)
}

fn sound_action(id: &str, action: &str, checked: bool) -> Result<(), String> {
    if id == "mixer" && action == "action" {
        return crate::sound::open_mixer();
    }
    if let Some(rest) = id.strip_prefix("pick:") {
        if action == "action" {
            if let Some((kind, name)) = rest.split_once(':') {
                return crate::sound::set_default(kind, name);
            }
        }
        return Ok(());
    }
    if let Some(kind) = id.strip_prefix("vol:") {
        if let Some(percent) = action.strip_prefix("slider:") {
            let name = crate::sound::default_device(kind)
                .ok_or_else(|| "no default device".to_string())?;
            let value: u32 = percent
                .parse()
                .map_err(|_| "volume must be a number".to_string())?;
            return crate::sound::set_volume(kind, &name, value);
        }
        return Ok(());
    }
    if let Some(kind) = id.strip_prefix("mute:") {
        if action == "toggle" {
            let name = crate::sound::default_device(kind)
                .ok_or_else(|| "no default device".to_string())?;
            return crate::sound::set_mute(kind, &name, !checked);
        }
        return Ok(());
    }
    if let Some(index) = id.strip_prefix("stream:") {
        if let Some(percent) = action.strip_prefix("slider:") {
            let index: u32 = index.parse().map_err(|_| "bad stream".to_string())?;
            let value: u32 = percent
                .parse()
                .map_err(|_| "volume must be a number".to_string())?;
            return crate::sound::set_stream_volume(index, value);
        }
        return Ok(());
    }
    if let Some(index) = id.strip_prefix("stream-mute:") {
        if action == "toggle" {
            let index: u32 = index.parse().map_err(|_| "bad stream".to_string())?;
            return crate::sound::set_stream_mute(index, !checked);
        }
    }
    Ok(())
}

// ----------------------------------------------------------------------- power

fn power() -> Page {
    let mut p = page(
        "power",
        "Devices",
        "Power & battery",
        "Battery, performance, and what happens when you step away.",
    );
    let info = data_battery();
    let mut h = hero(&info.headline, &info.detail);
    h.amount = info.percent.unwrap_or(-1.0);
    let (chart, area) = battery_chart();
    h.chart = chart;
    h.chart_area = area;
    h.arc = ring_arc(info.percent.unwrap_or(0.0));
    p = p.push(h);
    let mut performance = group("Performance");
    performance.rows = vec![
        Row {
            kind: "seg".into(),
            id: "power-mode".into(),
            label: "Power mode".into(),
            options: vec!["Battery saver".into(), "Balanced".into(), "Performance".into()],
            selected: power_mode_index(),
            ..Row::default()
        },
        toggle(
            "auto-power-mode",
            "Switch automatically",
            "Performance when plugged in. Balanced on battery, Battery saver below 20%.",
            false,
        ),
    ];
    p = p.push(performance);
    p = p.rows(
        "Screen",
        vec![Row {
            kind: "slider".into(),
            id: "brightness".into(),
            label: "Brightness".into(),
            value: format!("{}%", brightness_percent()),
            amount: brightness_percent() as f32,
            min: 0.0,
            max: 100.0,
            ..Row::default()
        }],
    );
    p = p.rows(
        "Battery",
        vec![toggle(
            "battery-protect",
            "Protect battery",
            "Stops charging at about 80% so the battery lasts longer. Needs your password.",
            false,
        )],
    );
    let mut away = group("When you step away");
    away.rows = vec![
        Row {
            kind: "select".into(),
            id: "dim".into(),
            label: "Dim the screen after".into(),
            value: "Never".into(),
            options: timeout_options(),
            ..Row::default()
        },
        Row {
            kind: "select".into(),
            id: "screen-off".into(),
            label: "Turn off the screen after".into(),
            value: "Never".into(),
            options: timeout_options(),
            ..Row::default()
        },
        Row {
            kind: "select".into(),
            id: "lock-after".into(),
            label: "Lock after".into(),
            value: "Never".into(),
            options: timeout_options(),
            ..Row::default()
        },
        Row {
            kind: "select".into(),
            id: "sleep-after".into(),
            label: "Sleep after".into(),
            value: "Never".into(),
            options: timeout_options(),
            ..Row::default()
        },
        toggle(
            "sleep-on-battery",
            "Sleep only on battery",
            "Stay awake while plugged in.",
            true,
        ),
    ];
    p = p.push(away);
    let mut buttons = group("Buttons and lid");
    buttons.hint = "These two take effect after you restart the computer.".into();
    buttons.rows = vec![
        Row {
            kind: "seg".into(),
            id: "lid".into(),
            label: "When I close the lid".into(),
            options: vec!["Sleep".into(), "Lock".into(), "Do nothing".into()],
            selected: 0,
            ..Row::default()
        },
        Row {
            kind: "seg".into(),
            id: "power-button".into(),
            label: "Power button".into(),
            options: vec![
                "Show menu".into(),
                "Sleep".into(),
                "Power off".into(),
                "Do nothing".into(),
            ],
            selected: 0,
            ..Row::default()
        },
    ];
    p.push(buttons)
}

fn timeout_options() -> Vec<String> {
    vec![
        "Never".into(),
        "1 minute".into(),
        "5 minutes".into(),
        "10 minutes".into(),
        "20 minutes".into(),
        "30 minutes".into(),
        "1 hour".into(),
    ]
}

fn hours_minutes(hours: f64) -> String {
    let total = (hours * 60.0).round() as i64;
    let (h, m) = (total / 60, total % 60);
    match (h, m) {
        (0, m) => format!("{m} m"),
        (h, 0) => format!("{h} h"),
        (h, m) => format!("{h} h {m} m"),
    }
}

fn capitalise(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

struct BatteryInfo {
    headline: String,
    detail: String,
    percent: Option<f32>,
}

// A 62px hero ring: start at twelve o'clock and sweep for `percent`.
fn ring_arc(percent: f32) -> String {
    let fraction = (percent / 100.0).clamp(0.0, 1.0);
    if fraction <= 0.0 {
        return String::new();
    }
    if fraction >= 0.999 {
        return "M 31 2 A 29 29 0 1 1 31 60 A 29 29 0 1 1 31 2".into();
    }
    let angle = (-90.0 + 360.0 * fraction).to_radians();
    let x = 31.0 + 29.0 * angle.cos();
    let y = 31.0 + 29.0 * angle.sin();
    let large = if fraction > 0.5 { 1 } else { 0 };
    format!("M 31 2 A 29 29 0 {large} 1 {x:.3} {y:.3}")
}

// The shipping hero draws the last 24 hours from UPower's charge history.
fn battery_chart() -> (String, String) {
    let Some(devices) = run("upower", &["-e"]) else {
        return (String::new(), String::new());
    };
    let Some(device) = devices.lines().find(|l| l.contains("battery_")) else {
        return (String::new(), String::new());
    };
    let Some(text) = run(
        "busctl",
        &[
            "--system",
            "--json=short",
            "call",
            "org.freedesktop.UPower",
            device.trim(),
            "org.freedesktop.UPower.Device",
            "GetHistory",
            "suu",
            "charge",
            "86400",
            "100",
        ],
    ) else {
        return (String::new(), String::new());
    };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) else {
        return (String::new(), String::new());
    };
    let Some(rows) = json
        .get("data")
        .and_then(|d| d.get(0))
        .and_then(|d| d.as_array())
    else {
        return (String::new(), String::new());
    };
    let mut points: Vec<(f64, f64)> = rows
        .iter()
        .filter_map(|row| {
            let t = row.get(0)?.as_f64()?;
            let pct = row.get(1)?.as_f64()?;
            Some((t, pct))
        })
        .collect();
    points.sort_by(|a, b| a.0.total_cmp(&b.0));
    if points.len() < 2 {
        return (String::new(), String::new());
    }
    let (t0, t1) = (points[0].0, points[points.len() - 1].0);
    let span = (t1 - t0).max(1.0);
    let mut line = String::new();
    for (i, (t, pct)) in points.iter().enumerate() {
        let x = (t - t0) / span * 200.0;
        let y = 48.0 - pct * 0.48;
        line.push_str(&format!(
            "{}{x:.1} {y:.1}",
            if i == 0 { "M" } else { " L" }
        ));
    }
    let area = format!("{line} L200 48 L0 48 Z");
    (line, area)
}

fn data_battery() -> BatteryInfo {
    let out = run("upower", &["-e"]).unwrap_or_default();
    let path = out
        .lines()
        .find(|l| l.contains("battery_"))
        .unwrap_or("")
        .trim()
        .to_string();
    if path.is_empty() {
        return BatteryInfo {
            headline: "Plugged in".into(),
            detail: "No battery found.".into(),
            percent: None,
        };
    }
    let info = run("upower", &["-i", &path]).unwrap_or_default();
    let field = |name: &str| {
        info.lines().find_map(|line| {
            let (k, v) = line.split_once(':')?;
            (k.trim() == name).then(|| v.trim().to_string())
        })
    };
    let percent = field("percentage")
        .and_then(|v| v.trim_end_matches('%').trim().parse::<f32>().ok());
    let state = field("state").unwrap_or_default();
    let rate = field("energy-rate")
        .map(|v| {
            v.trim_end_matches(" W")
                .parse::<f32>()
                .map(|w| format!("{w:.1} W"))
                .unwrap_or(v)
        })
        .unwrap_or_default();
    let health = field("capacity").unwrap_or_default();
    let time = match state.as_str() {
        "charging" => field("time to full"),
        _ => field("time to empty"),
    }
    .filter(|s| !s.is_empty());
    let headline = if let Some(t) = time {
        let hours: f64 = t
            .split_whitespace()
            .next()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0.0);
        if state == "charging" {
            format!("Full in {}", hours_minutes(hours))
        } else {
            format!("about {} left", hours_minutes(hours))
        }
    } else if state == "charging" {
        "Charging".into()
    } else {
        "Plugged in".into()
    };
    let mut parts = Vec::new();
    if !state.is_empty() {
        parts.push(capitalise(&state));
    }
    if !rate.is_empty() {
        parts.push(format!("drawing {rate}"));
    }
    if !health.is_empty() {
        let rounded = health
            .trim_end_matches('%')
            .parse::<f32>()
            .map(|v| format!("{v:.0}"))
            .unwrap_or_else(|_| health.clone());
        parts.push(format!("battery health {rounded}%"));
    }
    BatteryInfo {
        headline,
        detail: parts.join(" · "),
        percent,
    }
}

fn power_mode_index() -> i32 {
    let profile = std::fs::read_to_string("/sys/firmware/acpi/platform_profile")
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    match profile.as_str() {
        "low-power" | "quiet" => 0,
        "performance" => 2,
        _ => 1,
    }
}

fn brightness_percent() -> u32 {
    let out = run("brightnessctl", &["-m"]).unwrap_or_default();
    out.trim()
        .split(',')
        .find_map(|field| {
            field
                .trim()
                .strip_suffix('%')
                .and_then(|v| v.parse::<u32>().ok())
        })
        .unwrap_or(100)
}

// -------------------------------------------------------------------- displays

fn display_draft_cell() -> &'static Mutex<Option<Vec<crate::displays::Monitor>>> {
    static DRAFT: OnceLock<Mutex<Option<Vec<crate::displays::Monitor>>>> = OnceLock::new();
    DRAFT.get_or_init(|| Mutex::new(None))
}

fn display_draft() -> Vec<crate::displays::Monitor> {
    let mut draft = display_draft_cell()
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    draft
        .get_or_insert_with(crate::displays::load)
        .clone()
}

fn set_display_draft(layout: Vec<crate::displays::Monitor>) {
    *display_draft_cell()
        .lock()
        .unwrap_or_else(|e| e.into_inner()) = Some(layout);
}

fn reset_display_draft() {
    *display_draft_cell()
        .lock()
        .unwrap_or_else(|e| e.into_inner()) = None;
}

fn parse_mode(mode: &str) -> Option<(u32, u32, f64)> {
    let (size, refresh) = mode.split_once('@')?;
    let (width, height) = size.split_once('x')?;
    Some((
        width.parse().ok()?,
        height.parse().ok()?,
        refresh.trim_end_matches("Hz").parse().ok()?,
    ))
}

fn displays() -> Page {
    let mut p = page(
        "displays",
        "Devices",
        "Displays",
        "Arrange your monitors. Every change can be kept or reverted.",
    );
    let layout = display_draft();
    let pending = crate::displays::status();
    if pending.id != 0 {
        let mut block = group("Keep this layout?");
        block.hint = format!(
            "Reverting in about {} seconds puts the previous arrangement back.",
            pending.seconds_left
        );
        block.rows = vec![
            Row {
                kind: "line".into(),
                id: format!("keep:{}", pending.id),
                label: "Keep this layout".into(),
                action: "Keep".into(),
                primary: true,
                ..Row::default()
            },
            Row {
                kind: "line".into(),
                id: format!("revert:{}", pending.id),
                label: "Go back to how it was".into(),
                action: "Revert".into(),
                ..Row::default()
            },
        ];
        p = p.push(block);
    }
    if layout.is_empty() {
        p = p.rows(
            "Arrangement",
            vec![Row {
                kind: "hint".into(),
                id: "no-monitors".into(),
                label: "No monitors detected.".into(),
                ..Row::default()
            }],
        );
    }
    let names: Vec<String> = layout.iter().map(|m| m.name.clone()).collect();
    for (index, monitor) in layout.iter().enumerate() {
        let mut block = group(&monitor.name);
        block.hint = format!(
            "{}x{} at {},{}{}",
            monitor.width,
            monitor.height,
            monitor.x,
            monitor.y,
            if monitor.disabled { " · off" } else { "" }
        );
        block.rows.push(toggle(
            &format!("on:{}", monitor.name),
            "Use this display",
            "",
            !monitor.disabled,
        ));
        if !monitor.disabled {
            block.rows.push(Row {
                kind: "select".into(),
                id: format!("mode:{}", monitor.name),
                label: "Resolution".into(),
                value: crate::displays::current_mode(monitor),
                ..Row::default()
            });
            let scales = [1.0, 1.25, 1.5, 2.0];
            let scale = scales
                .iter()
                .copied()
                .min_by(|a, b| {
                    (a - monitor.scale)
                        .abs()
                        .partial_cmp(&(b - monitor.scale).abs())
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .unwrap_or(1.0);
            block.rows.push(Row {
                kind: "select".into(),
                id: format!("scale:{}", monitor.name),
                label: "Scale".into(),
                value: format!("{}%", (scale * 100.0).round() as u32),
                ..Row::default()
            });
            let turns = ["Landscape", "Right", "Upside down", "Left"];
            block.rows.push(Row {
                kind: "select".into(),
                id: format!("turn:{}", monitor.name),
                label: "Rotation".into(),
                value: turns
                    .get(monitor.transform as usize)
                    .copied()
                    .unwrap_or("Landscape")
                    .into(),
                ..Row::default()
            });
            let mut mirrors = vec!["None".to_string()];
            mirrors.extend(
                names
                    .iter()
                    .filter(|name| {
                        *name != &monitor.name
                            && layout
                                .iter()
                                .find(|other| &other.name == *name)
                                .map(|other| other.mirror_of.is_empty() && !other.disabled)
                                .unwrap_or(false)
                    })
                    .cloned(),
            );
            block.rows.push(Row {
                kind: "select".into(),
                id: format!("mirror:{}", monitor.name),
                label: "Mirror".into(),
                value: if monitor.mirror_of.is_empty() {
                    "None".into()
                } else {
                    monitor.mirror_of.clone()
                },
                ..Row::default()
            });
            block.rows.push(toggle(
                &format!("sync:{}", monitor.name),
                "Adaptive sync",
                "",
                monitor.vrr > 0,
            ));
            block.rows.push(Row {
                kind: "input".into(),
                id: format!("x:{}", monitor.name),
                label: "Horizontal position".into(),
                value: monitor.x.to_string(),
                action: "Set".into(),
                ..Row::default()
            });
            block.rows.push(Row {
                kind: "input".into(),
                id: format!("y:{}", monitor.name),
                label: "Vertical position".into(),
                value: monitor.y.to_string(),
                action: "Set".into(),
                ..Row::default()
            });
        }
        let _ = index;
        p = p.push(block);
    }
    if !layout.is_empty() {
        p = p.rows(
            "Layout",
            vec![
                Row {
                    kind: "leftbutton".into(),
                    id: "apply".into(),
                    action: "Apply layout".into(),
                    primary: true,
                    ..Row::default()
                },
                Row {
                    kind: "leftbutton".into(),
                    id: "discard".into(),
                    action: "Discard changes".into(),
                    ..Row::default()
                },
            ],
        );
    }
    let night = crate::nightlight::load().enabled;
    let mut night_block = group("NIGHT LIGHT");
    night_block.hint = "A warmer screen after dark helps you wind down.".into();
    night_block.rows = vec![toggle("night-light", "Night light", "", night)];
    p.push(night_block)
}

fn displays_action(id: &str, action: &str) -> Result<(), String> {
    if action == "toggle" && id == "night-light" {
        let mut cfg = crate::nightlight::load();
        cfg.enabled = !cfg.enabled;
        return crate::nightlight::save(&cfg);
    }
    if id == "discard" {
        reset_display_draft();
        return Ok(());
    }
    if id == "apply" && action == "action" {
        let layout = display_draft();
        crate::displays::apply(layout, 15)?;
        return Ok(());
    }
    if let Some(value) = id.strip_prefix("keep:") {
        let number: u64 = value.parse().map_err(|_| "bad change number".to_string())?;
        crate::displays::keep(number)?;
        reset_display_draft();
        return Ok(());
    }
    if let Some(value) = id.strip_prefix("revert:") {
        let number: u64 = value.parse().map_err(|_| "bad change number".to_string())?;
        let result = crate::displays::revert(number);
        reset_display_draft();
        return result;
    }
    let mut layout = display_draft();
    let Some((field, name)) = id.split_once(':') else {
        return Ok(());
    };
    let active_count = layout.iter().filter(|monitor| !monitor.disabled).count();
    let mirror_choices: Vec<String> = layout
        .iter()
        .filter(|other| other.name != name && other.mirror_of.is_empty() && !other.disabled)
        .map(|other| other.name.clone())
        .collect();
    let Some(monitor) = layout.iter_mut().find(|monitor| monitor.name == name) else {
        return Ok(());
    };
    match field {
        "on" if action == "toggle" => {
            if monitor.disabled && active_count == 0 {
                return Err("at least one display has to stay on".into());
            }
            monitor.disabled = !monitor.disabled;
        }
        "mode" if action == "select" => {
            let current = crate::displays::current_mode(monitor);
            let index = monitor
                .available_modes
                .iter()
                .position(|mode| mode == &current)
                .map(|index| (index + 1) % monitor.available_modes.len())
                .unwrap_or(0);
            if let Some(mode) = monitor.available_modes.get(index) {
                if let Some((width, height, refresh)) = parse_mode(mode) {
                    monitor.width = width;
                    monitor.height = height;
                    monitor.refresh = refresh;
                }
            }
        }
        "scale" if action == "select" => {
            let scales = [1.0, 1.25, 1.5, 2.0];
            let index = scales
                .iter()
                .position(|scale| (scale - monitor.scale).abs() < 0.01)
                .map(|index| (index + 1) % scales.len())
                .unwrap_or(0);
            monitor.scale = scales[index];
        }
        "turn" if action == "select" => monitor.transform = (monitor.transform + 1) % 4,
        "mirror" if action == "select" => {
            let mut choices = vec![String::new()];
            choices.extend(mirror_choices.iter().cloned());
            let index = choices
                .iter()
                .position(|choice| choice == &monitor.mirror_of)
                .map(|index| (index + 1) % choices.len())
                .unwrap_or(0);
            monitor.mirror_of = choices[index].clone();
        }
        "sync" if action == "toggle" => monitor.vrr = if monitor.vrr > 0 { 0 } else { 1 },
        "x" if action.starts_with("input:") => {
            monitor.x = action
                .trim_start_matches("input:")
                .trim()
                .parse()
                .map_err(|_| "positions must be whole numbers".to_string())?;
        }
        "y" if action.starts_with("input:") => {
            monitor.y = action
                .trim_start_matches("input:")
                .trim()
                .parse()
                .map_err(|_| "positions must be whole numbers".to_string())?;
        }
        _ => return Ok(()),
    }
    set_display_draft(layout);
    Ok(())
}

fn monitor_count() -> usize {
    let out = run("hyprctl", &["monitors", "-j"]).unwrap_or_default();
    out.matches("\"name\"").count()
}

// ------------------------------------------------------------------- bluetooth

fn sharing_action(id: &str, action: &str, checked: bool) -> Result<(), String> {
    if id == "ssh" && action == "toggle" {
        let verb = if checked { "disable" } else { "enable" };
        let out = Command::new("systemctl")
            .args([verb, "--now", "sshd.service"])
            .output()
            .map_err(|error| error.to_string())?;
        return if out.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
        };
    }
    if id == "hostname" && action.starts_with("input:") {
        let name = action.trim_start_matches("input:").trim();
        if name.is_empty() || name.contains(char::is_whitespace) {
            return Err("enter a single name".into());
        }
        let out = Command::new("hostnamectl")
            .args(["set-hostname", name])
            .output()
            .map_err(|error| error.to_string())?;
        return if out.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
        };
    }
    if id == "receive-files" && action == "action" {
        let flatpak = run("flatpak", &["info", "org.localsend.localsend_app"]).is_some();
        let mut command = if which("localsend") {
            Command::new("localsend")
        } else if flatpak {
            let mut command = Command::new("flatpak");
            command.args(["run", "org.localsend.localsend_app"]);
            command
        } else {
            return Err("install LocalSend first".into());
        };
        command
            .spawn()
            .map(|_| ())
            .map_err(|error| error.to_string())?;
        return Ok(());
    }
    if id == "phone-link" && action == "action" {
        if !which("kdeconnect-settings") {
            return Err("install KDE Connect first".into());
        }
        return Command::new("kdeconnect-settings")
            .spawn()
            .map(|_| ())
            .map_err(|error| error.to_string());
    }
    if id == "firewall" && action == "action" {
        return Ok(());
    }
    Ok(())
}

fn bluetooth() -> Page {
    use crate::bluetooth::State;
    let mut p = page(
        "bluetooth",
        "Connect",
        "Bluetooth",
        "Your devices, with battery where they report it.",
    );
    let state = crate::bluetooth::state();
    let mut devices = crate::bluetooth::devices();
    devices.retain(|device| device.paired);
    let connected = devices.iter().filter(|device| device.connected).count();
    let mut h = match &state {
        State::ServiceOff => hero(
            "Bluetooth service is off",
            "This computer has Bluetooth, but the service that runs it isn't started. Starting it asks for your password.",
        ),
        State::NoAdapter => hero(
            "No Bluetooth adapter found",
            "This computer doesn't have Bluetooth, or it is switched off in the firmware settings.",
        ),
        State::Ready { powered: false } => hero(
            "Bluetooth is off",
            "Turn it on to use your headphones, controllers and other devices.",
        ),
        State::Ready { powered: true } => hero(
            "Bluetooth is on",
            &format!("{connected} connected · {} paired", devices.len()),
        ),
    };
    h.icon = crate::icons::commands("bluetooth").into();
    if let State::Ready { powered } = state {
        h.tag = "toggle".into();
        h.on = powered;
        if powered {
            h.action = "Add device".into();
            h.action_icon = crate::icons::commands("plus").into();
            h.primary = true;
        }
    }
    p = p.push(h);

    match state {
        State::ServiceOff => {
            p = p.rows(
                "Bluetooth service",
                vec![
                    Row {
                        kind: "line".into(),
                        id: "bt-start".into(),
                        label: "Start Bluetooth".into(),
                        hint: "Starts the background service now.".into(),
                        action: "Start".into(),
                        primary: true,
                        ..Row::default()
                    },
                    Row {
                        kind: "line".into(),
                        id: "bt-check".into(),
                        label: "Check again".into(),
                        action: "Check".into(),
                        ..Row::default()
                    },
                ],
            );
            return p;
        }
        State::NoAdapter => return p,
        State::Ready { powered: false } => return p,
        State::Ready { powered: true } => {}
    }

    let adding = bluetooth_adding();
    if adding {
        let found = bluetooth_found();
        let mut block = group("Add a device");
        block.hint = "Put your device in pairing mode. It shows up here.".into();
        if found.is_empty() {
            block.rows.push(Row {
                kind: "hint".into(),
                id: "bt-none".into(),
                label: "Nothing found yet. Check the device is in pairing mode, then scan again."
                    .into(),
                ..Row::default()
            });
        }
        for device in found.iter().filter(|device| device.paired == false) {
            block.rows.push(Row {
                kind: "line".into(),
                id: format!("pair:{}", device.address),
                label: device.name.clone(),
                hint: device.address.clone(),
                action: "Pair".into(),
                ..Row::default()
            });
        }
        block.rows.push(Row {
            kind: "line".into(),
            id: "bt-scan".into(),
            label: "Scan again".into(),
            action: "Scan".into(),
            ..Row::default()
        });
        block.rows.push(Row {
            kind: "leftbutton".into(),
            id: "bt-done".into(),
            action: "Done".into(),
            primary: true,
            ..Row::default()
        });
        p = p.push(block);
    }

    let mut block = Block {
        kind: "devgrid".into(),
        ..Block::default()
    };
    let mut seen: HashMap<String, usize> = HashMap::new();
    for device in &devices {
        *seen.entry(device.name.clone()).or_default() += 1;
    }
    for device in devices {
        block.rows.push(Row {
            kind: "dev".into(),
            id: format!("bt:{}", device.address),
            label: device.name.clone(),
            hint: format!(
                "{}{}{}",
                if device.connected {
                    "Connected"
                } else {
                    "Not connected"
                },
                device
                    .battery
                    .map(|battery| format!(" · {battery}% battery"))
                    .unwrap_or_default(),
                if seen.get(&device.name).copied().unwrap_or(0) > 1 {
                    format!(" · {}", short_address(&device.address))
                } else {
                    String::new()
                }
            ),
            icon: crate::icons::commands("headphones").into(),
            action: if device.connected {
                "Disconnect".into()
            } else {
                "Connect".into()
            },
            value: "Forget".into(),
            checked: device.connected,
            primary: !device.connected,
            ..Row::default()
        });
    }
    p.push(block)
}

fn bluetooth_adding_cell() -> &'static Mutex<bool> {
    static ADDING: OnceLock<Mutex<bool>> = OnceLock::new();
    ADDING.get_or_init(|| Mutex::new(false))
}

fn bluetooth_adding() -> bool {
    *bluetooth_adding_cell()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

fn bluetooth_found_cell() -> &'static Mutex<Vec<crate::bluetooth::Device>> {
    static FOUND: OnceLock<Mutex<Vec<crate::bluetooth::Device>>> = OnceLock::new();
    FOUND.get_or_init(|| Mutex::new(Vec::new()))
}

fn bluetooth_found() -> Vec<crate::bluetooth::Device> {
    bluetooth_found_cell()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
}

fn bluetooth_action(id: &str, action: &str, checked: bool) -> Result<(), String> {
    if id == "bt-start" && action == "action" {
        crate::bluetooth::start_service()?;
        return Ok(());
    }
    if id == "bt-check" {
        return Ok(());
    }
    if id == "add-device" && action == "action" {
        let found = crate::bluetooth::scan();
        *bluetooth_found_cell()
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = found;
        *bluetooth_adding_cell()
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = true;
        return Ok(());
    }
    if id == "bt-scan" && action == "action" {
        let found = crate::bluetooth::scan();
        *bluetooth_found_cell()
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = found;
        return Ok(());
    }
    if id == "bt-done" {
        *bluetooth_adding_cell()
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = false;
        return Ok(());
    }
    if let Some(address) = id.strip_prefix("pair:") {
        if action == "action" {
            crate::bluetooth::pair(address)?;
            *bluetooth_adding_cell()
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = false;
        }
        return Ok(());
    }
    if let Some(address) = id.strip_prefix("bt:") {
        return match action {
            "action2" => crate::bluetooth::forget(address),
            _ => crate::bluetooth::connect(address, !checked),
        };
    }
    Ok(())
}

fn short_address(address: &str) -> String {
    let tail: String = address
        .chars()
        .rev()
        .take(5)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!("…{tail}")
}

fn bluetooth_devices() -> Vec<(String, String, bool, Option<u32>)> {
    let out = run("bluetoothctl", &["devices"]).unwrap_or_default();
    let mut rows = Vec::new();
    for line in out.lines() {
        let Some(rest) = line.strip_prefix("Device ") else {
            continue;
        };
        let Some((address, name)) = rest.split_once(' ') else {
            continue;
        };
        let connected = run("bluetoothctl", &["info", address])
            .map(|s| s.lines().any(|l| l.trim() == "Connected: yes"))
            .unwrap_or(false);
        let battery = run("bluetoothctl", &["info", address]).and_then(|s| {
            s.lines().find_map(|l| {
                l.trim()
                    .strip_prefix("Battery Percentage:")
                    .and_then(|v| v.split('(').next())
                    .and_then(|v| v.trim().parse::<u32>().ok())
            })
        });
        rows.push((
            name.trim().to_string(),
            address.to_string(),
            connected,
            battery,
        ));
    }
    rows
}

// --------------------------------------------------------------------- network

fn network() -> Page {
    let mut p = page(
        "network",
        "Connect",
        "Wi-Fi & network",
        "Connected networks, places that switch settings for you, and VPN.",
    );
    let info = crate::network::info();
    let (places, current_place) = crate::network::places();
    let scans = if info.radio {
        crate::network::scan()
    } else {
        Vec::new()
    };
    let active = scans.iter().find(|net| net.active);
    let mut h = if let Some(ssid) = &info.ssid {
        hero(
            ssid,
            &format!(
                "Connected{} · {}",
                active
                    .map(|net| if net.secured { " · secured" } else { " · open network" })
                    .unwrap_or(""),
                current_place.as_deref().unwrap_or("")
            ),
        )
    } else if let Some(ethernet) = &info.ethernet {
        hero(
            "Wired connection",
            &format!(
                "Connected by cable{}{}",
                if info.radio { " · Wi-Fi is on" } else { "" },
                current_place
                    .as_deref()
                    .map(|place| format!(" · {place}"))
                    .unwrap_or_default()
            ),
        )
    } else if info.radio {
        hero("Not connected", "Pick a network below.")
    } else {
        hero("Wi-Fi is off", "Turn it on to see networks.")
    };
    h.icon = crate::icons::commands(if info.ethernet.is_some() {
        "cable"
    } else {
        "wifi"
    })
    .into();
    h.tag = "toggle".into();
    h.on = info.radio;
    p = p.push(h);

    let mut places_block = group("Places");
    places_block.hint =
        "A place is a set of networks and rules. BOS picks the right one when it sees a familiar network nearby."
            .into();
    let mut chip_options: Vec<String> = places.iter().map(|place| place.name.clone()).collect();
    chip_options.push("Add a place".into());
    places_block.rows.push(Row {
        kind: "chips".into(),
        id: "place-tabs".into(),
        options: chip_options,
        selected: places
            .iter()
            .position(|place| Some(place.name.as_str()) == current_place.as_deref())
            .map(|index| index as i32)
            .unwrap_or(-1),
        ..Row::default()
    });
    let default_profile = crate::network::advanced().default_profile;
    for place in &places {
        let in_use = current_place.as_deref() == Some(place.name.as_str());
        let mut badges = Vec::new();
        if in_use {
            badges.push("in use".into());
        }
        if place.name == default_profile {
            badges.push("Fallback".into());
        }
        places_block.rows.push(Row {
            kind: "line".into(),
            id: format!("place:{}", place.name),
            label: place.name.clone(),
            hint: if place.networks.is_empty() {
                "No networks yet.".into()
            } else {
                format!("Join {}.", place.networks.join(", "))
            },
            action: "Edit".into(),
            badges,
            ..Row::default()
        });
    }
    if let Some(editing_name) = network_editing() {
        if let Some(place) = places.iter().find(|place| place.name == editing_name) {
            places_block.rows.extend(place_editor_rows(place, &default_profile));
        }
    }
    places_block.rows.push(Row {
        kind: "leftbutton".into(),
        id: "add-place".into(),
        action: "Add a place".into(),
        icon: crate::icons::commands("plus").into(),
        ..Row::default()
    });
    p = p.push(places_block);

    let mut nearby = group("Nearby networks");
    nearby.hint = if !info.radio {
        "Turn on Wi-Fi to see networks.".into()
    } else if scans.is_empty() {
        "No networks found.".into()
    } else {
        "".into()
    };
    let pending = network_pending();
    if let Some(ssid) = pending.as_deref() {
        nearby.rows.push(Row {
            kind: "input".into(),
            id: "password".into(),
            label: format!("Password for {ssid}"),
            hint: "Press Enter to join.".into(),
            action: "Join".into(),
            ..Row::default()
        });
    }
    for net in scans.iter().filter(|net| !net.active) {
        let strength = match net.signal {
            value if value >= 75 => "strong signal",
            value if value >= 50 => "good signal",
            value if value >= 25 => "weak signal",
            _ => "very weak signal",
        };
        nearby.rows.push(Row {
            kind: "line".into(),
            id: format!("wifi:{}", net.ssid),
            label: net.ssid.clone(),
            hint: format!(
                "{}{} · {} · {}%",
                if net.known { "Saved · " } else { "" },
                if net.secured { "Secured" } else { "Open" },
                strength,
                net.signal
            ),
            action: "Connect".into(),
            ..Row::default()
        });
    }
    nearby.rows.push(Row {
        kind: "line".into(),
        id: "scan".into(),
        label: "Scan again".into(),
        hint: "Look for networks that appeared since this page opened.".into(),
        action: "Scan".into(),
        enabled: info.radio,
        ..Row::default()
    });
    p = p.push(nearby);

    let saved = crate::network::breadcrumbs_networks();
    let mut saved_block = group(&format!("Saved networks ({})", saved.len()));
    if saved.is_empty() {
        saved_block.rows.push(Row {
            kind: "hint".into(),
            id: "no-saved".into(),
            label: "No saved networks.".into(),
            ..Row::default()
        });
    }
    for network in &saved {
        saved_block.rows.push(Row {
            kind: "line".into(),
            id: format!("saved:{}", network.ssid),
            label: network.ssid.clone(),
            hint: format!(
                "{}{}",
                if network.hidden { "Hidden network" } else { "Saved" },
                if network.has_password {
                    " · password saved"
                } else {
                    ""
                }
            ),
            action: "Edit".into(),
            ..Row::default()
        });
    }
    if let Some(editing) = network_saved_edit() {
        let current = saved.iter().find(|network| network.ssid == editing);
        saved_block.rows.push(Row {
            kind: "input".into(),
            id: "sn-ssid".into(),
            label: "Network name".into(),
            value: current.map(|network| network.ssid.clone()).unwrap_or_default(),
            action: "Save".into(),
            ..Row::default()
        });
        saved_block.rows.push(Row {
            kind: "input".into(),
            id: "sn-password".into(),
            label: "Password".into(),
            hint: "Leave empty to keep the current one.".into(),
            action: "Save".into(),
            ..Row::default()
        });
        saved_block.rows.push(Row {
            kind: "line".into(),
            id: "sn-hidden".into(),
            label: "Hidden network".into(),
            hint: "It doesn't announce its name.".into(),
            tag: "toggle".into(),
            checked: current.map(|network| network.hidden).unwrap_or(false),
            ..Row::default()
        });
        if current.is_some() {
            saved_block.rows.push(Row {
                kind: "line".into(),
                id: "sn-remove".into(),
                label: "Forget this network".into(),
                action: "Remove".into(),
                ..Row::default()
            });
        }
        saved_block.rows.push(Row {
            kind: "leftbutton".into(),
            id: "sn-done".into(),
            action: "Done".into(),
            primary: true,
            ..Row::default()
        });
    }
    saved_block.rows.push(Row {
        kind: "leftbutton".into(),
        id: "add-network".into(),
        action: "Add a network".into(),
        icon: crate::icons::commands("plus").into(),
        ..Row::default()
    });
    p = p.push(saved_block);

    let mut vpn = group("VPN");
    let vpns = crate::network::vpns();
    if vpns.is_empty() {
        vpn.rows.push(Row {
            kind: "hint".into(),
            id: "no-vpn".into(),
            label: "No VPNs set up. Add a WireGuard .conf or OpenVPN .ovpn file.".into(),
            ..Row::default()
        });
    }
    for connection in vpns {
        vpn.rows.push(Row {
            kind: "line".into(),
            id: format!("vpn:{}", connection.name),
            label: connection.name,
            hint: format!(
                "{}{}",
                match connection.kind.as_str() {
                    "wireguard" => "WireGuard",
                    "vpn" => "VPN",
                    other => other,
                },
                if connection.autoconnect {
                    " · connects by itself"
                } else {
                    ""
                }
            ),
            tag: "toggle".into(),
            checked: connection.active,
            ..Row::default()
        });
    }
    vpn.rows.push(Row {
        kind: "leftbutton".into(),
        id: "add-vpn".into(),
        action: "Add VPN".into(),
        icon: crate::icons::commands("plus").into(),
        ..Row::default()
    });
    p = p.push(vpn);

    let advanced = crate::network::advanced();
    let service = crate::network::breadcrumbs_service();
    p = p.rows(
        "Advanced network settings",
        vec![
            Row {
                kind: "input".into(),
                id: "adv-dns".into(),
                label: "DNS servers".into(),
                hint: "Separate several with commas. Empty leaves each network's own DNS alone."
                    .into(),
                value: advanced.dns,
                action: "Save".into(),
                ..Row::default()
            },
            Row {
                kind: "input".into(),
                id: "adv-exit-node".into(),
                label: "Tailscale exit node".into(),
                hint: "Used by places that send traffic through Tailscale.".into(),
                value: advanced.exit_node,
                action: "Save".into(),
                ..Row::default()
            },
            Row {
                kind: "input".into(),
                id: "adv-connectivity".into(),
                label: "Sign-in page check".into(),
                hint: "An address that answers when the internet works.".into(),
                value: advanced.connectivity_url,
                action: "Save".into(),
                ..Row::default()
            },
            Row {
                kind: "input".into(),
                id: "adv-ping".into(),
                label: "Reachability check".into(),
                hint: "An address BOS pings to confirm the connection is alive.".into(),
                value: advanced.ping_host,
                action: "Save".into(),
                ..Row::default()
            },
            Row {
                kind: "input".into(),
                id: "adv-wait".into(),
                label: "Give up joining after (seconds)".into(),
                value: advanced.connect_wait.to_string(),
                action: "Save".into(),
                ..Row::default()
            },
            Row {
                kind: "input".into(),
                id: "adv-watch".into(),
                label: "Check the connection every (seconds)".into(),
                value: advanced.watch_interval.to_string(),
                action: "Save".into(),
                ..Row::default()
            },
            Row {
                kind: "line".into(),
                id: "svc-toggle".into(),
                label: "Automatic network switching".into(),
                hint: format!(
                    "{} · {}",
                    if service.active { "Running" } else { "Stopped" },
                    if service.enabled {
                        "starts at login"
                    } else {
                        "doesn't start at login"
                    }
                ),
                action: if service.active { "Stop" } else { "Start" }.into(),
                ..Row::default()
            },
            Row {
                kind: "line".into(),
                id: "svc-restart".into(),
                label: "Restart automatic network switching".into(),
                action: "Restart".into(),
                enabled: service.active,
                ..Row::default()
            },
            Row {
                kind: "line".into(),
                id: "advanced-editor".into(),
                label: "More connection settings".into(),
                hint: "IP addresses, proxies, wired and mobile connections.".into(),
                action: "Open".into(),
                ..Row::default()
            },
        ],
    );
    p
}

fn place_editor_rows(place: &crate::network::Place, default_profile: &str) -> Vec<Row> {
    let mut rows = vec![
        Row {
            kind: "input".into(),
            id: "pe-name".into(),
            label: "Name".into(),
            value: place.name.clone(),
            action: "Rename".into(),
            ..Row::default()
        },
        Row {
            kind: "tags".into(),
            id: "pe-networks".into(),
            label: "Join these networks, in this order".into(),
            options: place.networks.clone(),
            ..Row::default()
        },
        Row {
            kind: "tags".into(),
            id: "pe-detect".into(),
            label: "Recognise this place when these are nearby".into(),
            options: place.detect.clone(),
            ..Row::default()
        },
        Row {
            kind: "line".into(),
            id: "pe-tailscale".into(),
            label: "Send traffic through Tailscale".into(),
            hint: "Waits for Tailscale before using the network.".into(),
            tag: "toggle".into(),
            checked: place.tailscale,
            ..Row::default()
        },
    ];
    if place.tailscale {
        rows.push(Row {
            kind: "input".into(),
            id: "pe-exit".into(),
            label: "Exit node".into(),
            value: place.exit_node.clone(),
            action: "Save".into(),
            ..Row::default()
        });
        rows.push(Row {
            kind: "input".into(),
            id: "pe-bootstrap".into(),
            label: "Sign in through first".into(),
            hint: "A network BOS joins first so Tailscale can start.".into(),
            value: place.bootstrap.clone(),
            action: "Save".into(),
            ..Row::default()
        });
    }
    rows.push(Row {
        kind: "line".into(),
        id: "pe-include".into(),
        label: "Also try any other saved network".into(),
        hint: "When none of the networks above are around.".into(),
        tag: "toggle".into(),
        checked: place.include_all,
        ..Row::default()
    });
    rows.push(Row {
        kind: "line".into(),
        id: "pe-default".into(),
        label: "Use when no place is recognised".into(),
        hint: "The fallback place.".into(),
        tag: "toggle".into(),
        checked: place.name == default_profile,
        ..Row::default()
    });
    rows.push(Row {
        kind: "line".into(),
        id: "pe-remove".into(),
        label: "Remove place".into(),
        hint: "Saved networks and other places are left alone.".into(),
        action: "Remove".into(),
        ..Row::default()
    });
    rows.push(Row {
        kind: "leftbutton".into(),
        id: "pe-done".into(),
        action: "Done".into(),
        primary: true,
        ..Row::default()
    });
    rows
}

fn network_saved_edit_cell() -> &'static Mutex<Option<String>> {
    static SAVED: OnceLock<Mutex<Option<String>>> = OnceLock::new();
    SAVED.get_or_init(|| Mutex::new(None))
}

fn network_saved_edit() -> Option<String> {
    network_saved_edit_cell()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
}

fn network_editing_cell() -> &'static Mutex<Option<String>> {
    static EDITING: OnceLock<Mutex<Option<String>>> = OnceLock::new();
    EDITING.get_or_init(|| Mutex::new(None))
}

fn network_editing() -> Option<String> {
    network_editing_cell()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
}

fn network_pending_cell() -> &'static Mutex<Option<String>> {
    static PENDING: OnceLock<Mutex<Option<String>>> = OnceLock::new();
    PENDING.get_or_init(|| Mutex::new(None))
}

fn network_pending() -> Option<String> {
    network_pending_cell().lock().unwrap_or_else(|e| e.into_inner()).clone()
}

fn network_action(id: &str, action: &str, checked: bool) -> Result<(), String> {
    if id == "wifi-radio" && action == "toggle" {
        return crate::network::set_radio(!checked);
    }
    if id == "scan" {
        return Ok(());
    }
    if let Some(ssid) = id.strip_prefix("wifi:") {
        if action == "action" {
            let net = crate::network::scan()
                .into_iter()
                .find(|net| net.ssid == ssid);
            if net.is_some_and(|net| net.secured && !net.known) {
                *network_pending_cell()
                    .lock()
                    .unwrap_or_else(|e| e.into_inner()) = Some(ssid.to_string());
                return Ok(());
            }
            return crate::network::connect(ssid, None);
        }
        return Ok(());
    }
    if let Some(ssid) = id.strip_prefix("saved:") {
        if action == "action" {
            *network_saved_edit_cell()
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = Some(ssid.to_string());
        }
        return Ok(());
    }
    if id == "add-network" && action == "action" {
        *network_saved_edit_cell()
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(String::new());
        return Ok(());
    }
    if id == "sn-done" {
        *network_saved_edit_cell()
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = None;
        return Ok(());
    }
    if id.starts_with("sn-") {
        let editing = network_saved_edit().unwrap_or_default();
        let current = crate::network::breadcrumbs_networks()
            .into_iter()
            .find(|network| network.ssid == editing);
        match id {
            "sn-remove" if action == "action" => {
                crate::network::forget_network(&editing)?;
                *network_saved_edit_cell()
                    .lock()
                    .unwrap_or_else(|e| e.into_inner()) = None;
                return Ok(());
            }
            "sn-hidden" if action == "toggle" => {
                let hidden = current.as_ref().is_some_and(|network| network.hidden);
                return crate::network::save_network(
                    &editing,
                    None,
                    !hidden,
                    Some(&editing),
                );
            }
            "sn-ssid" => {
                if let Some(text) = action.strip_prefix("input:") {
                    crate::network::save_network(
                        text,
                        None,
                        current.as_ref().is_some_and(|network| network.hidden),
                        Some(&editing),
                    )?;
                    *network_saved_edit_cell()
                        .lock()
                        .unwrap_or_else(|e| e.into_inner()) = Some(text.trim().to_string());
                }
                return Ok(());
            }
            "sn-password" => {
                if let Some(text) = action.strip_prefix("input:") {
                    return crate::network::save_network(
                        &editing,
                        Some(text),
                        current.as_ref().is_some_and(|network| network.hidden),
                        Some(&editing),
                    );
                }
                return Ok(());
            }
            _ => {}
        }
    }
    if id == "password" && action.starts_with("input:") {
        let password = action.trim_start_matches("input:");
        let ssid = network_pending_cell()
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
            .ok_or("no network is waiting for a password")?;
        return crate::network::connect(&ssid, Some(password));
    }
    if let Some(name) = id.strip_prefix("vpn:") {
        if action == "toggle" {
            return crate::network::vpn_toggle(name, !checked);
        }
        return Ok(());
    }
    if id == "add-vpn" && action == "action" {
        return crate::network::import_vpn_dialog().map(|_| ());
    }
    if let Some(name) = id.strip_prefix("place:") {
        if action == "action" {
            *network_editing_cell()
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = Some(name.to_string());
        }
        return Ok(());
    }
    if id == "pe-done" {
        *network_editing_cell()
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = None;
        return Ok(());
    }
    if id == "pe-remove" && action == "action" {
        let name = network_editing();
        if let Some(name) = name {
            crate::network::remove_place(&name)?;
            *network_editing_cell()
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = None;
        }
        return Ok(());
    }
    if id.starts_with("pe-") {
        let Some(place) = network_editing() else {
            return Ok(());
        };
        let key = match id {
            "pe-tailscale" => Some(("tailscale", true)),
            "pe-include" => Some(("include_all_known", true)),
            _ => None,
        };
        if let Some((key, _)) = key {
            if action == "toggle" {
                let current = crate::network::places()
                    .0
                    .into_iter()
                    .find(|candidate| candidate.name == place)
                    .map(|candidate| {
                        if key == "tailscale" {
                            candidate.tailscale
                        } else {
                            candidate.include_all
                        }
                    })
                    .unwrap_or(false);
                return crate::network::set_place_bool(&place, key, !current);
            }
        }
        if id == "pe-default" && action == "toggle" {
            return crate::network::set_default_place(&place);
        }
        if let Some(text) = action.strip_prefix("tag-add:") {
            let key = match id {
                "pe-networks" => "networks",
                "pe-detect" => "detect",
                _ => return Ok(()),
            };
            return crate::network::add_place_list(&place, key, text);
        }
        if let Some(index) = action
            .strip_prefix("tag-remove:")
            .and_then(|value| value.parse::<usize>().ok())
        {
            let key = match id {
                "pe-networks" => "networks",
                "pe-detect" => "detect",
                _ => return Ok(()),
            };
            return crate::network::remove_place_list(&place, key, index);
        }
        if let Some(text) = action.strip_prefix("input:") {
            return match id {
                "pe-name" => {
                    crate::network::rename_place(&place, text)?;
                    *network_editing_cell()
                        .lock()
                        .unwrap_or_else(|e| e.into_inner()) = Some(text.trim().to_string());
                    Ok(())
                }
                "pe-exit" => crate::network::set_place_string(&place, "exit_node", text),
                "pe-bootstrap" => crate::network::set_place_string(&place, "bootstrap", text),
                _ => Ok(()),
            };
        }
    }
    if id == "add-place" && action == "action" {
        let name = crate::network::add_place()?;
        return crate::network::switch_place(&name);
    }
    if id == "svc-toggle" && action == "action" {
        let active = crate::network::breadcrumbs_service().active;
        return crate::network::service_action(if active { "stop" } else { "start" });
    }
    if id == "svc-restart" && action == "action" {
        return crate::network::service_action("restart");
    }
    if id == "advanced-editor" && action == "action" {
        crate::network::open_editor();
        return Ok(());
    }
    if action.starts_with("input:") {
        let value = action.trim_start_matches("input:");
        let key = match id {
            "adv-dns" => Some("dns"),
            "adv-exit-node" => Some("exit_node"),
            "adv-connectivity" => Some("connectivity_url"),
            "adv-ping" => Some("ping_host"),
            "adv-wait" => Some("connect_wait"),
            "adv-watch" => Some("watch_interval"),
            _ => None,
        };
        if let Some(key) = key {
            return crate::network::save_setting(key, value);
        }
    }
    Ok(())
}

// ----------------------------------------------------------------------- mouse

fn mouse() -> Page {
    let mut p = page(
        "mouse",
        "Devices",
        "Mouse & touchpad",
        "Pointer speed, scrolling and touchpad gestures.",
    );
    let prefs = desktop_prefs();
    p = p.rows(
        "Pointer",
        vec![Row {
            kind: "slider".into(),
            id: "speed".into(),
            label: "Speed".into(),
            value: if prefs.pointer_speed.abs() < 0.01 {
                "Normal".into()
            } else {
                format!("{:+.2}", prefs.pointer_speed)
            },
            amount: prefs.pointer_speed as f32,
            min: -1.0,
            max: 1.0,
            ..Row::default()
        }],
    );
    p = p.rows(
        "Touchpad",
        vec![
            toggle("tap", "Tap to click", "", prefs.tap_to_click),
            toggle("natural", "Natural scrolling", "", prefs.natural_scroll),
            toggle(
                "disable-typing",
                "Disable while typing",
                "",
                prefs.disable_while_typing,
            ),
            Row {
                kind: "select".into(),
                id: "click-method".into(),
                label: "Click method".into(),
                value: if prefs.click_method == "buttonareas" {
                    "Bottom corners".into()
                } else {
                    "Click with fingers".into()
                },
                selected: if prefs.click_method == "buttonareas" { 1 } else { 0 },
                options: vec!["Click with fingers".into(), "Bottom corners".into()],
                ..Row::default()
            },
            Row {
                kind: "slider".into(),
                id: "scroll-factor".into(),
                label: "Scroll speed".into(),
                value: format!("{}%", (prefs.scroll_factor * 100.0).round() as i64),
                amount: prefs.scroll_factor as f32,
                min: 0.1,
                max: 5.0,
                ..Row::default()
            },
        ],
    );
    p
}

// ------------------------------------------------------------------------ lock

fn lock() -> Page {
    let mut p = page(
        "lock",
        "Personalise",
        "Lock screen",
        "What you see when the screen locks.",
    );
    let cfg = breadlock_conf();
    let mut rows = vec![
        Row {
            kind: "seg".into(),
            id: "background".into(),
            label: "Background".into(),
            options: vec!["Picture".into(), "Colour".into()],
            selected: if cfg.background_mode == "color" { 1 } else { 0 },
            ..Row::default()
        },
    ];
    if cfg.background_mode != "color" {
        rows.push(Row {
            kind: "input".into(),
            id: "background-path".into(),
            label: "Picture file".into(),
            hint: "A PNG on this computer.".into(),
            value: cfg.background_path.clone(),
            action: "Set".into(),
            ..Row::default()
        });
        rows.push(toggle(
            "background-blur",
            "Blur the picture",
            "",
            cfg.background_blur,
        ));
    }
    rows.push(Row {
        kind: "seg".into(),
        id: "clock".into(),
        label: "Clock".into(),
        options: vec!["24-hour".into(), "12-hour".into()],
        selected: if cfg.clock_format.contains("%I") { 1 } else { 0 },
        ..Row::default()
    });
    rows.push(toggle(
        "seconds",
        "Show seconds",
        "",
        cfg.clock_format.contains("%S"),
    ));
    rows.push(Row {
        kind: "select".into(),
        id: "font".into(),
        label: "Font".into(),
        value: cfg.font_family.clone(),
        options: LOCK_FONTS.iter().map(|font| font.to_string()).collect(),
        ..Row::default()
    });
    let waits = ["Don't wait", "1 second", "2 seconds"];
    rows.push(Row {
        kind: "seg".into(),
        id: "fail-timeout".into(),
        label: "After a wrong password, wait".into(),
        options: waits.iter().map(|wait| wait.to_string()).collect(),
        selected: match cfg.fail_timeout_ms {
            0 => 0,
            2000 => 2,
            _ => 1,
        },
        ..Row::default()
    });
    p = p.rows("Look", rows);
    p = p.rows(
        "Advanced",
        vec![
            Row {
                kind: "line".into(),
                id: "lock-now".into(),
                label: "Lock now to preview".into(),
                action: "Lock".into(),
                ..Row::default()
            },
            Row {
                kind: "line".into(),
                id: "open-lock-config".into(),
                label: "Settings file".into(),
                hint: "Every lock screen option, in a text editor.".into(),
                action: "Open".into(),
                ..Row::default()
            },
            Row {
                kind: "line".into(),
                id: "open-lock-example".into(),
                label: "Example settings file".into(),
                hint: "The full set of options.".into(),
                action: "Open".into(),
                ..Row::default()
            },
        ],
    );
    p
}

type LockConf = crate::lock::Config;

fn breadlock_conf() -> LockConf {
    crate::lock::load().unwrap_or_default()
}

const LOCK_FONTS: [&str; 4] = ["Varela Round", "Outfit", "Inter", "Noto Sans"];

fn privacy() -> Page {
    let mut p = page(
        "privacy",
        "System",
        "Privacy & security",
        "Who can reach this computer, and what apps can do.",
    );
    let mut firewall = group("Firewall");
    firewall.rows = vec![
        Row {
            kind: "line".into(),
            id: "firewall".into(),
            label: "Firewall".into(),
            hint: "Not checked yet. Check it to see whether unknown connections are blocked.".into(),
            action: "Check firewall".into(),
            ..Row::default()
        },
    ];
    p = p.push(firewall);

    let bug = bug_prefs();
    let mut reports = group("Bug reports");
    reports.rows = vec![
        toggle(
            "bug-upload",
            "Send bug reports",
            "Only when you start one.",
            bug.upload,
        ),
        toggle(
            "bug-notify",
            "Tell me when a fix ships",
            "",
            bug.notify,
        ),
        Row {
            kind: "line".into(),
            id: "bug-open".into(),
            label: "Report a problem".into(),
            hint: "Attach a screenshot and the last few minutes of logs.".into(),
            action: "Open".into(),
            ..Row::default()
        },
    ];
    p = p.push(reports);

    let mut included = group("Apps included in reports");
    included.rows = vec![Row {
        kind: "hint".into(),
        id: "bug-apps".into(),
        label: "Every app on this computer.".into(),
        ..Row::default()
    }];
    p = p.push(included);

    let mut contains = group("What a report contains");
    contains.rows = vec![
        Row {
            kind: "hint".into(),
            id: "bug-contains".into(),
            label: "The app's logs, your system version, and the settings that app reads. Never your files or passwords.".into(),
            ..Row::default()
        },
    ];
    p = p.push(contains);

    let mut mine = group("Your reports");
    mine.rows = vec![Row {
        kind: "hint".into(),
        id: "bug-none".into(),
        label: "You haven't sent any reports.".into(),
        ..Row::default()
    }];
    p = p.push(mine);

    let mut local = group("On this computer");
    local.rows = vec![Row {
        kind: "line".into(),
        id: "bug-folder".into(),
        label: "Saved reports".into(),
        hint: "~/.local/share/breadbug".into(),
        action: "Open".into(),
        ..Row::default()
    }];
    p.push(local)
}

struct BugPrefs {
    upload: bool,
    notify: bool,
}

fn bug_prefs() -> BugPrefs {
    let text = std::fs::read_to_string(config_dir().join("bread/breadbug.toml"))
        .unwrap_or_default();
    let value = text.parse::<toml::Value>().ok();
    let flag = |key: &str, fallback: bool| {
        value
            .as_ref()
            .and_then(|v| v.get(key))
            .and_then(|v| v.as_bool())
            .unwrap_or(fallback)
    };
    BugPrefs {
        upload: flag("upload_enabled", true),
        notify: flag("notify_fixed", true),
    }
}

// --------------------------------------------------------------------- updates

fn updates() -> Page {
    let mut p = page(
        "updates",
        "System",
        "Updates",
        "Keep your system, bread apps and firmware up to date.",
    );
    let pending = run("pacman", &["-Qu"])
        .map(|s| {
            s.lines()
                .filter(|l| !l.trim().is_empty())
                .map(|l| l.split_whitespace().next().unwrap_or("").to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let count = pending.len();
    let kernel = pending
        .iter()
        .any(|name| name == "linux" || name.starts_with("linux-"));
    let firmware = firmware_devices();
    let mut h = hero(
        match count {
            0 => "You're up to date",
            1 => "1 update ready",
            _ => "",
        },
        "",
    );
    if count > 1 {
        h.title = format!("{count} updates ready");
    }
    if count > 0 {
        let mut detail = if count == 1 {
            "1 system package".to_string()
        } else {
            format!("{count} system packages")
        };
        detail.push_str(" · a safety snapshot is taken first");
        if kernel {
            detail.push_str(" · includes a new kernel, restart after");
        }
        detail.push_str(&format!(
            ". {} firmware devices can be checked on the Firmware page.",
            firmware.len()
        ));
        h.hint = detail;
        h.icon = crate::icons::commands("download").into();
        h.action = "Update everything".into();
        h.value = "Check again".into();
        h.primary = true;
    } else {
        h.hint = "System and bread apps are current.".into();
        h.icon = crate::icons::commands("check").into();
    }
    p = p.push(h);
    let mut what = group("What will update");
    what.rows = vec![
        Row {
            kind: "disclosure".into(),
            id: "packages".into(),
            label: if count == 1 {
                "System packages (1), including a new kernel".into()
            } else if kernel {
                format!("System packages ({count}), including a new kernel")
            } else {
                format!("System packages ({count})")
            },
            ..Row::default()
        },
        Row {
            kind: "disclosure".into(),
            id: "firmware".into(),
            label: format!("Firmware devices ({})", firmware.len()),
            ..Row::default()
        },
        Row {
            kind: "disclosure".into(),
            id: "single".into(),
            label: "Update one part only".into(),
            ..Row::default()
        },
    ];
    p = p.push(what);
    p = p.rows(
        "If an update breaks something",
        vec![Row {
            kind: "line".into(),
            id: "rollback".into(),
            label: "Go back to before the update".into(),
            hint: "Every update takes a snapshot first. Restart and pick it from the boot menu, or choose it in Backup & restore.".into(),
            action: "Open".into(),
            ..Row::default()
        }],
    );
    if !firmware.is_empty() {
        let mut devices = group("Device firmware");
        devices.hint = "Updates for the computer itself: the BIOS, camera, dock and other parts that support it.".into();
        devices.rows = firmware
            .iter()
            .map(|(name, version)| Row {
                kind: "detail".into(),
                id: format!("fw:{name}"),
                label: name.clone(),
                hint: format!("Version {version}"),
                ..Row::default()
            })
            .collect();
        p = p.push(devices);
    }
    p
}

fn firmware_devices() -> Vec<(String, String)> {
    let Some(text) = run("fwupdmgr", &["get-devices", "--json"]) else {
        return Vec::new();
    };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) else {
        return Vec::new();
    };
    let Some(devices) = json.get("Devices").and_then(|d| d.as_array()) else {
        return Vec::new();
    };
    let mut devices: Vec<(bool, String, String)> = devices
        .iter()
        .filter(|d| {
            d.get("Flags")
                .and_then(|f| f.as_array())
                .map(|flags| flags.iter().any(|f| f.as_str() == Some("updatable")))
                .unwrap_or(false)
        })
        .filter_map(|d| {
            let name = d.get("Name")?.as_str()?.to_string();
            let security = is_security_database(d);
            Some((
                security,
                name,
                d.get("Version")
                    .and_then(|v| v.as_str())
                    .unwrap_or("-")
                    .to_string(),
            ))
        })
        .collect();
    // The shipping firmware list shows hardware first and keeps the Secure Boot
    // databases last; fwupd's own order interleaves them.
    devices.sort_by_key(|(security, _, _)| *security);
    devices
        .into_iter()
        .map(|(_, name, version)| (name, version))
        .collect()
}

fn is_security_database(device: &serde_json::Value) -> bool {
    let name = device
        .get("Name")
        .and_then(|n| n.as_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    name.contains("uefi dbx")
        || name.contains("uefi ca")
        || name.contains("kek ca")
        || device
            .get("Plugin")
            .and_then(|n| n.as_str())
            .is_some_and(|p| ["uefi-dbx", "uefi-kek", "uefi-db", "uefi-pk"].contains(&p))
}

// --------------------------------------------------------------------- restore

fn restore() -> Page {
    let mut p = page(
        "restore",
        "System",
        "Backup & restore",
        "Go back to an earlier system, or bring back your files.",
    );
    let snapshots = snapshot_rows();
    let mut snaps = group("System snapshots");
    snaps.hint = "Taken before every update and big change, so you can go back.".into();
    snaps.rows = snapshots;
    p = p.push(snaps);

    let repo = config_dir().join("bread/backup.toml");
    let repo_set = repo.exists();
    let mut backup = group("Back up your files");
    backup.rows = vec![
        Row {
            kind: "line".into(),
            id: "repo".into(),
            label: "Where backups go".into(),
            hint: if repo_set {
                "An external drive or network folder.".into()
            } else {
                "Not set up yet. Choose a folder on another drive.".into()
            },
            action: "Choose a folder".into(),
            ..Row::default()
        },
        toggle("backup-daily", "Back up every day", "", repo_set),
        Row {
            kind: "line".into(),
            id: "backup-now".into(),
            label: "Back up now".into(),
            hint: "Only changes since the last backup.".into(),
            action: "Back up now".into(),
            primary: true,
            ..Row::default()
        },
    ];
    p.push(backup)
}

fn snapshot_rows() -> Vec<Row> {
    let out = run("snapper", &["-c", "root", "list", "--columns", "number,date,description"]);
    let mut rows = vec![Row {
        kind: "line".into(),
        id: "now".into(),
        label: "Now".into(),
        hint: "The system as it is today.".into(),
        ..Row::default()
    }];
    if let Some(text) = out {
        for line in text.lines().skip(2) {
            let mut parts = line.split('|').map(str::trim);
            let (Some(number), Some(date)) = (parts.next(), parts.next()) else {
                continue;
            };
            let description = parts.next().unwrap_or("Snapshot");
            if number.is_empty() {
                continue;
            }
            rows.push(Row {
                kind: "line".into(),
                id: format!("snap:{number}"),
                label: description.to_string(),
                hint: date.to_string(),
                action: "Restore".into(),
                ..Row::default()
            });
        }
    }
    rows
}

// ---------------------------------------------------------------------- memory

const MEMORY_ACTIONS: [(&str, &str); 6] = [
    ("cgroup_soft_limit", "Soft memory limits"),
    ("reclaim_protection", "Reclaim protection"),
    ("unload_idle_model", "Unload idle models"),
    ("sleep_model", "Sleep model backends"),
    ("reduce_job_concurrency", "Reduce job workers"),
    ("queue_background_job", "Delay background jobs"),
];

fn memory_profile_index(cfg: &crate::breadram::Config) -> i32 {
    if cfg.mode.as_deref() == Some("monitor") {
        return 0;
    }
    if cfg
        .allowed_actions
        .iter()
        .any(|action| action == "queue_background_job")
    {
        return 2;
    }
    if cfg.mode.is_some() {
        return 1;
    }
    -1
}

fn json_u64(value: Option<&serde_json::Value>) -> Option<u64> {
    value.and_then(|v| v.as_u64())
}

fn json_text(value: Option<&serde_json::Value>) -> String {
    value
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string()
}

fn remember_bytes(bytes: Option<u64>) -> String {
    let Some(bytes) = bytes else {
        return "unknown".into();
    };
    if bytes >= 1024 * 1024 * 1024 {
        format!("{:.1} GiB", bytes as f64 / 1024.0 / 1024.0 / 1024.0)
    } else if bytes >= 1024 * 1024 {
        format!("{} MiB", bytes / 1024 / 1024)
    } else {
        format!("{} KiB", bytes / 1024)
    }
}

fn memory_history_row(record: &serde_json::Value) -> Row {
    let kind = json_text(record.get("kind"));
    let scope = json_text(record.get("scope"));
    let state = json_text(record.get("state"));
    let delta = |key: &str| {
        record
            .get(key)
            .and_then(|v| v.as_i64())
            .map(|v| {
                if v < 0 {
                    format!("freed {}", remember_bytes(Some((-v) as u64)))
                } else {
                    format!("used {}", remember_bytes(Some(v as u64)))
                }
            })
            .unwrap_or_else(|| "not measured".into())
    };
    Row {
        kind: "line".into(),
        id: format!("history:{scope}:{kind}"),
        label: format!("{kind} · {scope}"),
        hint: json_text(record.get("reason")),
        value: format!("{state} · expected {} · measured {}", delta("requested_host_ram_delta"), delta("observed_host_ram_delta")),
        ..Row::default()
    }
}

fn memory() -> Page {
    let mut p = page("memory", "System", "Memory", "");
    let live = crate::breadram::status();
    let actions = crate::breadram::actions();
    let present = live.get("present").and_then(|v| v.as_bool()).unwrap_or(false);

    let mut right = group("Right now");
    right.hint =
        "What the memory daemon is doing. Nothing here needs the daemon running to open this page."
            .into();
    if present {
        let memory = live.get("memory");
        let device = memory
            .and_then(|m| m.get("vram_bytes"))
            .and_then(|v| v.as_u64())
            .map(|v| remember_bytes(Some(v)))
            .or_else(|| {
                memory
                    .and_then(|m| m.get("gtt_used_bytes"))
                    .and_then(|v| v.as_u64())
                    .map(|used| {
                        format!(
                            "{} shared of {}",
                            remember_bytes(Some(used)),
                            remember_bytes(json_u64(memory.and_then(|m| m.get("gtt_total_bytes"))))
                        )
                    })
            })
            .unwrap_or_else(|| "nothing measured it".into());
        let mode = json_text(live.get("mode"));
        let paused = mode == "monitor";
        right.rows = vec![
            Row {
                kind: "value".into(),
                id: "pressure".into(),
                label: "Pressure".into(),
                value: json_text(live.get("pressure").and_then(|v| v.get("state"))),
                ..Row::default()
            },
            Row {
                kind: "value".into(),
                id: "live-mode".into(),
                label: "Mode".into(),
                value: mode.clone(),
                ..Row::default()
            },
            Row {
                kind: "value".into(),
                id: "available".into(),
                label: "Available".into(),
                value: format!(
                    "{} of {}",
                    remember_bytes(json_u64(memory.and_then(|m| m.get("ram_available_bytes")))),
                    remember_bytes(json_u64(memory.and_then(|m| m.get("ram_total_bytes"))))
                ),
                ..Row::default()
            },
            Row {
                kind: "value".into(),
                id: "device".into(),
                label: "Device memory".into(),
                value: device,
                ..Row::default()
            },
            Row {
                kind: "value".into(),
                id: "version".into(),
                label: "Daemon version".into(),
                value: json_text(live.get("version")),
                ..Row::default()
            },
            Row {
                kind: "button".into(),
                id: "pause".into(),
                label: "Pause".into(),
                hint: "Stops automatic changes now without saving anything. A restart goes back to the saved mode.".into(),
                action: if paused { "Resume".into() } else { "Pause".into() },
                primary: !paused,
                ..Row::default()
            },
        ];
    } else {
        right.rows = vec![Row {
            kind: "value".into(),
            id: "daemon".into(),
            label: "Daemon".into(),
            hint: "breadram is not running. Settings below still open and save.".into(),
            value: "Unavailable".into(),
            ..Row::default()
        }];
    }
    p = p.push(right);

    let cfg = match crate::breadram::load() {
        Ok(cfg) => cfg,
        Err(error) => {
            p.status = format!("Could not read the breadram config: {error}");
            return p.push(note(
                "The memory settings stay closed until that file parses again. Nothing was written.",
            ));
        }
    };

    let mut start = group("Starting point");
    start.hint =
        "Named setups that fill in the settings below. You can still change anything afterwards."
            .into();
    start.rows = vec![Row {
        kind: "seg".into(),
        id: "profile".into(),
        label: "Profile".into(),
        hint: "Monitor watches only. The other two act automatically with different caution."
            .into(),
        options: vec![
            "Monitor".into(),
            "Desktop".into(),
            "Headless server".into(),
        ],
        selected: memory_profile_index(&cfg),
        tag: "plain".into(),
        ..Row::default()
    }];
    p = p.push(start);

    let mut automation = group("Automation");
    automation.hint =
        "What the daemon may do on its own. Unticked actions never run automatically.".into();
    let mut rows = vec![Row {
        kind: "select".into(),
        id: "mode".into(),
        label: "Mode".into(),
        hint: "Monitor watches and explains. Automatic acts within the ticked actions.".into(),
        value: match cfg.mode.as_deref() {
            Some("automatic") => "Automatic".into(),
            _ => "Monitor".into(),
        },
        options: vec!["Monitor".into(), "Automatic".into()],
        selected: if cfg.mode.as_deref() == Some("automatic") { 1 } else { 0 },
        ..Row::default()
    }];
    for (code, label) in MEMORY_ACTIONS {
        let on = cfg.allowed_actions.iter().any(|action| action == code);
        rows.push(Row {
            kind: "check".into(),
            id: code.into(),
            label: label.into(),
            hint: if on {
                "Allowed.".into()
            } else {
                "Off. The daemon will not do this.".into()
            },
            checked: on,
            ..Row::default()
        });
    }
    rows.push(Row {
        kind: "tags".into(),
        id: "never-touch".into(),
        label: "Never touch".into(),
        options: cfg.protected.clone(),
        ..Row::default()
    });
    automation.rows = rows;
    p = p.push(automation);

    let mut backends = group("Model backends");
    backends.hint = "Extra programs the daemon may ask to release memory. Each one only watches the workload scope you name.".into();
    for provider in &cfg.providers {
        backends.rows.push(Row {
            kind: "provider".into(),
            id: format!("provider:{}", provider.name),
            label: provider.name.clone(),
            hint: provider.url.clone(),
            value: provider.url.clone(),
            detail: provider.scope.clone(),
            tag: provider.name.clone(),
            action: "Remove".into(),
            checked: provider.allow_automation,
            options: vec![provider.kind.clone()],
            ..Row::default()
        });
    }
    if backends.rows.is_empty() {
        backends.rows.push(Row {
            kind: "hint".into(),
            id: "no-providers".into(),
            label: "No model backends configured.".into(),
            ..Row::default()
        });
    }
    let mut add = row("leftbutton", "");
    add.id = "add-provider".into();
    add.action = "Add backend".into();
    add.icon = crate::icons::commands("plus").into();
    add.primary = true;
    backends.rows.push(add);
    p = p.push(backends);

    let mut jobs = group("Background jobs");
    jobs.hint = "Queued programs the daemon may run when memory allows. Only a supervised job carries a hard reservation, taken from its estimate.".into();
    jobs.rows = vec![
        toggle(
            "jobs-enabled",
            "Let the daemon run queued jobs",
            "",
            cfg.jobs_enabled.unwrap_or(false),
        ),
        Row {
            kind: "input".into(),
            id: "jobs-total".into(),
            label: "Total memory for jobs (GiB)".into(),
            value: format!("{}", cfg.jobs_total_budget_gib.unwrap_or(8.0)),
            ..Row::default()
        },
        Row {
            kind: "input".into(),
            id: "jobs-estimate".into(),
            label: "Guess per job (GiB)".into(),
            value: format!("{}", cfg.jobs_default_estimate_gib.unwrap_or(2.0)),
            ..Row::default()
        },
        toggle(
            "jobs-auto",
            "Start the next job on its own",
            "",
            cfg.jobs_auto_launch.unwrap_or(false),
        ),
    ];
    p = p.push(jobs);

    let mut effective = group("What the daemon thinks");
    effective.hint = "The settings actually in force, and why. File changes apply when the daemon restarts.".into();
    let rules = live.get("rules").filter(|v| !v.is_null());
    if present && rules.is_some() {
        let rules = rules.unwrap();
        let list = |key: &str| {
            rules
                .get(key)
                .and_then(|v| v.as_array())
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|v| v.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .filter(|text| !text.is_empty())
                .unwrap_or_else(|| "none".into())
        };
        effective.rows.push(Row {
            kind: "value".into(),
            id: "rules-mode".into(),
            label: "Mode in force".into(),
            value: json_text(rules.get("mode")),
            ..Row::default()
        });
        effective.rows.push(Row {
            kind: "value".into(),
            id: "rules-actions".into(),
            label: "Actions in force".into(),
            value: list("allowed_actions"),
            ..Row::default()
        });
        effective.rows.push(Row {
            kind: "value".into(),
            id: "rules-protected".into(),
            label: "Protected in force".into(),
            value: list("protected"),
            ..Row::default()
        });
        if let Some(budgets) = rules.get("budgets").and_then(|v| v.as_array()) {
            for budget in budgets {
                effective.rows.push(Row {
                    kind: "value".into(),
                    id: format!("budget:{}", json_text(budget.get("scope"))),
                    label: format!("Budget {}", json_text(budget.get("scope"))),
                    value: remember_bytes(json_u64(budget.get("max_bytes"))),
                    ..Row::default()
                });
            }
        }
        effective.rows.push(Row {
            kind: "value".into(),
            id: "config-path".into(),
            label: "Config file".into(),
            value: json_text(live.get("config_path")),
            ..Row::default()
        });
    } else {
        effective.rows.push(Row {
            kind: "value".into(),
            id: "effective".into(),
            label: "Effective settings".into(),
            hint: "Start the daemon to compare the file against what is running.".into(),
            value: "Daemon unavailable".into(),
            ..Row::default()
        });
    }
    p = p.push(effective);

    let mut capabilities = group("What this host can do");
    capabilities.hint = "Each capability the daemon probed, and why anything unavailable stays off. A workload is observation-only when its controls are missing here.".into();
    let entries = live
        .get("capabilities")
        .and_then(|v| v.get("capabilities"))
        .and_then(|v| v.as_array());
    if present && entries.map(|e| !e.is_empty()).unwrap_or(false) {
        for entry in entries.unwrap() {
            let state = json_text(entry.get("state"));
            let reason = entry
                .get("reason")
                .and_then(|v| v.as_str())
                .map(|v| format!(": {v}"))
                .unwrap_or_default();
            capabilities.rows.push(Row {
                kind: "value".into(),
                id: format!("cap:{}", json_text(entry.get("name"))),
                label: json_text(entry.get("name")),
                value: if state == "available" {
                    "Available".into()
                } else {
                    format!("{state}{reason}")
                },
                ..Row::default()
            });
        }
    } else {
        capabilities.rows.push(Row {
            kind: "value".into(),
            id: "cap-none".into(),
            label: "Capabilities".into(),
            hint: "Start the daemon to probe this host.".into(),
            value: "Daemon unavailable".into(),
            ..Row::default()
        });
    }
    p = p.push(capabilities);

    let mut recent = group("Recent changes");
    recent.hint = "Newest first. Expected is the plan's guess, measured is what the daemon saw.".into();
    if actions.get("present").and_then(|v| v.as_bool()).unwrap_or(false) {
        let payload = actions.get("actions");
        if let Some(plan) = payload.and_then(|p| p.get("plan")) {
            let explanation = plan
                .get("explanation")
                .and_then(|v| v.as_array())
                .map(|lines| {
                    lines
                        .iter()
                        .take(3)
                        .filter_map(|v| v.as_str())
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .filter(|text| !text.is_empty())
                .unwrap_or_else(|| "Nothing proposed.".into());
            recent.rows.push(Row {
                kind: "value".into(),
                id: "plan".into(),
                label: format!("Plan ({})", json_text(plan.get("state"))),
                value: explanation,
                ..Row::default()
            });
        }
        if let Some(history) = payload
            .and_then(|p| p.get("history"))
            .and_then(|v| v.as_array())
        {
            for record in history.iter().take(20) {
                recent.rows.push(memory_history_row(record));
            }
        }
    }
    if recent.rows.len() <= 1 {
        recent.rows.push(Row {
            kind: "value".into(),
            id: "history-none".into(),
            label: "History".into(),
            hint: "Nothing recorded yet, or the daemon is unavailable.".into(),
            value: "Empty".into(),
            ..Row::default()
        });
    }
    p.push(recent)
}

const KEYBOARD_LAYOUTS: [(&str, &str); 6] = [
    ("us", "English (US)"),
    ("gb", "English (UK)"),
    ("de", "German"),
    ("fr", "French"),
    ("es", "Spanish"),
    ("dvorak", "Dvorak"),
];

fn next_layout(current: &str) -> &'static str {
    let current = current.split(',').next().unwrap_or(current);
    let index = KEYBOARD_LAYOUTS
        .iter()
        .position(|(code, _)| *code == current)
        .unwrap_or(0);
    KEYBOARD_LAYOUTS[(index + 1) % KEYBOARD_LAYOUTS.len()].0
}

fn keyboard_action(id: &str, action: &str, checked: bool) -> Result<(), String> {
    if id == "layout" && action == "select" {
        let live = run("hyprctl", &["getoption", "input:kb_layout"])
            .and_then(|text| {
                text.strip_prefix("str: ")
                    .map(|value| value.trim().to_string())
            })
            .unwrap_or_else(|| "us".into());
        let next = next_layout(&live);
        let path = config_dir().join("hypr/settings.json");
        let mut stored = std::fs::read_to_string(&path)
            .ok()
            .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
            .unwrap_or_else(|| serde_json::json!({}));
        let object = stored
            .as_object_mut()
            .ok_or("appearance settings must be a JSON object")?;
        object.insert("kb_layout".into(), serde_json::Value::String(next.into()));
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        std::fs::write(
            &path,
            serde_json::to_string_pretty(&stored).map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?;
        let _ = std::process::Command::new("hyprctl").arg("reload").status();
        return Ok(());
    }
    if action == "action" && id == "ime-config" {
        return std::process::Command::new("fcitx5-configtool")
            .spawn()
            .map(|_| ())
            .map_err(|error| error.to_string());
    }
    if action == "toggle" && id == "ime-enabled" {
        return crate::ime::set_enabled(!checked);
    }
    if action == "action" && id == "ime-package" {
        return Err("install fcitx5 from Get apps first".into());
    }
    Ok(())
}

fn set_bug_flag(path: &std::path::Path, key: &str, value: bool) -> Result<(), String> {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    let mut doc = text
        .parse::<toml_edit::DocumentMut>()
        .map_err(|error| error.to_string())?;
    doc.insert(key, toml_edit::value(value));
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    std::fs::write(path, doc.to_string()).map_err(|error| error.to_string())
}

fn privacy_action(id: &str, action: &str) -> Result<(), String> {
    if action == "toggle" {
        let key = match id {
            "bug-upload" => "upload_enabled",
            "bug-notify" => "notify_fixed",
            _ => return Ok(()),
        };
        let path = config_dir().join("bread/breadbug.toml");
        let prefs = bug_prefs();
        let current = if key == "upload_enabled" {
            prefs.upload
        } else {
            prefs.notify
        };
        return set_bug_flag(&path, key, !current);
    }
    if action == "action" && id == "bug-folder" {
        return std::process::Command::new("xdg-open")
            .arg(home_dir().join(".local/share/breadbug"))
            .spawn()
            .map(|_| ())
            .map_err(|error| error.to_string());
    }
    Ok(())
}

fn storage_action(id: &str, action: &str) -> Result<(), String> {
    if action != "action" {
        return Ok(());
    }
    match id {
        "trash" => write("gio", &["trash", "--empty"]),
        "app-cache" => std::process::Command::new("xdg-open")
            .arg(home_dir().join(".cache"))
            .spawn()
            .map(|_| ())
            .map_err(|error| error.to_string()),
        _ => Ok(()),
    }
}

fn notifications_action(id: &str, action: &str) -> Result<(), String> {
    if action == "toggle" {
        let key = match id {
            "dnd" => "dnd",
            "schedule" => "schedule",
            "lock-screen" => "lock_screen",
            "sound" => "sound",
            _ => return Ok(()),
        };
        return crate::notifications::toggle(key);
    }
    if action == "select" {
        if let Some(name) = id.strip_prefix("app:") {
            return crate::notifications::cycle_app(name);
        }
    }
    Ok(())
}

fn startup_action(id: &str, action: &str) -> Result<(), String> {
    let Some(path) = id.strip_prefix("startup:") else {
        return Ok(());
    };
    if action == "action" {
        std::process::Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map_err(|error| error.to_string())?;
        return Ok(());
    }
    if action != "toggle" {
        return Ok(());
    }
    let path = std::path::Path::new(path);
    let text = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
    let hidden = text.lines().any(|line| line.trim() == "Hidden=true");
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    let target = format!("Hidden={}", !hidden);
    if let Some(index) = lines
        .iter()
        .position(|line| line.trim_start().starts_with("Hidden="))
    {
        lines[index] = target;
    } else if let Some(index) = lines
        .iter()
        .position(|line| line.trim() == "[Desktop Entry]")
    {
        lines.insert(index + 1, target);
    } else {
        return Err("not a desktop entry".into());
    }
    std::fs::write(path, lines.join("\n") + "\n").map_err(|error| error.to_string())
}

fn spawn_editor(path: &std::path::Path) -> Result<(), String> {
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "nano".into());
    Command::new("kitty")
        .args(["-e", &editor])
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|error| error.to_string())
}

fn lock_action(id: &str, action: &str) -> Result<(), String> {
    let mut cfg = crate::lock::load()?;
    if let Some(index) = action.strip_prefix("seg:").and_then(|v| v.parse::<usize>().ok()) {
        match id {
            "background" => {
                cfg.background_mode = if index == 0 { "image" } else { "color" }.into();
                return crate::lock::save(&cfg);
            }
            "clock" => {
                let seconds = cfg.clock_format.contains("%S");
                cfg.clock_format = crate::lock::clock_format(index == 1, seconds).into();
                return crate::lock::save(&cfg);
            }
            _ => return Ok(()),
        }
    }
    if action == "toggle" && id == "seconds" {
        let twelve = cfg.clock_format.contains("%I");
        let seconds = !cfg.clock_format.contains("%S");
        cfg.clock_format = crate::lock::clock_format(twelve, seconds).into();
        return crate::lock::save(&cfg);
    }
    if action == "seg:" {
        return Ok(());
    }
    if let Some(index) = action.strip_prefix("seg:").and_then(|v| v.parse::<usize>().ok()) {
        if id == "fail-timeout" {
            cfg.fail_timeout_ms = match index {
                0 => 0,
                2 => 2000,
                _ => 1000,
            };
            return crate::lock::save(&cfg);
        }
    }
    if action == "toggle" && id == "background-blur" {
        cfg.background_blur = !cfg.background_blur;
        return crate::lock::save(&cfg);
    }
    if action.starts_with("input:") && id == "background-path" {
        cfg.background_path = action.trim_start_matches("input:").trim().to_string();
        return crate::lock::save(&cfg);
    }
    if action == "action" && id == "lock-now" {
        let _ = Command::new("loginctl").arg("lock-session").spawn();
        return Ok(());
    }
    if action == "action" && id == "open-lock-config" {
        return spawn_editor(&crate::lock::path());
    }
    if action == "action" && id == "open-lock-example" {
        let candidates = [
            "/usr/share/doc/breadlock/breadlock.example.toml",
            "/usr/share/breadlock/breadlock.example.toml",
            "/usr/share/doc/breadlock/examples/breadlock.example.toml",
        ];
        if let Some(path) = candidates
            .iter()
            .find(|path| std::path::Path::new(path).is_file())
        {
            return spawn_editor(std::path::Path::new(path));
        }
        return Err("no example file is installed".into());
    }
    if action == "select" && id == "font" {
        let current = LOCK_FONTS
            .iter()
            .position(|font| *font == cfg.font_family)
            .unwrap_or(0);
        cfg.font_family = LOCK_FONTS[(current + 1) % LOCK_FONTS.len()].into();
        return crate::lock::save(&cfg);
    }
    Ok(())
}

fn preferences_action(page: &str, id: &str, action: &str) -> Result<(), String> {
    let mut prefs = crate::prefs::load();
    match page {
        "mouse" => {
            if let Some(value) = action.strip_prefix("slider:") {
                let value: f64 = value.parse().map_err(|_| "enter a number")?;
                match id {
                    "speed" => prefs.pointer_speed = value,
                    "scroll-factor" => prefs.scroll_factor = value,
                    _ => return Ok(()),
                }
                return crate::prefs::save(&prefs);
            }
            if action == "toggle" {
                match id {
                    "tap" => prefs.tap_to_click = !prefs.tap_to_click,
                    "natural" => prefs.natural_scroll = !prefs.natural_scroll,
                    "disable-typing" => prefs.disable_while_typing = !prefs.disable_while_typing,
                    _ => return Ok(()),
                }
                return crate::prefs::save(&prefs);
            }
            if action == "select" && id == "click-method" {
                prefs.click_method = if prefs.click_method == "clickfinger" {
                    "buttonareas".into()
                } else {
                    "clickfinger".into()
                };
                return crate::prefs::save(&prefs);
            }
        }
        "a11y" => {
            if action == "toggle" {
                match id {
                    "stronger-borders" => prefs.stronger_borders = !prefs.stronger_borders,
                    "reduce-motion" => prefs.reduce_motion = !prefs.reduce_motion,
                    _ => return Ok(()),
                }
                return crate::prefs::save(&prefs);
            }
            if let Some(index) = action.strip_prefix("seg:").and_then(|v| v.parse::<usize>().ok()) {
                match id {
                    "text-size" => {
                        prefs.text_size = [14, 16, 18].get(index).copied().unwrap_or(14);
                        return crate::prefs::save(&prefs);
                    }
                    "cursor-size" => {
                        prefs.cursor_size = [24, 32, 48].get(index).copied().unwrap_or(24);
                        return crate::prefs::save(&prefs);
                    }
                    _ => return Ok(()),
                }
            }
        }
        _ => {}
    }
    Ok(())
}

fn memory_action(id: &str, action: &str) -> Result<(), String> {
    memory_action_at(&crate::breadram::path(), id, action)
}

fn memory_action_at(
    path: &std::path::Path,
    id: &str,
    action: &str,
) -> Result<(), String> {
    let mut cfg = crate::breadram::load_from(path)?;
    if action == "select" && id == "mode" {
        cfg.mode = Some(if cfg.mode.as_deref() == Some("automatic") {
            "monitor".to_string()
        } else {
            "automatic".to_string()
        });
        return crate::breadram::save_to(path, &cfg);
    }
    if action == "action" && id == "pause" {
        let live = crate::breadram::status();
        if live.get("present").and_then(|v| v.as_bool()) != Some(true) {
            return Err("the daemon could not be reached".into());
        }
        let paused = live.get("mode").and_then(|v| v.as_str()) == Some("monitor");
        crate::breadram::set_mode(if paused { "automatic" } else { "monitor" }, false)?;
        return Ok(());
    }
    if action == "toggle" {
        if id == "jobs-enabled" {
            cfg.jobs_enabled = Some(!cfg.jobs_enabled.unwrap_or(false));
            return crate::breadram::save_to(path, &cfg);
        }
        if id == "jobs-auto" {
            cfg.jobs_auto_launch = Some(!cfg.jobs_auto_launch.unwrap_or(false));
            return crate::breadram::save_to(path, &cfg);
        }
        if !MEMORY_ACTIONS.iter().any(|(code, _)| *code == id) {
            return Ok(());
        }
        if let Some(index) = cfg.allowed_actions.iter().position(|code| code == id) {
            cfg.allowed_actions.remove(index);
        } else {
            cfg.allowed_actions.push(id.to_string());
        }
        return crate::breadram::save_to(path, &cfg);
    }
    if let Some(index) = action.strip_prefix("seg:").and_then(|v| v.parse().ok()) {
        if id == "profile" {
            let profile = match index {
                0 => "monitor",
                1 => "desktop",
                2 => "headless-server",
                _ => return Ok(()),
            };
            return crate::breadram::apply_profile_to(path, profile);
        }
    }
    if let Some(index) = action
        .strip_prefix("tag-remove:")
        .and_then(|v| v.parse::<usize>().ok())
    {
        if cfg.protected.get(index).is_some() {
            cfg.protected.remove(index);
            return crate::breadram::save_to(path, &cfg);
        }
    }
    if action == "action" {
        if id == "add-provider" {
            cfg.providers.push(crate::breadram::Provider {
                name: "New backend".into(),
                kind: "ollama".into(),
                url: "http://127.0.0.1:11434".into(),
                scope: String::new(),
                allow_automation: false,
            });
            return crate::breadram::save_to(path, &cfg);
        }
        if let Some(name) = id.strip_prefix("provider:") {
            cfg.providers.retain(|provider| provider.name != name);
            return crate::breadram::save_to(path, &cfg);
        }
    }
    if let Some(text) = action.strip_prefix("tag-add:") {
        let text = text.trim();
        if !text.is_empty() && !cfg.protected.iter().any(|scope| scope == text) {
            cfg.protected.push(text.to_string());
            return crate::breadram::save_to(path, &cfg);
        }
    }
    if let Some(text) = action.strip_prefix("input:") {
        let text = text.trim();
        match id {
            "jobs-total" => {
                cfg.jobs_total_budget_gib = Some(text.parse().map_err(|_| "enter a number of GiB")?);
                return crate::breadram::save_to(path, &cfg);
            }
            "jobs-estimate" => {
                cfg.jobs_default_estimate_gib =
                    Some(text.parse().map_err(|_| "enter a number of GiB")?);
                return crate::breadram::save_to(path, &cfg);
            }
            _ => {}
        }
    }
    if let Some(name) = id.strip_prefix("provider:") {
        let Some(provider) = cfg
            .providers
            .iter_mut()
            .find(|provider| provider.name == name)
        else {
            return Ok(());
        };
        if let Some(text) = action.strip_prefix("provider-name:") {
            let text = text.trim();
            if text.is_empty() {
                return Err("a backend needs a name".into());
            }
            provider.name = text.to_string();
            return crate::breadram::save_to(path, &cfg);
        }
        if let Some(text) = action.strip_prefix("provider-url:") {
            provider.url = text.trim().to_string();
            return crate::breadram::save_to(path, &cfg);
        }
        if let Some(text) = action.strip_prefix("provider-scope:") {
            provider.scope = text.trim().to_string();
            return crate::breadram::save_to(path, &cfg);
        }
        if action == "provider-kind" {
            let kinds = ["ollama", "llamacpp", "vllm", "breadsearch", "hook"];
            let current = kinds
                .iter()
                .position(|kind| {
                    *kind == provider.kind
                        || (*kind == "llamacpp" && provider.kind == "llama_cpp")
                })
                .unwrap_or(0);
            provider.kind = kinds[(current + 1) % kinds.len()].to_string();
            return crate::breadram::save_to(path, &cfg);
        }
        if action == "provider-automation" {
            provider.allow_automation = !provider.allow_automation;
            return crate::breadram::save_to(path, &cfg);
        }
    }
    Ok(())
}

// --------------------------------------------------------------------- getapps

fn getapps() -> Page {
    let mut p = page(
        "getapps",
        "Apps",
        "Get apps",
        "Apps from Flathub, the BOS repositories and the AUR, in one place.",
    );
    let mut search = group("Search apps");
    search.rows = vec![Row {
        kind: "input".into(),
        id: "app-search".into(),
        label: "Search for an app".into(),
        value: String::new(),
        ..Row::default()
    }];
    p = p.push(search);

    let mut categories = Block {
        kind: "chips".into(),
        chips: vec![
            "Communication".into(),
            "Media".into(),
            "Games".into(),
            "Creativity".into(),
            "Development".into(),
        ],
        chip: -1,
        ..Block::default()
    };
    categories.value = "sources".into();
    p = p.push(categories);

    let mut sources = Block {
        kind: "chips".into(),
        chips: vec![
            "All".into(),
            "BOS repositories".into(),
            "Flathub".into(),
            "AUR".into(),
        ],
        chip: 0,
        ..Block::default()
    };
    sources.value = "sources".into();
    p = p.push(sources);

    let mut recommended = group("Recommended apps");
    recommended.hint = "Optional BOS apps and everyday tools, ready to install.".into();
    recommended.rows = vec![
        Row {
            kind: "line".into(),
            id: "rec-breadshot".into(),
            label: "Screenshots".into(),
            hint: "Capture, annotate and share. Already installed.".into(),
            ..Row::default()
        },
        Row {
            kind: "line".into(),
            id: "rec-breadpad".into(),
            label: "Quick notes".into(),
            hint: "Notes, reminders and tasks. Already installed.".into(),
            ..Row::default()
        },
        Row {
            kind: "line".into(),
            id: "rec-breadmon".into(),
            label: "System monitor".into(),
            hint: "CPU, memory and disk at a glance. Already installed.".into(),
            ..Row::default()
        },
    ];
    p = p.push(recommended);

    let installed = run("pacman", &["-Q"]).map(|s| s.lines().count()).unwrap_or(0);
    let mut packages = group("Bread ecosystem (bakery)");
    packages.hint = "Everything installed from the BOS repositories.".into();
    packages.rows = vec![
        Row {
            kind: "line".into(),
            id: "pkg-count".into(),
            label: format!("{installed} packages installed"),
            hint: "Search by name to see versions and remove one.".into(),
            action: "Open".into(),
            ..Row::default()
        },
        Row {
            kind: "input".into(),
            id: "pkg-search".into(),
            label: "Search installed packages".into(),
            value: String::new(),
            ..Row::default()
        },
    ];
    p.push(packages)
}

// --------------------------------------------------------------------- desktop

fn desktop() -> Page {
    let mut p = page(
        "desktop",
        "Personalise",
        "Desktop & bar",
        "Bar, launcher, screenshots and desktop tools.",
    );

    let mut bar = group("Bar");
    bar.rows = vec![
        Row {
            kind: "select".into(),
            id: "font".into(),
            label: "Font".into(),
            value: "Varela Round".into(),
            options: vec![
                "Varela Round".into(),
                "Outfit".into(),
                "Inter".into(),
                "Noto Sans".into(),
                "JetBrains Mono".into(),
            ],
            ..Row::default()
        },
        Row {
            kind: "slider".into(),
            id: "text-size".into(),
            label: "Text size".into(),
            value: "14 px".into(),
            amount: 14.0,
            min: 8.0,
            max: 32.0,
            ..Row::default()
        },
        Row {
            kind: "slider".into(),
            id: "workspace-size".into(),
            label: "Workspace number size".into(),
            value: "20 px".into(),
            amount: 20.0,
            min: 8.0,
            max: 40.0,
            ..Row::default()
        },
        Row {
            kind: "slider".into(),
            id: "inactive".into(),
            label: "Inactive workspaces".into(),
            hint: "How visible the numbers of workspaces you're not on are.".into(),
            value: "45%".into(),
            amount: 0.45,
            min: 0.0,
            max: 1.0,
            ..Row::default()
        },
        Row {
            kind: "disclosure".into(),
            id: "shape".into(),
            label: "Shape and spacing".into(),
            ..Row::default()
        },
    ];
    p = p.push(bar);

    let mut style = group("Bar style sheet");
    style.hint = "The CSS file behind the settings above, for changes they don't cover.".into();
    style.rows = vec![Row {
        kind: "disclosure".into(),
        id: "css".into(),
        label: "Edit the CSS".into(),
        ..Row::default()
    }];
    p = p.push(style);

    let mut launcher = group("Launcher");
    launcher.hint = "Apps and categories the launcher lists first.".into();
    launcher.rows = vec![
        Row {
            kind: "line".into(),
            id: "shown-first".into(),
            label: "Shown first".into(),
            hint: "Zen Browser, kitty, Files".into(),
            action: "Edit".into(),
            ..Row::default()
        },
        Row {
            kind: "disclosure".into(),
            id: "launcher-more".into(),
            label: "More".into(),
            ..Row::default()
        },
    ];
    p = p.push(launcher);

    let mut shots = group("Screenshots");
    shots.rows = vec![
        Row {
            kind: "line".into(),
            id: "save-to".into(),
            label: "Save to".into(),
            hint: "~/Pictures/Screenshots".into(),
            action: "Change".into(),
            ..Row::default()
        },
        toggle(
            "shot-notify",
            "Show a notification",
            "With a preview after each screenshot.",
            true,
        ),
        toggle(
            "shot-freeze",
            "Freeze the screen while selecting",
            "Handy for menus and videos that would change.",
            false,
        ),
        Row {
            kind: "line".into(),
            id: "shot-shortcuts".into(),
            label: "Shortcuts".into(),
            hint: "I set up. Change them in Keyboard.".into(),
            action: "Open".into(),
            value: "Take one now".into(),
            ..Row::default()
        },
        Row {
            kind: "disclosure".into(),
            id: "shot-advanced".into(),
            label: "Advanced".into(),
            ..Row::default()
        },
    ];
    p = p.push(shots);

    let mut clip = group("Clipboard");
    clip.rows = vec![
        Row {
            kind: "line".into(),
            id: "clip-history".into(),
            label: "Clipboard history".into(),
            hint: "Super + V shows what you copied recently.".into(),
            action: "Open".into(),
            ..Row::default()
        },
        Row {
            kind: "line".into(),
            id: "clip-service".into(),
            label: "Keep clipboard history".into(),
            hint: "Stopped · doesn't start at login".into(),
            action: "Start".into(),
            value: "Logs".into(),
            ..Row::default()
        },
    ];
    p = p.push(clip);

    let mut notes = group("Quick notes");
    notes.hint = "Super + U opens a note. BOS sorts it into a note, reminder or task.".into();
    notes.rows = vec![
        Row {
            kind: "line".into(),
            id: "notes-folder".into(),
            label: "Notes folder".into(),
            hint: "~/Documents/Notes".into(),
            action: "Open".into(),
            ..Row::default()
        },
        toggle("notes-login", "Start on login", "", true),
    ];
    p = p.push(notes);

    let mut calendar = group("Calendar");
    calendar.hint = "Notes with a date show up here, and on the bar.".into();
    calendar.rows = vec![Row {
        kind: "line".into(),
        id: "cal-source".into(),
        label: "Calendar".into(),
        hint: "No calendars connected.".into(),
        action: "Add".into(),
        ..Row::default()
    }];
    p = p.push(calendar);

    let mut search = group("File search");
    search.hint = "Find files by name from the launcher.".into();
    search.rows = vec![
        Row {
            kind: "select".into(),
            id: "search-scope".into(),
            label: "Search".into(),
            value: "My files".into(),
            options: vec!["My files".into(), "Everything".into()],
            ..Row::default()
        },
        toggle("search-index", "Keep the index up to date", "", true),
        Row {
            kind: "line".into(),
            id: "search-rebuild".into(),
            label: "Rebuild the index".into(),
            hint: "Takes a minute or two.".into(),
            action: "Rebuild".into(),
            ..Row::default()
        },
    ];
    p = p.push(search);

    let mut auto = group("Automations");
    auto.hint = "Rules that run when something happens.".into();
    auto.rows = vec![
        Row {
            kind: "hint".into(),
            id: "auto-none".into(),
            label: "No automations yet.".into(),
            ..Row::default()
        },
        Row {
            kind: "leftbutton".into(),
            id: "auto-add".into(),
            action: "Add an automation".into(),
            ..Row::default()
        },
    ];
    p = p.push(auto);

    let mut daemon = group("Daemon");
    daemon.rows = vec![
        toggle("breadd", "Run in the background", "", true),
        Row {
            kind: "line".into(),
            id: "breadd-restart".into(),
            label: "Restart the daemon".into(),
            hint: "Reloads every module and rule.".into(),
            action: "Restart".into(),
            ..Row::default()
        },
    ];
    p = p.push(daemon);

    let mut lua = group("Lua");
    lua.rows = vec![Row {
        kind: "line".into(),
        id: "lua-entry".into(),
        label: "Entry point".into(),
        hint: "~/.config/bread/init.lua".into(),
        action: "Edit".into(),
        ..Row::default()
    }];
    p = p.push(lua);

    let mut modules = group("Modules");
    modules.rows = vec![Row {
        kind: "line".into(),
        id: "modules".into(),
        label: "Loaded modules".into(),
        hint: "From ~/.config/bread/modules".into(),
        action: "Open".into(),
        ..Row::default()
    }];
    p = p.push(modules);

    let mut adapters = group("Adapters");
    adapters.rows = vec![
        toggle("adapter-hyprland", "Hyprland", "", true),
        toggle("adapter-udev", "Devices", "", true),
        toggle("adapter-power", "Power", "", true),
        toggle("adapter-network", "Network", "", true),
        toggle("adapter-bluetooth", "Bluetooth", "", true),
    ];
    p = p.push(adapters);

    let mut events = group("Events");
    events.rows = vec![Row {
        kind: "select".into(),
        id: "dedup".into(),
        label: "Ignore repeats within".into(),
        value: "250 ms".into(),
        options: vec!["100 ms".into(), "250 ms".into(), "500 ms".into(), "1 s".into()],
        ..Row::default()
    }];
    p = p.push(events);

    let mut notifications = group("Notifications");
    notifications.rows = vec![toggle(
        "breadd-notify",
        "Show a desktop notification",
        "When an automation runs.",
        false,
    )];
    p = p.push(notifications);

    let mut help = group("Help");
    help.hint = "How the desktop fits together, and where to ask.".into();
    help.rows = vec![
        Row {
            kind: "line".into(),
            id: "help-guide".into(),
            label: "Desktop guide".into(),
            hint: "Bar, launcher, screenshots and automations.".into(),
            action: "Open".into(),
            ..Row::default()
        },
        Row {
            kind: "line".into(),
            id: "help-shortcuts".into(),
            label: "Keyboard shortcuts".into(),
            action: "Open".into(),
            ..Row::default()
        },
    ];
    p.push(help)
}

// -------------------------------------------------------------------- keyboard

fn keyboard() -> Page {
    let mut p = page(
        "keyboard",
        "Devices",
        "Keyboard",
        "Layouts, shortcuts and input methods.",
    );
    let layout = run("hyprctl", &["getoption", "input:kb_layout"])
        .and_then(|s| s.strip_prefix("str: ").map(|v| v.trim().to_string()))
        .unwrap_or_else(|| "us".into());
    p = p.rows(
        "Keyboard layout",
        vec![Row {
            kind: "select".into(),
            id: "layout".into(),
            label: "Keyboard layout".into(),
            value: layout_label(&layout),
            options: vec![
                "English (US)".into(),
                "English (UK)".into(),
                "German".into(),
                "French".into(),
                "Spanish".into(),
                "Dvorak".into(),
            ],
            ..Row::default()
        }],
    );

    let mut keys = group("Shortcuts");
    keys.hint = "Every shortcut on this computer, grouped by what it does.".into();
    keys.chips = vec![
        "All".into(),
        "Apps".into(),
        "Windows".into(),
        "Workspaces".into(),
        "Media".into(),
    ];
    keys.rows = std::iter::once(Row {
        kind: "chips".into(),
        id: "key-categories".into(),
        options: vec![
            "All".into(),
            "Apps".into(),
            "Windows".into(),
            "Workspaces".into(),
            "Media".into(),
        ],
        selected: 0,
        ..Row::default()
    })
    .chain(keybind_rows())
    .collect();
    p = p.push(keys);

    let ime_status = crate::ime::status();
    let mut ime = group("Input method");
    ime.hint = "Type Japanese, Chinese or Korean with an input method.".into();
    ime.rows = vec![
        toggle(
            "ime-enabled",
            "Use an input method",
            if ime_status.running {
                "fcitx5 is running."
            } else if ime_status.installed {
                "fcitx5 is installed but not running."
            } else {
                "fcitx5 is not installed yet."
            },
            ime_status.enabled,
        ),
        Row {
            kind: "line".into(),
            id: "ime-config".into(),
            label: "Configure".into(),
            hint: "Fcitx5 settings.".into(),
            action: "Open".into(),
            ..Row::default()
        },
    ];
    p = p.push(ime);
    p.rows(
        "Packages",
        vec![Row {
            kind: "line".into(),
            id: "ime-package".into(),
            label: "Fcitx5".into(),
            hint: if ime_status.installed {
                "Installed"
            } else {
                "Not installed"
            }
            .into(),
            action: "Install".into(),
            ..Row::default()
        }],
    )
}

fn layout_label(code: &str) -> String {
    match code.split(',').next().unwrap_or(code) {
        "us" => "English (US)",
        "gb" => "English (UK)",
        "de" => "German",
        "fr" => "French",
        "es" => "Spanish",
        "dvorak" => "Dvorak",
        _ => code,
    }
    .into()
}

fn keybind_rows() -> Vec<Row> {
    let path = config_dir().join("hypr/binds.json");
    let Ok(text) = std::fs::read_to_string(path) else {
        return vec![Row {
            kind: "hint".into(),
            id: "no-binds".into(),
            label: "No shortcuts set up.".into(),
            ..Row::default()
        }];
    };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) else {
        return Vec::new();
    };
    let Some(items) = json.get("bindings").and_then(|b| b.as_array()) else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(|b| {
            let label = b.get("label").and_then(|v| v.as_str())?;
            let key = b.get("key").and_then(|v| v.as_str()).unwrap_or("");
            let mods = b
                .get("mods")
                .and_then(|v| v.as_array())
                .map(|m| {
                    m.iter()
                        .filter_map(|v| v.as_str())
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .unwrap_or_default();
            let keys = format!("{mods} {key}").trim().to_uppercase();
            Some(Row {
                kind: "line".into(),
                id: format!("bind:{label}"),
                label: label.into(),
                hint: keys,
                ..Row::default()
            })
        })
        .collect()
}

pub fn to_blocks(page: Page) -> Vec<crate::SpecBlock> {
    spec_blocks(page.blocks)
}

/// Turns page data into Slint models, moving strings and vectors instead of
/// cloning them: the whole page is transient, so peak memory is one copy.
pub fn spec_blocks(blocks: Vec<Block>) -> Vec<crate::SpecBlock> {
    blocks
        .into_iter()
        .map(|block| {
            let title: slint::SharedString = if block.kind == "group" {
                block.title.to_uppercase().into()
            } else {
                block.title.into()
            };
            crate::SpecBlock {
                kind: block.kind.into(),
                title,
                hint: block.hint.into(),
                icon: block.icon.into(),
                value: block.value.into(),
                action: block.action.into(),
                tag: block.tag.into(),
                on: block.on,
                primary: block.primary,
                amount: block.amount,
                chart: block.chart.into(),
                chart_area: block.chart_area.into(),
                arc: block.arc.into(),
                action_icon: block.action_icon.into(),
                chip_icons: slint::ModelRc::new(slint::VecModel::from(
                    block
                        .chip_icons
                        .into_iter()
                        .map(slint::SharedString::from)
                        .collect::<Vec<_>>(),
                )),
                rows: slint::ModelRc::new(slint::VecModel::from(
                    block
                        .rows
                        .into_iter()
                        .map(|r| crate::SpecRow {
                            kind: r.kind.into(),
                            id: r.id.into(),
                            label: r.label.into(),
                            hint: r.hint.into(),
                            value: r.value.into(),
                            detail: r.detail.into(),
                            icon: r.icon.into(),
                            action: r.action.into(),
                            tag: r.tag.into(),
                            avatar: r.avatar.into(),
                            badges: slint::ModelRc::new(slint::VecModel::from(
                                r.badges
                                    .into_iter()
                                    .map(slint::SharedString::from)
                                    .collect::<Vec<_>>(),
                            )),
                            chip_icons: slint::ModelRc::new(slint::VecModel::from(
                                r.chip_icons
                                    .into_iter()
                                    .map(slint::SharedString::from)
                                    .collect::<Vec<_>>(),
                            )),
                            checked: r.checked,
                            primary: r.primary,
                            enabled: r.enabled,
                            amount: r.amount,
                            min: r.min,
                            max: r.max,
                            options: slint::ModelRc::new(slint::VecModel::from(
                                r.options
                                    .into_iter()
                                    .map(slint::SharedString::from)
                                    .collect::<Vec<_>>(),
                            )),
                            selected: r.selected,
                        })
                        .collect::<Vec<_>>(),
                )),
                chips: slint::ModelRc::new(slint::VecModel::from(
                    block
                        .chips
                        .into_iter()
                        .map(slint::SharedString::from)
                        .collect::<Vec<_>>(),
                )),
                chip: block.chip,
            }
        })
        .collect()
}

// ---- Actions -------------------------------------------------------------
// Filled in page by page; a page reload always reflects what the write left
// on disk, the same way the shipping views re-read after a command.

#[derive(Debug, PartialEq)]
struct Planned {
    program: &'static str,
    args: Vec<String>,
}

fn planned_row(page: &str, id: &str, checked: bool, action: &str) -> Option<Planned> {
    match page {
        "sound" if action == "action" => {
            let (kind, name) = id.split_once(':')?;
            let target = match kind {
                "sink" => "set-default-sink",
                "source" => "set-default-source",
                _ => return None,
            };
            Some(Planned {
                program: "pactl",
                args: vec![target.to_string(), name.to_string()],
            })
        }
        "bluetooth" => {
            let address = id.strip_prefix("bt:")?;
            let command = match action {
                "action2" => "remove",
                _ if checked => "disconnect",
                _ => "connect",
            };
            Some(Planned {
                program: "bluetoothctl",
                args: vec![command.to_string(), address.to_string()],
            })
        }
        "network" if action == "toggle" => {
            let name = id.strip_prefix("vpn:")?;
            Some(Planned {
                program: "nmcli",
                args: ["connection", if checked { "down" } else { "up" }, "id", name]
                    .into_iter()
                    .map(str::to_string)
                    .collect(),
            })
        }
        "power" if id == "brightness" && action.starts_with("slider:") => {
            let percent = action.strip_prefix("slider:")?.parse::<f32>().ok()?.round() as u32;
            Some(Planned {
                program: "brightnessctl",
                args: vec!["set".into(), format!("{percent}%")],
            })
        }
        "power" if id == "power-mode" && action.starts_with("seg:") => {
            let index = action.strip_prefix("seg:")?.parse::<usize>().ok()?;
            let profile = match index {
                0 => "power-saver",
                2 => "performance",
                _ => "balanced",
            };
            Some(Planned {
                program: "powerprofilesctl",
                args: vec!["set".into(), profile.into()],
            })
        }
        _ => None,
    }
}

fn planned_block(page: &str, on: bool, action: &str) -> Option<Planned> {
    // Only the hero switch toggles the radio. Add device, Add a place and
    // Add VPN share the hero action buttons and must never change radio state.
    if action != "toggle" {
        return None;
    }
    match page {
        "network" => Some(Planned {
            program: "nmcli",
            args: vec!["radio".into(), "wifi".into(), if on { "off" } else { "on" }.into()],
        }),
        "bluetooth" => Some(Planned {
            program: "bluetoothctl",
            args: vec!["power".into(), if on { "off" } else { "on" }.into()],
        }),
        _ => None,
    }
}

fn run_planned(planned: Option<Planned>) -> Result<(), String> {
    let Some(planned) = planned else {
        return Ok(());
    };
    let args: Vec<&str> = planned.args.iter().map(String::as_str).collect();
    write(planned.program, &args)
}

/// Runs the write behind one row control, then reloads the page so the shown
/// state comes from the system again. Errors are returned in Page.status.
pub fn apply_action(page: &str, block: usize, row: usize, action: &str) -> Page {
    let current = load(page);
    if page == "network" {
        if let Some(index) = action
            .strip_prefix("chip:")
            .and_then(|value| value.parse::<usize>().ok())
        {
            return select_chip(page, block, index);
        }
        if action == "toggle" {
            if let Some(hero) = current.blocks.get(block).filter(|b| b.kind == "hero") {
                let error = crate::network::set_radio(!hero.on).err();
                let mut reloaded = load(page);
                if let Some(message) = error {
                    reloaded.status = format!("Could not apply that setting: {message}");
                }
                return reloaded;
            }
        }
        let id = current
            .blocks
            .get(block)
            .and_then(|b| b.rows.get(row))
            .map(|r| r.id.clone())
            .unwrap_or_default();
        let checked = current
            .blocks
            .get(block)
            .and_then(|b| b.rows.get(row))
            .map(|r| r.checked)
            .unwrap_or(false);
        let error = network_action(&id, action, checked).err();
        let mut reloaded = load(page);
        if let Some(message) = error {
            reloaded.status = format!("Could not apply that setting: {message}");
        }
        return reloaded;
    }
    if page == "printers" {
        let id = current
            .blocks
            .get(block)
            .and_then(|b| b.rows.get(row))
            .map(|r| r.id.clone())
            .unwrap_or_default();
        let action_name = action.to_string();
        let error = (|| -> Result<(), String> {
            if let Some(name) = id.strip_prefix("printer:") {
                if action_name == "action" {
                    let out = Command::new("lpoptions")
                        .args(["-d", name])
                        .output()
                        .map_err(|error| error.to_string())?;
                    if !out.status.success() {
                        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
                    }
                }
                return Ok(());
            }
            if (id == "find" || id == "printer-settings") && action_name == "action" {
                Command::new("system-config-printer")
                    .spawn()
                    .map(|_| ())
                    .map_err(|error| error.to_string())?;
            }
            Ok(())
        })()
        .err();
        let mut reloaded = load(page);
        if let Some(message) = error {
            reloaded.status = format!("Could not change the printer: {message}");
        }
        return reloaded;
    }
    if page == "keyboard" {
        let id = current
            .blocks
            .get(block)
            .and_then(|b| b.rows.get(row))
            .map(|r| r.id.clone())
            .unwrap_or_default();
        let checked = current
            .blocks
            .get(block)
            .and_then(|b| b.rows.get(row))
            .map(|r| r.checked)
            .unwrap_or(false);
        let error = keyboard_action(&id, action, checked).err();
        let mut reloaded = load(page);
        if let Some(message) = error {
            reloaded.status = format!("Could not change the keyboard: {message}");
        }
        return reloaded;
    }
    if page == "sound" {
        let id = current
            .blocks
            .get(block)
            .and_then(|b| b.rows.get(row))
            .map(|r| r.id.clone())
            .unwrap_or_default();
        let checked = current
            .blocks
            .get(block)
            .and_then(|b| b.rows.get(row))
            .map(|r| r.checked)
            .unwrap_or(false);
        let error = sound_action(&id, action, checked).err();
        let mut reloaded = load(page);
        if let Some(message) = error {
            reloaded.status = format!("Could not change the sound: {message}");
        }
        return reloaded;
    }
    if page == "sharing" {
        let id = current
            .blocks
            .get(block)
            .and_then(|b| b.rows.get(row))
            .map(|r| r.id.clone())
            .unwrap_or_default();
        let checked = current
            .blocks
            .get(block)
            .and_then(|b| b.rows.get(row))
            .map(|r| r.checked)
            .unwrap_or(false);
        let error = sharing_action(&id, action, checked).err();
        let mut reloaded = load(page);
        if let Some(message) = error {
            reloaded.status = format!("Could not apply that setting: {message}");
        }
        return reloaded;
    }
    if page == "bluetooth" {
        if action == "toggle" {
            if let Some(hero) = current.blocks.get(block).filter(|b| b.kind == "hero") {
                let error = crate::bluetooth::set_power(!hero.on).err();
                let mut reloaded = load(page);
                if let Some(message) = error {
                    reloaded.status = format!("Could not apply that setting: {message}");
                }
                return reloaded;
            }
        }
        if action == "action" {
            if let Some(hero) = current.blocks.get(block).filter(|b| b.kind == "hero") {
                if hero.action == "Add device" {
                    let error = bluetooth_action("add-device", "action", false).err();
                    let mut reloaded = load(page);
                    if let Some(message) = error {
                        reloaded.status = format!("Could not scan: {message}");
                    }
                    return reloaded;
                }
            }
        }
        let id = current
            .blocks
            .get(block)
            .and_then(|b| b.rows.get(row))
            .map(|r| r.id.clone())
            .unwrap_or_default();
        let checked = current
            .blocks
            .get(block)
            .and_then(|b| b.rows.get(row))
            .map(|r| r.checked)
            .unwrap_or(false);
        let error = bluetooth_action(&id, action, checked).err();
        let mut reloaded = load(page);
        if let Some(message) = error {
            reloaded.status = format!("Could not apply that setting: {message}");
        }
        return reloaded;
    }
    if page == "privacy" {
        let id = current
            .blocks
            .get(block)
            .and_then(|b| b.rows.get(row))
            .map(|r| r.id.clone())
            .unwrap_or_default();
        let error = privacy_action(&id, action).err();
        let mut reloaded = load(page);
        if let Some(message) = error {
            reloaded.status = format!("Could not save that setting: {message}");
        }
        return reloaded;
    }
    if page == "storage" {
        let id = current
            .blocks
            .get(block)
            .and_then(|b| b.rows.get(row))
            .map(|r| r.id.clone())
            .unwrap_or_default();
        let error = storage_action(&id, action).err();
        let mut reloaded = load(page);
        if let Some(message) = error {
            reloaded.status = format!("Could not clean that up: {message}");
        }
        return reloaded;
    }
    if page == "notifications" {
        let id = current
            .blocks
            .get(block)
            .and_then(|b| b.rows.get(row))
            .map(|r| r.id.clone())
            .unwrap_or_default();
        let error = notifications_action(&id, action).err();
        let mut reloaded = load(page);
        if let Some(message) = error {
            reloaded.status = format!("Could not save that setting: {message}");
        }
        return reloaded;
    }
    if page == "displays" {
        let id = current
            .blocks
            .get(block)
            .and_then(|b| b.rows.get(row))
            .map(|r| r.id.clone())
            .unwrap_or_default();
        let error = displays_action(&id, action).err();
        let mut reloaded = load(page);
        if let Some(message) = error {
            reloaded.status = format!("Could not apply that setting: {message}");
        }
        return reloaded;
    }
    if page == "startup" {
        let id = current
            .blocks
            .get(block)
            .and_then(|b| b.rows.get(row))
            .map(|r| r.id.clone())
            .unwrap_or_default();
        let error = startup_action(&id, action).err();
        let mut reloaded = load(page);
        if let Some(message) = error {
            reloaded.status = format!("Could not apply that setting: {message}");
        }
        return reloaded;
    }
    if page == "lock" {
        let id = current
            .blocks
            .get(block)
            .and_then(|b| b.rows.get(row))
            .map(|r| r.id.clone())
            .unwrap_or_default();
        let error = lock_action(&id, action).err();
        let mut reloaded = load(page);
        if let Some(message) = error {
            reloaded.status = format!("Could not save that setting: {message}");
        }
        return reloaded;
    }
    if page == "memory" {
        let id = current
            .blocks
            .get(block)
            .and_then(|b| b.rows.get(row))
            .map(|r| r.id.clone())
            .unwrap_or_default();
        let error = memory_action(&id, action).err();
        let mut reloaded = load(page);
        if let Some(message) = error {
            reloaded.status = format!("Could not save that setting: {message}");
        }
        return reloaded;
    }
    if page == "mouse" || page == "a11y" {
        let id = current
            .blocks
            .get(block)
            .and_then(|b| b.rows.get(row))
            .map(|r| r.id.clone())
            .unwrap_or_default();
        let error = preferences_action(page, &id, action).err();
        let mut reloaded = load(page);
        if let Some(message) = error {
            reloaded.status = format!("Could not save that setting: {message}");
        }
        return reloaded;
    }
    let planned = current
        .blocks
        .get(block)
        .and_then(|b| b.rows.get(row))
        .and_then(|r| planned_row(page, &r.id, r.checked, action));
    let error = run_planned(planned).err();
    let mut reloaded = load(page);
    if let Some(message) = error {
        reloaded.status = format!("Could not apply that setting: {message}");
    }
    reloaded
}

/// Hero toggles change radio state rather than a row, so they look at the
/// block's current value to decide which way to switch.
pub fn apply_block_action(page: &str, block: usize, action: &str) -> Page {
    let current = load(page);
    let planned = current
        .blocks
        .get(block)
        .and_then(|b| planned_block(page, b.on, action));
    let error = run_planned(planned).err();
    let mut reloaded = load(page);
    if let Some(message) = error {
        reloaded.status = format!("Could not apply that setting: {message}");
    }
    reloaded
}

pub fn select_chip(page: &str, _block: usize, chip: usize) -> Page {
    if page == "network" {
        let (places, _) = crate::network::places();
        let error = if chip < places.len() {
            crate::network::switch_place(&places[chip].name).err()
        } else {
            crate::network::add_place()
                .and_then(|name| {
                    *network_editing_cell()
                        .lock()
                        .unwrap_or_else(|e| e.into_inner()) = Some(name.clone());
                    crate::network::switch_place(&name)
                })
                .err()
        };
        let mut reloaded = load(page);
        if let Some(message) = error {
            reloaded.status = format!("Could not switch place: {message}");
        }
        return reloaded;
    }
    load(page)
}

#[cfg(test)]
mod action_tests {
    use super::{planned_block, planned_row, Planned};

    fn plan(page: &str, id: &str, checked: bool, action: &str) -> Option<Planned> {
        planned_row(page, id, checked, action)
    }

    #[test]
    fn sound_use_switches_the_default_device() {
        assert_eq!(
            plan("sound", "sink:alsa_output.hdmi", false, "action"),
            Some(Planned {
                program: "pactl",
                args: vec!["set-default-sink".into(), "alsa_output.hdmi".into()],
            })
        );
        assert_eq!(
            plan("sound", "source:alsa_input.mic", false, "action"),
            Some(Planned {
                program: "pactl",
                args: vec!["set-default-source".into(), "alsa_input.mic".into()],
            })
        );
        assert_eq!(plan("sound", "mixer", false, "action"), None);
    }

    #[test]
    fn bluetooth_actions_use_the_address() {
        assert_eq!(
            plan("bluetooth", "bt:AC:80:0A:C9:E6", true, "action"),
            Some(Planned {
                program: "bluetoothctl",
                args: vec!["disconnect".into(), "AC:80:0A:C9:E6".into()],
            })
        );
        assert_eq!(
            plan("bluetooth", "bt:AC:80:0A:C9:E6", false, "action"),
            Some(Planned {
                program: "bluetoothctl",
                args: vec!["connect".into(), "AC:80:0A:C9:E6".into()],
            })
        );
        assert_eq!(
            plan("bluetooth", "bt:AC:80:0A:C9:E6", true, "action2"),
            Some(Planned {
                program: "bluetoothctl",
                args: vec!["remove".into(), "AC:80:0A:C9:E6".into()],
            })
        );
    }

    #[test]
    fn vpn_toggle_reflects_the_shown_state() {
        assert_eq!(
            plan("network", "vpn:wg0", true, "toggle"),
            Some(Planned {
                program: "nmcli",
                args: vec!["connection".into(), "down".into(), "id".into(), "wg0".into()],
            })
        );
        assert_eq!(
            plan("network", "vpn:wg0", false, "toggle"),
            Some(Planned {
                program: "nmcli",
                args: vec!["connection".into(), "up".into(), "id".into(), "wg0".into()],
            })
        );
    }

    #[test]
    fn power_slider_and_profile_map_to_their_tools() {
        assert_eq!(
            plan("power", "brightness", false, "slider:42"),
            Some(Planned {
                program: "brightnessctl",
                args: vec!["set".into(), "42%".into()],
            })
        );
        assert_eq!(
            plan("power", "power-mode", false, "seg:0"),
            Some(Planned {
                program: "powerprofilesctl",
                args: vec!["set".into(), "power-saver".into()],
            })
        );
        assert_eq!(
            plan("power", "power-mode", false, "seg:2"),
            Some(Planned {
                program: "powerprofilesctl",
                args: vec!["set".into(), "performance".into()],
            })
        );
    }

    #[test]
    fn write_reports_a_missing_command() {
        assert!(super::write("bos-no-such-command", &[]).is_err());
    }

    #[test]
    fn radios_toggle_against_the_shown_state() {
        assert_eq!(
            planned_block("network", true, "toggle"),
            Some(Planned {
                program: "nmcli",
                args: vec!["radio".into(), "wifi".into(), "off".into()],
            })
        );
        assert_eq!(
            planned_block("bluetooth", false, "toggle"),
            Some(Planned {
                program: "bluetoothctl",
                args: vec!["power".into(), "on".into()],
            })
        );
        assert_eq!(planned_block("updates", false, "toggle"), None);
    }

    #[test]
    fn keyboard_layouts_cycle_through_the_list() {
        assert_eq!(super::next_layout("us"), "gb");
        assert_eq!(super::next_layout("gb,us"), "de");
        assert_eq!(super::next_layout("dvorak"), "us");
        assert_eq!(super::next_layout("unknown"), "gb");
    }

    #[test]
    fn bug_flags_keep_the_rest_of_the_file() {
        let dir = std::env::temp_dir().join(format!(
            "bos-settings-slint-bug-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("breadbug.toml");
        std::fs::write(&path, "# keep\nnotify_fixed = true\nexcluded = [\"x\"]\n").unwrap();
        super::set_bug_flag(&path, "upload_enabled", true).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("# keep"));
        assert!(text.contains("excluded = [\"x\"]"));
        assert!(text.contains("notify_fixed = true"));
        assert!(text.contains("upload_enabled = true"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn memory_actions_write_the_real_config_shape() {
        let dir = std::env::temp_dir().join(format!(
            "bos-settings-slint-memory-actions-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("breadram.toml");
        std::fs::write(
            &path,
            "mode = \"monitor\"\n\n[automation]\nallowed_actions = [\"reclaim_protection\"]\n\n[protection]\nprotected = [\"exe:Hyprland\", \"unit:logind.service\"]\n\n[[providers]]\nname = \"llama-server\"\nkind = \"llama_cpp\"\nurl = \"http://127.0.0.1:7301\"\nscope = \"unit:llamacpp.service\"\nallow_automation = true\ncommand = [\"/usr/bin/hook\"]\n",
        )
        .unwrap();
        super::memory_action_at(&path, "mode", "select").unwrap();
        assert_eq!(
            crate::breadram::load_from(&path).unwrap().mode.as_deref(),
            Some("automatic")
        );
        super::memory_action_at(&path, "cgroup_soft_limit", "toggle").unwrap();
        let cfg = crate::breadram::load_from(&path).unwrap();
        assert!(cfg
            .allowed_actions
            .contains(&"cgroup_soft_limit".to_string()));
        super::memory_action_at(&path, "reclaim_protection", "toggle").unwrap();
        let cfg = crate::breadram::load_from(&path).unwrap();
        assert!(!cfg
            .allowed_actions
            .contains(&"reclaim_protection".to_string()));
        super::memory_action_at(&path, "never-touch", "tag-remove:0").unwrap();
        let cfg = crate::breadram::load_from(&path).unwrap();
        assert_eq!(cfg.protected, vec!["unit:logind.service".to_string()]);
        super::memory_action_at(&path, "profile", "seg:2").unwrap();
        let cfg = crate::breadram::load_from(&path).unwrap();
        assert_eq!(cfg.mode.as_deref(), Some("automatic"));
        assert!(cfg
            .allowed_actions
            .contains(&"queue_background_job".to_string()));
        super::memory_action_at(&path, "never-touch", "tag-add:unit:foo.service").unwrap();
        let cfg = crate::breadram::load_from(&path).unwrap();
        assert!(cfg.protected.contains(&"unit:foo.service".to_string()));
        super::memory_action_at(&path, "jobs-total", "input:12.5").unwrap();
        let cfg = crate::breadram::load_from(&path).unwrap();
        assert_eq!(cfg.jobs_total_budget_gib, Some(12.5));
        super::memory_action_at(&path, "provider:llama-server", "provider-url:http://127.0.0.1:7401").unwrap();
        let cfg = crate::breadram::load_from(&path).unwrap();
        assert_eq!(cfg.providers[0].url, "http://127.0.0.1:7401");
        super::memory_action_at(&path, "provider:llama-server", "provider-kind").unwrap();
        let cfg = crate::breadram::load_from(&path).unwrap();
        assert_eq!(cfg.providers[0].kind, "vllm");
        super::memory_action_at(&path, "add-provider", "action").unwrap();
        let cfg = crate::breadram::load_from(&path).unwrap();
        assert_eq!(cfg.providers.len(), 2);
        super::memory_action_at(&path, "provider:llama-server", "action").unwrap();
        let cfg = crate::breadram::load_from(&path).unwrap();
        assert_eq!(cfg.providers.len(), 1);
        assert_eq!(cfg.providers[0].name, "New backend");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn hero_add_buttons_do_not_touch_the_radio() {
        for page in ["network", "bluetooth"] {
            assert_eq!(
                planned_block(page, true, "action"),
                None,
                "{page} Add action must not toggle"
            );
            assert_eq!(
                planned_block(page, false, "again"),
                None,
                "{page} second Add action must not toggle"
            );
        }
        assert!(planned_block("network", true, "toggle").is_some());
    }
}

#[cfg(test)]
mod startup_tests {
    #[test]
    fn toggling_hidden_keeps_the_entry() {
        let dir = std::env::temp_dir().join(format!(
            "bos-settings-slint-startup-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("thing.desktop");
        std::fs::write(
            &path,
            "[Desktop Entry]\nType=Application\nName=Thing\nExec=thing\n",
        )
        .unwrap();
        let id = format!("startup:{}", path.display());
        super::startup_action(&id, "toggle").unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("Hidden=true"));
        assert!(text.contains("Exec=thing"));
        super::startup_action(&id, "toggle").unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(!text.contains("Hidden=true"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
