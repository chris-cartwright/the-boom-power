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

// Listed in desired order.
enum PowerState {
    Off,
    EnableSubwoofers(Timer),
    On,
    DisableMixer(Timer),
    RpiShutdown,
}

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

    millis::init(dp.TC0);
    unsafe {
        avr_device::interrupt::enable();
    }

    let pin_power_signal = pins.d2.into_pull_up_input();
    let mut power_signal = Debounce::new(pin_power_signal, None);

    let pin_rpi_signal = pins.d3.into_floating_input();
    let mut rpi_signal = Debounce::new(pin_rpi_signal, None);
    let mut pin_rpi_state = pins.d4.into_output();

    let mut pin_relay_mixer = pins.d5.into_output();
    let mut pin_relay_subwoofers = pins.d6.into_output();
    let mut pin_rpi_power = pins.d7.into_output();

    let mut power_state = PowerState::Off;

    let mut led_last = millis::now();
    let mut led = pins.d13.into_output();

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
        power_state = match power_state
        {
            // Power on
            PowerState::Off if changed == Some(PinState::High) => {
                power_signal.clear();

                pin_relay_mixer.set_high();
                pin_rpi_state.set_high();
                pin_rpi_power.set_high();
                PowerState::EnableSubwoofers(Timer::new(TimeSpan::Seconds(5)))
            }
            PowerState::Off => { power_state }
            PowerState::EnableSubwoofers(timer) if timer.has_elapsed() => {
                pin_relay_subwoofers.set_high();
                PowerState::On
            }
            PowerState::EnableSubwoofers(_) => { power_state }

            // Power off
            PowerState::On if changed == Some(PinState::Low) => {
                power_signal.clear();

                pin_relay_subwoofers.set_low();
                pin_rpi_state.set_low();

                // 250ms to let power stabilize a bit
                PowerState::DisableMixer(Timer::new(250u16.into()))
            }
            PowerState::On => { power_state }
            PowerState::DisableMixer(timer) if timer.has_elapsed() => {
                pin_relay_mixer.set_low();
                PowerState::RpiShutdown
            }
            PowerState::DisableMixer(_) => { power_state }
            PowerState::RpiShutdown if rpi_signal.changed() == Some(PinState::Low) => {
                rpi_signal.clear();
                pin_rpi_power.set_low();
                PowerState::Off
            }
            PowerState::RpiShutdown => { power_state }
        };
    }
}
