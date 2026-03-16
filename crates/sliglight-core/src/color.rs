//! Color math utilities for RGB animations.

use sliglight_usb::Color;

/// Linear interpolation between two colors. `t` clamped to [0.0, 1.0].
pub fn lerp(c1: Color, c2: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    (
        (c1.0 as f32 + (c2.0 as f32 - c1.0 as f32) * t) as u8,
        (c1.1 as f32 + (c2.1 as f32 - c1.1 as f32) * t) as u8,
        (c1.2 as f32 + (c2.2 as f32 - c1.2 as f32) * t) as u8,
    )
}

/// Scale a color by brightness (0-100).
pub fn apply_brightness(color: Color, brightness: u8) -> Color {
    let f = brightness as f32 / 100.0;
    (
        (color.0 as f32 * f) as u8,
        (color.1 as f32 * f) as u8,
        (color.2 as f32 * f) as u8,
    )
}

/// Build a looping gradient through a list of colors.
pub fn build_gradient(colors: &[Color], steps_per_pair: usize) -> Vec<Color> {
    let mut gradient = Vec::new();
    for i in 0..colors.len() {
        let c1 = colors[i];
        let c2 = colors[(i + 1) % colors.len()];
        for step in 0..steps_per_pair {
            gradient.push(lerp(c1, c2, step as f32 / steps_per_pair as f32));
        }
    }
    gradient
}

/// Map speed (0-100) to gradient steps per color pair.
pub fn speed_to_gradient_steps(speed: u8) -> usize {
    (128 - (speed as usize) * 116 / 100).max(12)
}

/// Flash intensity (0.0-1.0) at a given frame position.
pub fn flash_intensity(frame_num: i32, fade_in: i32, hold: i32, fade_out: i32) -> f32 {
    if frame_num < 0 {
        return 0.0;
    }
    if frame_num < fade_in {
        return frame_num as f32 / fade_in as f32;
    }
    let frame_num = frame_num - fade_in;
    if frame_num < hold {
        return 1.0;
    }
    let frame_num = frame_num - hold;
    if frame_num < fade_out {
        return 1.0 - frame_num as f32 / fade_out as f32;
    }
    0.0
}

/// Blend two frames together. `t` = 0.0 returns `a`, `t` = 1.0 returns `b`.
pub fn blend_frames(
    a: &sliglight_usb::Frame,
    b: &sliglight_usb::Frame,
    t: f32,
) -> sliglight_usb::Frame {
    let t = t.clamp(0.0, 1.0);
    sliglight_usb::Frame {
        upper: a
            .upper
            .iter()
            .zip(b.upper.iter())
            .map(|(ca, cb)| lerp(*ca, *cb, t))
            .collect(),
        lower: a
            .lower
            .iter()
            .zip(b.lower.iter())
            .map(|(ca, cb)| lerp(*ca, *cb, t))
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lerp_at_zero_returns_first_color() {
        assert_eq!(lerp((255, 0, 0), (0, 255, 0), 0.0), (255, 0, 0));
    }

    #[test]
    fn lerp_at_one_returns_second_color() {
        assert_eq!(lerp((255, 0, 0), (0, 255, 0), 1.0), (0, 255, 0));
    }

    #[test]
    fn lerp_clamps_out_of_range() {
        assert_eq!(lerp((255, 0, 0), (0, 255, 0), -1.0), (255, 0, 0));
        assert_eq!(lerp((255, 0, 0), (0, 255, 0), 2.0), (0, 255, 0));
    }

    #[test]
    fn brightness_100_is_identity() {
        assert_eq!(apply_brightness((200, 100, 50), 100), (200, 100, 50));
    }

    #[test]
    fn brightness_0_is_black() {
        assert_eq!(apply_brightness((200, 100, 50), 0), (0, 0, 0));
    }

    #[test]
    fn brightness_50_halves_values() {
        assert_eq!(apply_brightness((200, 100, 50), 50), (100, 50, 25));
    }

    #[test]
    fn gradient_loops_through_colors() {
        let colors = vec![(255, 0, 0), (0, 255, 0)];
        let g = build_gradient(&colors, 4);
        assert_eq!(g.len(), 8); // 2 pairs * 4 steps
        assert_eq!(g[0], (255, 0, 0)); // start of pair 1
        assert_eq!(g[4], (0, 255, 0)); // start of pair 2
    }

    #[test]
    fn flash_intensity_envelope() {
        // fade_in=4, hold=2, fade_out=4
        assert_eq!(flash_intensity(-1, 4, 2, 4), 0.0);
        assert_eq!(flash_intensity(0, 4, 2, 4), 0.0); // start of fade in
        assert_eq!(flash_intensity(2, 4, 2, 4), 0.5); // midway fade in
        assert_eq!(flash_intensity(4, 4, 2, 4), 1.0); // hold
        assert_eq!(flash_intensity(5, 4, 2, 4), 1.0); // still hold
        assert_eq!(flash_intensity(10, 4, 2, 4), 0.0); // after fade out
    }
}
