//! Pending package, bakery and firmware updates.

use std::process::Command;

#[derive(Clone, Debug)]
pub struct Pending {
    pub name: String,
    pub current: String,
    pub latest: String,
}

#[derive(Clone, Debug)]
pub struct Firmware {
    pub name: String,
    pub version: String,
}

fn text(program: &str, args: &[&str]) -> Option<(bool, String)> {
    let out = Command::new(program).args(args).output().ok()?;
    Some((
        out.status.success(),
        format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
    ))
}

pub fn pacman_updates() -> Result<Vec<Pending>, String> {
    let (ok, body) =
        text("pacman", &["-Qu"]).ok_or_else(|| "couldn't run pacman".to_string())?;
    if !ok && !body.trim().is_empty() {
        return Err(body.trim().to_string());
    }
    Ok(body
        .lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let name = parts.next()?.to_string();
            let current = parts.next()?.to_string();
            if parts.next()? != "->" {
                return None;
            }
            let latest = parts.next()?.to_string();
            Some(Pending {
                name,
                current,
                latest,
            })
        })
        .collect())
}

pub fn bakery_updates() -> Result<Vec<Pending>, String> {
    let (ok, body) = text("bakery", &["--dry-run", "update", "--all"])
        .ok_or_else(|| "couldn't run bakery".to_string())?;
    if !ok {
        return Err(body.trim().to_string());
    }
    let plain = strip_ansi(&body);
    Ok(plain
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            let marker = line.find(" -> ")?;
            let before = line[..marker].trim();
            let latest = line[marker + 4..]
                .split_whitespace()
                .next()
                .unwrap_or("")
                .to_string();
            let name = before
                .split_whitespace()
                .last()
                .unwrap_or("")
                .trim_start_matches('+')
                .to_string();
            if name.is_empty() {
                return None;
            }
            Some(Pending {
                name,
                current: before.to_string(),
                latest,
            })
        })
        .collect())
}

fn strip_ansi(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' {
            if chars.peek() == Some(&'[') {
                chars.next();
                for next in chars.by_ref() {
                    if next.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            out.push(ch);
        }
    }
    out
}

pub fn firmware() -> Vec<Firmware> {
    let Some((_, body)) = text("fwupdmgr", &["get-devices", "--json"]) else {
        return Vec::new();
    };
    let Ok(root) = serde_json::from_str::<serde_json::Value>(&body) else {
        return Vec::new();
    };
    root.get("Devices")
        .and_then(|value| value.as_array())
        .map(|devices| {
            devices
                .iter()
                .filter(|device| {
                    device
                        .get("Flags")
                        .and_then(|flags| flags.as_array())
                        .map(|flags| flags.iter().any(|flag| flag.as_str() == Some("updatable")))
                        .unwrap_or(false)
                })
                .filter_map(|device| {
                    Some(Firmware {
                        name: device.get("Name")?.as_str()?.to_string(),
                        version: device
                            .get("Version")
                            .and_then(|value| value.as_str())
                            .unwrap_or("unknown")
                            .to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

pub fn update(kind: &str, name: &str) -> Result<(), String> {
    let args: Vec<String> = match kind {
        "pacman" => vec!["-Syu".into()],
        "bakery" if name.is_empty() => vec!["update".into(), "--all".into()],
        "bakery" => vec!["update".into(), name.into()],
        "firmware" => vec!["update".into()],
        _ => return Err("Unknown update kind".into()),
    };
    let program = match kind {
        "pacman" | "bakery" => "yay",
        _ => "fwupdmgr",
    };
    Command::new("kitty")
        .arg("-e")
        .arg(program)
        .args(&args)
        .spawn()
        .map(|_| ())
        .map_err(|error| error.to_string())
}

pub fn pacman_install(packages: &[String]) -> Result<(), String> {
    let allowed = crate::util::allowed_pacman_packages(packages)?;
    Command::new("kitty")
        .args(["-e", "pacman", "-S"])
        .args(&allowed)
        .spawn()
        .map(|_| ())
        .map_err(|error| error.to_string())
}

pub fn bakery_install(name: &str) -> Result<(), String> {
    crate::util::allowed_bakery_install(name)?;
    Command::new("kitty")
        .args(["-e", "bakery", "install", name])
        .spawn()
        .map(|_| ())
        .map_err(|error| error.to_string())
}

pub fn firmware_refresh() -> Result<(), String> {
    Command::new("kitty")
        .args(["-e", "fwupdmgr", "refresh"])
        .spawn()
        .map(|_| ())
        .map_err(|error| error.to_string())
}

pub fn track() -> Result<String, String> {
    let (ok, body) = text("bakery", &["track", "show"])
        .ok_or_else(|| "couldn't run bakery".to_string())?;
    if !ok {
        return Err(body.trim().to_string());
    }
    Ok(parse_track(&body))
}

fn parse_track(text: &str) -> String {
    for line in text.lines() {
        let line = line.trim().to_ascii_lowercase();
        if let Some(rest) = line.strip_prefix("current track:") {
            let track = rest.trim();
            if ["stable", "beta", "dev"].contains(&track) {
                return track.to_string();
            }
        }
        if ["stable", "beta", "dev"].contains(&line.as_str()) {
            return line;
        }
    }
    "stable".into()
}

pub fn set_track(track: &str) -> Result<(), String> {
    if !["stable", "beta", "dev"].contains(&track) {
        return Err("unknown track".into());
    }
    let (ok, body) = text("bakery", &["track", "set", track])
        .ok_or_else(|| "couldn't run bakery".to_string())?;
    if ok {
        Ok(())
    } else {
        Err(body.trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pacman_lines_are_parsed() {
        let body = "linux 6.10.1-1 -> 6.10.2-1\nvim 9.1-1 -> 9.1-2\n";
        let updates: Vec<Pending> = body
            .lines()
            .filter_map(|line| {
                let mut parts = line.split_whitespace();
                Some(Pending {
                    name: parts.next()?.into(),
                    current: parts.next()?.into(),
                    latest: parts.nth(1)?.into(),
                })
            })
            .collect();
        assert_eq!(updates[0].name, "linux");
        assert_eq!(updates[1].latest, "9.1-2");
    }

    #[test]
    fn track_line_wins() {
        assert_eq!(parse_track("current track: dev\n"), "dev");
        assert_eq!(parse_track("beta"), "beta");
    }
}
