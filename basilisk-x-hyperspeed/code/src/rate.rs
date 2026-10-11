//! How often the mouse reports where it is.

use crate::protocol::{Command, ask};

/// How often the mouse can report, as the device has it.
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum Rate {
    Hz125 = 0x08,
    Hz500 = 0x02,
    Hz1000 = 0x01,
}

/// Those in the order `device.toml` lists them: the setting is a place
/// among these.
const RATES: [Rate; 3] = [Rate::Hz125, Rate::Hz500, Rate::Hz1000];

/// The place of the rate in use. One the list has not got is shown as the
/// fastest.
pub fn read() -> Option<i32> {
    let now = ask(Command::GetRate, &[0])?[0];
    let place = RATES.iter().position(|rate| *rate as u8 == now).unwrap_or(RATES.len() - 1);
    Some(place as i32)
}

/// Has the mouse report at the rate at a place.
pub fn write(place: i32) -> bool {
    let rate = usize::try_from(place).ok().and_then(|place| RATES.get(place));
    rate.is_some_and(|rate| ask(Command::SetRate, &[*rate as u8]).is_some())
}
