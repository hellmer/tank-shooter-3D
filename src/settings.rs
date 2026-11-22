//! Player options (volumes, look sensitivity and the VHS effect), saved between games in a small text file.

use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Settings {
    /// 0..1
    pub effects_volume: f32,
    /// 0..1
    pub music_volume: f32,
    /// Multiplier on the default mouse sensitivity (0.25..3).
    pub mouse_sensitivity: f32,
    /// Multiplier on the default stick look speed (0.25..3).
    pub stick_sensitivity: f32,
    /// VHS tape effect with scanlines.
    pub vhs: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            effects_volume: 0.8,
            music_volume: 0.6,
            mouse_sensitivity: 1.0,
            stick_sensitivity: 1.0,
            vhs: true,
        }
    }
}

pub const VOLUME_STEP: f32 = 0.1;
pub const SENSITIVITY_STEP: f32 = 0.25;
pub const SENSITIVITY_RANGE: (f32, f32) = (0.25, 3.0);

impl Settings {
    /// Reads `key = value` lines; unknown keys and bad values keep their defaults.
    pub fn parse(text: &str) -> Settings {
        let mut s = Settings::default();
        for line in text.lines() {
            let Some((k, v)) = line.split_once('=') else {
                continue;
            };
            let Ok(v) = v.trim().parse::<f32>() else {
                continue;
            };
            if !v.is_finite() {
                continue;
            }
            let (lo, hi) = SENSITIVITY_RANGE;
            match k.trim() {
                "effects_volume" => s.effects_volume = v.clamp(0.0, 1.0),
                "music_volume" => s.music_volume = v.clamp(0.0, 1.0),
                "mouse_sensitivity" => s.mouse_sensitivity = v.clamp(lo, hi),
                "stick_sensitivity" => s.stick_sensitivity = v.clamp(lo, hi),
                "vhs" => s.vhs = v != 0.0,
                _ => {}
            }
        }
        s
    }

    pub fn to_text(self) -> String {
        format!(
            "effects_volume = {:.2}\nmusic_volume = {:.2}\nmouse_sensitivity = {:.2}\nstick_sensitivity = {:.2}\nvhs = {}\n",
            self.effects_volume,
            self.music_volume,
            self.mouse_sensitivity,
            self.stick_sensitivity,
            self.vhs as u8
        )
    }

    /// `~/.config/tank_shooter/settings.txt` (or the platform's equivalent).
    fn path() -> Option<PathBuf> {
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("APPDATA").map(PathBuf::from))
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
        Some(base.join("tank_shooter").join("settings.txt"))
    }

    pub fn load() -> Settings {
        Settings::path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .map(|t| Settings::parse(&t))
            .unwrap_or_default()
    }

    /// Best effort: a failed save only means the options are not remembered.
    pub fn save(&self) {
        let Some(path) = Settings::path() else {
            return;
        };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Err(e) = std::fs::write(&path, self.to_text()) {
            eprintln!("Could not save settings to {}: {e}", path.display());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_text() {
        let s = Settings {
            effects_volume: 0.3,
            music_volume: 1.0,
            mouse_sensitivity: 1.75,
            stick_sensitivity: 0.5,
            vhs: false,
        };
        assert_eq!(Settings::parse(&s.to_text()), s);
    }

    #[test]
    fn bad_lines_keep_defaults_and_values_are_clamped() {
        let s = Settings::parse(
            "music_volume = loud\nnonsense\neffects_volume = 7\nmouse_sensitivity = 0\n",
        );
        assert_eq!(s.music_volume, Settings::default().music_volume);
        assert_eq!(s.effects_volume, 1.0);
        assert_eq!(s.mouse_sensitivity, SENSITIVITY_RANGE.0);
    }
}
