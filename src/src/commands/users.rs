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

/// The account running this app. `USER` is set for the normal desktop
/// session; `LOGNAME` covers sessions that only export that. An empty value
/// simply means no override account is added.
fn current_user() -> String {
    ["USER", "LOGNAME"]
        .iter()
        .find_map(|key| {
            std::env::var(key)
                .ok()
                .filter(|value| !value.trim().is_empty())
        })
        .unwrap_or_default()
}

/// A shell a person can actually log in with. Service accounts use
/// `nologin`/`false`; `/bin/sync` and empty shells are not login shells
/// either.
fn is_login_shell(shell: &str) -> bool {
    let shell = shell.trim();
    !shell.is_empty()
        && !shell.ends_with("nologin")
        && !shell.ends_with("/false")
        && shell != "/bin/sync"
}

/// A home directory rather than a service path. System accounts mostly use
/// `/`, `/run`, `/var/lib/...` or `/usr/share/...` (and `nobody` uses `/`).
fn has_real_home(home: &str) -> bool {
    let home = home.trim();
    if home.is_empty() {
        return false;
    }
    if home == "/" || home == "/nonexistent" || home.starts_with("/nonexistent/") {
        return false;
    }
    !["/run", "/var", "/usr", "/proc", "/sys", "/dev"]
        .iter()
        .any(|root| home == *root || home.starts_with(&format!("{root}/")))
}

/// Human accounts are decided by shell and home, not only a UID window:
/// BOS's live user is UID 60001 (BSET-99), well above the old 1000..60000
/// filter. A UID floor still keeps system accounts out alongside the shell
/// and home checks, and `nobody` (65534) never counts as a person.
fn is_human_account(uid: u32, shell: &str, home: &str) -> bool {
    uid != 0 && (1000..65534).contains(&uid) && is_login_shell(shell) && has_real_home(home)
}

fn list_accounts_from(passwd: &str, group: &str, current: &str) -> Vec<Account> {
    let admins = admin_members(group);
    let mut accounts: Vec<Account> = passwd
        .lines()
        .filter_map(|line| {
            let f: Vec<&str> = line.split(':').collect();
            if f.len() < 7 {
                return None;
            }
            let uid: u32 = f[2].parse().ok()?;
            let username = f[0];
            // Always show the current user, even if a UID, shell or home
            // rule would otherwise drop them (BSET-99).
            if username != current && !is_human_account(uid, f[6], f[5]) {
                return None;
            }
            Some(Account {
                username: username.to_string(),
                full_name: f[4].split(',').next().unwrap_or("").to_string(),
                admin: admins.iter().any(|a| a == username),
            })
        })
        .collect();
    // The env can name an account that /etc/passwd lacks (or one the rules
    // drop); the page still needs the person who is logged in.
    if !current.is_empty() && !accounts.iter().any(|a| a.username == current) {
        accounts.push(Account {
            username: current.to_string(),
            full_name: String::new(),
            admin: admins.iter().any(|a| a == current),
        });
    }
    accounts
}

#[derive(Serialize)]
pub struct UsersInfo {
    accounts: Vec<Account>,
    current_user: String,
}

#[tauri::command]
pub fn get_users_info() -> UsersInfo {
    let current_user = current_user();
    let passwd = std::fs::read_to_string("/etc/passwd").unwrap_or_default();
    let group = std::fs::read_to_string("/etc/group").unwrap_or_default();
    UsersInfo {
        accounts: list_accounts_from(&passwd, &group, &current_user),
        current_user,
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

    const PASSWD: &str = "root:x:0:0:root:/root:/bin/bash\n\
systemd-network:x:990:990::/:/usr/bin/nologin\n\
git:x:970:970::/usr/share/git:/bin/bash\n\
nobody:x:65534:65534::/:/usr/bin/nologin\n\
daemon:x:998:998::/:/usr/bin/nologin\n\
alice:x:1000:1000:Alice:/home/alice:/bin/bash\n\
liveuser:x:60001:60001:Live User:/home/liveuser:/bin/bash\n";
    const GROUP: &str = "wheel:x:998:alice\n";

    fn usernames(accounts: &[Account]) -> Vec<&str> {
        accounts.iter().map(|a| a.username.as_str()).collect()
    }

    #[test]
    fn liveuser_uid_60001_is_listed() {
        let accounts = list_accounts_from(PASSWD, GROUP, "liveuser");
        assert!(
            usernames(&accounts).contains(&"liveuser"),
            "the live ISO user is UID 60001, above the old 60000 cutoff: {:?}",
            usernames(&accounts)
        );
        let live = accounts.iter().find(|a| a.username == "liveuser").unwrap();
        assert_eq!(live.full_name, "Live User");
        assert!(!live.admin);
    }

    #[test]
    fn normal_uid_1000_user_is_listed_and_can_be_an_admin() {
        let accounts = list_accounts_from(PASSWD, GROUP, "alice");
        let alice = accounts.iter().find(|a| a.username == "alice").unwrap();
        assert_eq!(alice.full_name, "Alice");
        assert!(alice.admin, "wheel membership still marks an administrator");
    }

    #[test]
    fn system_and_service_accounts_are_excluded() {
        let accounts = list_accounts_from(PASSWD, GROUP, "liveuser");
        let names = usernames(&accounts);
        for system in ["root", "systemd-network", "git", "nobody", "daemon"] {
            assert!(
                !names.contains(&system),
                "{system} is not a person: {names:?}"
            );
        }
        assert_eq!(names, vec!["alice", "liveuser"]);
    }

    #[test]
    fn current_user_survives_a_filter_that_would_drop_them() {
        let passwd = "svc:x:999:999::/usr/share/svc:/usr/bin/nologin\n";
        let accounts = list_accounts_from(passwd, "", "svc");
        assert!(usernames(&accounts).contains(&"svc"));
    }

    #[test]
    fn current_user_missing_from_passwd_is_still_listed() {
        let accounts =
            list_accounts_from("alice:x:1000:1000::/home/alice:/bin/bash\n", "", "ghost");
        assert!(usernames(&accounts).contains(&"ghost"));
    }

    #[test]
    fn human_accounts_need_a_login_shell_and_a_real_home() {
        assert!(is_human_account(1500, "/bin/bash", "/home/x"));
        assert!(!is_human_account(1500, "/usr/bin/nologin", "/home/x"));
        assert!(!is_human_account(1500, "/bin/false", "/home/x"));
        assert!(!is_human_account(1500, "/bin/bash", "/var/lib/x"));
        assert!(!is_human_account(1500, "/bin/bash", "/"));
        assert!(!is_human_account(0, "/bin/bash", "/root"));
        assert!(!is_human_account(65534, "/bin/bash", "/home/nobody"));
    }
}
