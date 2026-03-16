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
