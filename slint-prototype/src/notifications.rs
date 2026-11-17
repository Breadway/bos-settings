//! Notification preferences owned by the bar, over its fixed D-Bus API.
//! The JSON is kept as a value so unmodelled keys survive a save.

use std::process::Command;

use serde_json::Value;

fn call(method: &str, argument: Option<&str>) -> Result<Value, String> {
    let mut command = Command::new("busctl");
    command.args([
        "--user",
        "--json=short",
        "call",
        "org.freedesktop.Notifications",
        "/dev/breadway/Bar",
        "dev.breadway.Bar",
        method,
    ]);
    if let Some(argument) = argument {
        command.arg("s").arg(argument);
    }
    let output = command.output().map_err(|error| error.to_string())?;
    if !output.status.success() {
        let message = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if message.is_empty() {
            "the running notification server does not expose settings".into()
        } else {
            message
        });
    }
    if argument.is_some() {
        return Ok(Value::Null);
    }
    serde_json::from_slice(&output.stdout).map_err(|error| error.to_string())
}

pub fn load() -> Result<Value, String> {
    let answer = call("GetNotificationPreferences", None)?;
    let text = answer
        .get("data")
        .and_then(|data| data.get(0))
        .and_then(|value| value.as_str())
        .ok_or("invalid notification settings response")?;
    serde_json::from_str(text).map_err(|error| error.to_string())
}

pub fn save(preferences: &Value) -> Result<(), String> {
    let text = serde_json::to_string(preferences).map_err(|error| error.to_string())?;
    call("SetNotificationPreferences", Some(&text)).map(|_| ())
}

pub fn toggle(key: &str) -> Result<(), String> {
    let mut preferences = load()?;
    let object = preferences
        .as_object_mut()
        .ok_or("notification settings must be an object")?;
    let value = object.get(key).and_then(Value::as_bool).unwrap_or(false);
    object.insert(key.to_string(), Value::Bool(!value));
    save(&preferences)
}

pub fn cycle_app(name: &str) -> Result<(), String> {
    let mut preferences = load()?;
    let apps = preferences
        .get_mut("apps")
        .and_then(Value::as_object_mut)
        .ok_or("notification settings have no apps map")?;
    let current = apps
        .get(name)
        .and_then(Value::as_str)
        .unwrap_or("all")
        .to_string();
    let next = match current.as_str() {
        "all" => "banners",
        "banners" => "history",
        "history" => "off",
        _ => "all",
    };
    apps.insert(name.to_string(), Value::String(next.into()));
    save(&preferences)
}

/// Cycle a policy value without touching D-Bus, so the mapping is testable.
pub fn next_policy(current: &str) -> &'static str {
    match current {
        "all" => "banners",
        "banners" => "history",
        "history" => "off",
        _ => "all",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policies_cycle_through_every_state() {
        assert_eq!(next_policy("all"), "banners");
        assert_eq!(next_policy("banners"), "history");
        assert_eq!(next_policy("history"), "off");
        assert_eq!(next_policy("off"), "all");
        assert_eq!(next_policy("nonsense"), "all");
    }

    #[test]
    fn unmodelled_keys_survive_a_toggle_edit() {
        let mut preferences = serde_json::json!({
            "dnd": false,
            "schedule": true,
            "start": "22:00",
            "end": "07:00",
            "apps": {"chat": "banners"},
            "future_key": {"keep": true}
        });
        let value = preferences.get("dnd").and_then(Value::as_bool).unwrap_or(false);
        preferences["dnd"] = Value::Bool(!value);
        preferences["apps"]["chat"] = Value::String(next_policy("banners").into());
        assert_eq!(preferences["dnd"], Value::Bool(true));
        assert_eq!(preferences["apps"]["chat"], Value::String("history".into()));
        assert_eq!(preferences["future_key"]["keep"], Value::Bool(true));
    }
}
