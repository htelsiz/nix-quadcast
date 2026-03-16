//! Animation frame generators for QuadCast 2S RGB modes.
//!
//! Each animation struct holds its state and implements `next_frame()`.
//! The GUI/CLI calls `next_frame()` at ~30fps.

use serde::{Deserialize, Serialize};
use sliglight_usb::{Color, Frame, LOWER_COUNT, UPPER_COUNT};

use crate::color::{
    apply_brightness, build_gradient, flash_intensity, lerp, speed_to_gradient_steps,
};

const BLACK: Color = (0, 0, 0);

pub const RAINBOW: &[Color] = &[
    (255, 0, 0),
    (255, 0, 158),
    (205, 0, 255),
    (43, 0, 255),
    (0, 104, 255),
    (0, 255, 255),
    (0, 255, 103),
    (50, 255, 0),
    (206, 255, 0),
];

/// All supported animation modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Mode {
    Solid,
    Blink,
    Cycle,
    Wave,
    Lightning,
    Pulse,
    AudioReactive,
}

impl Mode {
    pub const ALL: &[Mode] = &[
        Mode::Solid,
        Mode::Blink,
        Mode::Cycle,
        Mode::Wave,
        Mode::Lightning,
        Mode::Pulse,
        Mode::AudioReactive,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            Mode::Solid => "solid",
            Mode::Blink => "blink",
            Mode::Cycle => "cycle",
            Mode::Wave => "wave",
            Mode::Lightning => "lightning",
            Mode::Pulse => "pulse",
            Mode::AudioReactive => "audio",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Mode::Solid => "\u{25cf}",
            Mode::Blink => "\u{25cc}",
            Mode::Cycle => "\u{1f308}",
            Mode::Wave => "\u{2248}",
            Mode::Lightning => "\u{26a1}",
            Mode::Pulse => "\u{2665}",
            Mode::AudioReactive => "\u{1f3a4}",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Mode::Solid => "Static color",
            Mode::Blink => "Blinking effect",
            Mode::Cycle => "Rainbow cycle",
            Mode::Wave => "Wave animation",
            Mode::Lightning => "Lightning strikes",
            Mode::Pulse => "Pulsing glow",
            Mode::AudioReactive => "VU meter reacting to mic input",
        }
    }

    pub fn from_name(s: &str) -> Option<Self> {
        match s {
            "solid" => Some(Mode::Solid),
            "blink" => Some(Mode::Blink),
            "cycle" => Some(Mode::Cycle),
            "wave" => Some(Mode::Wave),
            "lightning" => Some(Mode::Lightning),
            "pulse" => Some(Mode::Pulse),
            "audio" => Some(Mode::AudioReactive),
            _ => None,
        }
    }
}

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// Zone selection for LED masking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Zone {
    Both,
    Upper,
    Lower,
}

/// Stateful animation that produces frames.
pub struct Animation {
    mode: Mode,
    colors: Vec<Color>,
    brightness: u8,
    speed: u8,
    zone: Zone,
    // Internal state
    frame_num: usize,
    gradient: Vec<Color>,
    color_idx: usize,
}

impl Animation {
    pub fn new(
        mode: Mode,
        colors: Vec<Color>,
        brightness: u8,
        speed: u8,
        zone: Zone,
    ) -> Self {
        let effective_colors = if colors.is_empty() {
            vec![(255, 0, 0)]
        } else {
            colors
        };

        let gradient = match mode {
            Mode::Cycle | Mode::Wave => {
                let palette = if effective_colors.len() < 2 {
                    RAINBOW.to_vec()
                } else {
                    effective_colors.clone()
                };
                let steps = speed_to_gradient_steps(speed);
                build_gradient(&palette, steps)
                    .into_iter()
                    .map(|c| apply_brightness(c, brightness))
                    .collect()
            }
            _ => Vec::new(),
        };

        Self {
            mode,
            colors: effective_colors,
            brightness,
            speed,
            zone,
            frame_num: 0,
            gradient,
            color_idx: 0,
        }
    }

    /// Generate the next animation frame.
    pub fn next_frame(&mut self) -> Frame {
        let raw = match self.mode {
            Mode::Solid => self.solid(),
            Mode::Blink => self.blink(),
            Mode::Cycle => self.cycle(),
            Mode::Wave => self.wave(),
            Mode::Lightning => self.lightning(),
            Mode::Pulse => self.pulse(),
            // AudioReactive frames are built by the engine from peak level.
            // Fallback: solid at current brightness.
            Mode::AudioReactive => self.solid(),
        };
        self.apply_zone_mask(raw)
    }

    /// Build a VU-meter frame from a peak level (0.0–1.0) and the current colors/brightness.
    pub fn audio_reactive_frame(&self, peak: f32) -> Frame {
        let peak = peak.clamp(0.0, 1.0);
        let total_leds = UPPER_COUNT + LOWER_COUNT;
        let lit_count = (peak * total_leds as f32).round() as usize;
        let base_color = apply_brightness(self.colors[0], self.brightness);

        // Lower LEDs light up first, then upper.
        let lower: Vec<Color> = (0..LOWER_COUNT)
            .map(|i| if i < lit_count.min(LOWER_COUNT) { base_color } else { BLACK })
            .collect();
        let upper: Vec<Color> = (0..UPPER_COUNT)
            .map(|i| {
                let global_i = LOWER_COUNT + i;
                if global_i < lit_count { base_color } else { BLACK }
            })
            .collect();

        self.apply_zone_mask(Frame { upper, lower })
    }

    fn solid(&self) -> Frame {
        let c = apply_brightness(self.colors[0], self.brightness);
        Frame::uniform(c)
    }

    fn blink(&mut self) -> Frame {
        let on_frames = ((101 - self.speed as usize) * 4 / 10).max(1);
        let off_frames = (on_frames * 4 / 10).max(1);
        let total = on_frames + off_frames;
        let pos = self.frame_num % total;

        let frame = if pos < on_frames {
            let c = apply_brightness(
                self.colors[self.color_idx % self.colors.len()],
                self.brightness,
            );
            Frame::uniform(c)
        } else {
            Frame::uniform(BLACK)
        };

        self.frame_num += 1;
        if self.frame_num % total == 0 {
            self.color_idx += 1;
        }
        frame
    }

    fn cycle(&mut self) -> Frame {
        let total = self.gradient.len();
        if total == 0 {
            return Frame::uniform(BLACK);
        }
        let c = self.gradient[self.frame_num % total];
        self.frame_num += 1;
        Frame::uniform(c)
    }

    fn wave(&mut self) -> Frame {
        let total = self.gradient.len();
        if total == 0 {
            return Frame::uniform(BLACK);
        }
        let offset = total / 2;
        let upper_c = self.gradient[self.frame_num % total];
        let lower_c = self.gradient[(self.frame_num + offset) % total];
        self.frame_num += 1;
        Frame {
            upper: vec![upper_c; UPPER_COUNT],
            lower: vec![lower_c; LOWER_COUNT],
        }
    }

    fn lightning(&mut self) -> Frame {
        let fade_in = (10i32 - self.speed as i32 * 7 / 100).max(2);
        let hold = (3i32 - self.speed as i32 * 2 / 100).max(1);
        let fade_out = (40i32 - self.speed as i32 * 35 / 100).max(5);
        let pause = (20i32 - self.speed as i32 * 17 / 100).max(3);
        let lower_delay = (fade_in / 2).max(1);
        let total_cycle = (fade_in + hold + fade_out + lower_delay + pause) as usize;

        let pos = (self.frame_num % total_cycle) as i32;
        let c = apply_brightness(
            self.colors[self.color_idx % self.colors.len()],
            self.brightness,
        );

        let upper_t = flash_intensity(pos, fade_in, hold, fade_out);
        let lower_t = flash_intensity(pos - lower_delay, fade_in, hold, fade_out);

        let frame = Frame {
            upper: vec![lerp(BLACK, c, upper_t); UPPER_COUNT],
            lower: vec![lerp(BLACK, c, lower_t); LOWER_COUNT],
        };

        self.frame_num += 1;
        if self.frame_num % total_cycle == 0 {
            self.color_idx += 1;
        }
        frame
    }

    fn pulse(&mut self) -> Frame {
        let fade_in = (10i32 - self.speed as i32 * 7 / 100).max(2);
        let hold = (3i32 - self.speed as i32 * 2 / 100).max(1);
        let fade_out = (40i32 - self.speed as i32 * 35 / 100).max(5);
        let pause = (20i32 - self.speed as i32 * 17 / 100).max(3);
        let total_cycle = (fade_in + hold + fade_out + pause) as usize;

        let pos = (self.frame_num % total_cycle) as i32;
        let c = apply_brightness(
            self.colors[self.color_idx % self.colors.len()],
            self.brightness,
        );
        let t = flash_intensity(pos, fade_in, hold, fade_out);

        let frame = Frame::uniform(lerp(BLACK, c, t));

        self.frame_num += 1;
        if self.frame_num % total_cycle == 0 {
            self.color_idx += 1;
        }
        frame
    }

    fn apply_zone_mask(&self, frame: Frame) -> Frame {
        match self.zone {
            Zone::Both => frame,
            Zone::Upper => Frame {
                upper: frame.upper,
                lower: vec![BLACK; LOWER_COUNT],
            },
            Zone::Lower => Frame {
                upper: vec![BLACK; UPPER_COUNT],
                lower: frame.lower,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solid_produces_uniform_frame() {
        let mut anim = Animation::new(Mode::Solid, vec![(255, 0, 0)], 100, 50, Zone::Both);
        let f = anim.next_frame();
        assert!(f.upper.iter().all(|c| *c == (255, 0, 0)));
        assert!(f.lower.iter().all(|c| *c == (255, 0, 0)));
    }

    #[test]
    fn zone_upper_blacks_out_lower() {
        let mut anim = Animation::new(Mode::Solid, vec![(255, 0, 0)], 100, 50, Zone::Upper);
        let f = anim.next_frame();
        assert!(f.upper.iter().all(|c| *c == (255, 0, 0)));
        assert!(f.lower.iter().all(|c| *c == BLACK));
    }

    #[test]
    fn zone_lower_blacks_out_upper() {
        let mut anim = Animation::new(Mode::Solid, vec![(255, 0, 0)], 100, 50, Zone::Lower);
        let f = anim.next_frame();
        assert!(f.upper.iter().all(|c| *c == BLACK));
        assert!(f.lower.iter().all(|c| *c == (255, 0, 0)));
    }

    #[test]
    fn blink_alternates_on_off() {
        let mut anim = Animation::new(Mode::Blink, vec![(255, 0, 0)], 100, 50, Zone::Both);
        let mut saw_on = false;
        let mut saw_off = false;
        for _ in 0..100 {
            let f = anim.next_frame();
            if f.upper[0] == (255, 0, 0) {
                saw_on = true;
            }
            if f.upper[0] == BLACK {
                saw_off = true;
            }
        }
        assert!(saw_on && saw_off);
    }

    #[test]
    fn cycle_with_single_color_uses_rainbow() {
        let mut anim = Animation::new(Mode::Cycle, vec![(255, 0, 0)], 100, 50, Zone::Both);
        let f1 = anim.next_frame();
        for _ in 0..50 {
            anim.next_frame();
        }
        let f2 = anim.next_frame();
        assert_ne!(f1.upper[0], f2.upper[0]);
    }

    #[test]
    fn mode_from_name_roundtrip() {
        for mode in Mode::ALL {
            assert_eq!(Mode::from_name(mode.name()), Some(*mode));
        }
        assert_eq!(Mode::from_name("garbage"), None);
    }
}
