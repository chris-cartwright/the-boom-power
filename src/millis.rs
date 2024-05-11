/*!
 * A basic implementation of the `millis()` function from Arduino:
 *
 *     https://www.arduino.cc/reference/en/language/functions/time/millis/
 *
 * Uses timer TC0 and one of its interrupts to update a global millisecond
 * counter.  A walkthrough of this code is available here:
 *
 *     https://blog.rahix.de/005-avr-hal-millis/
 */

use core::{cell, fmt};
use avr_device::interrupt::{Mutex, free};

// Possible Values:
//
// ╔═══════════╦══════════════╦═══════════════════╗
// ║ PRESCALER ║ TIMER_COUNTS ║ Overflow Interval ║
// ╠═══════════╬══════════════╬═══════════════════╣
// ║        64 ║          250 ║              1 ms ║
// ║       256 ║          125 ║              2 ms ║
// ║       256 ║          250 ║              4 ms ║
// ║      1024 ║          125 ║              8 ms ║
// ║      1024 ║          250 ║             16 ms ║
// ╚═══════════╩══════════════╩═══════════════════╝
const PRESCALER: u64 = 1024;
const TIMER_COUNTS: u8 = 125;

const MILLIS_INCREMENT: u64 = PRESCALER * (TIMER_COUNTS as u64) / 16000;

static MILLIS_COUNTER: Mutex<cell::Cell<u64>> = Mutex::new(cell::Cell::new(0));

pub fn init(tc0: arduino_hal::pac::TC0) {
    // Configure the timer for the above interval (in CTC mode)
    // and enable its interrupt.
    tc0.tccr0a.write(|w| w.wgm0().ctc());
    tc0.ocr0a.write(|w| w.bits(TIMER_COUNTS));
    tc0.tccr0b.write(|w| match PRESCALER {
        8 => w.cs0().prescale_8(),
        64 => w.cs0().prescale_64(),
        256 => w.cs0().prescale_256(),
        1024 => w.cs0().prescale_1024(),
        _ => panic!(),
    });
    tc0.timsk0.write(|w| w.ocie0a().set_bit());

    // Reset the global millisecond counter
    free(|cs| {
        MILLIS_COUNTER.borrow(cs).set(0);
    });
}

#[avr_device::interrupt(atmega328p)]
fn TIMER0_COMPA() {
    free(|cs| {
        let counter_cell = MILLIS_COUNTER.borrow(cs);
        let counter = counter_cell.get();
        counter_cell.set(counter + MILLIS_INCREMENT);
    })
}

pub type Millis = u64;

pub fn now() -> Millis {
    free(|cs| MILLIS_COUNTER.borrow(cs).get())
}

#[derive(Clone, Copy)]
pub enum Duration {
    Ref { end: Millis, duration: Millis },
    NoRef(Millis),
}

impl fmt::Display for Duration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Duration::Ref {end, duration} => write!(f, "({}:{})", end, duration),
            Duration::NoRef(millis) => write!(f, "{}", millis)
        }
    }
}

// Broken out into a function to ensure the math is correct
pub fn has_elapsed(start: Millis, duration: Duration) -> bool {
    match duration {
        Duration::Ref { end, duration } => (end - start) >= duration,
        Duration::NoRef(duration) => (now() - start) >= duration,
    }
}

#[derive(Clone, Copy)]
pub enum TimeSpan {
    Milliseconds(u16),
    Seconds(u16),
    Minutes(u16),
    Hours(u8),
}

pub fn milliseconds(ts: TimeSpan) -> u32 {
    match ts {
        TimeSpan::Milliseconds(m) => m as u32,
        TimeSpan::Seconds(s) => s as u32 * 1000,
        TimeSpan::Minutes(m) => m as u32 * 60 * 1000,
        TimeSpan::Hours(h) => h as u32 * 60 * 60 * 1000,
    }
}

impl From<u8> for TimeSpan
{
    fn from(value: u8) -> Self {
        TimeSpan::Milliseconds(value as u16)
    }
}

impl From<u16> for TimeSpan
{
    fn from(value: u16) -> Self {
        TimeSpan::Milliseconds(value)
    }
}

pub struct Timer {
    start: Millis,
    duration: Duration,
}

impl Timer {
    pub fn new(timespan: TimeSpan) -> Self {
        Timer {
            start: now(),
            duration: Duration::NoRef(milliseconds(timespan) as u64),
        }
    }

    pub fn reset(&mut self) {
        self.start = now();
    }

    pub fn has_elapsed(&self) -> bool {
        has_elapsed(self.start, self.duration)
    }
}

impl fmt::Display for Timer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}:{})", self.start, self.duration)
    }
}