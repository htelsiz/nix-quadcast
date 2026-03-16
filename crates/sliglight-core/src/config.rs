//! Persistent application configuration via XDG-compliant config files.
//!
//! Config path: `~/.config/sliglight/config.toml` (managed by confy).

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::animations::{Mode, Zone};
use crate::presets;

/// A single lighting profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub mode: Mode,
    pub zone: Zone,
    pub brightness: u8,
    pub speed: u8,
    pub colors: Vec<(u8, u8, u8)>,
}

impl Profile {
    /// Serialize this profile to a TOML string.
    pub fn to_toml(&self) -> Result<String, toml::ser::Error> {
        toml::to_string_pretty(self)
    }

    /// Deserialize a profile from a TOML string.
    pub fn from_toml(s: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(s)
    }
}

/// Top-level application configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub active_profile: String,
    pub profiles: HashMap<String, Profile>,
    pub mute_indicator_enabled: bool,
    pub screen_lock_blackout: bool,
    pub close_to_tray: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        let mut profiles = HashMap::new();
        for (name, profile) in presets::all() {
            profiles.insert(name, profile);
        }
        Self {
            active_profile: "Catppuccin Mocha".to_string(),
            profiles,
            mute_indicator_enabled: true,
            screen_lock_blackout: true,
            close_to_tray: false,
        }
    }
}

impl AppConfig {
    /// Load config from disk, falling back to defaults.
    pub fn load() -> Self {
        confy::load("sliglight", "config").unwrap_or_default()
    }

    /// Save config to disk.
    pub fn save(&self) {
        let _ = confy::store("sliglight", "config", self);
    }

    /// Get the currently active profile, or `None` if it doesn't exist.
    pub fn active_profile(&self) -> Option<&Profile> {
        self.profiles.get(&self.active_profile)
    }
}
