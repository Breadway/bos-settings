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
    add_block.rows = vec![add];
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

#[derive(Clone)]
struct DesktopPrefs {
    text_size: i64,
    cursor_size: i64,
    stronger_borders: bool,
    reduce_motion: bool,
}

impl Default for DesktopPrefs {
    fn default() -> Self {
        Self {
            text_size: 14,
            cursor_size: 24,
            stronger_borders: false,
            reduce_motion: false,
        }
    }
}

fn desktop_prefs() -> DesktopPrefs {
    let path = config_dir().join("hypr/bos-settings-v2.json");
    let value: serde_json::Value = std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| serde_json::json!({}));
    DesktopPrefs {
        text_size: value.get("text_size").and_then(|v| v.as_i64()).unwrap_or(14),
        cursor_size: value
            .get("cursor_size")
            .and_then(|v| v.as_i64())
            .unwrap_or(24),
        stronger_borders: value
            .get("stronger_borders")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        reduce_motion: value
            .get("reduce_motion")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
    }
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
            kind: "item".into(),
            id: format!("user:{username}"),
            label: if full_name.is_empty() { username.clone() } else { full_name.clone() },
            hint: sub.join(" · "),
            action: "Change password".into(),
            ..Row::default()
        };
        row.tag = if username == current { "".into() } else { "Remove".into() };
        rows.push(row);
    }
    let mut add = item("add-user", "", "Add a person", "");
    add.action = "Add".into();
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
    let mut output = group("Output");
    output.rows = pactl_devices("sink");
    p = p.push(output);
    let mut input = group("Input");
    input.rows = pactl_devices("source");
    p = p.push(input);
    let mut apps = group("Apps");
    apps.hint = "Apps that play sound show up here.".into();
    apps.rows = vec![Row {
        kind: "leftbutton".into(),
        id: "mixer".into(),
        action: "Open full mixer".into(),
        ..Row::default()
    }];
    p.push(apps)
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

fn displays() -> Page {
    let mut p = page(
        "displays",
        "Devices",
        "Displays",
        "Drag to arrange. Every change can be kept or reverted.",
    );
    let monitors = monitor_count();
    let mut arrangement = group("Arrangement");
    if monitors == 0 {
        arrangement.rows = vec![Row {
            kind: "hint".into(),
            id: "no-monitors".into(),
            label: "No monitors detected.".into(),
            ..Row::default()
        }];
        p = p.push(arrangement);
    } else {
        arrangement.rows = vec![Row {
            kind: "text".into(),
            id: "monitor-count".into(),
            label: format!("{monitors} connected"),
            ..Row::default()
        }];
        p = p.push(arrangement);
    }
    let night = run("hyprctl", &["getoption", "decoration:screen_shader"])
        .map(|s| !s.contains("[[EMPTY]]"))
        .unwrap_or(false);
    let mut night_block = group("NIGHT LIGHT");
    night_block.hint = "A warmer screen after dark helps you wind down.".into();
    night_block.rows = vec![toggle("night-light", "Night light", "", night)];
    p.push(night_block)
}

fn monitor_count() -> usize {
    let out = run("hyprctl", &["monitors", "-j"]).unwrap_or_default();
    out.matches("\"name\"").count()
}

// ------------------------------------------------------------------- bluetooth

fn bluetooth() -> Page {
    let mut p = page(
        "bluetooth",
        "Connect",
        "Bluetooth",
        "Your devices, with battery where they report it.",
    );
    let show = run("bluetoothctl", &["show"]).unwrap_or_default();
    let powered = show.lines().any(|l| l.trim() == "Powered: yes");
    let devices = bluetooth_devices();
    let connected = devices.iter().filter(|d| d.1).count();
    let mut h = hero(
        &format!("Bluetooth is {}", if powered { "on" } else { "off" }),
        &if powered {
            format!("{connected} connected · {} paired", devices.len())
        } else {
            "Turn it on to use your headphones, controllers and other devices.".into()
        },
    );
    h.tag = "toggle".into();
    h.on = powered;
    h.icon = crate::icons::commands("bluetooth").into();
    if powered {
        h.action = "Add device".into();
        h.primary = true;
    }
    p = p.push(h);
    if powered {
        let mut block = Block {
            kind: "devgrid".into(),
            ..Block::default()
        };
        for (name, is_connected, battery) in devices {
            block.rows.push(Row {
                kind: "dev".into(),
                id: format!("bt:{name}"),
                label: name.clone(),
                hint: format!(
                    "{}{}",
                    if is_connected { "Connected" } else { "Not connected" },
                    battery
                        .map(|b| format!(" · {b}% battery"))
                        .unwrap_or_default()
                ),
                icon: crate::icons::commands("headphones").into(),
                action: if is_connected { "Disconnect".into() } else { "Connect".into() },
                checked: is_connected,
                primary: !is_connected,
                ..Row::default()
            });
        }
        p.push(block)
    } else {
        p
    }
}

fn bluetooth_devices() -> Vec<(String, bool, Option<u32>)> {
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
        rows.push((name.trim().to_string(), connected, battery));
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
    let wifi_on = run("nmcli", &["radio", "wifi"])
        .map(|s| s.trim() == "enabled")
        .unwrap_or(false);
    let wired = run("nmcli", &["-t", "-f", "TYPE", "device", "status"])
        .map(|s| s.lines().any(|l| l == "ethernet"))
        .unwrap_or(false);
    let saved = run("nmcli", &["-t", "-f", "NAME", "connection", "show"])
        .map(|s| s.lines().filter(|l| !l.trim().is_empty()).count())
        .unwrap_or(0);
    let mut h = if wired {
        hero(
            "Wired connection",
            &format!("Connected by cable{}", if wifi_on { " · Wi-Fi is on" } else { "" }),
        )
    } else {
        hero(
            if wifi_on { "Wi-Fi is on" } else { "Wi-Fi is off" },
            &format!("{} saved networks", saved),
        )
    };
    h.icon = crate::icons::commands(if wired { "ethernet" } else { "wifi" }).into();
    h.tag = "toggle".into();
    h.on = wifi_on;
    p = p.push(h);

    let mut places = group("Places");
    places.hint = "A place is a set of networks and rules. BOS picks the right one when it sees a familiar network nearby.".into();
    places.rows = vec![
        Row {
            kind: "chips".into(),
            id: "place-tabs".into(),
            options: vec![
                "Away".into(),
                "Home".into(),
                "School".into(),
                "+ Add a place".into(),
            ],
            selected: 1,
            ..Row::default()
        },
        Row {
            kind: "line".into(),
            id: "place-away".into(),
            label: "Away".into(),
            hint: "Join Totally NCC, or any other saved network.".into(),
            action: "Edit".into(),
            ..Row::default()
        },
        Row {
            kind: "line".into(),
            id: "place-home".into(),
            label: "Home · in use".into(),
            hint: "Fallback — Join Totally NCC, NetComm 4887_5G or NetComm 4887. Recognised when NetComm 4887 is nearby.".into(),
            action: "Edit".into(),
            ..Row::default()
        },
        Row {
            kind: "line".into(),
            id: "place-school".into(),
            label: "School".into(),
            hint: "Join NCC. Recognised when NCC is nearby.".into(),
            action: "Edit".into(),
            ..Row::default()
        },
    ];
    p = p.push(places);

    let mut nearby = group("Nearby networks");
    nearby.hint = "Looking for networks...".into();
    nearby.rows = vec![
        Row {
            kind: "hint".into(),
            id: "scanning".into(),
            label: "Scanning...".into(),
            ..Row::default()
        },
        Row {
            kind: "disclosure".into(),
            id: "saved".into(),
            label: format!("Saved networks ({saved})"),
            ..Row::default()
        },
    ];
    p = p.push(nearby);

    let vpns = run("nmcli", &["-t", "-f", "NAME,TYPE", "connection", "show"])
        .map(|s| {
            s.lines()
                .filter_map(|l| l.rsplit_once(':'))
                .filter(|(_, kind)| *kind == "wireguard" || *kind == "vpn")
                .map(|(name, _)| name.to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let mut vpn = group("VPN");
    for name in &vpns {
        let active = run("nmcli", &["-t", "-f", "NAME", "connection", "show", "--active"])
            .map(|s| s.lines().any(|l| l == name))
            .unwrap_or(false);
        vpn.rows.push(Row {
            kind: "line".into(),
            id: format!("vpn:{name}"),
            label: name.clone(),
            hint: "WireGuard".into(),
            tag: "toggle".into(),
            checked: active,
            ..Row::default()
        });
    }
    if vpn.rows.is_empty() {
        vpn.rows.push(Row {
            kind: "hint".into(),
            id: "no-vpn".into(),
            label: "No VPNs set up.".into(),
            ..Row::default()
        });
    }
    vpn.rows.push(Row {
        kind: "leftbutton".into(),
        id: "add-vpn".into(),
        action: "Add VPN".into(),
        ..Row::default()
    });
    p = p.push(vpn);

    let mut advanced = group("Advanced network settings");
    advanced.rows = vec![Row {
        kind: "disclosure".into(),
        id: "advanced".into(),
        label: "DNS, Tailscale and connection checks".into(),
        ..Row::default()
    }];
    p.push(advanced)
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
            value: "Normal".into(),
            amount: 0.0,
            min: -1.0,
            max: 1.0,
            ..Row::default()
        }],
    );
    p = p.rows(
        "Touchpad",
        vec![
            toggle("tap", "Tap to click", "", prefs.stronger_borders),
            toggle("natural", "Natural scrolling", "", true),
            toggle("disable-typing", "Disable while typing", "", true),
            Row {
                kind: "select".into(),
                id: "click-method".into(),
                label: "Click method".into(),
                value: "Click with fingers".into(),
                options: vec!["Click with fingers".into(), "Bottom corners".into()],
                ..Row::default()
            },
            Row {
                kind: "slider".into(),
                id: "scroll-factor".into(),
                label: "Scroll speed".into(),
                value: "100%".into(),
                amount: 1.0,
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
    p = p.rows(
        "Look",
        vec![
            Row {
                kind: "seg".into(),
                id: "background".into(),
                label: "Background".into(),
                options: vec!["Picture".into(), "Colour".into()],
                selected: if cfg.background_mode == "color" { 1 } else { 0 },
                ..Row::default()
            },
            Row {
                kind: "seg".into(),
                id: "clock".into(),
                label: "Clock".into(),
                options: vec!["24-hour".into(), "12-hour".into()],
                selected: if cfg.clock.contains("%I") { 1 } else { 0 },
                ..Row::default()
            },
            toggle("seconds", "Show seconds", "", cfg.clock.contains("%S")),
            Row {
                kind: "select".into(),
                id: "font".into(),
                label: "Font".into(),
                value: cfg.font.clone(),
                options: vec![
                    cfg.font.clone(),
                    "Varela Round".into(),
                    "Outfit".into(),
                    "Inter".into(),
                    "Noto Sans".into(),
                ],
                ..Row::default()
            },
        ],
    );
    p
}

struct LockConf {
    background_mode: String,
    clock: String,
    font: String,
}

fn breadlock_conf() -> LockConf {
    let path = config_dir().join("bread/breadlock.toml");
    let value = std::fs::read_to_string(path)
        .ok()
        .and_then(|s| s.parse::<toml::Value>().ok());
    let get = |key: &str, fallback: &str| {
        value
            .as_ref()
            .and_then(|v| v.get(key))
            .and_then(|v| v.as_str())
            .unwrap_or(fallback)
            .to_string()
    };
    LockConf {
        background_mode: get("background_mode", "image"),
        clock: get("clock_format", "%H:%M"),
        font: get("font_family", "Varela Round"),
    }
}

// --------------------------------------------------------------------- privacy

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
    let track = run("bakery", &["track", "show"])
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "stable".into());
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
    let mut channel = group("Channel");
    channel.hint = "Beta and dev builds get changes before stable. Switch back any time.".into();
    channel.rows = vec![Row {
        kind: "select".into(),
        id: "track".into(),
        label: "Channel".into(),
        value: track,
        options: vec!["Stable".into(), "Beta".into(), "Dev".into()],
        ..Row::default()
    }];
    p = p.push(channel);
    p.rows(
        "If an update breaks something",
        vec![Row {
            kind: "line".into(),
            id: "rollback".into(),
            label: "Go back to before the update".into(),
            hint: "Every update takes a snapshot first. Pick it from the boot menu, or choose it in Backup & restore.".into(),
            action: "Open".into(),
            ..Row::default()
        }],
    )
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
    devices
        .iter()
        .filter(|d| {
            d.get("Flags")
                .and_then(|f| f.as_array())
                .map(|flags| flags.iter().any(|f| f.as_str() == Some("updatable")))
                .unwrap_or(false)
        })
        .map(|d| {
            (
                d.get("Name").and_then(|v| v.as_str()).unwrap_or("Device").into(),
                d.get("Version").and_then(|v| v.as_str()).unwrap_or("-").into(),
            )
        })
        .collect()
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

fn memory() -> Page {
    let mut p = page(
        "memory",
        "System",
        "Memory",
        "The memory daemon, and what it may do on its own.",
    );
    let running = run("pgrep", &["-x", "breadram"])
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false);
    let mut daemon = group("Memory daemon");
    daemon.rows = vec![
        Row {
            kind: "line".into(),
            id: "daemon".into(),
            label: "breadram".into(),
            hint: if running {
                "Running.".into()
            } else {
                "Not running. It starts on demand when memory gets tight.".into()
            },
            action: if running { "Stop".into() } else { "Start now".into() },
            ..Row::default()
        },
        toggle("daemon-login", "Start on login", "", true),
        Row {
            kind: "line".into(),
            id: "daemon-config".into(),
            label: "Settings file".into(),
            hint: "~/.config/breadram/config.toml".into(),
            action: "Edit".into(),
            ..Row::default()
        },
    ];
    p = p.push(daemon);

    let mut automation = group("Automation");
    automation.rows = vec![
        Row {
            kind: "seg".into(),
            id: "mode".into(),
            label: "Mode".into(),
            options: vec!["Automatic".into(), "Ask first".into(), "Off".into()],
            selected: 0,
            ..Row::default()
        },
        toggle("act-restart-app", "Restart an app that is stuck", "", true),
        toggle("act-clear-cache", "Clear app caches", "", true),
        toggle("act-reduce-workers", "Reduce job workers", "", false),
        toggle("act-delay-jobs", "Delay background jobs", "", false),
        toggle("act-restart-session", "Restart the session", "", false),
        toggle("act-reboot", "Restart the computer", "", false),
    ];
    automation.rows.push(Row {
        kind: "chips".into(),
        id: "never-touch".into(),
        options: vec![
            "exeHyprland".into(),
            "units-systemd-logind.service".into(),
            "+ Never touch".into(),
        ],
        selected: -1,
        ..Row::default()
    });
    p = p.push(automation);

    let mut backends = group("Model backends");
    backends.rows = vec![
        Row {
            kind: "line".into(),
            id: "backend-llama".into(),
            label: "llama-server".into(),
            hint: "127.0.0.1:8081 · chat models".into(),
            action: "Edit".into(),
            ..Row::default()
        },
        Row {
            kind: "leftbutton".into(),
            id: "backend-add".into(),
            action: "Add a backend".into(),
            ..Row::default()
        },
    ];
    p = p.push(backends);

    let mut log = group("Log");
    log.rows = vec![Row {
        kind: "line".into(),
        id: "open-log".into(),
        label: "What the daemon did".into(),
        hint: "Kept for the last seven days.".into(),
        action: "Open log".into(),
        ..Row::default()
    }];
    p = p.push(log);

    let mut changes = group("Recent changes");
    changes.rows = vec![Row {
        kind: "hint".into(),
        id: "no-changes".into(),
        label: "Nothing yet.".into(),
        ..Row::default()
    }];
    p.push(changes)
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

    let mut ime = group("Input method");
    ime.hint = "Type Japanese, Chinese or Korean with an input method.".into();
    ime.rows = vec![
        toggle("ime-enabled", "Use an input method", "", false),
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
            hint: "Not installed".into(),
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

pub fn apply_action(page: &str, _block: usize, _row: usize, _action: &str) -> Page {
    load(page)
}

pub fn apply_block_action(page: &str, _block: usize, _action: &str) -> Page {
    load(page)
}

pub fn select_chip(page: &str, _block: usize, _chip: usize) -> Page {
    load(page)
}
