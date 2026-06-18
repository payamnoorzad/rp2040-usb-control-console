![Language](https://img.shields.io/badge/language-Rust-orange)
![Target](https://img.shields.io/badge/target-RP2040-blue)
![License](https://img.shields.io/badge/license-MIT-green)

# RP2040 USB Control Console

A small **Rust `no_std` firmware** for RP2040-based boards. It exposes a USB CDC serial command console that can control a status RGB LED and reboot the board into BOOTSEL mode for fast flashing.

This project is intended as a clean embedded Rust starting point for board-control firmware, toolhead electronics, test fixtures, and automation workflows.

## Features

- Written in Rust for RP2040 / Raspberry Pi Pico compatible boards
- `no_std` firmware using `rp-pico`, `rp2040-hal`, USB CDC, and PIO
- USB serial command console
- RGB status LED control through PIO
- Software BOOTSEL command for fast development flashing
- Windows PowerShell flashing helper script
- Simple, stable command responses for scripts and production tools

## Supported Commands

Commands are ASCII text and should be sent over the USB CDC serial port.

| Command | Description | Example |
| --- | --- | --- |
| `HELP` | Print supported commands and response format | `HELP` |
| `BOOT` | Reboot RP2040 into BOOTSEL mode | `BOOT` |
| `RGB <r> <g> <b>` | Set RGB LED color. Each value must be `0..255` | `RGB 255 0 0` |
| `OFF` | Turn off the RGB LED | `OFF` |

Successful responses use:

```text
OK <COMMAND>
```

Error responses use:

```text
ERR UNKNOWN_COMMAND
ERR INVALID_ARGUMENTS
ERR COMMAND_TOO_LONG
```

## Hardware

Default configuration:

| Item | Value |
| --- | --- |
| MCU | RP2040 |
| Board family | Raspberry Pi Pico compatible |
| External crystal | 12 MHz |
| RGB LED pin | GPIO16 |
| LED driver | PIO0 / SM0 |
| USB interface | USB CDC serial |

If your board uses a different RGB LED pin, update `STATUS_LED_PIN_NUM` and the selected GPIO in `src/main.rs`.

## Project Structure

```text
.
├── .cargo/config.toml       # Cargo target and elf2uf2 runner
├── Cargo.toml               # Rust crate metadata and dependencies
├── memory.x                 # RP2040 linker memory layout
├── scripts/flash.ps1        # Windows flashing helper
└── src
    ├── main.rs              # Firmware entry point and command handling
    ├── status_led.rs        # PIO-based RGB LED driver
    └── usb_console.rs       # USB CDC console and command parser
```

## Requirements

Install the Rust embedded target:

```powershell
rustup target add thumbv6m-none-eabi
```

Install `elf2uf2-rs`:

```powershell
cargo install elf2uf2-rs
```

## Build

```powershell
cargo build
```

## Flash

If the board is already in BOOTSEL mode:

```powershell
cargo run
```

For normal development, after the firmware has already been flashed once, use:

```powershell
.\scripts\flash.ps1
```

The script sends the `BOOT` command over USB CDC, waits for the `RPI-RP2` drive, and then runs `cargo run`.

Useful options:

```powershell
.\scripts\flash.ps1 -Port COM8
.\scripts\flash.ps1 -NoBuild
.\scripts\flash.ps1 -TimeoutSeconds 30
.\scripts\flash.ps1 -Help
```

## Serial Usage

Open the USB serial port with any terminal program and send commands with CRLF line ending.

Example:

```text
HELP
RGB 0 0 255
OFF
BOOT
```

## Roadmap

Planned improvements:

- Add firmware version command
- Add board information command
- Add structured command parser tests where possible
- Add optional GRB/RGB LED color order selection
- Add protocol documentation for host-side tools

## License

Licensed under the MIT license.
