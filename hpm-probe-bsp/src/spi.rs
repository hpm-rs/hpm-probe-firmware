// Copyright 2019 Adam Greig
// Dual licensed under the Apache 2.0 and MIT licenses.

use core::sync::atomic::{AtomicU32, Ordering};

use crate::ral::spi;
use crate::ral::{modify_reg, read_reg, write_reg};

use crate::clock::Clocks;
use crate::delay::Delay;
use crate::gpio::Pins;
use crate::dma::DMA;

pub struct Spi<const N: u8> {
    spi: spi::Instance<N>,
    base_clock: AtomicU32,
}

#[repr(u32)]
#[derive(Copy, Clone, Debug)]
pub enum SPIPrescaler {
    Div2 = 0b000,
    Div4 = 0b001,
    Div8 = 0b010,
    Div16 = 0b011,
    Div32 = 0b100,
    Div64 = 0b101,
    Div128 = 0b110,
    Div256 = 0b111,
}

impl<const N: u8> Spi<N> {
    pub fn new(spi: spi::Instance<N>) -> Self {
        Spi {
            spi,
            base_clock: AtomicU32::new(0),
        }
    }

    /// Set up SPI peripheral for SWD mode.
    ///
    /// Defaults to 1.5MHz clock which should be slow enough to work on most targets.
    pub fn setup_swd(&self) {
        // Timing config
        let sclk_div = (self.base_clock.load(Ordering::SeqCst) / 1_500_000) / 2 - 1;
        write_reg!(
            spi,
            self.spi,
            TIMING,
            SCLK_DIV: sclk_div,
            CSHT: 12 - 1,
            CS2SCLK: 4 - 1
        );
        // Format config
        write_reg!(
            spi,
            self.spi,
            TRANSFMT,
            DATALEN: 8 - 1,
            DATAMERGE: 0,
            MOSIBIDIR: UniDirectional,
            LSB: Lsb,
            SLVMODE: Master,
            CPOL: High,
            CPHA: Odd
        );
        // Transfer control
        write_reg!(
            spi,
            self.spi,
            TRANSCTRL,
            SLVDATAONLY: Disable,
            CMDEN: Disable,
            ADDREN: Disable,
            TRANSMODE: ReadWhileWrite,
            DUALQUAD: Single,
            TOKENEN: Disable
        );
    }

    /// Set up SPI peripheral for JTAG mode
    pub fn setup_jtag(&self) {
        // Timing config
        let sclk_div = (self.base_clock.load(Ordering::SeqCst) / 1_500_000) / 2 - 1;
        write_reg!(
            spi,
            self.spi,
            TIMING,
            SCLK_DIV: sclk_div,
            CSHT: 12 - 1,
            CS2SCLK: 4 - 1
        );
        // Format config
        write_reg!(
            spi,
            self.spi,
            TRANSFMT,
            DATALEN: 8 - 1,
            DATAMERGE: 0,
            MOSIBIDIR: UniDirectional,
            LSB: Lsb,
            SLVMODE: Master,
            CPOL: Low,
            CPHA: Odd
        );
        // Transfer control
        write_reg!(
            spi,
            self.spi,
            TRANSCTRL,
            SLVDATAONLY: Disable,
            CMDEN: Disable,
            ADDREN: Disable,
            TRANSMODE: ReadWhileWrite,
            DUALQUAD: Single,
            TOKENEN: Disable
        );
        write_reg!(spi, self.spi, CTRL, TXDMAEN: Enable, RXDMAEN: Enable);
    }

    pub fn calculate_prescaler(&self, max_frequency: u32) -> Option<SPIPrescaler> {
        let base_clock = self.base_clock.load(Ordering::SeqCst);
        if base_clock == 0 {
            return None;
        }

        if (base_clock / 4) <= max_frequency {
            return Some(SPIPrescaler::Div2);
        }
        if (base_clock / 8) <= max_frequency {
            return Some(SPIPrescaler::Div4);
        }
        if (base_clock / 16) <= max_frequency {
            return Some(SPIPrescaler::Div8);
        }
        if (base_clock / 32) <= max_frequency {
            return Some(SPIPrescaler::Div16);
        }
        if (base_clock / 64) <= max_frequency {
            return Some(SPIPrescaler::Div32);
        }
        if (base_clock / 128) <= max_frequency {
            return Some(SPIPrescaler::Div64);
        }
        if (base_clock / 256) <= max_frequency {
            return Some(SPIPrescaler::Div128);
        }
        if (base_clock / 512) <= max_frequency {
            return Some(SPIPrescaler::Div256);
        }
        None
    }

    /// Change SPI clock rate to one of the SPIClock variants
    pub fn set_prescaler(&self, prescaler: SPIPrescaler) {
        // modify_reg!(spi, self.spi, TIMING, SCLK_DIV: prescaler as u32);
    }

    /// Wait for any pending operation then disable SPI
    pub fn disable(&self) {
        self.wait_busy();
    }

    /// Transmit `txdata` and write the same number of bytes into `rxdata`.
    pub fn jtag_exchange(&self, dma: &DMA, txdata: &[u8], rxdata: &mut [u8]) {
        debug_assert!(rxdata.len() >= 64);

        // Set up DMA transfer (configures NDTR and MAR and enables streams)
        dma.spi2_enable(txdata, &mut rxdata[..txdata.len()]);

        // Busy wait for RX DMA completion (at most 43µs)
        while dma.spi2_busy() {}

        // Disable DMA
        dma.spi2_disable();
    }

    /// Transmit 4 bits
    pub fn tx4(&self, data: u8) {
        modify_reg!(spi, self.spi, TRANSFMT, DATALEN: 4 - 1);
        modify_reg!(spi, self.spi, TRANSCTRL, WRTRANCNT: 0, RDTRANCNT: 0);
        // Write dummy command to start transfer
        write_reg!(spi, &self.spi, CMD, 0xff);

        self.write_dr_u8(data);
        self.wait_rxne();
    }

    /// Transmit 8 bits
    pub fn tx8(&self, data: u8) {
        modify_reg!(spi, self.spi, TRANSFMT, DATALEN: 8 - 1);
        modify_reg!(spi, self.spi, TRANSCTRL, WRTRANCNT: 0, RDTRANCNT: 0);
        // Write dummy command to start transfer
        write_reg!(spi, &self.spi, CMD, 0xff);

        self.write_dr_u8(data);
        self.wait_rxne();
    }

    /// Transmit 16 bits
    pub fn tx16(&self, data: u16) {
        modify_reg!(spi, self.spi, TRANSFMT, DATALEN: 16 - 1);
        modify_reg!(spi, self.spi, TRANSCTRL, WRTRANCNT: 0, RDTRANCNT: 0);
        // Write dummy command to start transfer
        write_reg!(spi, &self.spi, CMD, 0xff);

        self.write_dr_u16(data);
        self.wait_rxne();
    }

    /// Transmit an SWD WDATA phase, with 32 bits of data and 1 bit of parity.
    ///
    /// We transmit an extra 7 trailing idle bits after the parity bit because
    /// it's much quicker to do that than reconfigure SPI to a smaller data size.
    pub fn swd_wdata_phase(&self, data: u32, parity: u8) {
        modify_reg!(spi, self.spi, TRANSFMT, DATALEN: 8 - 1);
        modify_reg!(spi, self.spi, TRANSCTRL, WRTRANCNT: 4, RDTRANCNT: 4);
        // Write dummy command to start transfer
        write_reg!(spi, &self.spi, CMD, 0xff);
        // Trigger 4 words, filling the FIFO
        for i in 0..4 {
            self.write_dr_u8(((data >> i * 8) & 0xFF) as u8);
        }
        // Trigger fifth and final word
        self.write_dr_u8(parity & 1);
        for _ in 0..5 {
            self.read_dr_u8();
        }
    }

    /// Receive 4 bits
    pub fn rx4(&self) -> u8 {
        modify_reg!(spi, self.spi, TRANSFMT, DATALEN: 4 - 1);
        modify_reg!(spi, self.spi, TRANSCTRL, WRTRANCNT: 0, RDTRANCNT: 0);
        // Write dummy command to start transfer
        write_reg!(spi, &self.spi, CMD, 0xff);

        self.write_dr_u8(0);
        self.wait_rxne();
        self.read_dr_u8()
    }

    /// Receive 5 bits
    pub fn rx5(&self) -> u8 {
        modify_reg!(spi, self.spi, TRANSFMT, DATALEN: 5 - 1);
        modify_reg!(spi, self.spi, TRANSCTRL, WRTRANCNT: 0, RDTRANCNT: 0);
        // Write dummy command to start transfer
        write_reg!(spi, &self.spi, CMD, 0xff);

        self.write_dr_u8(0);
        self.wait_rxne();
        self.read_dr_u8()
    }

    /// Receive an SWD RDATA phase, with 32 bits of data and 1 bit of parity.
    ///
    /// This method requires `Pins` be passed in so it can directly control
    /// the SWD lines at the end of RDATA in order to correctly sample PARITY
    /// and then resume driving SWDIO.
    pub fn swd_rdata_phase(&self, pin: &Pins, delay: &Delay) -> (u32, u8) {
        modify_reg!(spi, self.spi, TRANSFMT, DATALEN: 8 - 1);
        modify_reg!(spi, self.spi, TRANSCTRL, WRTRANCNT: 4 - 1, RDTRANCNT: 4 - 1);
        // Write dummy command to start transfer
        write_reg!(spi, &self.spi, CMD, 0xff);

        // Trigger 4 words, filling the FIFO
        self.write_dr_u8(0xff);
        self.write_dr_u8(0xff);
        self.write_dr_u8(0xff);
        self.write_dr_u8(0xff);

        let mut data = self.read_dr_u8() as u32;
        data |= (self.read_dr_u8() as u32) << 8;
        data |= (self.read_dr_u8() as u32) << 16;

        // While we wait for the final word to be available in the RXFIFO,
        // handle the parity bit. First wait for current transaction to complete.
        self.wait_rxne();

        // The parity bit is currently being driven onto the bus by the target.
        // On the next rising edge, the target will release the bus, and we need
        // to then start driving it before sending any more clocks to avoid a false START.
        // Take direct control of SWCLK
        pin.swd_clk_direct();
        for _ in 0..2 {
            pin.spi1_clk.set_low();
            delay.delay_ticks(5);
            pin.spi1_clk.set_high();
            delay.delay_ticks(5);
        }
        // Restore SWCLK to SPI control
        pin.swd_clk_spi();

        // Now read the final data word that was waiting in RXFIFO
        data |= (self.read_dr_u8() as u32) << 24;

        (data, 0)
    }

    /// Empty the receive FIFO
    pub fn drain(&self) {
        modify_reg!(
            spi,
            self.spi,
            CTRL,
            TXFIFORST: 1,
            RXFIFORST: 1,
            SPIRST: 1
        );
    }

    /// Wait for current SPI operation to complete
    #[inline(always)]
    pub fn wait_busy(&self) {
        while read_reg!(spi, self.spi, STATUS, SPIACTIVE == 1) {}
    }

    /// Wait for RXNE
    #[inline(always)]
    pub fn wait_rxne(&self) {
        while read_reg!(spi, self.spi, STATUS, RXEMPTY == 1) {}
    }

    /// Wait for TXE
    #[inline(always)]
    fn wait_txe(&self) {
        while read_reg!(spi, self.spi, STATUS, TXEMPTY == 0) {}
    }

    /// Perform an 8-bit read from DR
    #[inline(always)]
    pub fn read_dr_u8(&self) -> u8 {
        read_reg!(spi, self.spi, DATA) as u8
    }

    /// Perform an 8-bit write to DR
    #[inline(always)]
    fn write_dr_u8(&self, data: u8) {
        // Write data
        write_reg!(spi, self.spi, DATA, data as u32);
    }

    /// Perform a 16-bit write to DR
    ///
    /// Note that in 8-bit or smaller data mode, this enqueues two transmissions.
    #[inline(always)]
    fn write_dr_u16(&self, data: u16) {
        // Write data
        write_reg!(spi, self.spi, DATA, data as u32);
    }
}

impl Spi<1> {
    pub fn set_base_clock(&self, clocks: &Clocks) {
        self.base_clock
            .store(clocks.get_clk_spi2_freq(), Ordering::SeqCst);
    }
}

impl Spi<2> {
    pub fn set_base_clock(&self, clocks: &Clocks) {
        self.base_clock
            .store(clocks.get_clk_spi1_freq(), Ordering::SeqCst);
    }
}
