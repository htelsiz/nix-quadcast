// TODO

/// Number of upper ring LEDs.
pub const UPPER_COUNT: usize = 8;
/// Number of lower ring LEDs.
pub const LOWER_COUNT: usize = 8;
/// Total LED count.
pub const TOTAL_LEDS: usize = UPPER_COUNT + LOWER_COUNT;

/// RGB color.
#[derive(Debug, Clone, Copy)]
pub struct Color;

/// A complete LED frame.
#[derive(Debug)]
pub struct Frame;
