#![allow(dead_code)]

use crate::ral::uart;
use crate::ral::{modify_reg, read_reg, write_reg};
use core::cmp::Ordering;
use core::fmt::Write;
use core::ops::Deref;

use super::clock::Clocks;
use super::dma::DMA;

pub struct Uart<'a, const N: u8> {
    uart: uart::Instance<N>,
    dma: &'a DMA,
    buffer: [u8; 256],
    last_idx: usize,
    fck: u32,
}

impl<'a, const N: u8> Uart<'a, N> {
    pub fn new(uart: uart::Instance<N>, dma: &'a DMA) -> Self {
        Self {
            uart,
            dma,
            buffer: [0; 256],
            last_idx: 0,
            fck: 24_000_000,
        }
    }

    fn set_fck(&mut self, clock: &Clocks) {
        self.fck = match self.uart.deref() as *const uart::RegisterBlock {
            uart::UART0 => clock.get_clk_uart0_freq(),
            uart::UART9 => clock.get_clk_uart9_freq(),
            _ => panic!("not support yet"),
        };
    }

    pub fn setup(&mut self, clock: &Clocks) {
        // Disable all interrupt
        write_reg!(uart, self.uart, DLM, 0);

        self.set_fck(clock);

        // Word length to 8 bits
        modify_reg!(uart, self.uart, LCR, WLS: Bits8);
        // Enable DMA
        modify_reg!(uart, self.uart, FCR, DMAE: 1);
    }

    pub fn set_baud(&self, baud: u32) -> u32 {
        // Set DLAB to 1
        modify_reg!(uart, self.uart, LCR, DLAB: 1);

        let div = self.fck / (baud * 16);
        modify_reg!(uart, self.uart, DLL, DLL: div);
        modify_reg!(uart, self.uart, DLM, DLM: div >> 8);

        // Set DLAB to 0
        modify_reg!(uart, self.uart, LCR, DLAB: 0);

        self.fck / div / 16
    }

    #[inline]
    fn is_tx_fifo_empty(&self) -> bool {
        return read_reg!(uart, self.uart, LSR, THRE) == 1;
    }

    #[inline]
    pub fn send_byte(&self, byte: u8) {
        while !self.is_tx_fifo_empty() {}
        write_reg!(uart, self.uart, DLL, DLL: byte as u32);
    }

    /// Begin UART reception into buffer.
    ///
    /// UART::poll must be called regularly after starting.
    pub fn start(&mut self) {
        self.last_idx = 0;
        modify_reg!(uart, self.uart, FCR, FIFOE: 1);
        // self.dma.uart9_start(&mut self.buffer);
    }

    /// End UART reception.
    pub fn stop(&self) {
        self.dma.uart9_stop();
        modify_reg!(uart, self.uart, FCR, FIFOE: 0);
    }

    /// Returns true if UART currently enabled
    pub fn is_active(&self) -> bool {
        read_reg!(uart, self.uart, FCR, FIFOE == 1)
    }

    /// Return length of internal buffer
    pub fn buffer_len(&self) -> usize {
        self.buffer.len()
    }

    /// Fetch current number of bytes available.
    ///
    /// Subsequent calls to read() may return a different amount of data.
    pub fn bytes_available(&self) -> usize {
        let dma_idx = self.buffer.len() - self.dma.uart9_ndtr();
        if dma_idx >= self.last_idx {
            dma_idx - self.last_idx
        } else {
            (self.buffer.len() - self.last_idx) + dma_idx
        }
    }

    /// Read new UART data.
    ///
    /// Returns number of bytes written to buffer.
    ///
    /// Reads at most rx.len() new bytes, which may be less than what was received.
    /// Remaining data will be read on the next call, so long as the internal buffer
    /// doesn't overflow, which is not detected.
    pub fn read(&mut self, rx: &mut [u8]) -> usize {
        // See what index the DMA is going to write next, and copy out
        // all prior data. Even if the DMA writes new data while we're
        // processing we won't get out of sync and will handle the new
        // data next time read() is called.
        let dma_idx = self.buffer.len() - self.dma.uart9_ndtr();

        match dma_idx.cmp(&self.last_idx) {
            Ordering::Equal => {
                // No action required if no data has been received.
                0
            }
            Ordering::Less => {
                // Wraparound occurred:
                // Copy from last_idx to end, and from start to new dma_idx.
                let mut n1 = self.buffer.len() - self.last_idx;
                let mut n2 = dma_idx;
                let mut new_last_idx = dma_idx;

                // Ensure we don't overflow rx buffer
                if n1 > rx.len() {
                    n1 = rx.len();
                    n2 = 0;
                    new_last_idx = self.last_idx + n1;
                } else if (n1 + n2) > rx.len() {
                    n2 = rx.len() - n1;
                    new_last_idx = n2;
                }

                rx[..n1].copy_from_slice(&self.buffer[self.last_idx..self.last_idx + n1]);
                rx[n1..(n1 + n2)].copy_from_slice(&self.buffer[..n2]);

                self.last_idx = new_last_idx;
                n1 + n2
            }
            Ordering::Greater => {
                // New data, no wraparound:
                // Copy from last_idx to new dma_idx.
                let mut n = dma_idx - self.last_idx;

                // Ensure we don't overflow rx buffer
                if n > rx.len() {
                    n = rx.len();
                }

                rx[..n].copy_from_slice(&self.buffer[self.last_idx..self.last_idx + n]);

                self.last_idx += n;
                n
            }
        }
    }
}

impl<'a> Uart<'a, 0> {
    pub fn send_bytes(&self, s: &[u8]) {
        while !self.is_tx_fifo_empty() {}
        self.dma.uart0_start_tx_transfer(s);
    }
}

impl<'a> Write for Uart<'a, 0> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.send_bytes(s.as_bytes());
        Ok(())
    }
}
