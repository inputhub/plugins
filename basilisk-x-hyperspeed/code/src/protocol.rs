//! A report to the mouse and its answer.
//!
//! The protocol is the one the openrazer project has written down. One
//! report of ninety bytes goes each way: who is spoken to, what is asked or
//! told, what it is about, and a check. The answer is the same report with
//! what was asked for filled in and a word on how it went.

use std::cell::Cell;

use inputhub_plugin::device;

/// The parts of the mouse a thing asked or told belongs to. In openrazer
/// this is the command class.
#[derive(Clone, Copy)]
#[repr(u8)]
enum Class {
    /// The mouse as a device: how often it reports.
    Device = 0x00,
    /// Its sensor: the speed in use, and the steps the button goes through.
    Sensor = 0x04,
    /// Its battery.
    Power = 0x07,
}

/// Something the mouse is asked or told.
#[derive(Clone, Copy)]
pub enum Command {
    GetRate,
    SetRate,
    GetSpeed,
    SetSpeed,
    GetSteps,
    SetSteps,
    GetBattery,
}

impl Command {
    /// The part of the mouse it belongs to, and which thing of that part
    /// it is: the command id in openrazer, where one that asks has the top
    /// bit set and the one that tells the same thing has not.
    fn said_as(self) -> (Class, u8) {
        match self {
            Command::GetRate => (Class::Device, 0x85),
            Command::SetRate => (Class::Device, 0x05),
            Command::GetSpeed => (Class::Sensor, 0x85),
            Command::SetSpeed => (Class::Sensor, 0x05),
            Command::GetSteps => (Class::Sensor, 0x86),
            Command::SetSteps => (Class::Sensor, 0x06),
            Command::GetBattery => (Class::Power, 0x80),
        }
    }
}

/// Where a value is kept: for now only, or by the device itself.
#[derive(Clone, Copy)]
#[repr(u8)]
pub enum Store {
    ForNow = 0x00,
    Kept = 0x01,
}

/// How the device says a command went.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Status {
    /// It has not answered yet: what is read is the question itself, or
    /// the answer to an earlier one.
    Waiting,
    /// It did what it was asked.
    Done,
    /// It will not, or it is asleep: no use waiting.
    Refused,
}

impl Status {
    fn of(said: u8) -> Self {
        match said {
            0x00 | 0x01 => Status::Waiting,
            0x02 => Status::Done,
            _ => Status::Refused,
        }
    }
}

// Where things are in a report. It goes under no number, which is the
// first byte, and the ninety follow.
const LENGTH: usize = 91;
const STATUS: usize = 1;
const TRANSACTION: usize = 2;
const SIZE: usize = 6;
const CLASS: usize = 7;
const COMMAND: usize = 8;
const FIRST: usize = 9;
const CHECK: usize = 89;

/// What a kind of device may expect to be spoken to under; which one is
/// found by asking.
const TRANSACTIONS: [u8; 3] = [0x1f, 0x3f, 0xff];

/// How long the device is given between tries to answer, and how many
/// tries. Short and many: it answers in a few milliseconds when awake,
/// and is not kept waiting past that.
const WAIT_MS: u32 = 3;
const TRIES: usize = 60;

thread_local! {
    /// What this device is spoken to under, once it has answered.
    static SPOKEN_UNDER: Cell<Option<u8>> = const { Cell::new(None) };
    /// The device did not answer at all in this run: it is asleep or off,
    /// and is not waited for again.
    static SILENT: Cell<bool> = const { Cell::new(false) };
}

/// What a command is about, as answered.
pub type Arguments = [u8; CHECK - FIRST];

/// A report that says a command, under one way of being spoken to.
fn report(transaction: u8, command: Command, arguments: &[u8]) -> [u8; LENGTH] {
    let (class, id) = command.said_as();
    let mut report = [0u8; LENGTH];
    report[TRANSACTION] = transaction;
    report[SIZE] = arguments.len() as u8;
    report[CLASS] = class as u8;
    report[COMMAND] = id;
    report[FIRST..FIRST + arguments.len()].copy_from_slice(arguments);
    // What tells a report from a damaged one: all of it between the start
    // and the check itself, folded together.
    report[CHECK] = report[TRANSACTION + 1..CHECK].iter().fold(0, |all, byte| all ^ byte);
    report
}

/// Sends a command under one way of being spoken to and waits for the
/// answer to it.
fn ask_under(transaction: u8, command: Command, arguments: &[u8]) -> Option<Arguments> {
    let sent = report(transaction, command, arguments);
    if !device::send(&sent) {
        return None;
    }
    for _ in 0..TRIES {
        inputhub_plugin::wait(WAIT_MS);
        let mut got = [0u8; LENGTH];
        if !device::read(&mut got) {
            return None;
        }
        let ours = [TRANSACTION, CLASS, COMMAND].iter().all(|at| got[*at] == sent[*at]);
        match Status::of(got[STATUS]) {
            Status::Done if ours => return got[FIRST..CHECK].try_into().ok(),
            Status::Refused if ours => return None,
            _ => {}
        }
    }
    None
}

/// Sends a command and waits for the answer, finding out first, if it is
/// not known yet, what this device is spoken to under.
pub fn ask(command: Command, arguments: &[u8]) -> Option<Arguments> {
    if SILENT.get() {
        return None;
    }
    if let Some(transaction) = SPOKEN_UNDER.get() {
        return ask_under(transaction, command, arguments);
    }
    for transaction in TRANSACTIONS {
        if let Some(answered) = ask_under(transaction, command, arguments) {
            SPOKEN_UNDER.set(Some(transaction));
            return Some(answered);
        }
    }
    SILENT.set(true);
    inputhub_plugin::log("the mouse does not answer; it may be asleep or switched off");
    None
}
