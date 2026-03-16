//! USB protocol for HyperX QuadCast 2S RGB LED control.

mod device;
mod error;
mod protocol;

pub use device::QuadCast2S;
pub use error::UsbError;
pub use protocol::{Color, Frame, LOWER_COUNT, TOTAL_LEDS, UPPER_COUNT};
