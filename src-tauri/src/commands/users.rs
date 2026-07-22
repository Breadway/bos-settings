//! User account management — add/remove users, change passwords. Everything
//! here needs root (useradd/userdel/chpasswd), so every action goes through
//! `pkexec`.

use serde::Serialize;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

#[derive(Serialize, Clone)]
pub struct Account {
    username: String,
    full_name: String,
}

fn list_accounts() -> Vec<Account> {
    let Ok(text) = std::fs::read_to_string("/etc/passwd") else {
        return Vec::new();
    };
    text.lines()
        .filter_map(|line| {
            let f: Vec<&str> = line.split(':').collect();
            if f.len() < 7 {
                return None;
            }
            let uid: u32 = f[2].parse().ok()?;
            let shell = f[6];
            // Real human accounts: normal UID range, a real login shell
            // (excludes system/service accounts like greeter, avahi, etc).
            if !(1000..60000).contains(&uid) || shell.ends_with("nologin") || shell.ends_with("/false") {
                return None;
            }
            Some(Account { username: f[0].to_string(), full_name: f[4].split(',').next().unwrap_or("").to_string() })
        })
        .collect()
}

#[derive(Serialize)]
pub struct UsersInfo {
    accounts: Vec<Account>,
    current_user: String,
}

#[tauri::command]
pub fn get_users_info() -> UsersInfo {
    UsersInfo { accounts: list_accounts(), current_user: std::env::var("USER").unwrap_or_default() }
}

/// Runs a root command that needs a line of input on stdin (chpasswd's own
/// "user:password" format). `pkexec` inherits the spawning process's stdin
/// only when explicitly piped, so this pipes it through.
async fn run_with_stdin(args: &[&str], input: String) -> bool {
    let Ok(mut child) = Command::new(args[0]).args(&args[1..]).stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn()
    else {
        return false;
    };
    if let Some(mut stdin) = child.stdin.take() {
        if stdin.write_all(input.as_bytes()).await.is_err() {
            return false;
        }
    }
    child.wait().await.map(|s| s.success()).unwrap_or(false)
}

#[tauri::command]
pub async fn change_password(username: String, password: String) -> Result<(), String> {
    let input = format!("{username}:{password}\n");
    if run_with_stdin(&["pkexec", "chpasswd"], input).await {
        Ok(())
    } else {
        Err("Failed to change password".into())
    }
}

#[tauri::command]
pub async fn remove_user(username: String) -> Result<(), String> {
    let output = Command::new("pkexec").args(["userdel", "-r", &username]).output().await.map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

#[tauri::command]
pub async fn add_user(username: String, full_name: String, password: String) -> Result<(), String> {
    let username = username.trim().to_string();
    let mut useradd_args = vec!["pkexec".to_string(), "useradd".to_string(), "-m".to_string(), "-s".to_string(), "/bin/bash".to_string()];
    if !full_name.trim().is_empty() {
        useradd_args.push("-c".to_string());
        useradd_args.push(full_name.trim().to_string());
    }
    useradd_args.push(username.clone());
    let args_ref: Vec<&str> = useradd_args.iter().map(String::as_str).collect();
    let output = Command::new(args_ref[0]).args(&args_ref[1..]).output().await.map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    let input = format!("{username}:{password}\n");
    if run_with_stdin(&["pkexec", "chpasswd"], input).await {
        Ok(())
    } else {
        Err("User created, but setting the password failed.".into())
    }
}
