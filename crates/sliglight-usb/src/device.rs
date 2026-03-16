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
