// Copyright (c) 2026 Luka Dežulović
// SPDX-License-Identifier: Apache-2.0
//! Local preferences and one-time import of settings from the Godot prototype.
use bevy::{prelude::*, window::PresentMode};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TailColor {
    #[default]
    None,
    NaturalGreen,
    Violet,
    Pink,
    LagoonBlue,
    Coral,
    Gold,
    Magic,
    Custom([u8; 3]),
}

impl TailColor {
    pub const PRESETS: [Self; 8] = [
        Self::None,
        Self::NaturalGreen,
        Self::Violet,
        Self::Pink,
        Self::LagoonBlue,
        Self::Coral,
        Self::Gold,
        Self::Magic,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::NaturalGreen => "Natural green",
            Self::Violet => "Violet",
            Self::Pink => "Pink",
            Self::LagoonBlue => "Lagoon blue",
            Self::Coral => "Coral",
            Self::Gold => "Gold",
            Self::Magic => "Magic",
            Self::Custom(_) => "Custom",
        }
    }

    pub fn rgb(self) -> Option<[u8; 3]> {
        match self {
            Self::None | Self::Magic => None,
            Self::NaturalGreen => Some([65, 190, 125]),
            Self::Violet => Some([155, 95, 230]),
            Self::Pink => Some([240, 110, 180]),
            Self::LagoonBlue => Some([55, 180, 220]),
            Self::Coral => Some([245, 125, 100]),
            Self::Gold => Some([235, 185, 70]),
            Self::Custom(rgb) => Some(rgb),
        }
    }
}

fn deserialize_tail_color<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<TailColor, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum SavedColor {
        Named(TailColor),
        Legacy([u8; 3]),
    }
    Ok(match Option::<SavedColor>::deserialize(deserializer)? {
        None => TailColor::None,
        Some(SavedColor::Named(color)) => color,
        Some(SavedColor::Legacy(rgb)) => TailColor::Custom(rgb),
    })
}

#[derive(Resource, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub language: usize,
    pub best: i32,
    #[serde(default)]
    pub scoring_version: u8,
    pub gentle: bool,
    pub rest: f32,
    pub visitors: i32,
    pub fullscreen: bool,
    pub vsync: bool,
    pub show_fps: bool,
    pub quality: u8,
    pub reduced_motion: bool,
    pub rumble: u8,
    pub music: f32,
    pub effects: f32,
    pub voice: f32,
    #[serde(deserialize_with = "deserialize_tail_color")]
    pub tail_color: TailColor,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            language: 0,
            best: 0,
            scoring_version: 2,
            gentle: true,
            rest: 55.0,
            visitors: 2,
            fullscreen: true,
            vsync: true,
            show_fps: false,
            quality: 1,
            reduced_motion: false,
            rumble: 2,
            music: 0.30,
            effects: 0.55,
            voice: 0.90,
            tail_color: TailColor::None,
        }
    }
}

impl Preferences {
    pub fn path() -> PathBuf {
        let home = PathBuf::from(std::env::var_os("HOME").unwrap_or_default());
        let base = if cfg!(target_os = "windows") {
            std::env::var_os("APPDATA")
                .map(PathBuf::from)
                .unwrap_or(home)
        } else if cfg!(target_os = "macos") {
            home.join("Library/Application Support")
        } else {
            std::env::var_os("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| home.join(".config"))
        };
        base.join("sabrinina-podvodna-bajka/settings.json")
    }
    // First launch carries forward the Godot preferences without editing its save.
    fn legacy() -> Option<Self> {
        let home = PathBuf::from(std::env::var_os("HOME").unwrap_or_default());
        let base = if cfg!(target_os = "windows") {
            std::env::var_os("APPDATA").map(PathBuf::from)?
        } else if cfg!(target_os = "macos") {
            home.join("Library/Application Support")
        } else {
            std::env::var_os("XDG_DATA_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| home.join(".local/share"))
        };
        let data = std::fs::read_to_string(
            base.join("godot/app_userdata/Sabrinina podvodna bajka/settings.cfg"),
        )
        .ok()?;
        let mut preferences = Self::default();
        let mut section = "";
        for line in data.lines().map(str::trim) {
            if line.starts_with('[') && line.ends_with(']') {
                section = &line[1..line.len() - 1];
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let value = value.trim();
            macro_rules! read {
                ($field:ident) => {
                    if let Ok(v) = value.parse() {
                        preferences.$field = v;
                    }
                };
            }
            match (section, key.trim()) {
                ("game", "language") => read!(language),
                ("game", "gentle") => read!(gentle),
                ("game", "rest") => read!(rest),
                ("game", "visitors") => read!(visitors),
                ("video", "fullscreen") => read!(fullscreen),
                ("video", "quality") => read!(quality),
                ("video", "reduced_motion") => read!(reduced_motion),
                ("audio", "music") => read!(music),
                ("audio", "effects") => read!(effects),
                ("audio", "voice") => read!(voice),
                _ => {}
            }
        }
        Some(preferences)
    }

    pub fn load() -> Self {
        let mut preferences: Self = std::fs::read(Self::path())
            .ok()
            .and_then(|s| serde_json::from_slice(&s).ok())
            .or_else(Self::legacy)
            .unwrap_or_default();
        preferences.language %= 3;
        preferences.migrate_score();
        preferences.quality %= 3;
        preferences.rumble = preferences.rumble.min(2);
        preferences.rest = preferences.rest.clamp(25.0, 150.0);
        preferences.visitors = preferences.visitors.clamp(1, 3);
        preferences.music = preferences.music.clamp(0.0, 1.0);
        preferences.effects = preferences.effects.clamp(0.0, 1.0);
        preferences.voice = preferences.voice.clamp(0.0, 1.0);
        preferences
    }

    fn migrate_score(&mut self) {
        if self.scoring_version != 2 {
            info!("Resetting best score for the skill-based scoring rules.");
            self.best = 0;
            self.scoring_version = 2;
        }
    }

    pub fn save(&self) {
        if cfg!(test)
            || std::env::args().any(|a| a == "--verify-input" || a.starts_with("--capture-"))
        {
            return;
        }
        let path = Self::path();
        if let Some(parent) = path.parent()
            && let Err(e) = std::fs::create_dir_all(parent)
        {
            warn!("Cannot create settings directory: {e}");
            return;
        }
        if let Ok(data) = serde_json::to_vec_pretty(self)
            && let Err(e) = std::fs::write(path, data)
        {
            warn!("Cannot save preferences: {e}")
        }
    }

    /// Bevy tries relaxed FIFO, then FIFO. Late relaxed-FIFO frames may tear.
    pub fn present_mode(&self) -> PresentMode {
        if self.vsync {
            PresentMode::AutoVsync
        } else {
            PresentMode::AutoNoVsync
        }
    }

    pub fn simulation(&self, aquarium: bool) -> reef_sim::Settings {
        reef_sim::Settings {
            gentle: self.gentle,
            aquarium,
            rest_seconds: self.rest,
            visitor_limit: self.visitors,
            reduced_motion: self.reduced_motion,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tail_color_defaults_to_original_and_round_trips() {
        let mut preferences: Preferences = serde_json::from_str("{}").unwrap();
        assert_eq!(preferences.tail_color, TailColor::None);
        for color in TailColor::PRESETS.into_iter().chain([
            TailColor::Custom([0, 0, 0]),
            TailColor::Custom([255, 255, 255]),
            TailColor::Custom([123, 45, 210]),
        ]) {
            preferences.tail_color = color;
            let restored: Preferences =
                serde_json::from_str(&serde_json::to_string(&preferences).unwrap()).unwrap();
            assert_eq!(restored.tail_color, color);
        }
        assert!(serde_json::from_str::<Preferences>(r#"{"tail_color":[256,0,0]}"#).is_err());
        let original: Preferences = serde_json::from_str(r#"{"tail_color":null}"#).unwrap();
        assert_eq!(original.tail_color, TailColor::None);
    }

    #[test]
    fn removed_color_preferences_are_ignored_without_losing_tail_color() {
        let preferences: Preferences = serde_json::from_str(
            r#"{"tail_color":[123,45,210],"hair_color":[255,0,0],"bikini_color":[0,255,0]}"#,
        )
        .unwrap();
        assert_eq!(preferences.tail_color, TailColor::Custom([123, 45, 210]));
        let saved = serde_json::to_value(&preferences).unwrap();
        assert!(saved.get("hair_color").is_none());
        assert!(saved.get("bikini_color").is_none());
    }

    #[test]
    fn rumble_defaults_to_normal_and_all_levels_survive_serialization() {
        let mut preferences: Preferences = serde_json::from_str("{}").unwrap();
        assert_eq!(preferences.rumble, 2);
        for level in 0..=2 {
            preferences.rumble = level;
            let restored: Preferences =
                serde_json::from_str(&serde_json::to_string(&preferences).unwrap()).unwrap();
            assert_eq!(restored.rumble, level);
        }
    }

    #[test]
    fn fps_counter_defaults_off_and_survives_serialization() {
        let mut preferences: Preferences = serde_json::from_str("{}").unwrap();
        assert!(!preferences.show_fps);
        preferences.show_fps = true;
        let saved = serde_json::to_string(&preferences).unwrap();
        let restored: Preferences = serde_json::from_str(&saved).unwrap();
        assert!(restored.show_fps);
    }

    #[test]
    fn vsync_defaults_on_and_selects_present_mode() {
        let mut preferences: Preferences = serde_json::from_str(r#"{"show_fps":true}"#).unwrap();
        assert!(preferences.vsync);
        assert_eq!(preferences.present_mode(), PresentMode::AutoVsync);
        preferences.vsync = false;
        let saved = serde_json::to_string(&preferences).unwrap();
        let restored: Preferences = serde_json::from_str(&saved).unwrap();
        assert!(!restored.vsync);
        assert_eq!(restored.present_mode(), PresentMode::AutoNoVsync);
    }

    #[test]
    fn old_records_reset_without_discarding_other_preferences() {
        let mut preferences: Preferences =
            serde_json::from_str(r#"{"best":12345,"gentle":false,"language":2}"#).unwrap();
        assert_eq!(preferences.scoring_version, 0);
        preferences.migrate_score();
        assert_eq!(preferences.best, 0);
        assert_eq!(preferences.scoring_version, 2);
        assert!(!preferences.gentle);
        assert_eq!(preferences.language, 2);
    }

    #[test]
    fn current_records_survive_loading_and_repeated_migration() {
        let mut preferences: Preferences =
            serde_json::from_str(r#"{"best":450,"scoring_version":2}"#).unwrap();
        preferences.migrate_score();
        preferences.migrate_score();
        assert_eq!(preferences.best, 450);
        let saved = serde_json::to_string(&preferences).unwrap();
        let restored: Preferences = serde_json::from_str(&saved).unwrap();
        assert_eq!(restored.scoring_version, 2);
        assert_eq!(restored.best, 450);
    }
}
