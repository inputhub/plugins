//! How fast the pointer goes, in DPI.

use crate::protocol::{Command, Store, ask};

/// The slowest and fastest the sensor goes.
pub const SLOWEST: i32 = 100;
pub const FASTEST: i32 = 16000;

/// How a speed is said: across, then up and down, two bytes each. The two
/// are always kept the same here.
pub const SAID: usize = 4;

/// A speed as it is said to the mouse, held to what the sensor can do.
pub fn said(speed: i32) -> [u8; SAID] {
    let [high, low] = (speed.clamp(SLOWEST, FASTEST) as u16).to_be_bytes();
    [high, low, high, low]
}

/// The speed that is said in these bytes: the one across.
pub fn heard(said: &[u8]) -> i32 {
    i32::from(u16::from_be_bytes([said[0], said[1]]))
}

/// The speed in use just now.
pub fn read() -> Option<i32> {
    let now = ask(Command::GetSpeed, &[Store::ForNow as u8, 0, 0, 0, 0, 0, 0])?;
    Some(heard(&now[1..]))
}

/// Puts the pointer at a speed, which the mouse keeps.
pub fn write(speed: i32) -> bool {
    let [across_high, across_low, up_high, up_low] = said(speed);
    ask(Command::SetSpeed, &[Store::Kept as u8, across_high, across_low, up_high, up_low, 0, 0]).is_some()
}
