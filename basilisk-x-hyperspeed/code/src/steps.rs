//! The speeds the DPI button goes through.
//!
//! The mouse is told of all of them at once, and says all of them at once,
//! so one is changed by reading them, changing it, and writing them back.

use std::cell::Cell;

use crate::protocol::{Arguments, Command, Store, ask};
use crate::speed;

/// How many speeds the button can go through at most, and the ones a step
/// starts at when it is added.
pub const MOST: usize = 5;
const USUAL: [i32; MOST] = [800, 1800, 3600, 7200, 16000];

// How what is said about the steps is laid out. First where they are kept,
// which is in use, counted from 1, and how many there are.
const ACTIVE: usize = 1;
const COUNT: usize = 2;
const HEAD: usize = 3;
// Then for each: its number, its speed, and two bytes of nothing.
const SPEED: usize = 1;
const STEP: usize = SPEED + speed::SAID + 2;
const LENGTH: usize = HEAD + MOST * STEP;

thread_local! {
    /// The steps as read in this run, so that each of the settings made of
    /// them does not ask the device for all of them again.
    static READ: Cell<Option<Steps>> = const { Cell::new(None) };
}

/// The speeds the button goes through: which is in use, counted from 1,
/// how many there are, and the speed at each.
#[derive(Clone, Copy)]
pub struct Steps {
    active: u8,
    pub count: usize,
    speeds: [i32; MOST],
}

impl Steps {
    /// The steps as the mouse said them. One past how many there are is
    /// not said, and is at its usual speed.
    fn heard(said: &Arguments) -> Self {
        let (head, each) = said.split_at(HEAD);
        let count = usize::from(head[COUNT]).clamp(1, MOST);
        let mut speeds = USUAL;
        for (speed, step) in speeds.iter_mut().zip(each.chunks_exact(STEP)).take(count) {
            *speed = speed::heard(&step[SPEED..]);
        }
        Steps { active: head[ACTIVE].clamp(1, count as u8), count, speeds }
    }

    /// The steps as they are said to the mouse.
    fn said(&self) -> [u8; LENGTH] {
        let mut said = [0u8; LENGTH];
        let (head, each) = said.split_at_mut(HEAD);
        head.copy_from_slice(&[Store::Kept as u8, self.active.min(self.count as u8), self.count as u8]);
        for (number, (speed, step)) in self.speeds.iter().zip(each.chunks_exact_mut(STEP)).take(self.count).enumerate() {
            step[0] = number as u8;
            step[SPEED..SPEED + speed::SAID].copy_from_slice(&speed::said(*speed));
        }
        said
    }

    /// The speed of a step, if there are that many.
    pub fn speed_of(&self, step: usize) -> Option<i32> {
        self.speeds[..self.count].get(step).copied()
    }

    /// These with a step at another speed, if there are that many.
    pub fn with_speed(mut self, step: usize, speed: i32) -> Option<Self> {
        *self.speeds[..self.count].get_mut(step)? = speed;
        Some(self)
    }

    /// These with more or fewer of them.
    pub fn with_count(self, count: i32) -> Self {
        Steps { count: usize::try_from(count).unwrap_or(1).clamp(1, MOST), ..self }
    }

    /// Tells the mouse of these. It goes to the speed of the step in use
    /// when it is told, so the speed it was at is put back after.
    pub fn write(&self) -> bool {
        let was = speed::read();
        READ.set(None);
        ask(Command::SetSteps, &self.said()).is_some() && was.is_none_or(speed::write)
    }
}

/// The steps the mouse has.
pub fn read() -> Option<Steps> {
    if READ.get().is_none() {
        let mut asked = [0u8; LENGTH];
        asked[0] = Store::Kept as u8;
        READ.set(Some(Steps::heard(&ask(Command::GetSteps, &asked)?)));
    }
    READ.get()
}
