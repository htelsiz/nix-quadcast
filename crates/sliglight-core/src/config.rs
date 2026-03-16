//! Persistent application configuration via XDG-compliant config files.
//!
//! Config path: `~/.config/sliglight/config.toml` (managed by confy).
//! Built-in profiles (8 theme + 5 practical) are inserted on first run.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::animations::{Mode, Zone};

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
        Self {
            active_profile: "Catppuccin Mocha".to_string(),
            profiles: builtin_profiles(),
            mute_indicator_enabled: true,
            screen_lock_blackout: true,
            close_to_tray: false,
        }
    }
}

/// Built-in profiles shipped with every fresh config (8 theme + 5 practical).
pub fn builtin_profiles() -> HashMap<String, Profile> {
    HashMap::from([
        // ── Theme profiles ─────────────────────────────────────────
        ("Catppuccin Mocha".into(), Profile {
            mode: Mode::Wave, zone: Zone::Both, brightness: 80, speed: 40,
            colors: vec![
                (203, 166, 247), (137, 180, 250), (116, 199, 236),
                (166, 227, 161), (249, 226, 175), (250, 179, 135), (243, 139, 168),
            ],
        }),
        ("Catppuccin Latte".into(), Profile {
            mode: Mode::Wave, zone: Zone::Both, brightness: 90, speed: 35,
            colors: vec![
                (136, 57, 239), (30, 102, 245), (32, 159, 181),
                (64, 160, 43), (223, 142, 29), (254, 100, 11), (210, 15, 57),
            ],
        }),
        ("Nord".into(), Profile {
            mode: Mode::Cycle, zone: Zone::Both, brightness: 75, speed: 30,
            colors: vec![
                (136, 192, 208), (129, 161, 193), (94, 129, 172),
                (163, 190, 140), (180, 142, 173),
            ],
        }),
        ("Dracula".into(), Profile {
            mode: Mode::Wave, zone: Zone::Both, brightness: 85, speed: 45,
            colors: vec![
                (189, 147, 249), (255, 121, 198), (139, 233, 253),
                (80, 250, 123), (255, 184, 108), (241, 250, 140),
            ],
        }),
        ("Gruvbox".into(), Profile {
            mode: Mode::Cycle, zone: Zone::Both, brightness: 80, speed: 35,
            colors: vec![
                (251, 73, 52), (250, 189, 47), (184, 187, 38),
                (131, 165, 152), (211, 134, 155), (254, 128, 25),
            ],
        }),
        ("Rose Pine".into(), Profile {
            mode: Mode::Wave, zone: Zone::Both, brightness: 75, speed: 40,
            colors: vec![
                (235, 188, 186), (196, 167, 231), (156, 207, 216),
                (246, 193, 119), (234, 154, 151),
            ],
        }),
        ("Tokyo Night".into(), Profile {
            mode: Mode::Cycle, zone: Zone::Both, brightness: 80, speed: 35,
            colors: vec![
                (122, 162, 247), (187, 154, 247), (125, 207, 255),
                (158, 206, 106), (224, 175, 104), (247, 118, 142),
            ],
        }),
        ("Solarized".into(), Profile {
            mode: Mode::Cycle, zone: Zone::Both, brightness: 80, speed: 30,
            colors: vec![
                (38, 139, 210), (42, 161, 152), (133, 153, 0),
                (203, 75, 22), (220, 50, 47), (108, 113, 196),
            ],
        }),
        // ── Practical profiles ─────────────────────────────────────
        ("On Air Red".into(), Profile {
            mode: Mode::Solid, zone: Zone::Both, brightness: 100, speed: 50,
            colors: vec![(255, 0, 0)],
        }),
        ("Podcast Warm".into(), Profile {
            mode: Mode::Solid, zone: Zone::Both, brightness: 70, speed: 50,
            colors: vec![(255, 160, 40)],
        }),
        ("Cyberpunk".into(), Profile {
            mode: Mode::Wave, zone: Zone::Both, brightness: 100, speed: 60,
            colors: vec![(255, 0, 200), (0, 255, 255)],
        }),
        ("Minimal White".into(), Profile {
            mode: Mode::Pulse, zone: Zone::Both, brightness: 60, speed: 25,
            colors: vec![(255, 255, 255)],
        }),
        ("Rainbow Party".into(), Profile {
            mode: Mode::Wave, zone: Zone::Both, brightness: 100, speed: 70,
            colors: vec![
                (255, 0, 0), (255, 127, 0), (255, 255, 0), (0, 255, 0),
                (0, 0, 255), (75, 0, 130), (148, 0, 211),
            ],
        }),
    ])
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
