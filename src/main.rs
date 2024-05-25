#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]
#![allow(dead_code)]

mod millis;
mod debounce;

use embedded_hal::digital::PinState;
use panic_halt as _;
use crate::debounce::Debounce;
use crate::millis::{Timer, TimeSpan};
use avr_device::interrupt;
use core::cell::RefCell;
use ufmt::derive::{uDebug};

type Console = arduino_hal::hal::usart::Usart0<arduino_hal::DefaultClock>;

static CONSOLE: interrupt::Mutex<RefCell<Option<Console>>> =
    interrupt::Mutex::new(RefCell::new(None));

#[allow(unused_macros)]
macro_rules! print {
    ($($t:tt)*) => {
        interrupt::free(
            |cs| {
                if let Some(console) = CONSOLE.borrow(cs).borrow_mut().as_mut() {
                    let _ = ufmt::uwrite!(console, $($t)*);
                }
            },
        )
    };
}

#[allow(unused_macros)]
macro_rules! println {
    ($($t:tt)*) => {
        interrupt::free(
            |cs| {
                if let Some(console) = CONSOLE.borrow(cs).borrow_mut().as_mut() {
                    let _ = ufmt::uwriteln!(console, $($t)*);
                }
            },
        )
    };
}

fn put_console(console: Console) {
    interrupt::free(|cs| {
        *CONSOLE.borrow(cs).borrow_mut() = Some(console);
    })
}

// Listed in desired order.
#[derive(Clone, Copy)]
#[derive(uDebug)]
enum PowerState {
    Off,
    EnableSubwoofers(Timer),
    On,
    DisableMixer(Timer),
    RpiShutdown,
    PowerSignalLow,
}

impl PartialEq for PowerState
{
    fn eq(&self, other: &Self) -> bool {
        use PowerState::*;
        match (self, other) {
            (Off, Off) => true,
            (EnableSubwoofers(_), EnableSubwoofers(_)) => true,
            (On, On) => true,
            (DisableMixer(_), DisableMixer(_)) => true,
            (RpiShutdown, RpiShutdown) => true,
            (_, _) => false
        }
    }
}

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

    millis::init(dp.TC0);
    unsafe {
        interrupt::enable();
    }

    let serial = arduino_hal::default_serial!(dp, pins, 57600);
    put_console(serial);

    let pin_power_signal = pins.d2.into_pull_up_input();
    let mut power_signal = Debounce::new(pin_power_signal, None);

    let pin_rpi_signal = pins.d3.into_floating_input();
    let mut rpi_signal = Debounce::new(pin_rpi_signal, None);
    let mut pin_rpi_state = pins.d4.into_output();

    let mut pin_relay_mixer = pins.d5.into_output_high();
    let mut pin_relay_subwoofers = pins.d6.into_output_high();
    let mut pin_rpi_power = pins.d7.into_output();

    let mut power_state = PowerState::Off;

    let mut led_last = millis::now();
    let mut led = pins.d13.into_output();

    let mut ep = arduino_hal::Eeprom::new(dp.EEPROM);

    println!("Hardware setup complete.");

    let state_offset = 150;

    let mut state = [0u8; 1];
    if ep.read(state_offset, &mut state).is_err() || state[0] > 0
    {
        power_state = PowerState::PowerSignalLow;
    }

    match power_state {
        PowerState::Off => println!("State check complete."),
        _ => println!("Improper shutdown. Waiting for switch reset.")
    }

    loop {
        power_signal.tick();
        rpi_signal.tick();

        let now = millis::now();
        if now - led_last >= 1000
        {
            led.toggle();
            led_last = now;
        }

        let changed = power_signal.changed();
        let prev_state = power_state.clone();
        power_state = match power_state
        {
            // Power on
            PowerState::Off if changed == Some(PinState::High) => {
                power_signal.clear();

                ep.write_byte(state_offset, 1);
                pin_relay_mixer.set_low();
                pin_rpi_state.set_high();
                pin_rpi_power.set_high();
                PowerState::EnableSubwoofers(Timer::new(TimeSpan::Seconds(5)))
            }
            PowerState::Off => { power_state }
            PowerState::EnableSubwoofers(timer) if timer.has_elapsed() => {
                pin_relay_subwoofers.set_low();
                PowerState::On
            }
            PowerState::EnableSubwoofers(_) => { power_state }

            // Power off
            PowerState::On if changed == Some(PinState::Low) => {
                power_signal.clear();

                pin_relay_subwoofers.set_high();
                pin_rpi_state.set_low();

                // 250ms to let power stabilize a bit
                PowerState::DisableMixer(Timer::new(250u16.into()))
            }
            PowerState::On => { power_state }
            PowerState::DisableMixer(timer) if timer.has_elapsed() => {
                pin_relay_mixer.set_high();
                PowerState::RpiShutdown
            }
            PowerState::DisableMixer(_) => { power_state }
            PowerState::RpiShutdown if rpi_signal.state() == PinState::Low => {
                rpi_signal.clear();
                pin_rpi_power.set_low();
                ep.erase_byte(state_offset);
                PowerState::Off
            }
            PowerState::RpiShutdown => { power_state }

            // Clean up after abrupt power loss
            PowerState::PowerSignalLow if power_signal.state() == PinState::Low => {
                ep.erase_byte(state_offset);
                PowerState::Off
            }
            PowerState::PowerSignalLow => { power_state }
        };

        if prev_state != power_state
        {
            println!(
                "[{}] State change from {:?} to {:?}.",
                millis::now(),
                prev_state,
                power_state
            );
        }
    }
}
