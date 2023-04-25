use crate::bsp::clock::Clocks;
use crate::bsp::delay::Delay;
use crate::bsp::gpio::Pins;

pub struct App<'a> {
    clocks: Clocks,
    pins: Pins<'a>,
    delay: Delay,
}

impl<'a> App<'a> {
    pub fn new(clocks: Clocks, pins: Pins<'a>, delay: Delay) -> Self {
        App {
            clocks,
            pins,
            delay,
        }
    }

    pub unsafe fn setup(&self) {
        // Configure GPIOs
        self.pins.setup();

        self.delay.set_base_clock(&self.clocks);
    }

    pub fn poll(&mut self) {
        self.pins.led_b.toggle();
        self.delay.delay_us(100 * 1000);
    }
}
