//! RP2040 USB Control Console Firmware
//!
//! This firmware exposes a small USB CDC command interface for RP2040-based
//! boards. It currently supports RGB status LED control and software-triggered
//! BOOTSEL reset for fast development and flashing.
//!
//! Target: RP2040 / Raspberry Pi Pico compatible boards
//! Language: Rust, no_std

#![no_std]
#![no_main]

mod status_led;
mod usb_console;

use panic_halt as _;

use rp_pico::entry;
use rp_pico::hal::{
    clocks::init_clocks_and_plls, gpio::FunctionPio0, pac, rom_data::reset_to_usb_boot, sio::Sio,
    usb::UsbBus, watchdog::Watchdog,
};

use usb_console::{ConsoleCommand, UsbConsole};

use usb_device::class_prelude::UsbBusAllocator;

const EXTERNAL_XTAL_FREQ_HZ: u32 = 12_000_000;
const STATUS_LED_PIN_NUM: u8 = 16;

#[entry]
fn main() -> ! {
    let mut pac = pac::Peripherals::take().unwrap();
    let mut watchdog = Watchdog::new(pac.WATCHDOG);

    let clocks = init_clocks_and_plls(
        EXTERNAL_XTAL_FREQ_HZ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let sio = Sio::new(pac.SIO);

    let pins = rp_pico::hal::gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let led_pin = pins.gpio16.into_function::<FunctionPio0>();

    let mut status_led =
        status_led::StatusLed::new(pac.PIO0, &mut pac.RESETS, led_pin, STATUS_LED_PIN_NUM);

    status_led.off();

    let usb_bus = UsbBusAllocator::new(UsbBus::new(
        pac.USBCTRL_REGS,
        pac.USBCTRL_DPRAM,
        clocks.usb_clock,
        true,
        &mut pac.RESETS,
    ));

    let mut console = UsbConsole::new(&usb_bus);

    loop {
        if let Some(command) = console.poll() {
            handle_console_command(command, &mut console, &mut status_led);
        }
    }
}

fn handle_console_command(
    command: ConsoleCommand,
    console: &mut UsbConsole,
    status_led: &mut status_led::StatusLed,
) {
    match command {
        ConsoleCommand::Help => {
            console.print_help();
        }

        ConsoleCommand::Boot => {
            console.write_line(b"OK BOOT");
            delay_before_usb_reset();
            reset_to_usb_boot(0, 0);
        }

        ConsoleCommand::Rgb { r, g, b } => {
            status_led.set_rgb(r, g, b);
            console.write_line(b"OK RGB");
        }

        ConsoleCommand::Off => {
            status_led.off();
            console.write_line(b"OK OFF");
        }

        ConsoleCommand::InvalidArguments => {
            console.write_line(b"ERR INVALID_ARGUMENTS");
        }

        ConsoleCommand::CommandTooLong => {
            console.write_line(b"ERR COMMAND_TOO_LONG");
        }

        ConsoleCommand::Unknown => {
            console.write_line(b"ERR UNKNOWN_COMMAND");
        }
    }
}

fn delay_before_usb_reset() {
    for _ in 0..200_000 {
        cortex_m::asm::nop();
    }
}
