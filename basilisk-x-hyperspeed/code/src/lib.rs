//! The code of the Razer Basilisk X HyperSpeed plugin, and an example of a
//! plugin that talks to a device.
//!
//! InputHub takes the mouse's buttons itself. What only Razer's own
//! protocol can do is asked for here: the code is handed the one device the
//! plugin's description names, through the `inputhub-plugin` crate, and
//! nothing else of the system. That crate is what crosses to InputHub and
//! back, so none of the code here needs `unsafe`.
//!
//! InputHub calls three functions, named to it by `device!` below:
//!
//! - [`get`] with the place of a setting among those `device.toml` lists,
//!   the first being 0, for its value;
//! - [`set`] with a place and a value;
//! - [`battery`] for how full the battery is.
//!
//! The code is started anew each time InputHub wants something, so nothing
//! is remembered from one thing asked to the next: InputHub remembers what
//! each setting was put at, and puts it back.
//!
//! How it is laid out:
//!
//! - [`protocol`]: a report to the mouse and its answer, which is all of
//!   Razer's protocol that any of the rest needs to know;
//! - [`speed`], [`steps`], [`rate`] and [`power`]: one thing of the mouse
//!   each, read and written through the protocol;
//! - here: which setting of `device.toml` is which of those.
//!
//! Build with `cargo build --release` and copy the result beside the
//! manifest as `plugin.wasm` (see the README).

#![forbid(unsafe_code)]

mod power;
mod protocol;
mod rate;
mod speed;
mod steps;

use inputhub_plugin::device::Battery;

inputhub_plugin::device! { get: get, set: set, battery: battery }

/// The settings, in the order `device.toml` lists them.
enum Setting {
    PointerSpeed,
    ReportRate,
    StepCount,
    /// The speed of one step, the first being 0.
    Step(usize),
}

impl Setting {
    /// The setting at a place in `device.toml`, if there is one there.
    fn at(place: usize) -> Option<Self> {
        match place {
            0 => Some(Setting::PointerSpeed),
            1 => Some(Setting::ReportRate),
            2 => Some(Setting::StepCount),
            // The steps are the last, one after another.
            step => Some(Setting::Step(step - 3)).filter(|_| step - 3 < steps::MOST),
        }
    }
}

/// The value of a setting, or nothing for one the device has not got just
/// now: a step past how many there are, or anything while it sleeps.
fn get(place: usize) -> Option<i32> {
    match Setting::at(place)? {
        Setting::PointerSpeed => speed::read(),
        Setting::ReportRate => rate::read(),
        Setting::StepCount => Some(steps::read()?.count as i32),
        Setting::Step(step) => steps::read()?.speed_of(step),
    }
}

/// Puts a setting at a value, and says whether the device has taken it.
fn set(place: usize, value: i32) -> bool {
    match Setting::at(place) {
        Some(Setting::PointerSpeed) => speed::write(value),
        Some(Setting::ReportRate) => rate::write(value),
        Some(Setting::StepCount) => steps::read().is_some_and(|steps| steps.with_count(value).write()),
        Some(Setting::Step(step)) => steps::read().and_then(|steps| steps.with_speed(step, value)).is_some_and(|steps| steps.write()),
        None => false,
    }
}

/// How full the battery is, if the mouse says.
fn battery() -> Option<Battery> {
    power::battery()
}
