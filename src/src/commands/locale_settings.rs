//! Locale choices are restricted to installed locales; localed owns system persistence.
use super::util;
use serde::{Deserialize, Serialize};
use tokio::process::Command;
#[derive(Serialize)]
pub struct LocaleInfo {
    pub locales: Vec<String>,
    pub language: String,
    pub region: String,
}
#[derive(Deserialize)]
pub struct LocaleChoice {
    pub language: String,
    pub region: String,
}
async fn locales() -> Result<Vec<String>, String> {
    let out = Command::new("locale")
        .arg("-a")
        .output()
        .await
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(util::fail_output(&out, "Reading installed locales"));
    }
    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(String::from)
        .collect())
}
fn property(text: &str, key: &str) -> Option<String> {
    text.lines().find_map(|l| {
        l.strip_prefix(&format!("{key}="))
            .map(|s| s.trim_matches('"').to_string())
    })
}
fn canonical_locale(choice: &str, installed: &[String]) -> Option<String> {
    let normalized = |s: &str| s.to_ascii_lowercase().replace("utf-8", "utf8");
    installed
        .iter()
        .find(|s| normalized(s) == normalized(choice))
        .cloned()
}
#[tauri::command]
pub async fn get_locale_info() -> Result<LocaleInfo, String> {
    let text = std::fs::read_to_string("/etc/locale.conf").unwrap_or_default();
    let language = property(&text, "LANG").unwrap_or_else(|| "C.UTF-8".into());
    let region = property(&text, "LC_TIME").unwrap_or_else(|| language.clone());
    let installed = locales().await?;
    let language = canonical_locale(&language, &installed).unwrap_or(language);
    let region = canonical_locale(&region, &installed).unwrap_or(region);
    Ok(LocaleInfo {
        locales: installed,
        language,
        region,
    })
}
#[tauri::command]
pub async fn save_locale_choice(choice: LocaleChoice) -> Result<(), String> {
    let installed = locales().await?;
    let language = canonical_locale(&choice.language, &installed)
        .ok_or("Choose an installed language locale")?;
    let region =
        canonical_locale(&choice.region, &installed).ok_or("Choose an installed region locale")?;
    let args = [
        format!("LANG={language}"),
        format!("LC_TIME={}", region),
        format!("LC_NUMERIC={}", region),
        format!("LC_MONETARY={}", region),
        format!("LC_MEASUREMENT={}", region),
    ];
    let out = Command::new("pkexec")
        .args(["localectl", "set-locale"])
        .args(args)
        .output()
        .await
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(util::fail_output(&out, "Saving language and region"))
    }
}

#[tauri::command]
pub fn get_clock_format() -> bool {
    std::fs::read_to_string(super::config::config_dir().join("breadbar/clock.json"))
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v.get("clock_24h").and_then(|x| x.as_bool()))
        .unwrap_or(true)
}
#[tauri::command]
pub async fn set_clock_format(clock_24h: bool) -> Result<(), String> {
    // Require the companion API before offering settings its running binary cannot consume.
    super::notification_settings::get_notification_preferences().await?;
    super::config::atomic_write(
        &super::config::config_dir().join("breadbar/clock.json"),
        &serde_json::json!({"clock_24h":clock_24h}).to_string(),
    )
    .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn locale_aliases_still_require_installed_locale() {
        let installed = vec!["en_US.utf8".into()];
        assert_eq!(
            canonical_locale("en_US.UTF-8", &installed),
            Some("en_US.utf8".into())
        );
        assert!(canonical_locale("en_US.UTF-8; reboot", &installed).is_none());
    }
}
