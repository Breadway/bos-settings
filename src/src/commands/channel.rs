//! Bakery track (stable / beta / dev). Preference only — `bakery update
//! --all` afterwards actually installs the new track's builds.

use serde::Serialize;
use tokio::process::Command;

use super::util::{fail_output, strip_ansi};

const TRACKS: &[&str] = &["stable", "beta", "dev"];

#[derive(Serialize)]
pub struct BakeryTrack {
    current: String,
    tracks: Vec<String>,
}

fn parse_track_show(text: &str) -> String {
    let text = strip_ansi(text);
    for line in text.lines() {
        let line = line.trim();
        let lower = line.to_ascii_lowercase();
        if let Some(rest) = lower.strip_prefix("current track:") {
            let raw = line[line.len() - rest.len()..].trim();
            return raw.to_ascii_lowercase();
        }
        if TRACKS.contains(&line) {
            return line.to_string();
        }
    }
    let lower = text.to_ascii_lowercase();
    for track in TRACKS {
        if lower.contains(track) {
            return (*track).to_string();
        }
    }
    "stable".into()
}

#[tauri::command]
pub async fn get_bakery_track() -> Result<BakeryTrack, String> {
    let output = Command::new("bakery")
        .args(["track", "show"])
        .output()
        .await
        .map_err(|e| format!("couldn't run bakery: {e}"))?;
    if !output.status.success() {
        return Err(fail_output(&output, "bakery track show"));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    Ok(BakeryTrack {
        current: parse_track_show(&text),
        tracks: TRACKS.iter().map(|s| (*s).to_string()).collect(),
    })
}

#[tauri::command]
pub async fn set_bakery_track(track: String) -> Result<BakeryTrack, String> {
    let track = track.trim().to_ascii_lowercase();
    if !TRACKS.contains(&track.as_str()) {
        return Err(format!(
            "unknown track '{track}' — expected stable, beta, or dev"
        ));
    }
    let output = Command::new("bakery")
        .args(["track", "set", &track])
        .output()
        .await
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(fail_output(&output, "bakery track set"));
    }
    get_bakery_track().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_current_track_line() {
        assert_eq!(parse_track_show("current track: dev\n"), "dev");
        assert_eq!(parse_track_show("current track: stable"), "stable");
        assert_eq!(parse_track_show("beta"), "beta");
    }

    #[test]
    fn rejects_unknown_in_set_guard() {
        assert!(!TRACKS.contains(&"nightly"));
        assert!(TRACKS.contains(&"stable"));
    }
}
