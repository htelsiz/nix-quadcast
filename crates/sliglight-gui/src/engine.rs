//! Long-lived background animation engine using iced subscriptions.
//!
//! Spawns a single subscription that lives for the app's lifetime.
//! Config updates are pushed via `tokio::sync::watch` — no subscription restarts.

use std::time::Duration;

use iced::futures::SinkExt;
use iced::Subscription;
use sliglight_core::animations::{Animation, Mode, Zone};
use sliglight_core::color::blend_frames;
use sliglight_usb::{Color, Frame, QuadCast2S};
use tokio::sync::watch;

const TARGET_FPS: u64 = 30;
const FRAME_INTERVAL: Duration = Duration::from_millis(1000 / TARGET_FPS);
const RECONNECT_DELAY: Duration = Duration::from_secs(2);
const TRANSITION_FRAMES: usize = 10;

#[derive(Debug, Clone)]
pub enum Event {
    Ready(watch::Sender<EngineConfig>),
    Connected,
    Disconnected,
    Reconnecting,
    FrameSent { upper: Color, lower: Color },
    Error(String),
}

#[derive(Clone, Debug)]
pub struct EngineConfig {
    pub mode: Mode,
    pub colors: Vec<Color>,
    pub brightness: u8,
    pub speed: u8,
    pub zone: Zone,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            mode: Mode::Solid,
            colors: vec![(255, 0, 0)],
            brightness: 80,
            speed: 81,
            zone: Zone::Both,
        }
    }
}

/// Mic peak level — read by engine for AudioReactive mode.
static PEAK_LEVEL: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// Desktop audio peak level — read by engine for MusicReactive mode.
static MUSIC_PEAK_LEVEL: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// Set mic peak level (called from audio subscription handler).
pub fn set_peak_level(peak: f32) {
    let bits = peak.clamp(0.0, 1.0).to_bits();
    PEAK_LEVEL.store(bits, std::sync::atomic::Ordering::Relaxed);
}

/// Set desktop audio peak level (called from audio subscription handler).
pub fn set_music_peak_level(peak: f32) {
    let bits = peak.clamp(0.0, 1.0).to_bits();
    MUSIC_PEAK_LEVEL.store(bits, std::sync::atomic::Ordering::Relaxed);
}

pub fn get_peak_level() -> f32 {
    f32::from_bits(PEAK_LEVEL.load(std::sync::atomic::Ordering::Relaxed))
}

pub fn get_music_peak_level() -> f32 {
    f32::from_bits(MUSIC_PEAK_LEVEL.load(std::sync::atomic::Ordering::Relaxed))
}

/// Always-on subscription — call once from `App::subscription()`.
pub fn subscription() -> Subscription<Event> {
    Subscription::run(engine_worker)
}

fn engine_worker() -> impl iced::futures::Stream<Item = Event> {
    iced::stream::channel(32, async move |mut output| {
        // Stop systemd service once on startup.
        let _ = std::process::Command::new("systemctl")
            .args(["--user", "stop", "quadcast-rgb.service"])
            .output();

        // Create the watch channel and send the Sender back to the app.
        let (config_tx, mut config_rx) = watch::channel(EngineConfig::default());
        let _ = output.send(Event::Ready(config_tx)).await;

        // Outer loop: auto-reconnect on device loss.
        loop {
            let device = loop {
                match QuadCast2S::open() {
                    Ok(d) => {
                        let _ = output.send(Event::Connected).await;
                        break d;
                    }
                    Err(_) => {
                        let _ = output.send(Event::Reconnecting).await;
                        tokio::time::sleep(RECONNECT_DELAY).await;
                    }
                }
            };

            // Build initial animation from current config.
            let cfg = config_rx.borrow().clone();
            let mut anim_mode = cfg.mode;
            let mut anim = Animation::new(cfg.mode, cfg.colors, cfg.brightness, cfg.speed, cfg.zone);

            // Transition state: when config changes, blend from snapshot to new over N frames.
            let mut transition: Option<(Frame, usize)> = None; // (snapshot, frames_remaining)
            let mut last_frame: Option<Frame> = None;

            // Inner loop: generate frames, react to config changes.
            let mut connected = true;
            while connected {
                tokio::select! {
                    _ = config_rx.changed() => {
                        let cfg = config_rx.borrow_and_update().clone();
                        anim_mode = cfg.mode;
                        // Start transition from last sent frame.
                        if let Some(ref lf) = last_frame {
                            transition = Some((lf.clone(), TRANSITION_FRAMES));
                        }
                        anim = Animation::new(cfg.mode, cfg.colors, cfg.brightness, cfg.speed, cfg.zone);
                    }
                    _ = tokio::time::sleep(FRAME_INTERVAL) => {
                        let new_frame = match anim_mode {
                            Mode::AudioReactive => anim.audio_reactive_frame(get_peak_level()),
                            Mode::MusicReactive => anim.music_reactive_frame(get_music_peak_level()),
                            _ => anim.next_frame(),
                        };

                        // Apply transition blend if active.
                        let frame = if let Some((ref snapshot, remaining)) = transition {
                            let progress = 1.0 - remaining as f32 / TRANSITION_FRAMES as f32;
                            blend_frames(snapshot, &new_frame, progress)
                        } else {
                            new_frame
                        };

                        // Tick transition counter.
                        if let Some((_, ref mut remaining)) = transition {
                            *remaining = remaining.saturating_sub(1);
                            if *remaining == 0 {
                                transition = None;
                            }
                        }

                        let upper = frame.upper.first().copied().unwrap_or((0, 0, 0));
                        let lower = frame.lower.first().copied().unwrap_or((0, 0, 0));

                        if let Err(e) = device.send_frame(&frame) {
                            let _ = output.send(Event::Error(e.to_string())).await;
                            let _ = output.send(Event::Disconnected).await;
                            connected = false;
                        } else {
                            last_frame = Some(frame);
                            let _ = output.send(Event::FrameSent { upper, lower }).await;
                        }
                    }
                }
            }

            // Brief pause before reconnect attempt.
            tokio::time::sleep(RECONNECT_DELAY).await;
        }
    })
}

