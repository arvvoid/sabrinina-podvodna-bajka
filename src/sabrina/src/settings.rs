// Copyright (c) 2026 Luka Dežulović
// SPDX-License-Identifier: MIT
//! Local preferences and one-time import of settings from the Godot prototype.
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Resource, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub language: usize,
    pub best: i32,
    pub gentle: bool,
    pub rest: f32,
    pub visitors: i32,
    pub fullscreen: bool,
    pub quality: u8,
    pub reduced_motion: bool,
    pub music: f32,
    pub effects: f32,
    pub voice: f32,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            language: 0,
            best: 0,
            gentle: true,
            rest: 55.0,
            visitors: 2,
            fullscreen: true,
            quality: 1,
            reduced_motion: false,
            music: 0.30,
            effects: 0.55,
            voice: 0.90,
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
                ("game", "best") => read!(best),
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
        preferences.quality %= 3;
        preferences.rest = preferences.rest.clamp(25.0, 150.0);
        preferences.visitors = preferences.visitors.clamp(1, 3);
        preferences.music = preferences.music.clamp(0.0, 1.0);
        preferences.effects = preferences.effects.clamp(0.0, 1.0);
        preferences.voice = preferences.voice.clamp(0.0, 1.0);
        preferences
    }

    pub fn save(&self) {
        if std::env::args().any(|a| a == "--verify-input" || a.starts_with("--capture-")) {
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
