//! App discovery across pacman, the AUR and Flathub.

use std::collections::HashSet;
use std::process::Command;

#[derive(Clone, Debug)]
pub struct App {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub source: String,
    pub installed: bool,
    pub votes: Option<u64>,
}

#[derive(Clone, Debug)]
pub struct Optional {
    pub id: String,
    pub title: String,
    pub detail: String,
    pub installed: bool,
}

fn text(program: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(program).args(args).output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).to_string())
}

fn installed_pacman() -> HashSet<String> {
    text("pacman", &["-Qq"])
        .map(|text| text.lines().map(str::trim).map(str::to_string).collect())
        .unwrap_or_default()
}

fn installed_flatpak() -> HashSet<String> {
    text("flatpak", &["list", "--app", "--columns=application"])
        .map(|text| text.lines().map(str::trim).map(str::to_string).collect())
        .unwrap_or_default()
}

fn parse_pacman(body: &str, source: &str, installed: &HashSet<String>) -> Vec<App> {
    let mut apps = Vec::new();
    let mut lines = body.lines().peekable();
    while let Some(header) = lines.next() {
        let line = header.trim();
        if line.is_empty() {
            continue;
        }
        let Some((repo_and_name, rest)) = line.split_once(' ') else {
            continue;
        };
        let name = repo_and_name
            .split_once('/')
            .map(|(_, name)| name)
            .unwrap_or(repo_and_name);
        let version = rest.split_whitespace().next().unwrap_or("").to_string();
        let votes = rest
            .split('(')
            .nth(1)
            .and_then(|tail| tail.split_whitespace().next())
            .and_then(|votes| votes.trim_start_matches('+').parse().ok());
        let description = lines.next().unwrap_or("").trim().to_string();
        apps.push(App {
            id: name.to_string(),
            name: name.to_string(),
            version,
            description,
            source: source.to_string(),
            installed: installed.contains(name),
            votes,
        });
    }
    apps
}

pub fn search(query: &str) -> (Vec<App>, Vec<String>) {
    let query = query.trim();
    if query.is_empty() || query.len() > 100 || query.starts_with('-') {
        return (Vec::new(), vec!["Enter an app name".into()]);
    }
    let installed = installed_pacman();
    let flat_installed = installed_flatpak();
    let mut apps = Vec::new();
    let mut errors = Vec::new();
    match text("pacman", &["-Ss", query]) {
        Some(body) => apps.extend(parse_pacman(&body, "repository", &installed)),
        None => errors.push("The package database did not answer.".into()),
    }
    if let Some(body) = text("yay", &["-Ss", "--aur", query]) {
        apps.extend(parse_pacman(&body, "aur", &installed));
    }
    if let Some(body) = text(
        "flatpak",
        &[
            "search",
            "--columns=application,name,description,version,remotes",
            query,
        ],
    ) {
        for line in body.lines() {
            let mut parts = line.split('\t');
            let id = parts.next().unwrap_or("").trim();
            if id.is_empty() || id == "application" {
                continue;
            }
            let name = parts.next().unwrap_or(id).trim().to_string();
            let description = parts.next().unwrap_or("").trim().to_string();
            let version = parts.next().unwrap_or("").trim().to_string();
            apps.push(App {
                id: id.to_string(),
                name,
                version,
                description,
                source: "flathub".into(),
                installed: flat_installed.contains(id),
                votes: None,
            });
        }
    } else {
        errors.push("Flathub search is not available.".into());
    }
    let lower = query.to_lowercase();
    apps.sort_by_key(|app| {
        let name = app.name.to_lowercase();
        (
            if name == lower {
                0
            } else if name.starts_with(&lower) {
                1
            } else if name.contains(&lower) {
                2
            } else {
                3
            },
            match app.source.as_str() {
                "repository" => 0,
                "flathub" => 1,
                _ => 2,
            },
            std::cmp::Reverse(app.votes.unwrap_or(0)),
            name,
        )
    });
    apps.truncate(100);
    (apps, errors)
}

pub fn install(id: &str, source: &str) -> Result<(), String> {
    if !id
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-' | '@'))
        || id.is_empty()
        || id.starts_with('-')
    {
        return Err("Invalid package name".into());
    }
    match source {
        "flathub" => Command::new("flatpak")
            .args(["install", "-y", "flathub", id])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map(|_| ())
            .map_err(|error| error.to_string()),
        "aur" | "repository" => Command::new("kitty")
            .args(["-e", "yay", "-S", id])
            .spawn()
            .map(|_| ())
            .map_err(|error| error.to_string()),
        _ => Err("Unknown app source".into()),
    }
}

pub fn optional() -> Vec<Optional> {
    let installed = installed_pacman();
    let flatpak = which("flatpak");
    let flathub = flatpak
        && text("flatpak", &["remotes"])
            .map(|text| text.to_ascii_lowercase().contains("flathub"))
            .unwrap_or(false);
    vec![
        Optional {
            id: "breadcast".into(),
            title: "Breadcast".into(),
            detail: "A bread app that isn't installed by default.".into(),
            installed: installed.contains("breadcast") || which("breadcast"),
        },
        Optional {
            id: "flatpak".into(),
            title: "Flathub apps".into(),
            detail: "Lets you install apps from Flathub, the biggest Linux app store.".into(),
            installed: flatpak && flathub,
        },
        Optional {
            id: "office".into(),
            title: "Office and PDF".into(),
            detail: "LibreOffice for documents and spreadsheets, and a PDF viewer.".into(),
            installed: installed.contains("libreoffice-fresh")
                && (installed.contains("papers") || installed.contains("evince")),
        },
        Optional {
            id: "steam".into(),
            title: "Steam".into(),
            detail: "Games from the Steam store.".into(),
            installed: installed.contains("steam") || which("steam"),
        },
    ]
}

pub fn enable_flathub() -> Result<(), String> {
    if !which("flatpak") {
        return Err("flatpak is not installed".into());
    }
    let out = Command::new("flatpak")
        .args([
            "remote-add",
            "--if-not-exists",
            "--user",
            "flathub",
            "https://dl.flathub.org/repo/flathub.flatpakrepo",
        ])
        .output()
        .map_err(|error| error.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

pub fn install_optional(id: &str) -> Result<(), String> {
    match id {
        "flatpak" => {
            if which("flatpak") {
                return enable_flathub();
            }
            install("flatpak", "repository")
        }
        "breadcast" => install("breadcast", "repository"),
        "office" => install("libreoffice-fresh", "repository"),
        "steam" => install("steam", "repository"),
        _ => Err("Unknown optional app".into()),
    }
}

fn which(program: &str) -> bool {
    Command::new("sh")
        .args(["-c", "command -v \"$1\" >/dev/null 2>&1", "sh", program])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

pub fn bakery_packages() -> Vec<(String, String)> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/".into());
    let path = std::path::Path::new(&home).join(".local/state/bakery/installed.json");
    let Some(text) = std::fs::read_to_string(path).ok() else {
        return Vec::new();
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
        return Vec::new();
    };
    let Some(packages) = value.get("packages").and_then(|v| v.as_object()) else {
        return Vec::new();
    };
    let mut list: Vec<(String, String)> = packages
        .iter()
        .map(|(name, value)| {
            (
                name.clone(),
                value
                    .get("version")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string(),
            )
        })
        .collect();
    list.sort_by(|a, b| a.0.cmp(&b.0));
    list
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pacman_headers_become_apps() {
        let body = "extra/firefox 130.0-1 [installed]\n    Fast browser\n";
        let installed = HashSet::from(["firefox".to_string()]);
        let apps = parse_pacman(body, "repository", &installed);
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].name, "firefox");
        assert!(apps[0].installed);
        assert_eq!(apps[0].description, "Fast browser");
    }

    #[test]
    fn bad_names_are_refused() {
        assert!(install("--help", "repository").is_err());
        assert!(install("a;rm", "aur").is_err());
    }
}
