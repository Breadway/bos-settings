//! Search fixed providers, retaining interactive AUR review.
use super::{streaming, util};
use serde::{Deserialize, Serialize};
use tokio::process::Command;
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AppSource {
    Repository,
    Aur,
    Flathub,
}
#[derive(Clone, Serialize)]
pub struct AppResult {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub source: AppSource,
    pub installed: bool,
    pub votes: Option<u64>,
}
#[derive(Serialize)]
pub struct AppSearch {
    apps: Vec<AppResult>,
    errors: Vec<String>,
}
async fn output(program: &str, args: &[&str]) -> Result<String, String> {
    let out = Command::new(program)
        .args(args)
        .env("LC_ALL", "C")
        .output()
        .await
        .map_err(|e| e.to_string())?;
    if !out.status.success()
        && !(args.contains(&"-Ss") && out.status.code() == Some(1) && out.stderr.is_empty())
    {
        return Err(util::fail_output(&out, "Searching apps"));
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}
fn parse_packages(text: &str, source: AppSource, installed: &str) -> Vec<AppResult> {
    let mut lines = text.lines();
    let mut rows = vec![];
    while let Some(line) = lines.next() {
        if line.starts_with(char::is_whitespace) {
            continue;
        }
        let Some((_, rest)) = line.split_once('/') else {
            continue;
        };
        let mut fields = rest.split_whitespace();
        let Some(id) = fields.next() else { continue };
        if !util::valid_pkg_name(id) {
            continue;
        }
        let version = fields.next().unwrap_or("").into();
        let votes = fields.find_map(|f| f.strip_prefix("(+").and_then(|s| s.parse::<u64>().ok()));
        rows.push(AppResult {
            id: id.into(),
            name: id.into(),
            version,
            description: lines.next().unwrap_or("").trim().into(),
            source: source.clone(),
            installed: installed.lines().any(|p| p == id),
            votes,
        });
    }
    rows
}
fn parse_flatpak(text: &str, installed: &str) -> Vec<AppResult> {
    text.lines()
        .filter_map(|line| {
            let f: Vec<_> = line.split('\t').collect();
            if f.len() < 5
                || !valid_flatpak(f[0])
                || !f[4].split([',', ' ']).any(|remote| remote == "flathub")
            {
                return None;
            }
            Some(AppResult {
                id: f[0].into(),
                name: f[1].into(),
                description: f[2].into(),
                version: f[3].into(),
                source: AppSource::Flathub,
                installed: installed.lines().any(|p| p == f[0]),
                votes: None,
            })
        })
        .collect()
}
fn valid_flatpak(id: &str) -> bool {
    id.len() < 200
        && id.split('.').count() >= 3
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}
fn rank(app: &AppResult, query: &str) -> (u8, u8, std::cmp::Reverse<u64>, String) {
    let n = app.name.to_lowercase();
    let q = query.to_lowercase();
    (
        if n == q {
            0
        } else if n.starts_with(&q) {
            1
        } else if n.contains(&q) {
            2
        } else {
            3
        },
        if app.source == AppSource::Repository {
            0
        } else if app.source == AppSource::Flathub {
            1
        } else {
            2
        },
        std::cmp::Reverse(app.votes.unwrap_or(0)),
        n,
    )
}
#[tauri::command]
pub async fn search_apps(query: String) -> AppSearch {
    let q = query.trim();
    if q.is_empty() || q.len() > 100 || q.starts_with('-') || q.contains(['\n', '\0']) {
        return AppSearch {
            apps: vec![],
            errors: vec!["Enter an app name".into()],
        };
    }
    let (installed, flat_installed) = tokio::join!(
        output("pacman", &["-Qq"]),
        output("flatpak", &["list", "--app", "--columns=application"])
    );
    let (repo, aur, flat) = tokio::join!(
        async { output("pacman", &["-Ss", q]).await },
        async { output("yay", &["-Ss", "--aur", q]).await },
        async {
            output(
                "flatpak",
                &[
                    "search",
                    "--columns=application,name,description,version,remotes",
                    q,
                ],
            )
            .await
        }
    );
    let mut apps = vec![];
    let mut errors = vec![];
    for (source, result) in [(AppSource::Repository, repo), (AppSource::Aur, aur)] {
        match result {
            Ok(text) => apps.extend(parse_packages(
                &text,
                source,
                installed.as_deref().unwrap_or(""),
            )),
            Err(e) => errors.push(e),
        }
    }
    match flat {
        Ok(text) => apps.extend(parse_flatpak(
            &text,
            flat_installed.as_deref().unwrap_or(""),
        )),
        Err(e) => errors.push(e),
    }
    apps.sort_by_key(|a| rank(a, q));
    apps.truncate(100);
    AppSearch { apps, errors }
}
#[tauri::command]
pub async fn install_store_app(
    app: tauri::AppHandle,
    session_id: String,
    source: AppSource,
    id: String,
) -> Result<bool, String> {
    match source {
        AppSource::Aur => {
            super::aur::install_aur_package(id)?;
            Ok(true)
        }
        AppSource::Repository => {
            if !util::valid_pkg_name(&id) {
                return Err("Invalid package".into());
            }
            output("pacman", &["-Si", &id]).await?;
            Ok(streaming::run_hardcoded(
                app,
                session_id,
                "pkexec",
                &["pacman", "-S", "--needed", "--noconfirm", &id],
            )
            .await)
        }
        AppSource::Flathub => {
            if !valid_flatpak(&id) {
                return Err("Invalid app ID".into());
            }
            output("flatpak", &["remote-info", "flathub", &id]).await?;
            Ok(streaming::run_hardcoded(
                app,
                session_id,
                "flatpak",
                &["install", "--noninteractive", "flathub", &id],
            )
            .await)
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn package_descriptions_and_votes() {
        let rows = parse_packages(
            "aur/localsend-bin 1.2 (+412 4.5)\n    Send files\n",
            AppSource::Aur,
            "localsend-bin\n",
        );
        assert_eq!(rows[0].votes, Some(412));
        assert!(rows[0].installed);
        assert_eq!(rows[0].description, "Send files");
    }
    #[test]
    fn flatpak_ids_and_ranking() {
        assert!(!valid_flatpak("--system"));
        assert!(!valid_flatpak("org.example.X;reboot"));
        let rows = parse_flatpak(
            "org.localsend.localsend_app\tLocalSend\tFile sharing\t1.18\tflathub\n",
            "",
        );
        assert_eq!(rank(&rows[0], "localsend").0, 0);
        assert_eq!(rows.len(), 1);
        assert!(parse_flatpak(
            "org.example.Other\tOther\tUntrusted remote\t1\tcustom\n",
            ""
        )
        .is_empty());
    }
}
