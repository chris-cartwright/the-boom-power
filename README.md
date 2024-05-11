the-boom-power
==============

Rust project for the _Arduino Nano_.

## Design

Intended as a safe way to automatically power my audio stack up or down.

Subwoofer crossover and amplifier must be turned on after the mixer. The mixer and Raspberry Pi do not have power
switches. Additionally, it would be nice for the system to stay powered off after power loss.

### Notes
- Monitors will be always on; use power switch on back.
- Record player will always be powered.
- Subwoofers refers to both the amplifier and cross over.
  - Some testing shows powering both on or off at the same time should be fine.

## Desired sequence of events:

### Power on:
- Turn on mixer.
- Set up signal
- Power on Raspberry Pi.
- Wait for things to settle; 5s?
- Turn on subwoofers.

### Power off:
- Signal to Raspberry Pi to shut down.
- Turn off subwoofers
- Wait 1s for everything to stabilize
- Turn off mixer
- Wait for Raspberry Pi to signal shutdown
- Cut power to Raspberry Pi 

## Build Instructions
1. Install prerequisites as described in the [`avr-hal` README] (`avr-gcc`, `avr-libc`, `avrdude`, [`ravedude`]).

2. Run `cargo build` to build the firmware.

3. Run `cargo run` to flash the firmware to a connected board.  If `ravedude`
   fails to detect your board, check its documentation at
   <https://crates.io/crates/ravedude>.

4. `ravedude` will open a console session after flashing where you can interact
   with the UART console of your board.

[`avr-hal` README]: https://github.com/Rahix/avr-hal#readme
[`ravedude`]: https://crates.io/crates/ravedude