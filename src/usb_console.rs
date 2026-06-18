//! USB CDC command parser and console interface.
//!
//! The console accepts simple ASCII commands over USB serial and returns stable
//! machine-readable responses. This keeps the firmware easy to test from a
//! terminal, a Python script, or a production flashing tool.

use rp_pico::hal::usb::UsbBus;

use usb_device::{class_prelude::UsbBusAllocator, prelude::*};

use usbd_serial::SerialPort;

const READ_BUFFER_SIZE: usize = 64;
const LINE_BUFFER_SIZE: usize = 64;

pub enum ConsoleCommand {
    Help,
    Boot,
    Rgb { r: u8, g: u8, b: u8 },
    Off,
    InvalidArguments,
    CommandTooLong,
    Unknown,
}

pub struct UsbConsole<'a> {
    serial: SerialPort<'a, UsbBus>,
    usb_dev: UsbDevice<'a, UsbBus>,
    read_buf: [u8; READ_BUFFER_SIZE],
    line_buf: [u8; LINE_BUFFER_SIZE],
    line_len: usize,
}

impl<'a> UsbConsole<'a> {
    pub fn new(usb_bus: &'a UsbBusAllocator<UsbBus>) -> Self {
        let serial = SerialPort::new(usb_bus);

        let usb_dev = UsbDeviceBuilder::new(usb_bus, UsbVidPid(0x16c0, 0x27dd))
            .strings(&[StringDescriptors::default()
                .manufacturer("Rust Embedded")
                .product("RP2040 USB Control Console")
                .serial_number("RP2040-RUST-CONSOLE")])
            .unwrap()
            .device_class(usbd_serial::USB_CLASS_CDC)
            .build();

        Self {
            serial,
            usb_dev,
            read_buf: [0u8; READ_BUFFER_SIZE],
            line_buf: [0u8; LINE_BUFFER_SIZE],
            line_len: 0,
        }
    }

    pub fn poll(&mut self) -> Option<ConsoleCommand> {
        self.usb_dev.poll(&mut [&mut self.serial]);

        if !self.serial.dtr() {
            self.line_len = 0;
            return None;
        }

        match self.serial.read(&mut self.read_buf) {
            Ok(count) if count > 0 => {
                for index in 0..count {
                    if let Some(command) = self.push_byte(self.read_buf[index]) {
                        return Some(command);
                    }
                }
            }
            _ => {}
        }

        None
    }

    pub fn write_line(&mut self, data: &[u8]) {
        let _ = self.serial.write(data);
        let _ = self.serial.write(b"\r\n");
    }

    pub fn print_help(&mut self) {
        self.write_line(b"RP2040 USB Control Console");
        self.write_line(b"Language: Rust no_std");
        self.write_line(b"");
        self.write_line(b"Commands:");
        self.write_line(b"  HELP              Show command help");
        self.write_line(b"  BOOT              Reboot RP2040 into BOOTSEL mode");
        self.write_line(b"  RGB <r> <g> <b>   Set RGB LED color, each value 0..255");
        self.write_line(b"  OFF               Turn off RGB LED");
        self.write_line(b"");
        self.write_line(b"Responses:");
        self.write_line(b"  OK <COMMAND>");
        self.write_line(b"  ERR UNKNOWN_COMMAND");
        self.write_line(b"  ERR INVALID_ARGUMENTS");
        self.write_line(b"  ERR COMMAND_TOO_LONG");
    }

    fn push_byte(&mut self, byte: u8) -> Option<ConsoleCommand> {
        if byte == b'\r' {
            return None;
        }

        if byte == b'\n' {
            let command = self.process_line();
            self.line_len = 0;
            return command;
        }

        if self.line_len < self.line_buf.len() {
            self.line_buf[self.line_len] = byte;
            self.line_len += 1;
            return None;
        }

        self.line_len = 0;
        Some(ConsoleCommand::CommandTooLong)
    }

    fn process_line(&mut self) -> Option<ConsoleCommand> {
        if self.line_len == 0 {
            return None;
        }

        let line = &self.line_buf[..self.line_len];

        if eq_ignore_ascii_case(line, b"HELP") {
            Some(ConsoleCommand::Help)
        } else if eq_ignore_ascii_case(line, b"BOOT") {
            Some(ConsoleCommand::Boot)
        } else if eq_ignore_ascii_case(line, b"OFF") {
            Some(ConsoleCommand::Off)
        } else if starts_with_ignore_ascii_case(line, b"RGB") {
            match parse_rgb_command(line) {
                Some((r, g, b)) => Some(ConsoleCommand::Rgb { r, g, b }),
                None => Some(ConsoleCommand::InvalidArguments),
            }
        } else {
            Some(ConsoleCommand::Unknown)
        }
    }
}

fn eq_ignore_ascii_case(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    for index in 0..a.len() {
        if a[index].to_ascii_uppercase() != b[index].to_ascii_uppercase() {
            return false;
        }
    }

    true
}

fn starts_with_ignore_ascii_case(data: &[u8], prefix: &[u8]) -> bool {
    if data.len() < prefix.len() {
        return false;
    }

    eq_ignore_ascii_case(&data[..prefix.len()], prefix)
}

fn parse_rgb_command(command: &[u8]) -> Option<(u8, u8, u8)> {
    let mut index = 3;

    if index < command.len() && command[index] != b' ' {
        return None;
    }

    let r = parse_next_u8(command, &mut index)?;
    let g = parse_next_u8(command, &mut index)?;
    let b = parse_next_u8(command, &mut index)?;

    skip_spaces(command, &mut index);

    if index != command.len() {
        return None;
    }

    Some((r, g, b))
}

fn parse_next_u8(data: &[u8], index: &mut usize) -> Option<u8> {
    skip_spaces(data, index);

    if *index >= data.len() {
        return None;
    }

    let mut value: u16 = 0;
    let mut has_digit = false;

    while *index < data.len() {
        let byte = data[*index];

        if !byte.is_ascii_digit() {
            break;
        }

        has_digit = true;
        value = value * 10 + ((byte - b'0') as u16);

        if value > 255 {
            return None;
        }

        *index += 1;
    }

    if !has_digit {
        return None;
    }

    Some(value as u8)
}

fn skip_spaces(data: &[u8], index: &mut usize) {
    while *index < data.len() && data[*index] == b' ' {
        *index += 1;
    }
}
