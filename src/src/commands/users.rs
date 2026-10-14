//! User account management: add/remove users, change passwords. Everything
//! here needs root (useradd/userdel/chpasswd), so every action goes through
//! `pkexec`.

use serde::Serialize;
use tokio::process::Command;

use super::util;

#[derive(Serialize, Clone)]
pub struct Account {
    username: String,
    full_name: String,
    admin: bool,
}

/// Members of the groups that grant sudo (`wheel` on Arch, `sudo` on Debian).
fn admin_members(group_text: &str) -> Vec<String> {
    group_text
        .lines()
        .filter_map(|line| {
            let f: Vec<&str> = line.split(':').collect();
            if f.len() < 4 || !matches!(f[0], "wheel" | "sudo") {
                return None;
            }
            Some(
                f[3].split(',')
                    .filter(|m| !m.is_empty())
                    .map(str::to_string)
                    .collect::<Vec<_>>(),
            )
        })
        .flatten()
        .collect()
}

fn list_accounts() -> Vec<Account> {
    let Ok(text) = std::fs::read_to_string("/etc/passwd") else {
        return Vec::new();
    };
    let admins = admin_members(&std::fs::read_to_string("/etc/group").unwrap_or_default());
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
                admin: admins.iter().any(|a| a == f[0]),
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

/// chpasswd reads one `user:password` line and splits it at the *first*
/// `:` only (shadow-utils `stpsep`), and usernames can't contain `:`, so a
/// colon in the password is safe (BSET-64). A line break or NUL would start
/// a second entry, so those stay out.
fn chpasswd_password_problem(password: &str) -> Option<&'static str> {
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
    if let Some(problem) = chpasswd_password_problem(password) {
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

#[tauri::command]
pub async fn change_password(username: String, password: String) -> Result<(), String> {
    // chpasswd_input validates the username *and* the password (rejects the
    // newline / `:` that would let one entry smuggle another).
    let input = chpasswd_input(&username, &password)?;
    if util::run_with_stdin(&["pkexec", "chpasswd"], &input).await {
        Ok(())
    } else {
        Err("The password wasn't changed. The password prompt may have been cancelled, or the system refused it.".into())
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
        assert!(chpasswd_input("alice", "pw\0x").is_err());
        assert!(chpasswd_input("alice", "").is_err());
        assert_eq!(chpasswd_input("alice", "secret").unwrap(), "alice:secret\n");
    }

    /// chpasswd splits at the first `:`, and the username has none, so the
    /// rest of the line is the password verbatim.
    #[test]
    fn colon_in_password_is_one_entry() {
        let line = chpasswd_input("alice", "a:b::c").unwrap();
        assert_eq!(line.lines().count(), 1);
        let (user, pw) = line.trim_end_matches('\n').split_once(':').unwrap();
        assert_eq!((user, pw), ("alice", "a:b::c"));
    }

    #[test]
    fn admin_members_reads_wheel_and_sudo() {
        let text =
            "root:x:0:\nwheel:x:998:alice,bob\nusers:x:100:carol\nsudo:x:27:dave\nempty:x:5:\n";
        assert_eq!(admin_members(text), vec!["alice", "bob", "dave"]);
        assert!(admin_members("wheel:x:998:\nbroken").is_empty());
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
