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
