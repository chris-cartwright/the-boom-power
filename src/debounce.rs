use arduino_hal::port::{Pin, mode::Input, PinOps};
use arduino_hal::port::mode::InputMode;
use embedded_hal::digital::PinState;
use crate::millis::{Timer};

pub struct Debounce<MODE, PIN> {
    pin: Pin<Input<MODE>, PIN>,
    settle_delay: u16,
    state: PinState,
    last_change: Option<Timer>,
    change: Option<PinState>
}

impl<MODE: InputMode, PIN: PinOps> Debounce<MODE, PIN> {
    pub fn new(pin: Pin<Input<MODE>, PIN>, settle_delay: Option<u16>) -> Self {
        let state = PinState::from(pin.is_high());
        Debounce {
            pin,
            settle_delay: settle_delay.unwrap_or(10u16),
            state,
            last_change: None,
            change: None
        }
    }

    pub fn tick(&mut self) {
        let current = PinState::from(self.pin.is_high());
        if current == self.state
        {
            match &self.last_change {
                None => {}
                Some(_) => { self.last_change = None; }
            }
            return;
        }

        match &self.last_change {
            None => {
                self.last_change = Some(Timer::new(self.settle_delay.into()))
            }
            Some(timer) if timer.has_elapsed() => {
                self.change = Some(current);
                self.state = current;
                self.last_change = None
            }
            Some(_) => {}
        }
    }

    pub fn state(&self) -> PinState
    {
        self.state
    }

    pub fn changed(&mut self) -> Option<PinState> {
        self.change
    }

    pub fn clear(&mut self) {
        self.change = None;
    }
}