//! User account management — add/remove users, change passwords. Everything
//! here needs root (useradd/userdel/chpasswd), so every action goes through
//! `pkexec`.

use serde::Serialize;
use tokio::process::Command;

use super::util;

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
            if !(1000..60000).contains(&uid)
                || shell.ends_with("nologin")
                || shell.ends_with("/false")
            {
                return None;
            }
            Some(Account {
                username: f[0].to_string(),
                full_name: f[4].split(',').next().unwrap_or("").to_string(),
            })
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
    UsersInfo {
        accounts: list_accounts(),
        current_user: std::env::var("USER").unwrap_or_default(),
    }
}

/// shadow-utils `USER_NAME_MAX` is 32; keep chpasswd/useradd operands inside it.
const USERNAME_MAX: usize = 32;

/// `[a-z_][a-z0-9_-]*`, length-capped, no leading `-`. Also rejects `:`,
/// newlines, and other chpasswd field/line separators.
fn valid_username(name: &str) -> bool {
    let bytes = name.as_bytes();
    if bytes.is_empty() || bytes.len() > USERNAME_MAX {
        return false;
    }
    let first = bytes[0];
    if first != b'_' && !first.is_ascii_lowercase() {
        return false;
    }
    bytes[1..]
        .iter()
        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(*b, b'_' | b'-'))
}

/// chpasswd reads `user:password` lines — a `:`, `\n`, or `\r` in either
/// field injects extra passwd entries or shifts columns.
fn valid_chpasswd_password(password: &str) -> bool {
    !password.is_empty()
        && password.len() <= 512
        && !password.contains('\n')
        && !password.contains('\r')
        && !password.contains(':')
        && !password.contains('\0')
}

fn chpasswd_input(username: &str, password: &str) -> Result<String, String> {
    if !valid_username(username) {
        return Err("invalid username".into());
    }
    if !valid_chpasswd_password(password) {
        return Err("invalid password".into());
    }
    Ok(format!("{username}:{password}\n"))
}

fn may_delete_user(username: &str, current: &str) -> Result<(), String> {
    if !valid_username(username) {
        return Err("invalid username".into());
    }
    if username == "root" {
        return Err("refusing to remove root".into());
    }
    if !current.is_empty() && username == current {
        return Err("refusing to remove the current user".into());
    }
    Ok(())
}

#[tauri::command]
pub async fn change_password(username: String, password: String) -> Result<(), String> {
    // chpasswd_input validates the username *and* the password (rejects the
    // newline / `:` that would let one entry smuggle another).
    let input = chpasswd_input(&username, &password)?;
    if util::run_with_stdin(&["pkexec", "chpasswd"], &input).await {
        Ok(())
    } else {
        Err("Failed to change password".into())
    }
}

#[tauri::command]
pub async fn remove_user(username: String) -> Result<(), String> {
    let current = std::env::var("USER").unwrap_or_default();
    // Validates the username and refuses `root` / the current user.
    may_delete_user(&username, &current)?;
    // `--` so a username can never be read as a userdel option.
    let output = Command::new("pkexec")
        .args(["userdel", "-r", "--", &username])
        .output()
        .await
        .map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

#[tauri::command]
pub async fn add_user(username: String, full_name: String, password: String) -> Result<(), String> {
    let username = username.trim();
    // Validates the username and the password (newline / `:` injection).
    let input = chpasswd_input(username, &password)?;
    // GECOS field is otherwise free text; strip control chars and the field
    // separators so `-c` can't smuggle extra passwd fields or arguments.
    let gecos: String = full_name
        .chars()
        .filter(|c| !matches!(c, '\n' | '\r' | '\0' | ',' | ':'))
        .collect();
    let mut useradd_args = vec![
        "pkexec".to_string(),
        "useradd".to_string(),
        "-m".to_string(),
        "-s".to_string(),
        "/bin/bash".to_string(),
    ];
    if !gecos.trim().is_empty() {
        useradd_args.push("-c".to_string());
        useradd_args.push(gecos.trim().to_string());
    }
    // `--` so the username can never be read as a useradd option.
    useradd_args.push("--".to_string());
    useradd_args.push(username.to_string());
    let args_ref: Vec<&str> = useradd_args.iter().map(String::as_str).collect();
    let output = Command::new(args_ref[0])
        .args(&args_ref[1..])
        .output()
        .await
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    if util::run_with_stdin(&["pkexec", "chpasswd"], &input).await {
        Ok(())
    } else {
        Err("User created, but setting the password failed.".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chpasswd_rejects_newline_injection() {
        assert!(chpasswd_input("alice", "pw\nroot:evil").is_err());
        assert!(chpasswd_input("alice\nroot", "pw").is_err());
        assert!(chpasswd_input("alice\rroot", "pw").is_err());
        assert!(chpasswd_input("alice", "pw\rroot:x").is_err());
        assert!(chpasswd_input("al:ice", "pw").is_err());
        assert!(chpasswd_input("alice", "p:w").is_err());
        assert_eq!(chpasswd_input("alice", "secret").unwrap(), "alice:secret\n");
    }

    #[test]
    fn username_grammar() {
        assert!(valid_username("alice"));
        assert!(valid_username("_svc"));
        assert!(valid_username("a1-b_c"));
        assert!(!valid_username(""));
        assert!(!valid_username("-alice"));
        assert!(!valid_username("Alice"));
        assert!(!valid_username("root user"));
        assert!(!valid_username(&"a".repeat(USERNAME_MAX + 1)));
    }

    #[test]
    fn remove_user_refuses_root_and_self() {
        assert!(may_delete_user("root", "alice").is_err());
        assert!(may_delete_user("alice", "alice").is_err());
        assert!(may_delete_user("root\n", "alice").is_err());
        assert!(may_delete_user("bob", "alice").is_ok());
    }
}
