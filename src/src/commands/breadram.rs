//! breadram/breadram.toml: memory management daemon configuration plus live
//! daemon state.
//!
//! Two sources stay visibly separate. The file holds what the operator asked
//! for; the daemon reports what is actually in effect (mode, pressure,
//! capabilities, history). A missing daemon is ordinary: every live call
//! returns `present: false` with no error, so the panel opens and edits
//! settings with nothing running. Writes that change live behavior go
//! through the daemon's own validated IPC (`set_mode`, `set_protection`,
//! with or without persisting); structural file edits are non-destructive
//! TOML like every other view in this app, with an atomic backed-up write.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::config;

fn config_path() -> std::path::PathBuf {
    config::config_dir().join("breadram/breadram.toml")
}

fn socket_path() -> std::path::PathBuf {
    match std::env::var_os("XDG_RUNTIME_DIR") {
        Some(dir) if !dir.is_empty() => std::path::PathBuf::from(dir).join("breadram.sock"),
        _ => std::env::temp_dir().join("breadram.sock"),
    }
}

/// One socket round trip: hello handshake, one request, one answer line.
fn call(body: serde_json::Value) -> Result<serde_json::Value, String> {
    let path = socket_path();
    let timeout = Some(Duration::from_secs(3));
    let stream = UnixStream::connect(&path).map_err(|_| "unavailable".to_string())?;
    stream
        .set_read_timeout(timeout)
        .map_err(|e| e.to_string())?;
    stream
        .set_write_timeout(timeout)
        .map_err(|e| e.to_string())?;
    let mut reader = BufReader::new(stream.try_clone().map_err(|e| e.to_string())?);
    let mut writer = stream;
    let mut id = 1u64;
    let mut round = |request: serde_json::Value| -> Result<serde_json::Value, String> {
        let mut line = serde_json::json!({"id": id});
        id += 1;
        for (key, value) in request.as_object().cloned().unwrap_or_default() {
            line[key] = value;
        }
        let mut text = serde_json::to_string(&line).map_err(|e| e.to_string())?;
        text.push('\n');
        writer
            .write_all(text.as_bytes())
            .map_err(|e| e.to_string())?;
        let mut answer = String::new();
        reader.read_line(&mut answer).map_err(|e| e.to_string())?;
        serde_json::from_str(answer.trim()).map_err(|e| e.to_string())
    };
    let hello = round(serde_json::json!({
        "type": "hello", "major": 1, "minor": 8, "client": "bos-settings",
    }))?;
    let major = hello.get("major").and_then(|v| v.as_u64()).unwrap_or(0);
    if hello.get("type").and_then(|v| v.as_str()) != Some("hello") || major != 1 {
        return Err("incompatible".to_string());
    }
    round(body)
}

fn str_field(value: &serde_json::Value, key: &str) -> Option<String> {
    value.get(key).and_then(|v| v.as_str()).map(str::to_string)
}

// --- file model ---

#[derive(Serialize, Deserialize, Default)]
pub struct BreadramConfig {
    mode: Option<String>,
    allowed_actions: Vec<String>,
    protected: Vec<String>,
    providers: Vec<ProviderEntry>,
    jobs_enabled: Option<bool>,
    jobs_total_budget_gib: Option<f64>,
    jobs_default_estimate_gib: Option<f64>,
    jobs_auto_launch: Option<bool>,
}

#[derive(Serialize, Deserialize, Default)]
pub struct ProviderEntry {
    name: String,
    kind: String,
    url: String,
    scope: String,
    allow_automation: bool,
}

fn array_strings(value: Option<&toml_edit::Item>) -> Vec<String> {
    value
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// Read a list that lives in a table in daemon files but may sit flat in
/// hand-written ones: `[automation] allowed_actions` or top-level
/// `allowed_actions`, same for protection. The flat spelling is accepted on
/// read and normalized to the nested form on write.
fn get_list(doc: &toml_edit::DocumentMut, table: &str, key: &str) -> Vec<String> {
    let nested = doc
        .get(table)
        .and_then(|t| t.as_table())
        .and_then(|t| t.get(key));
    let found = array_strings(nested);
    if found.is_empty() {
        array_strings(doc.get(key))
    } else {
        found
    }
}

fn load_config_from(path: &std::path::Path) -> BreadramConfig {
    let doc = config::load_doc(path);
    let providers = doc
        .get("providers")
        .and_then(|t| t.as_array_of_tables())
        .map(|a| {
            a.iter()
                .map(|t| ProviderEntry {
                    name: t
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .into(),
                    kind: t
                        .get("kind")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .into(),
                    url: t
                        .get("url")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .into(),
                    scope: t
                        .get("scope")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .into(),
                    allow_automation: t
                        .get("allow_automation")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                })
                .collect()
        })
        .unwrap_or_default();
    let gib = |keys: &[&str]| {
        config::get_i64(&doc, keys).map(|bytes| bytes as f64 / 1024.0 / 1024.0 / 1024.0)
    };
    BreadramConfig {
        mode: config::get_str(&doc, &["mode"]),
        allowed_actions: get_list(&doc, "automation", "allowed_actions"),
        protected: get_list(&doc, "protection", "protected"),
        providers,
        jobs_enabled: config::get_bool(&doc, &["jobs", "enabled"]),
        jobs_total_budget_gib: gib(&["jobs", "total_budget_bytes"]),
        jobs_default_estimate_gib: gib(&["jobs", "default_estimate_bytes"]),
        jobs_auto_launch: config::get_bool(&doc, &["jobs", "auto_launch"]),
    }
}

/// A GiB figure the daemon can actually use, or why it cannot.
///
/// Rejects NaN and infinities (which would serialize back wrong), negatives,
/// and anything past a pebibyte (larger than any host this manages). The
/// check happens before any write, so a refused save changes nothing.
fn gib_to_bytes(gib: f64) -> Result<i64, String> {
    if !gib.is_finite() {
        return Err("memory size must be a real number".to_string());
    }
    if gib < 0.0 {
        return Err("memory size cannot be negative".to_string());
    }
    if gib > 1_048_576.0 {
        return Err("memory size above one pebibyte is not a real budget".to_string());
    }
    Ok((gib * 1024.0 * 1024.0 * 1024.0).round() as i64)
}

#[tauri::command]
pub fn get_breadram_config() -> BreadramConfig {
    load_config_from(&config_path())
}

fn save_config_to(path: &std::path::Path, cfg: BreadramConfig) -> Result<(), String> {
    let mut doc = config::load_doc(path);
    if let Some(mode) = cfg.mode.as_deref() {
        if !matches!(mode, "monitor" | "automatic") {
            return Err(format!("unknown management mode: {mode}"));
        }
        config::set_str(&mut doc, &["mode"], mode);
    }
    // Automation and protection live in tables in daemon files; accept both
    // the flat and nested spellings on read, write the nested form the
    // daemon documents.
    doc.as_table_mut().remove("allowed_actions");
    doc.as_table_mut().remove("protected");
    let mut automation = doc
        .get("automation")
        .and_then(|t| t.as_table().cloned())
        .unwrap_or_default();
    automation.remove("allowed_actions");
    let mut actions = toml_edit::Array::new();
    for action in &cfg.allowed_actions {
        actions.push(action.as_str());
    }
    automation["allowed_actions"] = toml_edit::Item::Value(toml_edit::Value::Array(actions));
    doc["automation"] = toml_edit::Item::Table(automation);
    let mut protection = doc
        .get("protection")
        .and_then(|t| t.as_table().cloned())
        .unwrap_or_default();
    protection.remove("protected");
    let mut scopes = toml_edit::Array::new();
    for scope in &cfg.protected {
        scopes.push(scope.as_str());
    }
    protection["protected"] = toml_edit::Item::Value(toml_edit::Value::Array(scopes));
    doc["protection"] = toml_edit::Item::Table(protection);
    if let Some(enabled) = cfg.jobs_enabled {
        config::set_bool(&mut doc, &["jobs", "enabled"], enabled);
    }
    if let Some(gib) = cfg.jobs_total_budget_gib {
        config::set_i64(
            &mut doc,
            &["jobs", "total_budget_bytes"],
            gib_to_bytes(gib).map_err(|e| format!("job budget: {e}"))?,
        );
    }
    if let Some(gib) = cfg.jobs_default_estimate_gib {
        config::set_i64(
            &mut doc,
            &["jobs", "default_estimate_bytes"],
            gib_to_bytes(gib).map_err(|e| format!("job estimate: {e}"))?,
        );
    }
    if let Some(auto) = cfg.jobs_auto_launch {
        config::set_bool(&mut doc, &["jobs", "auto_launch"], auto);
    }
    // Providers merge by name into the existing tables: the five modelled
    // keys are written, and anything the panel does not model (hook
    // commands, credential references, integrated-GPU flags, hook timeouts)
    // stays exactly where it was. Entries the panel removed are gone on
    // purpose; entries it added start clean.
    let mut providers = toml_edit::ArrayOfTables::new();
    for provider in &cfg.providers {
        let mut table = doc
            .get("providers")
            .and_then(|v| v.as_array_of_tables())
            .and_then(|existing| {
                existing
                    .iter()
                    .find(|t| {
                        t.get("name").and_then(|v| v.as_str()) == Some(provider.name.as_str())
                    })
                    .cloned()
            })
            .unwrap_or_default();
        table["name"] = toml_edit::value(provider.name.clone());
        table["kind"] = toml_edit::value(provider.kind.clone());
        table["url"] = toml_edit::value(provider.url.clone());
        table["scope"] = toml_edit::value(provider.scope.clone());
        table["allow_automation"] = toml_edit::value(provider.allow_automation);
        providers.push(table);
    }
    doc["providers"] = toml_edit::Item::ArrayOfTables(providers);
    config::save_doc(path, &doc).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_breadram_config(cfg: BreadramConfig) -> Result<(), String> {
    save_config_to(&config_path(), cfg)
}

/// Apply a named starting point: monitor, desktop, or headless-server.
/// Writes the mode, allowed actions, and protection those names resolve to,
/// so the file states explicitly what the daemon will do.
#[tauri::command]
pub fn apply_breadram_profile(profile: String) -> Result<(), String> {
    apply_profile_to(&config_path(), &profile)
}

fn apply_profile_to(path: &std::path::Path, profile: &str) -> Result<(), String> {
    let mut doc = config::load_doc(path);
    let (mode, actions, protected): (&str, &[&str], &[&str]) = match profile {
        "monitor" => ("monitor", &[], &[]),
        "desktop" => (
            "automatic",
            &[
                "reclaim_protection",
                "cgroup_soft_limit",
                "unload_idle_model",
            ],
            &["exe:Hyprland", "unit:systemd-logind.service"],
        ),
        "headless-server" => (
            "automatic",
            &[
                "cgroup_soft_limit",
                "reclaim_protection",
                "queue_background_job",
            ],
            &[],
        ),
        other => return Err(format!("unknown profile: {other}")),
    };
    config::set_str(&mut doc, &["mode"], mode);
    let mut automation = toml_edit::Table::new();
    let mut list = toml_edit::Array::new();
    for action in actions {
        list.push(*action);
    }
    automation["allowed_actions"] = toml_edit::Item::Value(toml_edit::Value::Array(list));
    doc["automation"] = toml_edit::Item::Table(automation);
    let mut protection = toml_edit::Table::new();
    let mut scopes = toml_edit::Array::new();
    for scope in protected {
        scopes.push(*scope);
    }
    protection["protected"] = toml_edit::Item::Value(toml_edit::Value::Array(scopes));
    doc["protection"] = toml_edit::Item::Table(protection);
    config::save_doc(path, &doc).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "bos-settings-breadram-test-{}-{name}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("breadram.toml")
    }

    fn sample_config() -> BreadramConfig {
        BreadramConfig {
            mode: Some("automatic".to_string()),
            allowed_actions: vec!["cgroup_soft_limit".to_string()],
            protected: vec!["exe:Hyprland".to_string()],
            providers: vec![ProviderEntry {
                name: "ollama".to_string(),
                kind: "ollama".to_string(),
                url: "http://127.0.0.1:11434".to_string(),
                scope: "unit:ollama.service".to_string(),
                allow_automation: true,
            }],
            jobs_enabled: Some(true),
            jobs_total_budget_gib: Some(8.0),
            jobs_default_estimate_gib: Some(2.0),
            jobs_auto_launch: Some(false),
        }
    }

    #[test]
    fn round_trip_preserves_everything_including_unmodelled_keys() {
        let path = scratch("round-trip");
        std::fs::write(&path, "# operator comment\n[custom]\nnote = \"keep me\"\n").unwrap();
        save_config_to(&path, sample_config()).unwrap();
        let loaded = load_config_from(&path);
        assert_eq!(loaded.mode.as_deref(), Some("automatic"));
        assert_eq!(loaded.allowed_actions, vec!["cgroup_soft_limit"]);
        assert_eq!(loaded.protected, vec!["exe:Hyprland"]);
        assert_eq!(loaded.providers.len(), 1);
        assert_eq!(loaded.providers[0].url, "http://127.0.0.1:11434");
        assert_eq!(loaded.jobs_enabled, Some(true));
        assert_eq!(loaded.jobs_total_budget_gib, Some(8.0));
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(
            text.contains("# operator comment") && text.contains("keep me"),
            "unmodelled keys and comments survive a save"
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn profiles_resolve_to_the_documented_settings() {
        let path = scratch("profiles");
        std::fs::write(&path, "").unwrap();
        apply_profile_to(&path, "desktop").unwrap();
        let loaded = load_config_from(&path);
        assert_eq!(loaded.mode.as_deref(), Some("automatic"));
        assert!(loaded
            .allowed_actions
            .contains(&"unload_idle_model".to_string()));
        assert!(loaded.protected.contains(&"exe:Hyprland".to_string()));
        apply_profile_to(&path, "monitor").unwrap();
        let loaded = load_config_from(&path);
        assert_eq!(loaded.mode.as_deref(), Some("monitor"));
        assert!(loaded.allowed_actions.is_empty());
        assert!(apply_profile_to(&path, "nope").is_err());
        let _ = std::fs::remove_file(&path);
    }

    use std::io::Write;
    use std::os::unix::net::UnixListener;

    /// One test holds the process-global runtime dir for its whole body, so
    /// the present and absent cases live in a single test rather than racing
    /// two tests over the same variable.
    #[test]
    fn live_calls_report_presence_and_degrade_to_absence() {
        let dir = std::env::temp_dir().join(format!(
            "bos-settings-breadram-test-{}-live",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let socket_path = dir.join("breadram.sock");
        let _ = std::fs::remove_file(&socket_path);
        let listener = UnixListener::bind(&socket_path).unwrap();
        std::thread::spawn(move || {
            // Each connection carries a hello plus one request. Answers are
            // chosen by request type, like the real daemon dispatches.
            for _ in 0..8 {
                let Ok((stream, _)) = listener.accept() else {
                    return;
                };
                let reader = std::io::BufReader::new(stream.try_clone().unwrap());
                let mut stream = stream;
                use std::io::BufRead;
                for line in reader.lines() {
                    let Ok(line) = line else { break };
                    let answer = if line.contains(r#""hello""#) {
                        r#"{"id":0,"type":"hello","major":1,"minor":8,"server":"fake"}"#.to_string()
                    } else if line.contains(r#""diagnostics""#) {
                        r#"{"id":0,"type":"diagnostics","mode":"automatic","version":"0.1.0","pressure":{"state":"pressured"},"memory":{"ram_available_bytes":3221225472,"ram_total_bytes":17179869184}}"#.to_string()
                    } else if line.contains(r#""rules""#) {
                        r#"{"id":0,"type":"rules","mode":"automatic"}"#.to_string()
                    } else if line.contains(r#""actions""#) {
                        r#"{"id":0,"type":"actions","plan":{},"history":[]}"#.to_string()
                    } else {
                        r#"{"id":0,"type":"error","code":"unknown_request","message":"nope"}"#
                            .to_string()
                    };
                    if stream.write_all(answer.as_bytes()).is_err() {
                        break;
                    }
                    if stream.write_all(b"\n").is_err() {
                        break;
                    }
                }
            }
        });
        let previous = std::env::var_os("XDG_RUNTIME_DIR");
        std::env::set_var("XDG_RUNTIME_DIR", &dir);
        let status = breadram_status();
        assert_eq!(status.get("present"), Some(&serde_json::Value::Bool(true)));
        assert_eq!(
            status
                .get("pressure")
                .and_then(|p| p.get("state"))
                .and_then(|s| s.as_str()),
            Some("pressured")
        );
        let actions = breadram_actions();
        assert_eq!(actions.get("present"), Some(&serde_json::Value::Bool(true)));
        // New socket, nobody listening any more: absence, not an error.
        std::fs::remove_file(&socket_path).unwrap();
        let gone = breadram_status();
        assert_eq!(gone.get("present"), Some(&serde_json::Value::Bool(false)));
        assert!(set_breadram_mode("automatic".to_string(), false).is_err());
        match previous {
            Some(value) => std::env::set_var("XDG_RUNTIME_DIR", value),
            None => std::env::remove_var("XDG_RUNTIME_DIR"),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn provider_saves_keep_unmodelled_keys() {
        let path = scratch("provider-merge");
        std::fs::write(
            &path,
            "[[providers]]\nname = \"hook\"\nkind = \"hook\"\ncommand = [\"/usr/bin/do\"]\ncredential_ref = \"x\"\n",
        )
        .unwrap();
        let mut cfg = sample_config();
        cfg.providers = vec![ProviderEntry {
            name: "hook".to_string(),
            kind: "hook".to_string(),
            url: "hook:do".to_string(),
            scope: "exe:do".to_string(),
            allow_automation: true,
        }];
        save_config_to(&path, cfg).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(
            text.contains("command = [\"/usr/bin/do\"]") && text.contains("credential_ref"),
            "hook command and credential survive a save: {text}"
        );
        assert!(text.contains("allow_automation = true"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn gib_figures_must_be_realistic() {
        assert!(gib_to_bytes(f64::NAN).is_err());
        assert!(gib_to_bytes(f64::INFINITY).is_err());
        assert!(gib_to_bytes(-0.5).is_err());
        assert!(gib_to_bytes(2.0).is_ok());
        let path = scratch("gib-refusal");
        std::fs::write(&path, "").unwrap();
        let mut bad = sample_config();
        bad.jobs_total_budget_gib = Some(f64::NAN);
        assert!(save_config_to(&path, bad).is_err());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn availability_needs_a_binary_or_a_unit_file() {
        assert!(!breadram_available(false, &[]));
        assert!(breadram_available(true, &[]));

        let unit = scratch("installed-unit");
        std::fs::write(&unit, "[Service]\n").unwrap();
        assert!(breadram_available(false, std::slice::from_ref(&unit)));
        assert!(!breadram_available(
            false,
            &[unit.with_file_name("missing.service")]
        ));
        let _ = std::fs::remove_file(unit);
    }

    #[test]
    fn bad_values_are_refused_before_touching_the_file() {
        let path = scratch("refusals");
        std::fs::write(&path, "mode = \"automatic\"\n").unwrap();
        let mut bad = sample_config();
        bad.mode = Some("ludicrous".to_string());
        assert!(save_config_to(&path, bad).is_err());
        let mut bad = sample_config();
        bad.jobs_total_budget_gib = Some(-1.0);
        assert!(save_config_to(&path, bad).is_err());
        let text = std::fs::read_to_string(&path).unwrap();
        assert_eq!(
            text, "mode = \"automatic\"\n",
            "a refused save changes nothing"
        );
        let _ = std::fs::remove_file(&path);
    }
}

// --- live daemon state: every call degrades to {present:false} ---

#[tauri::command]
pub fn breadram_status() -> serde_json::Value {
    let diagnostics = call(serde_json::json!({"type": "diagnostics"}));
    let rules = call(serde_json::json!({"type": "rules"}));
    let Ok(diag) = diagnostics else {
        return serde_json::json!({"present": false});
    };
    serde_json::json!({
        "present": true,
        "mode": diag.get("mode").cloned().unwrap_or_default(),
        "version": diag.get("version").cloned().unwrap_or_default(),
        "pressure": diag.get("pressure").cloned().unwrap_or_default(),
        "memory": diag.get("memory").cloned().unwrap_or_default(),
        "capabilities": diag.get("capabilities").cloned().unwrap_or_default(),
        "journal": diag.get("journal").cloned().unwrap_or_default(),
        "config_path": diag.get("config_path").cloned().unwrap_or_default(),
        "rules": rules.unwrap_or(serde_json::Value::Null),
    })
}

fn breadram_available(binary: bool, unit_files: &[PathBuf]) -> bool {
    binary || unit_files.iter().any(|path| path.is_file())
}

/// Whether breadram is on this computer at all. Live state can be absent
/// because the daemon is stopped, but a machine without the package has no
/// binary and no unit; the panel uses this to hide controls that cannot work.
#[tauri::command]
pub fn breadram_installed() -> bool {
    let units: Vec<PathBuf> = [
        "/usr/lib/systemd/system/breadram.service",
        "/etc/systemd/system/breadram.service",
        "/lib/systemd/system/breadram.service",
    ]
    .iter()
    .map(PathBuf::from)
    .collect();
    let binary = super::util::command_exists("breadram")
        || ["/usr/bin/breadram", "/usr/local/bin/breadram"]
            .iter()
            .any(|path| Path::new(path).is_file());
    breadram_available(binary, &units)
}

#[tauri::command]
pub fn breadram_actions() -> serde_json::Value {
    match call(serde_json::json!({"type": "actions"})) {
        Ok(actions) => serde_json::json!({"present": true, "actions": actions}),
        Err(_) => serde_json::json!({"present": false}),
    }
}

/// Change the mode through the daemon, which validates it. Persisting also
/// writes the configuration file through the daemon's own path.
#[tauri::command]
pub fn set_breadram_mode(mode: String, persist: bool) -> Result<String, String> {
    if !matches!(mode.as_str(), "monitor" | "automatic") {
        return Err(format!("unknown management mode: {mode}"));
    }
    let answer = call(serde_json::json!({"type": "set_mode", "mode": mode, "persist": persist}))
        .map_err(|_| "the daemon could not be reached".to_string())?;
    if answer.get("type").and_then(|v| v.as_str()) == Some("applied") {
        Ok(str_field(&answer, "summary").unwrap_or_else(|| "mode changed".to_string()))
    } else {
        Err(answer
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("the daemon refused the change")
            .to_string())
    }
}

/// Change protection through the daemon, which validates it.
#[tauri::command]
pub fn set_breadram_protection(
    add: Vec<String>,
    remove: Vec<String>,
    persist: bool,
) -> Result<String, String> {
    let answer = call(serde_json::json!({
        "type": "set_protection", "add": add, "remove": remove, "persist": persist,
    }))
    .map_err(|_| "the daemon could not be reached".to_string())?;
    if answer.get("type").and_then(|v| v.as_str()) == Some("applied") {
        Ok(str_field(&answer, "summary").unwrap_or_else(|| "protection changed".to_string()))
    } else {
        Err(answer
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("the daemon refused the change")
            .to_string())
    }
}
