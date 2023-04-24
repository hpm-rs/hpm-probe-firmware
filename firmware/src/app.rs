use hpm_probe_bsp as bsp;

use embedded_hal::blocking::delay::DelayMs;
use riscv::delay;

pub struct App<'a> {
    clocks: bsp::clock::Clocks,
    pins: bsp::gpio::Pins<'a>,
    delay: delay::McycleDelay,
}

impl<'a> App<'a> {
    pub fn new(
        clocks: bsp::clock::Clocks,
        pins: bsp::gpio::Pins<'a>,
        delay: delay::McycleDelay,
    ) -> Self {
        App {
            clocks,
            pins,
            delay,
        }
    }

    pub unsafe fn setup(&self) {
        // Configure GPIOs
        self.pins.setup();
    }

    pub fn poll(&mut self) {
        self.pins.led_b.toggle();
        self.delay.delay_ms(100);
    }
}
