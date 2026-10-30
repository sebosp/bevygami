//! The settings and the place in the lessons, saved between runs.
//!
//! Native builds save to `~/.config/bevygami/settings.json` (or `$XDG_CONFIG_HOME`), Android to the
//! private storage of the app, the web build to the `localStorage` of the browser.
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::theme::ThemeMode;

/// The colors of the paper, its front side; the back is white.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum PaperColor {
    #[default]
    Kami,
    Mint,
    Lavender,
    Washi,
}

impl PaperColor {
    pub const ALL: [PaperColor; 4] = [
        PaperColor::Kami,
        PaperColor::Mint,
        PaperColor::Lavender,
        PaperColor::Washi,
    ];

    pub fn name(self) -> &'static str {
        match self {
            PaperColor::Kami => "Kami gold",
            PaperColor::Mint => "Mint",
            PaperColor::Lavender => "Lavender",
            PaperColor::Washi => "Washi white",
        }
    }

    /// The crease colors are validated against these (dataviz validator, ≥ 3:1).
    pub fn hex(self) -> &'static str {
        match self {
            PaperColor::Kami => "#f2c14e",
            PaperColor::Mint => "#9fd8c3",
            PaperColor::Lavender => "#c9bdf0",
            PaperColor::Washi => "#f6f3ec",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Resource)]
#[serde(default)]
pub struct Settings {
    pub theme: ThemeMode,
    pub paper: PaperColor,
    /// The arrow of the coming fold.
    pub arrows: bool,
    /// The dotted outline where the moving flap lands, the X-ray line of the diagrams.
    pub landing: bool,
    /// The crease pattern of the sheet, in a corner.
    pub crease_pattern: bool,
    /// No breathing lines, faster folds.
    pub reduce_motion: bool,
    pub lesson: usize,
    pub step: usize,
    /// The lessons with every step folded.
    pub done: Vec<usize>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            theme: ThemeMode::Dark,
            paper: PaperColor::Kami,
            arrows: true,
            landing: true,
            crease_pattern: true,
            reduce_motion: false,
            lesson: 0,
            step: 0,
            done: Vec::new(),
        }
    }
}

impl Settings {
    pub fn load() -> Self {
        if std::env::var_os("BEVYGAMI_FRESH").is_some() {
            return Settings::default();
        }
        storage::read()
            .and_then(|json| match serde_json::from_str(&json) {
                Ok(settings) => Some(settings),
                Err(err) => {
                    warn!("Ignoring the unreadable settings: {err}");
                    None
                }
            })
            .unwrap_or_default()
    }

    pub fn save(&self) {
        if std::env::var_os("BEVYGAMI_FRESH").is_some() {
            return;
        }
        match serde_json::to_string_pretty(self) {
            Ok(json) => storage::write(&json),
            Err(err) => error!("Unable to save the settings: {err}"),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod storage {
    use std::path::PathBuf;

    use bevy::prelude::*;

    pub fn config_dir() -> Option<PathBuf> {
        // The private storage of the app.
        #[cfg(target_os = "android")]
        {
            return bevy::android::ANDROID_APP
                .get()
                .and_then(|app| app.internal_data_path());
        }
        #[cfg(target_os = "windows")]
        {
            return std::env::var_os("APPDATA").map(|dir| PathBuf::from(dir).join("bevygami"));
        }
        #[allow(unreachable_code)]
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;
        Some(base.join("bevygami"))
    }

    pub fn read() -> Option<String> {
        std::fs::read_to_string(config_dir()?.join("settings.json")).ok()
    }

    pub fn write(json: &str) {
        let Some(dir) = config_dir() else {
            warn!("No HOME, the settings are not saved");
            return;
        };
        let path = dir.join("settings.json");
        // Write to a temporary file first, the settings survive a crash while saving.
        let tmp = dir.join("settings.json.tmp");
        let result = std::fs::create_dir_all(&dir)
            .and_then(|_| std::fs::write(&tmp, json))
            .and_then(|_| std::fs::rename(&tmp, &path));
        if let Err(err) = result {
            error!("Unable to save the settings to {}: {err}", path.display());
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod storage {
    const KEY: &str = "bevygami.settings";

    fn local_storage() -> Option<web_sys::Storage> {
        web_sys::window()?.local_storage().ok().flatten()
    }

    pub fn read() -> Option<String> {
        local_storage()?.get_item(KEY).ok().flatten()
    }

    pub fn write(json: &str) {
        if let Some(storage) = local_storage() {
            let _ = storage.set_item(KEY, json);
        }
    }
}
