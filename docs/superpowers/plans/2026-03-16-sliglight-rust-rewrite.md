# Sliglight Rust Rewrite Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rewrite the Sliglight QuadCast RGB GUI from Python/PySide6 to Rust/iced, replacing the C CLI with a native Rust CLI, and killing all Python code and C patches.

**Architecture:** Cargo workspace with 3 crates: `sliglight-usb` (rusb device protocol), `sliglight-core` (animation engine + color math + CLI binary), `sliglight-gui` (iced GUI). The USB crate is the foundation, core builds animations on top, and the GUI consumes both. A `sliglight-cli` binary in the core crate replaces the upstream C `quadcastrgb`.

**Tech Stack:** Rust 2021 edition, iced 0.14 (GUI, has built-in CatppuccinMocha theme), rusb 0.9 (USB), clap (CLI), tokio (async runtime for iced subscription), env_logger + log (logging)

---

## Chunk 1: Project Scaffolding + USB Crate

### Task 1: Cargo Workspace Scaffolding

**Files:**
- Create: `Cargo.toml` (workspace root)
- Create: `crates/sliglight-usb/Cargo.toml`
- Create: `crates/sliglight-usb/src/lib.rs`
- Create: `crates/sliglight-core/Cargo.toml`
- Create: `crates/sliglight-core/src/lib.rs`
- Create: `crates/sliglight-gui/Cargo.toml`
- Create: `crates/sliglight-gui/src/main.rs`

- [ ] **Step 1: Create workspace Cargo.toml**

```toml
[workspace]
resolver = "2"
members = [
    "crates/sliglight-usb",
    "crates/sliglight-core",
    "crates/sliglight-gui",
]

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "GPL-2.0-only"
repository = "https://github.com/htelsiz/nix-quadcast"

[workspace.dependencies]
log = "0.4"
env_logger = "0.11"
thiserror = "2"
```

- [ ] **Step 2: Create sliglight-usb crate**

`crates/sliglight-usb/Cargo.toml`:
```toml
[package]
name = "sliglight-usb"
version.workspace = true
edition.workspace = true
license.workspace = true
description = "USB protocol for HyperX QuadCast 2S RGB LEDs"

[dependencies]
rusb = "0.9"
log.workspace = true
thiserror.workspace = true
```

`crates/sliglight-usb/src/lib.rs`:
```rust
//! USB protocol for HyperX QuadCast 2S RGB LED control.

mod device;
mod error;
mod protocol;

pub use device::QuadCast2S;
pub use error::UsbError;
pub use protocol::{Frame, Color, UPPER_COUNT, LOWER_COUNT, TOTAL_LEDS};
```

- [ ] **Step 3: Create sliglight-core crate**

`crates/sliglight-core/Cargo.toml`:
```toml
[package]
name = "sliglight-core"
version.workspace = true
edition.workspace = true
license.workspace = true
description = "Animation engine and CLI for Sliglight RGB control"

[[bin]]
name = "sliglight-cli"
path = "src/bin/cli.rs"

[dependencies]
sliglight-usb = { path = "../sliglight-usb" }
log.workspace = true
env_logger.workspace = true
clap = { version = "4", features = ["derive"] }
```

`crates/sliglight-core/src/lib.rs`:
```rust
//! Animation engine for Sliglight RGB modes.

pub mod animations;
pub mod color;
```

- [ ] **Step 4: Create sliglight-gui crate**

`crates/sliglight-gui/Cargo.toml`:
```toml
[package]
name = "sliglight-gui"
version.workspace = true
edition.workspace = true
license.workspace = true
description = "Sliglight — iced GUI for HyperX QuadCast RGB control"

[[bin]]
name = "sliglight"
path = "src/main.rs"

[dependencies]
sliglight-usb = { path = "../sliglight-usb" }
sliglight-core = { path = "../sliglight-core" }
iced = { version = "0.14", features = ["canvas", "tokio", "svg", "image"] }
tokio = { version = "1", features = ["time"] }
log.workspace = true
env_logger.workspace = true
```

`crates/sliglight-gui/src/main.rs`:
```rust
fn main() {
    println!("sliglight gui placeholder");
}
```

- [ ] **Step 5: Verify workspace compiles**

Run: `cargo check`
Expected: compiles with no errors

- [ ] **Step 6: Commit scaffolding**

```bash
git add Cargo.toml crates/
git commit -m "feat: cargo workspace scaffolding for Rust rewrite"
```

---

### Task 2: USB Protocol Types

**Files:**
- Create: `crates/sliglight-usb/src/protocol.rs`
- Create: `crates/sliglight-usb/src/error.rs`

- [ ] **Step 1: Write protocol types**

`crates/sliglight-usb/src/protocol.rs`:
```rust
//! Frame and color types for the QuadCast 2S LED protocol.

/// RGB color tuple.
pub type Color = (u8, u8, u8);

pub const UPPER_COUNT: usize = 54;
pub const LOWER_COUNT: usize = 54;
pub const TOTAL_LEDS: usize = UPPER_COUNT + LOWER_COUNT;

pub(crate) const PKT_SIZE: usize = 64;
pub(crate) const LEDS_PER_PKT: usize = 20;
pub(crate) const NUM_DATA_PKTS: usize = 6;

pub(crate) const HEADER_CMD: u8 = 0x44;
pub(crate) const HEADER_SUB: u8 = 0x01;
pub(crate) const DATA_CMD: u8 = 0x44;
pub(crate) const DATA_SUB: u8 = 0x02;
pub(crate) const PKT_COUNT_CODE: u8 = 0x06;

/// One animation frame with separate upper and lower LED arrays.
#[derive(Debug, Clone)]
pub struct Frame {
    pub upper: Vec<Color>,
    pub lower: Vec<Color>,
}

impl Frame {
    pub fn uniform(color: Color) -> Self {
        Self {
            upper: vec![color; UPPER_COUNT],
            lower: vec![color; LOWER_COUNT],
        }
    }

    /// All 108 LEDs as a flat slice (upper first, then lower).
    pub fn flat(&self) -> Vec<Color> {
        let mut out = Vec::with_capacity(TOTAL_LEDS);
        out.extend_from_slice(&self.upper);
        out.extend_from_slice(&self.lower);
        out
    }

    /// Build the raw USB packets for this frame (header + 6 data packets).
    pub(crate) fn to_packets(&self) -> Vec<[u8; PKT_SIZE]> {
        let leds = self.flat();
        let mut packets = Vec::with_capacity(1 + NUM_DATA_PKTS);

        // Header: 44 01 06 00 [zeros]
        let mut header = [0u8; PKT_SIZE];
        header[0] = HEADER_CMD;
        header[1] = HEADER_SUB;
        header[2] = PKT_COUNT_CODE;
        packets.push(header);

        // 6 data packets, 20 LEDs each
        let mut led_idx = 0;
        for pkt_num in 0..NUM_DATA_PKTS {
            let mut data = [0u8; PKT_SIZE];
            data[0] = DATA_CMD;
            data[1] = DATA_SUB;
            data[2] = pkt_num as u8;

            let mut offset = 4;
            for _ in 0..LEDS_PER_PKT {
                if led_idx < TOTAL_LEDS {
                    let (r, g, b) = leds[led_idx];
                    data[offset] = r;
                    data[offset + 1] = g;
                    data[offset + 2] = b;
                    led_idx += 1;
                }
                offset += 3;
            }
            packets.push(data);
        }

        packets
    }
}
```

- [ ] **Step 2: Write error types**

`crates/sliglight-usb/src/error.rs`:
```rust
//! Error types for USB device communication.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum UsbError {
    #[error("QuadCast 2S not found on USB bus")]
    DeviceNotFound,

    #[error("USB interface claimed by another process (stop quadcast-rgb.service first)")]
    DeviceBusy,

    #[error("USB transfer failed on EP 0x{endpoint:02x}: {message}")]
    Transfer {
        endpoint: u8,
        message: String,
    },

    #[error("USB error: {0}")]
    Usb(#[from] rusb::Error),
}
```

- [ ] **Step 3: Write tests for Frame packet building**

Add to bottom of `crates/sliglight-usb/src/protocol.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uniform_frame_has_correct_led_count() {
        let frame = Frame::uniform((255, 0, 0));
        assert_eq!(frame.upper.len(), UPPER_COUNT);
        assert_eq!(frame.lower.len(), LOWER_COUNT);
        assert_eq!(frame.flat().len(), TOTAL_LEDS);
    }

    #[test]
    fn to_packets_produces_7_packets() {
        let frame = Frame::uniform((255, 0, 0));
        let packets = frame.to_packets();
        assert_eq!(packets.len(), 7); // 1 header + 6 data
    }

    #[test]
    fn header_packet_format() {
        let frame = Frame::uniform((0, 0, 0));
        let packets = frame.to_packets();
        let h = &packets[0];
        assert_eq!(h[0], 0x44);
        assert_eq!(h[1], 0x01);
        assert_eq!(h[2], 0x06);
        assert_eq!(h[3], 0x00);
    }

    #[test]
    fn data_packet_carries_rgb() {
        let frame = Frame::uniform((0xAA, 0xBB, 0xCC));
        let packets = frame.to_packets();
        let d = &packets[1]; // first data packet
        assert_eq!(d[0], 0x44);
        assert_eq!(d[1], 0x02);
        assert_eq!(d[2], 0x00); // packet index 0
        // First LED at offset 4
        assert_eq!(d[4], 0xAA);
        assert_eq!(d[5], 0xBB);
        assert_eq!(d[6], 0xCC);
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p sliglight-usb`
Expected: all 4 tests pass

- [ ] **Step 5: Commit**

```bash
git add crates/sliglight-usb/
git commit -m "feat(usb): protocol types, frame packet encoding, error hierarchy"
```

---

### Task 3: USB Device Driver

**Files:**
- Create: `crates/sliglight-usb/src/device.rs`

- [ ] **Step 1: Implement QuadCast2S device driver**

`crates/sliglight-usb/src/device.rs`:
```rust
//! Direct USB communication with HyperX QuadCast 2S.

use std::time::Duration;

use log::info;
use rusb::{DeviceHandle, GlobalContext};

use crate::error::UsbError;
use crate::protocol::{Frame, PKT_SIZE};

const VID: u16 = 0x03F0; // HP (HyperX)
const PID: u16 = 0x02B5; // QuadCast 2S
const IFACE: u8 = 1;
const EP_OUT: u8 = 0x06;
const EP_IN: u8 = 0x85;
const TIMEOUT: Duration = Duration::from_secs(1);

/// Direct USB interface to QuadCast 2S RGB LEDs.
pub struct QuadCast2S {
    handle: DeviceHandle<GlobalContext>,
}

impl QuadCast2S {
    /// Find and open the QuadCast 2S, claim interface 1.
    pub fn open() -> Result<Self, UsbError> {
        let handle = rusb::open_device_with_vid_pid(VID, PID)
            .ok_or(UsbError::DeviceNotFound)?;

        // Detach kernel driver if active
        if handle.kernel_driver_active(IFACE).unwrap_or(false) {
            handle.detach_kernel_driver(IFACE)?;
        }

        handle.claim_interface(IFACE).map_err(|e| {
            if e == rusb::Error::Busy {
                UsbError::DeviceBusy
            } else {
                UsbError::Usb(e)
            }
        })?;

        info!("opened QuadCast 2S (VID={VID:#06x} PID={PID:#06x})");
        Ok(Self { handle })
    }

    /// Send a full 108-LED frame to the device.
    pub fn send_frame(&self, frame: &Frame) -> Result<(), UsbError> {
        for packet in frame.to_packets() {
            self.send_recv(&packet)?;
        }
        Ok(())
    }

    fn send_recv(&self, packet: &[u8; PKT_SIZE]) -> Result<(), UsbError> {
        self.handle
            .write_interrupt(EP_OUT, packet, TIMEOUT)
            .map_err(|e| UsbError::Transfer {
                endpoint: EP_OUT,
                message: e.to_string(),
            })?;

        let mut buf = [0u8; PKT_SIZE];
        self.handle
            .read_interrupt(EP_IN, &mut buf, TIMEOUT)
            .map_err(|e| UsbError::Transfer {
                endpoint: EP_IN,
                message: format!("ACK read failed: {e}"),
            })?;

        Ok(())
    }
}

impl Drop for QuadCast2S {
    fn drop(&mut self) {
        let _ = self.handle.release_interface(IFACE);
        info!("closed QuadCast 2S");
    }
}
```

- [ ] **Step 2: Update lib.rs exports**

`crates/sliglight-usb/src/lib.rs`:
```rust
//! USB protocol for HyperX QuadCast 2S RGB LED control.

mod device;
mod error;
mod protocol;

pub use device::QuadCast2S;
pub use error::UsbError;
pub use protocol::{Color, Frame, LOWER_COUNT, TOTAL_LEDS, UPPER_COUNT};
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check -p sliglight-usb`
Expected: compiles cleanly

- [ ] **Step 4: Commit**

```bash
git add crates/sliglight-usb/
git commit -m "feat(usb): QuadCast2S device driver with rusb"
```

---

## Chunk 2: Animation Core + CLI

### Task 4: Color Math

**Files:**
- Create: `crates/sliglight-core/src/color.rs`

- [ ] **Step 1: Write color utility functions with tests**

`crates/sliglight-core/src/color.rs`:
```rust
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
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p sliglight-core`
Expected: all 8 tests pass

- [ ] **Step 3: Commit**

```bash
git add crates/sliglight-core/
git commit -m "feat(core): color math with lerp, brightness, gradient, flash"
```

---

### Task 5: Animation Generators

**Files:**
- Create: `crates/sliglight-core/src/animations.rs`

- [ ] **Step 1: Implement all 6 animation modes**

`crates/sliglight-core/src/animations.rs`:
```rust
//! Animation frame generators for QuadCast 2S RGB modes.
//!
//! Each animation struct holds its state and implements `next_frame()`.
//! The GUI/CLI calls `next_frame()` at ~30fps.

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mode {
    Solid,
    Blink,
    Cycle,
    Wave,
    Lightning,
    Pulse,
}

impl Mode {
    pub const ALL: &[Mode] = &[
        Mode::Solid,
        Mode::Blink,
        Mode::Cycle,
        Mode::Wave,
        Mode::Lightning,
        Mode::Pulse,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            Mode::Solid => "solid",
            Mode::Blink => "blink",
            Mode::Cycle => "cycle",
            Mode::Wave => "wave",
            Mode::Lightning => "lightning",
            Mode::Pulse => "pulse",
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
        };
        self.apply_zone_mask(raw)
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
        // Advance many frames to see color change
        for _ in 0..50 {
            anim.next_frame();
        }
        let f2 = anim.next_frame();
        assert_ne!(f1.upper[0], f2.upper[0]); // colors should differ
    }

    #[test]
    fn mode_from_name_roundtrip() {
        for mode in Mode::ALL {
            assert_eq!(Mode::from_name(mode.name()), Some(*mode));
        }
        assert_eq!(Mode::from_name("garbage"), None);
    }
}
```

- [ ] **Step 2: Update lib.rs**

`crates/sliglight-core/src/lib.rs`:
```rust
//! Animation engine for Sliglight RGB modes.

pub mod animations;
pub mod color;
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p sliglight-core`
Expected: all 14 tests pass (8 color + 6 animation)

- [ ] **Step 4: Commit**

```bash
git add crates/sliglight-core/
git commit -m "feat(core): 6 animation modes with zone masking"
```

---

### Task 6: CLI Binary

**Files:**
- Create: `crates/sliglight-core/src/bin/cli.rs`

- [ ] **Step 1: Implement CLI**

`crates/sliglight-core/src/bin/cli.rs`:
```rust
//! Sliglight CLI — drop-in replacement for quadcastrgb.
//!
//! Usage: sliglight-cli solid ff0000
//!        sliglight-cli cycle
//!        sliglight-cli blink ff0000 00ff00 --speed 80 --brightness 60

use std::process;
use std::thread;
use std::time::Duration;

use clap::Parser;
use sliglight_core::animations::{Animation, Mode, Zone};
use sliglight_usb::{Color, QuadCast2S};

const TARGET_FPS: u64 = 30;
const FRAME_INTERVAL: Duration = Duration::from_millis(1000 / TARGET_FPS);

#[derive(Parser)]
#[command(name = "sliglight-cli", about = "RGB control for HyperX QuadCast")]
struct Cli {
    /// Animation mode: solid, blink, cycle, wave, lightning, pulse
    mode: String,

    /// Hex colors (e.g. ff0000 00ff00)
    #[arg(value_parser = parse_hex_color)]
    colors: Vec<Color>,

    /// Brightness 0-100
    #[arg(short, long, default_value_t = 100)]
    brightness: u8,

    /// Speed 0-100
    #[arg(short, long, default_value_t = 81)]
    speed: u8,
}

fn parse_hex_color(s: &str) -> Result<Color, String> {
    let s = s.strip_prefix('#').unwrap_or(s);
    if s.len() != 6 {
        return Err(format!("expected 6 hex digits, got '{s}'"));
    }
    let r = u8::from_str_radix(&s[0..2], 16).map_err(|e| e.to_string())?;
    let g = u8::from_str_radix(&s[2..4], 16).map_err(|e| e.to_string())?;
    let b = u8::from_str_radix(&s[4..6], 16).map_err(|e| e.to_string())?;
    Ok((r, g, b))
}

fn main() {
    env_logger::init();
    let cli = Cli::parse();

    let mode = Mode::from_name(&cli.mode).unwrap_or_else(|| {
        eprintln!("unknown mode '{}'. options: solid, blink, cycle, wave, lightning, pulse", cli.mode);
        process::exit(1);
    });

    let device = QuadCast2S::open().unwrap_or_else(|e| {
        eprintln!("error: {e}");
        process::exit(1);
    });

    let mut anim = Animation::new(mode, cli.colors, cli.brightness, cli.speed, Zone::Both);

    loop {
        let frame = anim.next_frame();
        if let Err(e) = device.send_frame(&frame) {
            eprintln!("USB error: {e}");
            process::exit(1);
        }
        thread::sleep(FRAME_INTERVAL);
    }
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo build -p sliglight-core --bin sliglight-cli`
Expected: compiles cleanly

- [ ] **Step 3: Commit**

```bash
git add crates/sliglight-core/
git commit -m "feat(cli): sliglight-cli replaces C quadcastrgb"
```

---

## Chunk 3: iced GUI

### Task 7: GUI App Shell + Catppuccin Theme

**Files:**
- Create: `crates/sliglight-gui/src/main.rs`

> **Note:** iced 0.14 has a built-in `Theme::CatppuccinMocha` — no manual theme file needed.

- [ ] **Step 1: Create GUI app shell with Elm architecture**

`crates/sliglight-gui/src/main.rs`:
```rust
//! Sliglight — iced GUI for HyperX QuadCast RGB control.

mod mic_preview;
mod engine;

use iced::widget::{button, column, container, horizontal_space, row, slider, text, Column, Row};
use iced::{Element, Length, Subscription, Task, Theme};

use sliglight_core::animations::{Mode, Zone};

fn main() -> iced::Result {
    env_logger::init();
    iced::application(App::new, App::update, App::view)
        .title("Sliglight")
        .theme(|_app| Theme::CatppuccinMocha)
        .subscription(App::subscription)
        .window_size((680.0, 750.0))
        .run()
}

struct App {
    zone: Zone,
    mode: Mode,
    brightness: u8,
    speed: u8,
    colors: Vec<(u8, u8, u8)>,
    status: Status,
    engine: Option<engine::Handle>,
}

enum Status {
    Idle,
    Connected,
    Error(String),
}

#[derive(Debug, Clone)]
enum Message {
    SetZone(Zone),
    SetMode(Mode),
    SetBrightness(u8),
    SetSpeed(u8),
    AddColor,
    RemoveColor(usize),
    SetColor(usize, (u8, u8, u8)),
    Apply,
    Reset,
    EngineEvent(engine::Event),
}

impl App {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                zone: Zone::Both,
                mode: Mode::Solid,
                brightness: 80,
                speed: 81,
                colors: vec![(255, 0, 0)],
                status: Status::Idle,
                engine: None,
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SetZone(z) => self.zone = z,
            Message::SetMode(m) => self.mode = m,
            Message::SetBrightness(b) => self.brightness = b,
            Message::SetSpeed(s) => self.speed = s,
            Message::AddColor => {
                if self.colors.len() < 11 {
                    let cycle = [
                        (0, 255, 0), (0, 0, 255), (255, 255, 0), (0, 255, 255),
                        (255, 0, 255), (255, 136, 0), (136, 255, 0), (255, 0, 136),
                        (0, 136, 255), (136, 0, 255),
                    ];
                    let c = cycle[(self.colors.len() - 1) % cycle.len()];
                    self.colors.push(c);
                }
            }
            Message::RemoveColor(i) => {
                if self.colors.len() > 1 && i < self.colors.len() {
                    self.colors.remove(i);
                }
            }
            Message::SetColor(i, c) => {
                if i < self.colors.len() {
                    self.colors[i] = c;
                }
            }
            Message::Apply => {
                self.engine = Some(engine::Handle::start(
                    self.mode,
                    self.colors.clone(),
                    self.brightness,
                    self.speed,
                    self.zone,
                ));
                self.status = Status::Idle;
            }
            Message::Reset => {
                self.zone = Zone::Both;
                self.mode = Mode::Solid;
                self.brightness = 80;
                self.speed = 81;
                self.colors = vec![(255, 0, 0)];
                self.engine = None;
                self.status = Status::Idle;
            }
            Message::EngineEvent(e) => match e {
                engine::Event::Connected => self.status = Status::Connected,
                engine::Event::Error(msg) => self.status = Status::Error(msg),
                engine::Event::FrameSent { .. } => {}
            },
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        if let Some(handle) = &self.engine {
            handle.subscription().map(Message::EngineEvent)
        } else {
            Subscription::none()
        }
    }

    fn view(&self) -> Element<Message> {
        // TODO: build full UI in Task 8
        let content = column![
            text("Sliglight").size(24),
            text("GUI coming next task..."),
        ]
        .spacing(20)
        .padding(20);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}
```

- [ ] **Step 2: Create engine module (background USB animation via iced Subscription)**

> **API note:** `Subscription::run` takes a `fn() -> Stream` function pointer (not a closure).
> `Subscription::run_with` takes `(data: D, fn(&D) -> Stream)` where D: Hash.
> We pack animation params into a hashable `EngineConfig` struct and pass it as data.

`crates/sliglight-gui/src/engine.rs`:
```rust
//! Background animation engine using iced subscriptions.

use std::hash::{Hash, Hasher};
use std::time::Duration;

use iced::futures::{SinkExt, Stream};
use iced::Subscription;
use sliglight_core::animations::{Animation, Mode, Zone};
use sliglight_usb::{Color, QuadCast2S};

const TARGET_FPS: u64 = 30;
const FRAME_INTERVAL: Duration = Duration::from_millis(1000 / TARGET_FPS);

#[derive(Debug, Clone)]
pub enum Event {
    Connected,
    Error(String),
    FrameSent { upper: Color, lower: Color },
}

/// Hashable configuration for the animation subscription.
/// When any field changes, iced restarts the subscription.
#[derive(Clone)]
pub struct EngineConfig {
    pub mode: Mode,
    pub colors: Vec<Color>,
    pub brightness: u8,
    pub speed: u8,
    pub zone: Zone,
}

impl Hash for EngineConfig {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.mode.hash(state);
        self.brightness.hash(state);
        self.speed.hash(state);
        self.zone.hash(state);
        for c in &self.colors {
            c.hash(state);
        }
    }
}

pub struct Handle {
    pub config: EngineConfig,
}

impl Handle {
    pub fn start(
        mode: Mode,
        colors: Vec<Color>,
        brightness: u8,
        speed: u8,
        zone: Zone,
    ) -> Self {
        // Stop systemd service before claiming USB
        let _ = std::process::Command::new("systemctl")
            .args(["--user", "stop", "quadcast-rgb.service"])
            .output();

        Self {
            config: EngineConfig { mode, colors, brightness, speed, zone },
        }
    }

    pub fn subscription(&self) -> Subscription<Event> {
        Subscription::run_with(self.config.clone(), animation_stream)
    }
}

fn animation_stream(config: &EngineConfig) -> impl Stream<Item = Event> {
    let mode = config.mode;
    let colors = config.colors.clone();
    let brightness = config.brightness;
    let speed = config.speed;
    let zone = config.zone;

    iced::futures::stream::channel(32, move |mut output| async move {
        let device = match QuadCast2S::open() {
            Ok(d) => {
                let _ = output.send(Event::Connected).await;
                d
            }
            Err(e) => {
                let _ = output.send(Event::Error(e.to_string())).await;
                loop {
                    tokio::time::sleep(Duration::from_secs(60)).await;
                }
            }
        };

        let mut anim = Animation::new(mode, colors, brightness, speed, zone);

        loop {
            let frame = anim.next_frame();
            let upper = frame.upper.first().copied().unwrap_or((0, 0, 0));
            let lower = frame.lower.first().copied().unwrap_or((0, 0, 0));

            if let Err(e) = device.send_frame(&frame) {
                let _ = output.send(Event::Error(e.to_string())).await;
                break;
            }

            let _ = output.send(Event::FrameSent { upper, lower }).await;
            tokio::time::sleep(FRAME_INTERVAL).await;
        }

        // Restart systemd service on exit
        let _ = std::process::Command::new("systemctl")
            .args(["--user", "start", "quadcast-rgb.service"])
            .output();

        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    })
}
```

- [ ] **Step 3: Create placeholder mic_preview module**

`crates/sliglight-gui/src/mic_preview.rs`:
```rust
//! Mic preview canvas widget — implemented in Task 9.

// Placeholder — will be implemented in Task 9
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo check -p sliglight-gui`
Expected: compiles (may have warnings about unused imports, that's fine)

- [ ] **Step 5: Commit**

```bash
git add crates/sliglight-gui/
git commit -m "feat(gui): iced app shell with Catppuccin theme and animation engine"
```

---

### Task 8: Full GUI Layout

**Files:**
- Modify: `crates/sliglight-gui/src/main.rs` (replace `view()` method)

- [ ] **Step 1: Replace the view method with full UI**

Replace the `view()` method in `main.rs` with the full layout: zone selector (3 radio buttons), mode grid (2x3), brightness/speed sliders, color swatches with add/remove, apply/reset buttons, status text. Use iced's built-in `button`, `slider`, `text`, `row`, `column`, `container` widgets. Style buttons with custom appearance based on selected state. The `Theme::CatppuccinMocha` built-in theme handles all widget colors automatically.

The view should mirror the Python GUI layout:
- Left side: mic preview canvas (placeholder container for now)
- Right side: stacked cards with Zone, Mode, Settings, Colors, Actions, Status

Use helper functions for each card section to keep the view method clean.

- [ ] **Step 2: Verify it compiles and runs**

Run: `cargo run -p sliglight-gui`
Expected: window opens with the full control layout (mic preview is placeholder)

- [ ] **Step 3: Commit**

```bash
git add crates/sliglight-gui/
git commit -m "feat(gui): full control layout with zone, mode, sliders, colors"
```

---

### Task 9: Mic Preview Canvas Widget

**Files:**
- Modify: `crates/sliglight-gui/src/mic_preview.rs`

- [ ] **Step 1: Implement Canvas-based mic preview**

Port the Python `MicPreview.paintEvent()` to an iced `Canvas` widget program. Draw:
- Mic body (rounded rect, dark fill)
- Upper LED zone (rounded rect with glow gradient)
- Lower LED zone (rounded rect with glow gradient)
- Mesh overlay lines
- Mute button ellipse
- Stand and base
- Zone labels ("Upper", "Lower")

The canvas state holds `upper_color` and `lower_color` and redraws when they change.

- [ ] **Step 2: Wire mic preview into main view**

Replace the placeholder container in the left column with the `MicPreview` canvas. Connect `EngineEvent::FrameSent` to update the preview colors in real-time.

- [ ] **Step 3: Verify it renders**

Run: `cargo run -p sliglight-gui`
Expected: mic preview renders with LED glow, updates colors when engine sends frames

- [ ] **Step 4: Commit**

```bash
git add crates/sliglight-gui/
git commit -m "feat(gui): canvas mic preview with LED glow zones"
```

---

### Task 10: Color Picker Integration

**Files:**
- Modify: `crates/sliglight-gui/src/main.rs`

- [ ] **Step 1: Implement color picker**

Add a simple inline color picker that opens when a swatch is clicked. Options:
1. Build a minimal HSV color picker using Canvas + sliders (hue slider + saturation/value square)
2. Use iced_aw's `ColorPicker` widget
3. Use 3 RGB sliders (simplest)

For simplicity, start with 3 sliders (R/G/B) in a popup overlay. Each swatch shows as a colored circle button — click to expand the RGB editor below the swatch row.

- [ ] **Step 2: Verify color picking works**

Run: `cargo run -p sliglight-gui`
Expected: clicking a swatch opens R/G/B sliders, changing them updates the swatch color

- [ ] **Step 3: Commit**

```bash
git add crates/sliglight-gui/
git commit -m "feat(gui): color picker for palette swatches"
```

---

## Chunk 4: Nix Packaging + Cleanup

### Task 11: Window Icon

**Files:**
- Modify: `crates/sliglight-gui/src/main.rs`
- Move: `gui/resources/sliglight.svg` → `resources/sliglight.svg`

- [ ] **Step 1: Embed SVG icon in binary**

Move `gui/resources/sliglight.svg` to `resources/sliglight.svg` (workspace root). In `main.rs`, use `include_bytes!("../../../resources/sliglight.svg")` to embed the SVG, then convert it to RGBA pixels using the `resvg` crate (add to dependencies) for `iced::window::icon::from_rgba()`. Set the icon in the application builder with `.window(window::Settings { icon: Some(icon), .. })`.

- [ ] **Step 2: Verify icon shows in taskbar**

Run: `cargo run -p sliglight-gui`
Expected: window and taskbar show the sliglight mic icon

- [ ] **Step 3: Commit**

```bash
git add crates/sliglight-gui/ resources/
git commit -m "feat(gui): embedded SVG window icon"
```

---

### Task 12: Nix Derivation

**Files:**
- Rewrite: `gui-package.nix` (Python → Rust)
- Delete: `package.nix`
- Modify: `flake.nix`
- Modify: `module.nix`
- Delete: `gui/` (entire Python directory)
- Delete: `*.patch` files

- [ ] **Step 1: Rewrite gui-package.nix for Rust**

```nix
{
  lib,
  rustPlatform,
  libusb1,
  pkg-config,
  cmake,
  makeWrapper,
  makeDesktopItem,
  # iced GUI runtime deps (Wayland + Vulkan + fonts)
  wayland,
  libxkbcommon,
  vulkan-loader,
  libGL,
  fontconfig,
  freetype,
  xorg,
}:
let
  desktopItem = makeDesktopItem {
    name = "sliglight";
    desktopName = "Sliglight";
    comment = "RGB lighting control for HyperX QuadCast microphones";
    exec = "sliglight";
    icon = "sliglight";
    categories = [ "Utility" "Settings" "HardwareSettings" ];
    keywords = [ "HyperX" "QuadCast" "RGB" "microphone" ];
  };

  runtimeLibs = [
    wayland
    libxkbcommon
    vulkan-loader
    libGL
    fontconfig
    freetype
    xorg.libX11
    xorg.libXcursor
    xorg.libXrandr
    xorg.libXi
  ];
in
rustPlatform.buildRustPackage {
  pname = "sliglight";
  version = "0.1.0";

  src = ./.;

  cargoLock.lockFile = ./Cargo.lock;

  nativeBuildInputs = [ pkg-config cmake makeWrapper ];

  buildInputs = [ libusb1 ] ++ runtimeLibs;

  postInstall = ''
    mkdir -p $out/share/applications
    cp ${desktopItem}/share/applications/*.desktop $out/share/applications/

    mkdir -p $out/share/icons/hicolor/scalable/apps
    cp resources/sliglight.svg $out/share/icons/hicolor/scalable/apps/sliglight.svg
  '';

  # iced needs runtime access to Wayland/Vulkan/font libraries
  postFixup = ''
    wrapProgram $out/bin/sliglight \
      --suffix LD_LIBRARY_PATH : ${lib.makeLibraryPath runtimeLibs}
  '';

  meta = {
    description = "RGB lighting control for HyperX QuadCast microphones";
    homepage = "https://github.com/htelsiz/nix-quadcast";
    license = lib.licenses.gpl2Only;
    platforms = lib.platforms.linux;
    mainProgram = "sliglight";
  };
}
```

- [ ] **Step 2: Update flake.nix**

Remove the `quadcastrgb` C package references. The single Rust workspace produces both `sliglight` (GUI) and `sliglight-cli` binaries. The overlay should export:

```nix
overlays.default = _final: prev: {
  sliglight = prev.callPackage ./gui-package.nix { };
};
```

And packages:
```nix
packages.${system} = {
  default = pkgs.sliglight;
  gui = pkgs.sliglight;
};
```

Remove the old `quadcastrgb` overlay entry entirely.

- [ ] **Step 3: Update module.nix**

Replace the entire `module.nix` with:

```nix
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.hardware.quadcast;
  sliglight = pkgs.callPackage ./gui-package.nix { };
in
{
  options.hardware.quadcast = {
    enable = lib.mkEnableOption "HyperX QuadCast RGB control (CLI + GUI + udev rules)";

    color = lib.mkOption {
      type = lib.types.nullOr lib.types.str;
      default = null;
      example = "ff0000";
      description = ''
        Hex color to apply on boot via a systemd user service.
        When set, a persistent `quadcast-rgb` service runs `sliglight-cli solid <color>`
        continuously (the mic reverts to default rainbow if the process stops).
        Set to null to disable the service.
      '';
    };

    mode = lib.mkOption {
      type = lib.types.str;
      default = "solid";
      example = "cycle";
      description = "RGB mode: solid, blink, cycle, wave, lightning, or pulse.";
    };
  };

  config = lib.mkIf cfg.enable {
    # Both sliglight (GUI) and sliglight-cli come from the same Rust workspace build
    environment.systemPackages = [ sliglight ];

    # Persistent systemd user service to keep RGB active
    systemd.user.services.quadcast-rgb = lib.mkIf (cfg.color != null) {
      description = "HyperX QuadCast RGB lighting";
      wantedBy = [ "graphical-session.target" ];
      after = [ "graphical-session.target" ];
      serviceConfig = {
        ExecStart = "${sliglight}/bin/sliglight-cli ${cfg.mode} ${cfg.color}";
        Restart = "on-failure";
        RestartSec = 3;
      };
    };

    # udev rules for non-root USB HID access to QuadCast microphones.
    services.udev.extraRules = ''
      # HyperX QuadCast S (Kingston)
      SUBSYSTEM=="usb", ATTR{idVendor}=="0951", ATTR{idProduct}=="171f", MODE="0666", TAG+="uaccess"
      # HyperX QuadCast 2S / DuoCast (HP)
      SUBSYSTEM=="usb", ATTR{idVendor}=="03f0", ATTR{idProduct}=="0f8b", MODE="0666", TAG+="uaccess"
      SUBSYSTEM=="usb", ATTR{idVendor}=="03f0", ATTR{idProduct}=="028c", MODE="0666", TAG+="uaccess"
      SUBSYSTEM=="usb", ATTR{idVendor}=="03f0", ATTR{idProduct}=="048c", MODE="0666", TAG+="uaccess"
      SUBSYSTEM=="usb", ATTR{idVendor}=="03f0", ATTR{idProduct}=="068c", MODE="0666", TAG+="uaccess"
      SUBSYSTEM=="usb", ATTR{idVendor}=="03f0", ATTR{idProduct}=="098c", MODE="0666", TAG+="uaccess"
      SUBSYSTEM=="usb", ATTR{idVendor}=="03f0", ATTR{idProduct}=="02b5", MODE="0666", TAG+="uaccess"
      SUBSYSTEM=="usb", ATTR{idVendor}=="03f0", ATTR{idProduct}=="0d84", MODE="0666", TAG+="uaccess"
    '';
  };
}
```

Key changes: `quadcastrgb` replaced with `sliglight`, `ExecStart` uses `sliglight-cli`, removed `enableGui` option (both binaries come from one Rust build), removed `package.nix` reference.

- [ ] **Step 4: Delete Python code and C patches**

```bash
rm -rf gui/
rm package.nix
rm *.patch
```

- [ ] **Step 5: Generate Cargo.lock**

Run: `cargo generate-lockfile`

- [ ] **Step 6: Build with Nix**

Run: `nix build .#gui`
Expected: builds successfully, produces `result/bin/sliglight` and `result/bin/sliglight-cli`

- [ ] **Step 7: Verify .desktop and icon**

```bash
ls result/share/applications/sliglight.desktop
ls result/share/icons/hicolor/scalable/apps/sliglight.svg
cat result/share/applications/sliglight.desktop
```

- [ ] **Step 8: Test both binaries**

```bash
result/bin/sliglight          # Should open GUI window
result/bin/sliglight-cli solid ff0000  # Should set mic to red
```

- [ ] **Step 9: Commit**

```bash
git add -A
git commit -m "feat: complete Rust rewrite, remove Python and C code"
```

---

### Task 13: Update NixOS Config

**Files:**
- Modify: `/etc/nixos/flake.nix` (update nix-quadcast input hash)

- [ ] **Step 1: Push nix-quadcast changes to GitHub**

```bash
cd /home/ht/projects/nix-quadcast
git push origin main
```

- [ ] **Step 2: Update flake input in nixos-config**

```bash
cd /etc/nixos
nix flake update nix-quadcast
```

- [ ] **Step 3: Rebuild NixOS**

```bash
nrb
```
Expected: system rebuilds with Rust-based sliglight

- [ ] **Step 4: Test sliglight from system PATH**

```bash
which sliglight
sliglight  # Should open GUI
```

- [ ] **Step 5: Commit nixos-config changes**

```bash
cd /etc/nixos
git add flake.lock
git commit -m "feat: update nix-quadcast to Rust rewrite"
```
