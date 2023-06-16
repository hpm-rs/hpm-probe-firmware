#![no_std]
#![no_main]

use core::cell::RefCell;
use core::fmt::Write;
use core::mem::MaybeUninit;
use core::panic::PanicInfo;
use core::sync::atomic::{self, Ordering};

pub use hpm_probe_bsp as bsp;
pub use hpm_ral as ral;

use bsp::clock::ClockConfigurator;
use bsp::delay::Delay;
use bsp::gpio::Gpio;
use bsp::uart::Uart;
use bsp::spi::Spi;
use critical_section::Mutex;
use hpm_rt::entry;

use git_version::git_version;

const GIT_VERSION: &str = git_version!();

const DAP1_PACKET_SIZE: u16 = 64;
const DAP2_PACKET_SIZE: u16 = 512;
const VCP_PACKET_SIZE: u16 = 512;

mod app;
mod dap;
mod jtag;
mod swd;
mod usb;
// mod vcp;

struct MyLogger<'a> {
    pub uart: Mutex<RefCell<Uart<'a, 0>>>,
}

impl<'a> log::Log for MyLogger<'a> {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        critical_section::with(|cs| {
            write!(
                self.uart.borrow_ref_mut(cs),
                "{} - {}\r\n",
                record.level(),
                record.args()
            )
                .unwrap()
        });
    }
    fn flush(&self) {}
}

unsafe impl<'a> Send for MyLogger<'a> {}

unsafe impl<'a> Sync for MyLogger<'a> {}

static mut LOGGER: MaybeUninit<MyLogger> = MaybeUninit::uninit();
static mut DMA: MaybeUninit<bsp::dma::DMA> = MaybeUninit::uninit();

#[entry]
unsafe fn main() -> ! {
    let gpio0 = unsafe { ral::gpio::GPIO0::instance() };
    let ioc = unsafe { ral::ioc::IOC0::instance() };
    let pioc = unsafe { ral::ioc::PIOC10::instance() };
    let sysctl = unsafe { ral::sysctl::SYSCTL::instance() };
    let pllctl = unsafe { ral::pllctl::PLLCTL::instance() };
    let mchtmr0 = unsafe { ral::mchtmr::MCHTMR::instance() };
    let hdma = unsafe { ral::dma::HDMA0::instance() };
    let dmamux = unsafe { ral::dmamux::DMAMUX::instance() };
    let uart0 = unsafe { ral::uart::UART0::instance() };
    let uart9 = unsafe { ral::uart::UART9::instance() };
    let spi1 = unsafe { ral::spi::SPI1::instance() };
    let spi2 = unsafe { ral::spi::SPI2::instance() };
    let usb = unsafe { ral::usb::USB0::instance() };

    let clk_cfgr = ClockConfigurator::new(sysctl, pllctl);
    let clocks = unsafe { clk_cfgr.freeze() };
    let delay = Delay::new(mchtmr0);

    DMA.write(bsp::dma::DMA::new(hdma, dmamux));

    let gpio = Gpio::new(gpio0, ioc, pioc);
    let pins = gpio.split();

    let mut uart0 = Uart::new(uart0, DMA.assume_init_ref());
    uart0.setup(&clocks);
    uart0.set_baud(115_200);
    uart0.start();
    unsafe {
        LOGGER.write(MyLogger {
            uart: Mutex::new(uart0.into()),
        });
        log::set_logger(LOGGER.assume_init_ref()).unwrap();
    }
    log::set_max_level(log::LevelFilter::Info);

    let swd_spi = Spi::new(spi1);
    let mut uart9 = Uart::new(uart9, DMA.assume_init_ref());
    let swd = swd::SWD::new(&swd_spi, &pins, &delay);

    let jtag_spi = Spi::new(spi2);
    let jtag = jtag::JTAG::new(&jtag_spi, DMA.assume_init_ref(), &pins, &delay);

    let mut dap = dap::DAP::new(swd, jtag, &mut uart9, &pins);

    let mut usb = usb::USB::new(usb);

    let mut app = app::App::new(&clocks, DMA.assume_init_ref(), &pins, &swd_spi, &jtag_spi, &mut usb, &mut dap, &delay);

    unsafe { app.setup("123456") };

    loop {
        app.poll();
    }
}

#[inline(never)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    log::error!("{info}");

    loop {
        atomic::compiler_fence(Ordering::SeqCst);
    }
}
