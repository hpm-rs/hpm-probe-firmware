#![allow(unused)]

use hpm_ral::{gpio, ioc};
use hpm_ral::{modify_reg, read_reg, write_reg};

#[derive(Clone, Copy)]
pub enum PinState {
    Low = 0,
    High,
}

pub enum Pull {
    PullDown = 0,
    PullUp,
    Floating,
}

pub struct Pin<'a, const PORT: char, const PIN: u8> {
    gpio: &'a gpio::GPIO0,
    ioc: &'a ioc::IOC0,
    pioc: &'a ioc::PIOC10,
}

macro_rules! impl_port {
    ($port:literal, $OE_VALUE:ident, $DO_SET:ident, $DO_CLEAR:ident, $DO_TOGGLE:ident, $DI_VALUE:ident) => {
        impl<'a, const PIN: u8> Pin<'a, $port, PIN> {
            #[inline]
            pub fn set_mode_output(&self) -> &Self {
                let offset = PIN;
                let mask = 0b1 << offset;
                modify_reg!(gpio, self.gpio, $OE_VALUE, |r| r | mask);
                self
            }

            #[inline]
            pub fn set_mode_input(&self) -> &Self {
                let offset = PIN;
                let mask = 0b1 << offset;
                modify_reg!(gpio, self.gpio, $OE_VALUE, |r| r & !mask);
                self
            }

            #[inline]
            fn set_high(&self) -> &Self {
                write_reg!(gpio, self.gpio, $DO_SET, 1 << PIN);
                self
            }

            #[inline]
            fn set_low(&self) -> &Self {
                write_reg!(gpio, self.gpio, $DO_CLEAR, 1 << PIN);
                self
            }

            #[inline]
            pub fn set_state(&self, state: PinState) -> &Self {
                match state {
                    PinState::Low => self.set_low(),
                    PinState::High => self.set_high(),
                }
            }

            #[inline]
            pub fn toggle(&self) -> &Self {
                write_reg!(gpio, self.gpio, $DO_TOGGLE, 1 << PIN);
                self
            }

            #[inline]
            pub fn get_sate(&self) -> PinState {
                match read_reg!(gpio, self.gpio, $DI_VALUE) >> PIN & 0b1 {
                    0 => PinState::Low,
                    1 => PinState::High,
                    _ => unreachable!(),
                }
            }

            #[inline]
            pub fn is_high(&self) -> bool {
                match self.get_sate() {
                    PinState::Low => false,
                    PinState::High => true,
                }
            }

            #[inline]
            pub fn is_low(&self) -> bool {
                match self.get_sate() {
                    PinState::Low => true,
                    PinState::High => false,
                }
            }
        }
    };
}

macro_rules! pin {
    ($PXX:ident: $port:literal, $pin:literal, $FUNC_CTL:ident, $PAD_CTL:ident) => {
        pub type $PXX<'a> = Pin<'a, $port, $pin>;

        impl<'a> $PXX<'a> {
            // For each pin
            fn new(gpio: &'a gpio::GPIO0, ioc: &'a ioc::IOC0, pioc: &'a ioc::PIOC10) -> Self {
                Pin { gpio, ioc, pioc }
            }

            #[inline]
            pub fn set_af(&self, alt: u32) -> &Self {
                assert!(alt < 32);
                modify_reg!(ioc, self.ioc, $FUNC_CTL, ALT_SELECT: alt);
                self
            }

            #[inline]
            pub fn set_push_pull(&self) -> &Self {
                modify_reg!(ioc, self.ioc, $PAD_CTL, OD: Disable);
                self
            }

            #[inline]
            pub fn set_open_drain(&self) -> &Self {
                modify_reg!(ioc, self.ioc, $PAD_CTL, OD: Enable);
                self
            }

            #[inline]
            pub fn set_pull(&self, pull: Pull) -> &Self {
                match pull {
                    Pull::Floating => modify_reg!(ioc, self.ioc, $PAD_CTL, PE: Disable),
                    _ => modify_reg!(ioc, self.ioc, $PAD_CTL, PE: Enable, PS: pull as u32),
                }
                self
            }

            #[inline]
            pub fn set_pull_down(&self) -> &Self {
                self.set_pull(Pull::PullDown)
            }

            #[inline]
            pub fn set_pull_up(&self) -> &Self {
                self.set_pull(Pull::PullUp)
            }

            #[inline]
            pub fn set_pull_floating(&self) -> &Self {
                self.set_pull(Pull::Floating)
            }
        }
    };
}

macro_rules! pins {
    ($(
        $port:literal: {
            $OE_VALUE:ident,
            $DO_SET:ident,
            $DO_CLEAR:ident,
            $DO_TOGGLE:ident,
            $DI_VALUE:ident,
            [$(($PXX:ident, $pxx:ident, $pin:literal, $FUNC_CTL:ident, $PAD_CTL:ident)),*]
        }
    ),*) => {
        $(
            impl_port!($port, $OE_VALUE, $DO_SET, $DO_CLEAR, $DO_TOGGLE, $DI_VALUE);

            $(pin!($PXX: $port, $pin, $FUNC_CTL, $PAD_CTL);)*
        )*

        pub struct Pins<'a> {
            $(
                $(pub $pxx: $PXX<'a>,)*
            )*
        }

        impl<'a> Pins<'a> {
            pub fn new(gpio: &'a gpio::GPIO0, ioc: &'a ioc::IOC0, pioc: &'a ioc::PIOC10) -> Self {
                Pins {
                    $(
                        $($pxx: $PXX::new(&gpio, &ioc, &pioc),)*
                    )*
                }
            }
        }
    };
}

pins!(
    'B': {
        OE_GPIOB_VALUE,
        DO_GPIOB_SET, DO_GPIOB_CLEAR, DO_GPIOB_TOGGLE,
        DI_GPIOB_VALUE,
        [
            (PB18, led_g,  18, PAD_PB18_FUNC_CTL, PAD_PB18_PAD_CTL),
            (PB19, led_r,  19, PAD_PB19_FUNC_CTL, PAD_PB19_PAD_CTL),
            (PB20, led_b,  20, PAD_PB20_FUNC_CTL, PAD_PB20_PAD_CTL)
        ]
    },
    'C': {
        OE_GPIOC_VALUE,
        DO_GPIOC_SET, DO_GPIOC_CLEAR, DO_GPIOC_TOGGLE,
        DI_GPIOC_VALUE,
        [
            (PC03, pc03,  3, PAD_PC03_FUNC_CTL, PAD_PC03_PAD_CTL)
        ]
    },
    'D': {
        OE_GPIOD_VALUE,
        DO_GPIOD_SET, DO_GPIOD_CLEAR, DO_GPIOD_TOGGLE,
        DI_GPIOD_VALUE,
        [
            (PD14, pd14, 14, PAD_PD14_FUNC_CTL, PAD_PD14_PAD_CTL),
            (PD15, pd15, 15, PAD_PD15_FUNC_CTL, PAD_PD15_PAD_CTL)
        ]
    }
);

pub struct Gpio {
    gpio: gpio::GPIO0,
    ioc: ioc::IOC0,
    pioc: ioc::PIOC10,
}

impl Gpio {
    pub fn new(gpio: gpio::GPIO0, ioc: ioc::IOC0, pioc: ioc::PIOC10) -> Self {
        Self { gpio, ioc, pioc }
    }

    pub fn split(&self) -> Pins {
        Pins::new(&self.gpio, &self.ioc, &self.pioc)
    }
}

impl<'a> Pins<'a> {
    pub fn setup(&self) {
        self.led_r.set_af(0).set_mode_output().set_high();
        self.led_g.set_af(0).set_mode_output().set_high();
        self.led_b.set_af(0).set_mode_output().set_high();
    }
}
