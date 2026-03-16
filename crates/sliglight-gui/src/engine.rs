//! Background animation engine using iced subscriptions.

use std::hash::{Hash, Hasher};
use std::time::Duration;

use iced::futures::SinkExt;
use iced::Subscription;
use sliglight_core::animations::{Animation, Mode, Zone};
use sliglight_usb::{Color, QuadCast2S};

const TARGET_FPS: u64 = 30;
const FRAME_INTERVAL: Duration = Duration::from_millis(1000 / TARGET_FPS);

#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields used in Task 8 (full GUI layout)
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
            config: EngineConfig {
                mode,
                colors,
                brightness,
                speed,
                zone,
            },
        }
    }

    pub fn subscription(&self) -> Subscription<Event> {
        Subscription::run_with(self.config.clone(), animation_stream)
    }
}

fn animation_stream(config: &EngineConfig) -> impl iced::futures::Stream<Item = Event> {
    let mode = config.mode;
    let colors = config.colors.clone();
    let brightness = config.brightness;
    let speed = config.speed;
    let zone = config.zone;

    iced::stream::channel(32, async move |mut output| {
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

        // Keep the stream alive (required by iced subscriptions)
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    })
}
