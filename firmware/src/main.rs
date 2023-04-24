#![no_std]
#![no_main]

mod app;

extern crate panic_halt;

use hpm_probe_bsp as bsp;
use hpm_ral as ral;
use hpm_rt::entry;

use riscv::delay;

#[entry]
fn main() -> ! {
    let gpio0 = unsafe { ral::gpio::GPIO0::instance() };
    let ioc = unsafe { ral::ioc::IOC0::instance() };
    let pioc = unsafe { ral::ioc::PIOC10::instance() };
    let sysctl = unsafe { ral::sysctl::SYSCTL::instance() };
    let pllctl = unsafe { ral::pllctl::PLLCTL::instance() };

    let clk_cfgr = bsp::clock::ClockConfigurator::new(sysctl, pllctl);
    let clocks = unsafe { clk_cfgr.freeze() };

    let delay = delay::McycleDelay::new(clocks.get_clk_cpu0_freq());

    let gpio = bsp::gpio::Gpio::new(gpio0, ioc, pioc);
    let pins = gpio.split();

    let mut app = app::App::new(clocks, pins, delay);

    unsafe { app.setup() };

    loop {
        app.poll();
    }
}
