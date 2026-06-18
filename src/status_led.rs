//! PIO-based RGB status LED driver for RP2040.
//!
//! The implementation is designed for single-wire addressable LEDs commonly
//! used on RP2040 boards and toolhead electronics, such as WS2812-compatible
//! RGB LEDs. The public API intentionally stays small and hardware-focused.

use pio_proc::pio_asm;

use rp_pico::hal::{
    gpio::{FunctionPio0, Pin, PinId, PullType},
    pac::PIO0,
    pio::{PIOBuilder, PIOExt, PinDir, ShiftDirection, Tx, SM0},
};

pub struct StatusLed {
    tx: Tx<(PIO0, SM0)>,
}

impl StatusLed {
    pub fn new<I, P>(
        pio0: PIO0,
        resets: &mut rp_pico::hal::pac::RESETS,
        pin: Pin<I, FunctionPio0, P>,
        pin_num: u8,
    ) -> Self
    where
        I: PinId,
        P: PullType,
    {
        let program = pio_asm!(
            ".side_set 1",
            ".define public T1 2",
            ".define public T2 5",
            ".define public T3 3",
            ".wrap_target",
            "bitloop:",
            "out x, 1 side 0 [T3 - 1]",
            "jmp !x do_zero side 1 [T1 - 1]",
            "do_one:",
            "jmp bitloop side 1 [T2 - 1]",
            "do_zero:",
            "nop side 0 [T2 - 1]",
            ".wrap",
        );

        let (mut pio, sm0, _, _, _) = pio0.split(resets);
        let installed = pio.install(&program.program).unwrap();

        let (mut sm, _, tx) = PIOBuilder::from_program(installed)
            .set_pins(pin_num, 1)
            .side_set_pin_base(pin_num)
            .out_shift_direction(ShiftDirection::Left)
            .autopull(true)
            .pull_threshold(24)
            .clock_divisor_fixed_point(15, 160)
            .build(sm0);

        sm.set_pindirs([(pin.id().num, PinDir::Output)]);
        sm.start();

        Self { tx }
    }

    pub fn set_rgb(&mut self, r: u8, g: u8, b: u8) {
        let rgb_word = pack_rgb(r, g, b);
        self.tx.write(rgb_word << 8);
    }

    pub fn off(&mut self) {
        self.set_rgb(0, 0, 0);
    }
}

fn pack_rgb(r: u8, g: u8, b: u8) -> u32 {
    ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}
