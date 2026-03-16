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
