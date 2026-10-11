//! How full the battery is.

use inputhub_plugin::device::Battery;

use crate::protocol::{Command, ask};

/// What a full battery is said as.
const FULL: u32 = 255;

/// How full the battery is, if the mouse says. It takes one AA cell, so it
/// is never charging.
pub fn battery() -> Option<Battery> {
    let level = u32::from(ask(Command::GetBattery, &[0, 0])?[1]);
    // To the nearest percent.
    Some(Battery { percent: ((level * 100 + FULL / 2) / FULL) as u8, charging: false })
}
