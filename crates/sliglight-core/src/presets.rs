//! Built-in lighting presets (8 theme + 5 practical).

use crate::animations::{Mode, Zone};
use crate::config::Profile;

/// Return all built-in presets as `(name, profile)` pairs.
pub fn all() -> Vec<(String, Profile)> {
    vec![
        // ── Theme presets ────────────────────────────────────────────
        (
            "Catppuccin Mocha".into(),
            Profile {
                mode: Mode::Wave,
                zone: Zone::Both,
                brightness: 80,
                speed: 40,
                colors: vec![
                    (203, 166, 247), // mauve
                    (137, 180, 250), // blue
                    (116, 199, 236), // sapphire
                    (166, 227, 161), // green
                    (249, 226, 175), // yellow
                    (250, 179, 135), // peach
                    (243, 139, 168), // red
                ],
            },
        ),
        (
            "Catppuccin Latte".into(),
            Profile {
                mode: Mode::Wave,
                zone: Zone::Both,
                brightness: 90,
                speed: 35,
                colors: vec![
                    (136, 57, 239),  // mauve
                    (30, 102, 245),  // blue
                    (32, 159, 181),  // sapphire
                    (64, 160, 43),   // green
                    (223, 142, 29),  // yellow
                    (254, 100, 11),  // peach
                    (210, 15, 57),   // red
                ],
            },
        ),
        (
            "Nord".into(),
            Profile {
                mode: Mode::Cycle,
                zone: Zone::Both,
                brightness: 75,
                speed: 30,
                colors: vec![
                    (136, 192, 208), // frost 1
                    (129, 161, 193), // frost 2
                    (94, 129, 172),  // frost 3
                    (163, 190, 140), // aurora green
                    (180, 142, 173), // aurora purple
                ],
            },
        ),
        (
            "Dracula".into(),
            Profile {
                mode: Mode::Wave,
                zone: Zone::Both,
                brightness: 85,
                speed: 45,
                colors: vec![
                    (189, 147, 249), // purple
                    (255, 121, 198), // pink
                    (139, 233, 253), // cyan
                    (80, 250, 123),  // green
                    (255, 184, 108), // orange
                    (241, 250, 140), // yellow
                ],
            },
        ),
        (
            "Gruvbox".into(),
            Profile {
                mode: Mode::Cycle,
                zone: Zone::Both,
                brightness: 80,
                speed: 35,
                colors: vec![
                    (251, 73, 52),   // red
                    (250, 189, 47),  // yellow
                    (184, 187, 38),  // green
                    (131, 165, 152), // aqua
                    (211, 134, 155), // purple
                    (254, 128, 25),  // orange
                ],
            },
        ),
        (
            "Rose Pine".into(),
            Profile {
                mode: Mode::Wave,
                zone: Zone::Both,
                brightness: 75,
                speed: 40,
                colors: vec![
                    (235, 188, 186), // rose
                    (196, 167, 231), // iris
                    (156, 207, 216), // foam
                    (246, 193, 119), // gold
                    (234, 154, 151), // love
                ],
            },
        ),
        (
            "Tokyo Night".into(),
            Profile {
                mode: Mode::Cycle,
                zone: Zone::Both,
                brightness: 80,
                speed: 35,
                colors: vec![
                    (122, 162, 247), // blue
                    (187, 154, 247), // purple
                    (125, 207, 255), // cyan
                    (158, 206, 106), // green
                    (224, 175, 104), // orange
                    (247, 118, 142), // red
                ],
            },
        ),
        (
            "Solarized".into(),
            Profile {
                mode: Mode::Cycle,
                zone: Zone::Both,
                brightness: 80,
                speed: 30,
                colors: vec![
                    (38, 139, 210),  // blue
                    (42, 161, 152),  // cyan
                    (133, 153, 0),   // green
                    (203, 75, 22),   // orange
                    (220, 50, 47),   // red
                    (108, 113, 196), // violet
                ],
            },
        ),
        // ── Practical presets ────────────────────────────────────────
        (
            "On Air Red".into(),
            Profile {
                mode: Mode::Solid,
                zone: Zone::Both,
                brightness: 100,
                speed: 50,
                colors: vec![(255, 0, 0)],
            },
        ),
        (
            "Podcast Warm".into(),
            Profile {
                mode: Mode::Solid,
                zone: Zone::Both,
                brightness: 70,
                speed: 50,
                colors: vec![(255, 160, 40)],
            },
        ),
        (
            "Cyberpunk".into(),
            Profile {
                mode: Mode::Wave,
                zone: Zone::Both,
                brightness: 100,
                speed: 60,
                colors: vec![
                    (255, 0, 200),  // magenta
                    (0, 255, 255),  // cyan
                ],
            },
        ),
        (
            "Minimal White".into(),
            Profile {
                mode: Mode::Pulse,
                zone: Zone::Both,
                brightness: 60,
                speed: 25,
                colors: vec![(255, 255, 255)],
            },
        ),
        (
            "Rainbow Party".into(),
            Profile {
                mode: Mode::Wave,
                zone: Zone::Both,
                brightness: 100,
                speed: 70,
                colors: vec![
                    (255, 0, 0),
                    (255, 127, 0),
                    (255, 255, 0),
                    (0, 255, 0),
                    (0, 0, 255),
                    (75, 0, 130),
                    (148, 0, 211),
                ],
            },
        ),
    ]
}
