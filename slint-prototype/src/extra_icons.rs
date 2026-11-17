//! Extra lucide glyphs used by pages added after the icon generator ran.
//! Geometry is the same 24x24, 2px round stroke data as src/icons.rs; the
//! generator's source package is not vendored in this worktree, so these are
//! kept here rather than regenerating the whole file.

pub fn commands(name: &str) -> &'static str {
    match name {
        "mic" => MIC,
        "headphones" => HEADPHONES,
        "speaker" => SPEAKER,
        "volume-x" => VOLUME_X,
        "phone" => PHONE,
        "laptop" => LAPTOP,
        "cable" => CABLE,
        "ethernet" => ETHERNET,
        "plus" => PLUS,
        "check" => CHECK,
        "alert" => ALERT,
        "cpu" => CPU,
        "gamepad" => GAMEPAD,
        "watch" => WATCH,
        "bt-off" => BT_OFF,
        "folder" => FOLDER,
        "sun" => SUN,
        "wrench" => WRENCH,
        "map-pin" => MAP_PIN,
        _ => "",
    }
}

pub const ETHERNET: &str = "M 15 20 L 18 17 H 20 A 2 2 0 0 0 22 15 V 6 A 2 2 0 0 0 20 4 H 4 A 2 2 0 0 0 2 6 V 15 A 2 2 0 0 0 4 17 H 6 L 9 20 Z M 6 8 V 9 M 10 8 V 9 M 14 8 V 9 M 18 8 V 9";

pub const MIC: &str = "M12 19v3 M19 10v2a7 7 0 0 1-14 0v-2 M 12 2 A 3 3 0 0 0 9 5 V 12 A 3 3 0 0 0 15 12 V 5 A 3 3 0 0 0 12 2 Z";
pub const HEADPHONES: &str = "M3 14h3a2 2 0 0 1 2 2v3a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-7a9 9 0 0 1 18 0v7a2 2 0 0 1-2 2h-1a2 2 0 0 1-2-2v-3a2 2 0 0 1 2-2h3";
pub const SPEAKER: &str = "M 6 3 H 18 A 2 2 0 0 1 20 5 V 19 A 2 2 0 0 1 18 21 H 6 A 2 2 0 0 1 4 19 V 5 A 2 2 0 0 1 6 3 Z M 10 7 A 2 2 0 1 0 14 7 A 2 2 0 1 0 10 7 Z M 9 15 A 3 3 0 1 0 15 15 A 3 3 0 1 0 9 15 Z";
pub const VOLUME_X: &str = "M11 4.702a.705.705 0 0 0-1.203-.498L6.413 7.587A1.4 1.4 0 0 1 5.416 8H3a1 1 0 0 0-1 1v6a1 1 0 0 0 1 1h2.416a1.4 1.4 0 0 1 .997.413l3.383 3.384A.705.705 0 0 0 11 19.298z M22 9l-6 6 M16 9l6 6";
pub const PHONE: &str = "M 8 2 H 16 A 2 2 0 0 1 18 4 V 20 A 2 2 0 0 1 16 22 H 8 A 2 2 0 0 1 6 20 V 4 A 2 2 0 0 1 8 2 Z M12 18h0.1";
pub const LAPTOP: &str = "M 4 5 H 20 A 2 2 0 0 1 22 7 V 15 A 2 2 0 0 1 20 17 H 4 A 2 2 0 0 1 2 15 V 7 A 2 2 0 0 1 4 5 Z M2 20h20";
pub const CABLE: &str = "M 4 9 V 6 A 2 2 0 0 1 6 4 H 8 A 2 2 0 0 1 10 6 V 9 M 14 15 V 18 A 2 2 0 0 0 16 20 H 18 A 2 2 0 0 0 20 18 V 15 M 2 9 H 12 M 12 15 H 22 M 12 9 V 15";
pub const PLUS: &str = "M5 12h14 M12 5v14";
pub const CHECK: &str = "M20 6 9 17l-5-5";
pub const ALERT: &str = "M 2 12 A 10 10 0 1 1 22 12 A 10 10 0 1 1 2 12 Z M12 8v4 M12 16h0.1";
pub const CPU: &str = "M 4 4 H 16 A 2 2 0 0 1 18 6 V 18 A 2 2 0 0 1 16 20 H 4 A 2 2 0 0 1 2 18 V 6 A 2 2 0 0 1 4 4 Z M 9 9 H 13 A 1 1 0 0 1 14 10 V 14 A 1 1 0 0 1 13 15 H 9 A 1 1 0 0 1 8 14 V 10 A 1 1 0 0 1 9 9 Z M12 2v2 M12 20v2 M2 12h2 M20 12h2 M2 7h2 M2 17h2 M20 7h2 M20 17h2 M7 2v2 M17 2v2 M7 20v2 M17 20v2";
pub const GAMEPAD: &str = "M6 12h4 M8 10v4 M15 13h0.1 M18 11h0.1 M 7 7 H 17 A 5 5 0 0 1 22 12 V 14 A 4 4 0 0 1 14 15 L 13 14 H 11 L 10 15 A 4 4 0 0 1 2 14 V 12 A 5 5 0 0 1 7 7 Z";
pub const WATCH: &str = "M 8 7 H 16 A 2 2 0 0 1 18 9 V 15 A 2 2 0 0 1 16 17 H 8 A 2 2 0 0 1 6 15 V 9 A 2 2 0 0 1 8 7 Z M12 10v2l1 1 M 7 17 L 6 22 H 18 L 17 17 M 7 7 L 6 2 H 18 L 17 7";
pub const BT_OFF: &str = "m17 17-5 5V12l-5 5 M2 2l20 20 M14.5 9.5 17 7l-5-5v4.5";
pub const FOLDER: &str = "m6 14 1.5-2.9A2 2 0 0 1 9.24 10H20a2 2 0 0 1 1.94 2.5l-1.54 6a2 2 0 0 1-1.95 1.5H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h3.9a2 2 0 0 1 1.69.9l.81 1.2a2 2 0 0 0 1.67.9H18a2 2 0 0 1 2 2v2";
pub const SUN: &str = "M 8 12 A 4 4 0 1 1 16 12 A 4 4 0 1 1 8 12 Z M12 2v2 M12 20v2 M4.93 4.93l1.42 1.42 M17.66 17.66l1.41 1.41 M2 12h2 M20 12h2 M6.34 17.66l-1.41 1.41 M19.07 4.93l-1.41 1.41";
pub const WRENCH: &str = "M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z";
pub const MAP_PIN: &str = "M20 10c0 4.993-5.539 10.193-7.399 11.799a1 1 0 0 1-1.202 0C9.539 20.193 4 14.993 4 10a8 8 0 0 1 16 0 M12 10h0.1";
