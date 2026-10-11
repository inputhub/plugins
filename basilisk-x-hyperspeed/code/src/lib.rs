//! The code of the Razer Basilisk X HyperSpeed plugin, and an example of a
//! plugin that talks to a device.
//!
//! InputHub takes the mouse's buttons itself. What only Razer's own
//! protocol can do is asked for here: the code is handed the one device the
//! plugin's description names, through the functions in [`inputhub`], and
//! nothing else of the system. It is run anew each time InputHub wants
//! something, and is gone after it, so nothing is remembered between runs.
//!
//! InputHub calls three functions:
//!
//! - [`get`] with the place of a setting among those `device.toml` lists,
//!   the first being 0, for its value;
//! - [`set`] with a place and a value;
//! - [`battery`] for how full the battery is.
//!
//! The protocol is the one the openrazer project has written down: one
//! report of ninety bytes each way.
//!
//! Build with `cargo build --release` and copy the result beside the
//! manifest as `plugin.wasm` (see the README).

use std::cell::Cell;

/// What a plugin that talks to a device is handed.
pub mod inputhub {
    #[link(wasm_import_module = "inputhub")]
    unsafe extern "C" {
        #[link_name = "send"]
        fn device_send(ptr: *const u8, len: usize) -> i32;
        #[link_name = "read"]
        fn device_read(ptr: *mut u8, len: usize) -> i32;
        #[link_name = "wait"]
        fn device_wait(ms: u32);
        #[link_name = "log"]
        fn host_log(ptr: *const u8, len: usize);
    }

    /// Sends the device a feature report. The first byte is the number the
    /// report goes under. Says whether it was sent.
    pub fn send(report: &[u8]) -> bool {
        unsafe { device_send(report.as_ptr(), report.len()) >= 0 }
    }

    /// Reads a feature report from the device, into `report`, whose first
    /// byte is the number of the report wanted. Says whether one was read.
    pub fn read(report: &mut [u8]) -> bool {
        unsafe { device_read(report.as_mut_ptr(), report.len()) >= 0 }
    }

    /// Waits, as a device needs between being asked and answering. A run
    /// may wait three seconds in all.
    pub fn wait(ms: u32) {
        unsafe { device_wait(ms) }
    }

    /// A line in InputHub's log, for you while you write the plugin.
    pub fn log(line: &str) {
        unsafe { host_log(line.as_ptr(), line.len()) }
    }
}

// The settings, by their place in `device.toml`.
const POINTER_SPEED: i32 = 0;
const REPORT_RATE: i32 = 1;
const STEP_COUNT: i32 = 2;
const FIRST_STEP: i32 = 3;

/// The slowest and fastest the sensor goes, in DPI.
const SLOWEST: i32 = 100;
const FASTEST: i32 = 16000;

/// How many speeds the button can go through at most, and the ones a step
/// starts at when it is added.
const STEPS: usize = 5;
const USUAL_STEPS: [i32; STEPS] = [800, 1800, 3600, 7200, 16000];

/// How often the mouse can report, as the device has it, in the order
/// `device.toml` lists them: 125, 500 and 1000 Hz.
const RATES: [u8; 3] = [0x08, 0x02, 0x01];

/// Where a value is kept: for now only, or by the device itself.
const FOR_NOW: u8 = 0x00;
const KEPT: u8 = 0x01;

// A report: the number it goes under, which is none, then ninety bytes.
const LENGTH: usize = 91;
const STATUS: usize = 1;
const TRANSACTION: usize = 2;
const SIZE: usize = 6;
const CLASS: usize = 7;
const COMMAND: usize = 8;
const FIRST: usize = 9;
const CHECK: usize = 89;

/// The answer that says the device did what it was asked.
const DONE: u8 = 0x02;

/// What a kind of device may expect to be spoken to under; which one is
/// found by asking.
const TRANSACTIONS: [u8; 3] = [0x1f, 0x3f, 0xff];

/// How long the device is given between tries to answer, and how many tries.
const WAIT_MS: u32 = 15;
const TRIES: usize = 12;

thread_local! {
    /// What this device is spoken to under, once it has answered.
    static SPOKEN_UNDER: Cell<Option<u8>> = const { Cell::new(None) };
    /// The device did not answer at all in this run: it is asleep or off,
    /// and is not waited for again.
    static SILENT: Cell<bool> = const { Cell::new(false) };
}

/// What a command is about, as answered.
type Arguments = [u8; CHECK - FIRST];

/// Sends a command under one way of being spoken to and waits for the
/// answer to it.
fn ask_under(transaction: u8, class: u8, command: u8, arguments: &[u8]) -> Option<Arguments> {
    let mut sent = [0u8; LENGTH];
    sent[TRANSACTION] = transaction;
    sent[SIZE] = arguments.len() as u8;
    sent[CLASS] = class;
    sent[COMMAND] = command;
    sent[FIRST..FIRST + arguments.len()].copy_from_slice(arguments);
    // What tells a report from a damaged one: all of it between the start
    // and the check itself, folded together.
    sent[CHECK] = sent[TRANSACTION + 1..CHECK].iter().fold(0, |all, byte| all ^ byte);
    if !inputhub::send(&sent) {
        return None;
    }
    for _ in 0..TRIES {
        inputhub::wait(WAIT_MS);
        let mut got = [0u8; LENGTH];
        if !inputhub::read(&mut got) {
            return None;
        }
        // What is read before the device has answered is the question
        // itself, or the answer to an earlier one.
        let ours = got[CLASS] == class && got[COMMAND] == command && got[TRANSACTION] == transaction;
        match got[STATUS] {
            DONE if ours => return got[FIRST..CHECK].try_into().ok(),
            // Asleep, or it will not: no use waiting.
            status if ours && status > DONE => return None,
            _ => {}
        }
    }
    None
}

/// Sends a command and waits for the answer, finding out first, if it is
/// not known yet, what this device is spoken to under.
fn ask(class: u8, command: u8, arguments: &[u8]) -> Option<Arguments> {
    if SILENT.get() {
        return None;
    }
    if let Some(transaction) = SPOKEN_UNDER.get() {
        return ask_under(transaction, class, command, arguments);
    }
    for transaction in TRANSACTIONS {
        if let Some(answered) = ask_under(transaction, class, command, arguments) {
            SPOKEN_UNDER.set(Some(transaction));
            return Some(answered);
        }
    }
    SILENT.set(true);
    inputhub::log("the mouse does not answer; it may be asleep or switched off");
    None
}

/// The speeds the button goes through: which is in use, counted from 1,
/// how many there are, and the speed at each.
struct Steps {
    active: u8,
    count: usize,
    speeds: [i32; STEPS],
}

/// How long what is said about the steps is: where they are kept, which is
/// in use, how many there are, and seven bytes for each.
const STEPS_SAID: usize = 3 + STEPS * 7;

fn read_steps() -> Option<Steps> {
    let mut asked = [0u8; STEPS_SAID];
    asked[0] = KEPT;
    let said = ask(0x04, 0x86, &asked)?;
    let count = usize::from(said[2]).clamp(1, STEPS);
    let mut speeds = USUAL_STEPS;
    for (step, speed) in speeds.iter_mut().enumerate().take(count) {
        let at = 3 + step * 7;
        *speed = i32::from(u16::from_be_bytes([said[at + 1], said[at + 2]]));
    }
    Some(Steps { active: said[1].clamp(1, count as u8), count, speeds })
}

fn write_steps(steps: &Steps) -> bool {
    let mut said = [0u8; STEPS_SAID];
    said[..3].copy_from_slice(&[KEPT, steps.active.clamp(1, steps.count as u8), steps.count as u8]);
    for (step, speed) in steps.speeds.iter().enumerate().take(steps.count) {
        let [high, low] = (*speed as u16).to_be_bytes();
        said[3 + step * 7..][..5].copy_from_slice(&[step as u8, high, low, high, low]);
    }
    ask(0x04, 0x06, &said).is_some()
}

/// The value of a setting, or less than 0 for one the device has not got
/// just now: a step past how many there are, or anything while it sleeps.
#[unsafe(no_mangle)]
pub extern "C" fn get(setting: i32) -> i64 {
    let value = match setting {
        POINTER_SPEED => ask(0x04, 0x85, &[FOR_NOW, 0, 0, 0, 0, 0, 0]).map(|now| i32::from(u16::from_be_bytes([now[1], now[2]]))),
        REPORT_RATE => ask(0x00, 0x85, &[0]).map(|now| RATES.iter().position(|rate| *rate == now[0]).unwrap_or(RATES.len() - 1) as i32),
        STEP_COUNT => read_steps().map(|steps| steps.count as i32),
        step if (FIRST_STEP..FIRST_STEP + STEPS as i32).contains(&step) => {
            let step = (step - FIRST_STEP) as usize;
            read_steps().filter(|steps| step < steps.count).map(|steps| steps.speeds[step])
        }
        _ => None,
    };
    value.map_or(-1, i64::from)
}

/// Puts a setting at a value. Gives 0 when the device has taken it.
#[unsafe(no_mangle)]
pub extern "C" fn set(setting: i32, value: i32) -> i32 {
    let speed = value.clamp(SLOWEST, FASTEST);
    let done = match setting {
        POINTER_SPEED => {
            let [high, low] = (speed as u16).to_be_bytes();
            ask(0x04, 0x05, &[KEPT, high, low, high, low, 0, 0]).is_some()
        }
        REPORT_RATE => usize::try_from(value).ok().and_then(|at| RATES.get(at)).is_some_and(|rate| ask(0x00, 0x05, &[*rate]).is_some()),
        STEP_COUNT => read_steps().is_some_and(|steps| write_steps(&Steps { count: usize::try_from(value).unwrap_or(1).clamp(1, STEPS), ..steps })),
        step if (FIRST_STEP..FIRST_STEP + STEPS as i32).contains(&step) => read_steps().is_some_and(|mut steps| {
            let step = (step - FIRST_STEP) as usize;
            steps.speeds[step] = speed;
            step < steps.count && write_steps(&steps)
        }),
        _ => false,
    };
    i32::from(!done)
}

/// How full the battery is, in percent, or less than 0 if the mouse does
/// not say. It takes one AA cell, so it is never charging: a device that
/// charges would add 256 while it does.
#[unsafe(no_mangle)]
pub extern "C" fn battery() -> i32 {
    match ask(0x07, 0x80, &[0, 0]) {
        Some(said) => (i32::from(said[1]) * 100 + 127) / 255,
        None => -1,
    }
}
