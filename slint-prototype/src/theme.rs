//! Colours for the prototype, taken from the same place the shipping app
//! gets them.
//!
//! `bread-theme` writes `$XDG_RUNTIME_DIR/bread/theme.css` with the GTK
//! `@define-color` names. The settings window's accent is that file's
//! `accent` (the wallpaper's color4), not a colour baked into the app.
//! The per-monitor palette JSON only carries the raw pywal slots and does
//! not name an accent, which is why reading it left the window teal.

use std::path::PathBuf;

#[derive(Clone, Copy)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Clone, Copy)]
pub struct Palette {
    pub bg: Rgba,
    pub surface: Rgba,
    pub fg: Rgba,
    pub accent: Rgba,
    pub on_accent: Rgba,
    pub red: Rgba,
}

pub fn load() -> Option<Palette> {
    let css = std::fs::read_to_string(runtime_dir()?.join("bread/theme.css")).ok()?;
    Some(Palette {
        bg: color(&css, "bg")?,
        surface: color(&css, "surface").or_else(|| color(&css, "color0"))?,
        fg: color(&css, "fg").or_else(|| color(&css, "foreground"))?,
        accent: color(&css, "accent")?,
        on_accent: color(&css, "on-accent").unwrap_or(Rgba {
            r: 17,
            g: 17,
            b: 27,
        }),
        red: color(&css, "red").unwrap_or(Rgba {
            r: 224,
            g: 115,
            b: 106,
        }),
    })
}

fn runtime_dir() -> Option<PathBuf> {
    std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from)
}

fn color(css: &str, name: &str) -> Option<Rgba> {
    let needle = format!("@define-color {name} ");
    let line = css.lines().find(|l| l.trim_start().starts_with(&needle))?;
    let hex = line.split_whitespace().nth(2)?.trim_end_matches(';');
    parse_hex(hex)
}

fn parse_hex(text: &str) -> Option<Rgba> {
    let hex = text.trim().trim_start_matches('#');
    if hex.len() < 6 {
        return None;
    }
    Some(Rgba {
        r: u8::from_str_radix(&hex[0..2], 16).ok()?,
        g: u8::from_str_radix(&hex[2..4], 16).ok()?,
        b: u8::from_str_radix(&hex[4..6], 16).ok()?,
    })
}

pub fn mix(a: Rgba, b: Rgba, t: f32) -> Rgba {
    let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Rgba {
        r: m(a.r, b.r),
        g: m(a.g, b.g),
        b: m(a.b, b.b),
    }
}

pub fn solid(color: Rgba) -> slint::Color {
    slint::Color::from_rgb_u8(color.r, color.g, color.b)
}

pub fn with_alpha(color: Rgba, alpha: f32) -> slint::Color {
    slint::Color::from_argb_u8((alpha * 255.0) as u8, color.r, color.g, color.b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_accent_not_a_raw_slot() {
        let css = "@define-color color4 #624f64;\n@define-color accent #ce9c65;\n@define-color bg #11111b;\n@define-color fg #e9ecef;\n@define-color surface #17191c;\n";
        assert_eq!(color(css, "accent").unwrap().r, 0xce);
        assert_eq!(color(css, "bg").unwrap().b, 0x1b);
    }
}
