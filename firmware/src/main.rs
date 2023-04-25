#![no_std]
#![no_main]

mod app;

extern crate panic_halt;

pub use hpm_probe_bsp as bsp;
pub use hpm_ral as ral;

use bsp::clock::{ClockConfigurator, Clocks};
use bsp::delay::Delay;
use bsp::gpio::{Gpio, Pins};
use hpm_rt::entry;

#[entry]
fn main() -> ! {
    let gpio0 = unsafe { ral::gpio::GPIO0::instance() };
    let ioc = unsafe { ral::ioc::IOC0::instance() };
    let pioc = unsafe { ral::ioc::PIOC10::instance() };
    let sysctl = unsafe { ral::sysctl::SYSCTL::instance() };
    let pllctl = unsafe { ral::pllctl::PLLCTL::instance() };
    let mchtmr0 = unsafe { ral::mchtmr::MCHTMR::instance() };

    let clk_cfgr = ClockConfigurator::new(sysctl, pllctl);
    let clocks = unsafe { clk_cfgr.freeze() };

    let delay = Delay::new(mchtmr0);

    let gpio = Gpio::new(gpio0, ioc, pioc);
    let pins = gpio.split();

    let mut app = app::App::new(clocks, pins, delay);

    unsafe { app.setup() };

    loop {
        app.poll();
    }
}
