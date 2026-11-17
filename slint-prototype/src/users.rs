//! User account management: add/remove users and change passwords. Everything
//! here needs root, so every action goes through `pkexec`.

use crate::util;

const USERNAME_MAX: usize = 32;

/// `[a-z_][a-z0-9_-]*`, length-capped, no leading `-`. Also rejects `:`,
/// newlines and other chpasswd field separators.
pub fn valid_username(name: &str) -> bool {
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
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(*byte, b'_' | b'-'))
}

fn password_problem(password: &str) -> Option<&'static str> {
    if password.is_empty() {
        Some("Enter a password.")
    } else if password.len() > 512 {
        Some("That password is too long. Use 512 characters or fewer.")
    } else if password.contains(['\n', '\r', '\0']) {
        Some("Passwords can't contain line breaks.")
    } else {
        None
    }
}

fn chpasswd_input(username: &str, password: &str) -> Result<String, String> {
    if !valid_username(username) {
        return Err("Usernames start with a lowercase letter or _, then use lowercase letters, numbers, _ or -.".into());
    }
    if let Some(problem) = password_problem(password) {
        return Err(problem.into());
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

pub fn change_password(username: &str, password: &str) -> Result<(), String> {
    let input = chpasswd_input(username, password)?;
    if util::run_with_stdin(&["pkexec", "chpasswd"], &input) {
        Ok(())
    } else {
        Err("The password wasn't changed. The password prompt may have been cancelled, or the system refused it.".into())
    }
}

pub fn remove(username: &str) -> Result<(), String> {
    let current = std::env::var("USER").unwrap_or_default();
    may_delete_user(username, &current)?;
    let out = std::process::Command::new("pkexec")
        .args(["userdel", "-r", "--", username])
        .output()
        .map_err(|error| error.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

pub fn add(username: &str, full_name: &str, password: &str) -> Result<(), String> {
    let username = username.trim();
    let input = chpasswd_input(username, password)?;
    let gecos: String = full_name
        .chars()
        .filter(|ch| !matches!(ch, '\n' | '\r' | '\0' | ',' | ':'))
        .collect();
    let mut args = vec![
        "pkexec".to_string(),
        "useradd".to_string(),
        "-m".to_string(),
        "-s".to_string(),
        "/bin/bash".to_string(),
    ];
    if !gecos.trim().is_empty() {
        args.push("-c".to_string());
        args.push(gecos.trim().to_string());
    }
    args.push("--".to_string());
    args.push(username.to_string());
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let out = std::process::Command::new(refs[0])
        .args(&refs[1..])
        .output()
        .map_err(|error| error.to_string())?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    if util::run_with_stdin(&["pkexec", "chpasswd"], &input) {
        Ok(())
    } else {
        Err("User created, but setting the password failed.".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chpasswd_rejects_injection() {
        assert!(chpasswd_input("alice", "pw\nroot:evil").is_err());
        assert!(chpasswd_input("alice\nroot", "pw").is_err());
        assert!(chpasswd_input("al:ice", "pw").is_err());
        assert!(chpasswd_input("alice", "").is_err());
        assert_eq!(chpasswd_input("alice", "secret").unwrap(), "alice:secret\n");
    }

    #[test]
    fn username_grammar() {
        assert!(valid_username("alice"));
        assert!(valid_username("_svc"));
        assert!(!valid_username("-alice"));
        assert!(!valid_username("Alice"));
    }

    #[test]
    fn root_and_self_are_not_removable() {
        assert!(may_delete_user("root", "alice").is_err());
        assert!(may_delete_user("alice", "alice").is_err());
        assert!(may_delete_user("bob", "alice").is_ok());
    }
}
