use crate::ral;
use crate::ral::dma;
use crate::ral::dmamux;
use crate::ral::{modify_reg, read_reg, write_reg};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Channel {
    Channel0 = 0,
    Channel1 = 1,
    Channel2 = 2,
    Channel3 = 3,
    Channel4 = 4,
    Channel5 = 5,
    Channel6 = 6,
    Channel7 = 7,
}

pub enum Source {
    Spi1Rx = 2,
    Spi1Tx = 3,
    Spi2Tx = 4,
    Spi2Rx = 5,
    Uart0Rx = 8,
    Uart0Tx = 9,
    Uart9Rx = 26,
}

const UART_THR_OFFSET: u32 = 0x20;
const SPI_DATA_OFFSET: u32 = 0x2c;

pub(crate) const SPI1_RX_CH: Channel = Channel::Channel0;
pub(crate) const SPI1_TX_CH: Channel = Channel::Channel1;
pub(crate) const SPI2_RX_CH: Channel = Channel::Channel2;
pub(crate) const SPI2_TX_CH: Channel = Channel::Channel3;
pub(crate) const UART0_RX_CH: Channel = Channel::Channel4;
pub(crate) const UART0_TX_CH: Channel = Channel::Channel5;
pub(crate) const UART9_RX_CH: Channel = Channel::Channel6;

macro_rules! setup_peri_to_memory {
    ($DMA:expr,
     $DMAMUX:expr,
     $CHCTRL_CTRL:ident,
     $CHCTRL_SRCADDR:ident,
     $MUXCFG_MUX:ident,
     $CHANNEL:ident,
     $DMASRC:expr,
     $SRCADDR:expr) => {
        write_reg!(
            dma,
            $DMA,
            $CHCTRL_CTRL,
            SRCBUSINFIDX: 0,
            DSTBUSINFIDX: 0,
            PRIORITY: Lower,
            SRCBURSTSIZE: Transfer1,
            SRCWIDTH: Byte,
            DSTWIDTH: Byte,
            SRCMODE: Handshake,
            DSTMODE: Normal,
            SRCADDRCTRL: Fixed,
            DSTADDRCTRL: Increment,
            SRCREQSEL: $CHANNEL as u32,
            DSTREQSEL: $CHANNEL as u32,
            INTABTMASK: Disable,
            INTERRMASK: Disable,
            INTTCMASK: Disable,
            ENABLE: Disable
        );
        write_reg!(
            dma,
            $DMA,
            $CHCTRL_SRCADDR,
            $SRCADDR
        );
        write_reg!(
            dmamux,
            $DMAMUX,
            $MUXCFG_MUX,
            ENABLE: Enable,
            SOURCE: $DMASRC as u32
        );
    };
}

macro_rules! setup_memory_to_peri {
    ($DMA:expr,
     $DMAMUX:expr,
     $CHCTRL_CTRL:ident,
     $CHCTRL_DSTADDR:ident,
     $MUXCFG_MUX:ident,
     $CHANNEL:ident,
     $DMASRC:expr,
     $DSTADDR:expr) => {
        write_reg!(
            dma,
            $DMA,
            $CHCTRL_CTRL,
            SRCBUSINFIDX: 0,
            DSTBUSINFIDX: 0,
            PRIORITY: Lower,
            SRCBURSTSIZE: Transfer1,
            SRCWIDTH: Byte,
            DSTWIDTH: Byte,
            SRCMODE: Normal,
            DSTMODE: Handshake,
            SRCADDRCTRL: Increment,
            DSTADDRCTRL: Fixed,
            SRCREQSEL: $CHANNEL as u32,
            DSTREQSEL: $CHANNEL as u32,
            INTABTMASK: Disable,
            INTERRMASK: Disable,
            INTTCMASK: Disable,
            ENABLE: Disable
        );
        write_reg!(
            dma,
            $DMA,
            $CHCTRL_DSTADDR,
            $DSTADDR
        );
        write_reg!(
            dmamux,
            $DMAMUX,
            $MUXCFG_MUX,
            ENABLE: Enable,
            SOURCE: $DMASRC as u32
        );
    };
}

pub struct DMA {
    dma: dma::HDMA0,
    dmamux: dmamux::DMAMUX,
}

impl DMA {
    pub fn new(dma: dma::HDMA0, dmamux: dmamux::DMAMUX) -> Self {
        DMA { dma, dmamux }
    }

    pub fn setup(&self) {
        // Setup channel 0 for for SPI1_RX
        setup_peri_to_memory!(
            self.dma,
            self.dmamux,
            CHCTRL_CH0_CTRL,
            CHCTRL_CH0_SRCADDR,
            MUXCFG_HDMA_MUX0,
            SPI1_RX_CH,
            Source::Spi1Rx,
            ral::spi::SPI1 as u32 + SPI_DATA_OFFSET
        );
        // Setup channel 1 for for SPI1_TX
        setup_memory_to_peri!(
            self.dma,
            self.dmamux,
            CHCTRL_CH1_CTRL,
            CHCTRL_CH1_DSTADDR,
            MUXCFG_HDMA_MUX1,
            SPI1_TX_CH,
            Source::Spi1Tx,
            ral::spi::SPI1 as u32 + SPI_DATA_OFFSET
        );
        // Setup channel 2 for for SPI2_RX
        setup_peri_to_memory!(
            self.dma,
            self.dmamux,
            CHCTRL_CH2_CTRL,
            CHCTRL_CH2_SRCADDR,
            MUXCFG_HDMA_MUX2,
            SPI2_RX_CH,
            Source::Spi2Rx,
            ral::spi::SPI2 as u32 + SPI_DATA_OFFSET
        );

        // Setup channel 3 for for SPI2_TX
        setup_memory_to_peri!(
            self.dma,
            self.dmamux,
            CHCTRL_CH3_CTRL,
            CHCTRL_CH3_DSTADDR,
            MUXCFG_HDMA_MUX3,
            SPI2_TX_CH,
            Source::Spi2Tx,
            ral::spi::SPI2 as u32 + SPI_DATA_OFFSET
        );

        // Setup channel 4 for for USART0_RX
        setup_peri_to_memory!(
            self.dma,
            self.dmamux,
            CHCTRL_CH4_CTRL,
            CHCTRL_CH4_SRCADDR,
            MUXCFG_HDMA_MUX4,
            UART0_RX_CH,
            Source::Uart0Rx,
            ral::uart::UART0 as u32 + UART_THR_OFFSET
        );

        // Setup channel 5 for for USART0_TX
        setup_memory_to_peri!(
            self.dma,
            self.dmamux,
            CHCTRL_CH5_CTRL,
            CHCTRL_CH5_DSTADDR,
            MUXCFG_HDMA_MUX5,
            UART0_TX_CH,
            Source::Uart0Tx,
            ral::uart::UART0 as u32 + UART_THR_OFFSET
        );

        // Setup channel 6 for for USART9_RX
        setup_peri_to_memory!(
            self.dma,
            self.dmamux,
            CHCTRL_CH6_CTRL,
            CHCTRL_CH6_SRCADDR,
            MUXCFG_HDMA_MUX6,
            UART9_RX_CH,
            Source::Uart9Rx,
            ral::uart::UART9 as u32 + UART_THR_OFFSET
        );
    }

    /// Sets up and enables a DMA transmit/receive for SPI1 (channel 1 and channel 0)
    pub fn spi1_enable(&self, tx: &[u8], rx: &mut [u8]) {
        write_reg!(
            dma,
            self.dma,
            INTSTATUS,
            TC: (1 << (SPI1_RX_CH as u32)) | (1 << (SPI1_TX_CH as u32)),
            ABORT: (1 << (SPI1_RX_CH as u32)) | (1 << (SPI1_TX_CH as u32)),
            ERROR: (1 << (SPI1_RX_CH as u32)) | (1 << (SPI1_TX_CH as u32))
        );
        write_reg!(dma, self.dma, CHCTRL_CH0_DSTADDR, rx.as_mut_ptr() as u32);
        write_reg!(dma, self.dma, CHCTRL_CH0_TRANSIZE, rx.len() as u32);
        write_reg!(dma, self.dma, CHCTRL_CH1_SRCADDR, tx.as_ptr() as u32);
        write_reg!(dma, self.dma, CHCTRL_CH1_TRANSIZE, tx.len() as u32);
        modify_reg!(dma, self.dma, CHCTRL_CH0_CTRL, ENABLE: Enable);
        modify_reg!(dma, self.dma, CHCTRL_CH1_CTRL, ENABLE: Enable);
    }

    /// Check if SPI1 transaction is still ongoing
    pub fn spi1_busy(&self) -> bool {
        (read_reg!(dma, self.dma, INTSTATUS, TC) >> (SPI1_TX_CH as u32)) & 0x01 != 0
    }

    /// Stop SPI1 DMA
    pub fn spi1_disable(&self) {
        modify_reg!(dma, self.dma, CHCTRL_CH0_CTRL, ENABLE: Disable);
        modify_reg!(dma, self.dma, CHCTRL_CH1_CTRL, ENABLE: Disable);
    }

    /// Sets up and enables a DMA transmit/receive for SPI1 (channel 3 and channel 2)
    pub fn spi2_enable(&self, tx: &[u8], rx: &mut [u8]) {
        write_reg!(
            dma,
            self.dma,
            INTSTATUS,
            TC: (1 << (SPI2_RX_CH as u32)) | (1 << (SPI2_TX_CH as u32)),
            ABORT: (1 << (SPI2_RX_CH as u32)) | (1 << (SPI2_TX_CH as u32)),
            ERROR: (1 << (SPI2_RX_CH as u32)) | (1 << (SPI2_TX_CH as u32))
        );
        write_reg!(dma, self.dma, CHCTRL_CH2_DSTADDR, rx.as_mut_ptr() as u32);
        write_reg!(dma, self.dma, CHCTRL_CH2_TRANSIZE, rx.len() as u32);
        write_reg!(dma, self.dma, CHCTRL_CH3_SRCADDR, tx.as_ptr() as u32);
        write_reg!(dma, self.dma, CHCTRL_CH3_TRANSIZE, tx.len() as u32);
        modify_reg!(dma, self.dma, CHCTRL_CH2_CTRL, ENABLE: Enable);
        modify_reg!(dma, self.dma, CHCTRL_CH3_CTRL, ENABLE: Enable);
    }

    /// Check if SPI2 transaction is still ongoing
    pub fn spi2_busy(&self) -> bool {
        (read_reg!(dma, self.dma, INTSTATUS, TC) >> (SPI2_TX_CH as u32)) & 0x01 != 0
    }

    /// Stop SPI2 DMA
    pub fn spi2_disable(&self) {
        modify_reg!(dma, self.dma, CHCTRL_CH2_CTRL, ENABLE: Disable);
        modify_reg!(dma, self.dma, CHCTRL_CH3_CTRL, ENABLE: Disable);
    }

    /// Start UART9 reception into provided buffer
    pub fn uart9_start(&self, rx: &mut [u8]) {
        write_reg!(
            dma,
            self.dma,
            INTSTATUS,
            TC: (1 << (UART9_RX_CH as u32)),
            ABORT: (1 << (UART9_RX_CH as u32)),
            ERROR: (1 << (UART9_RX_CH as u32))
        );
        write_reg!(dma, self.dma, CHCTRL_CH6_DSTADDR, rx.as_mut_ptr() as u32);
        write_reg!(dma, self.dma, CHCTRL_CH6_TRANSIZE, rx.len() as u32);
        modify_reg!(dma, self.dma, CHCTRL_CH6_CTRL, ENABLE: Enable);
    }

    /// Return how many bytes are left to transfer for UART9
    pub fn uart9_ndtr(&self) -> usize {
        read_reg!(dma, self.dma, CHCTRL_CH6_TRANSIZE) as usize
    }

    /// Stop UART9 DMA
    pub fn uart9_stop(&self) {
        modify_reg!(dma, self.dma, CHCTRL_CH6_CTRL, ENABLE: Disable);
    }

    /// Start UART0 reception into provided buffer
    pub fn uart0_start_rx(&self, rx: &mut [u8]) {
        write_reg!(
            dma,
            self.dma,
            INTSTATUS,
            TC: (1 << (UART0_RX_CH as u32)),
            ABORT: (1 << (UART0_RX_CH as u32)),
            ERROR: (1 << (UART0_RX_CH as u32))
        );
        write_reg!(dma, self.dma, CHCTRL_CH4_DSTADDR, rx.as_mut_ptr() as u32);
        write_reg!(dma, self.dma, CHCTRL_CH4_TRANSIZE, rx.len() as u32);
        modify_reg!(dma, self.dma, CHCTRL_CH4_CTRL, ENABLE: Enable);
    }

    /// Return how many bytes are left to transfer for UART0 RX
    pub fn uart0_rx_ndtr(&self) -> usize {
        read_reg!(dma, self.dma, CHCTRL_CH4_TRANSIZE) as usize
    }
    /// Return how many bytes are left to transfer for UART0 TX
    pub fn uart0_tx_ndtr(&self) -> usize {
        read_reg!(dma, self.dma, CHCTRL_CH5_TRANSIZE) as usize
    }

    /// Start a DMA transfer for UART0 TX
    pub fn uart0_start_tx_transfer(&self, tx: &[u8]) {
        write_reg!(
            dma,
            self.dma,
            INTSTATUS,
            TC: (1 << (UART0_TX_CH as u32)),
            ABORT: (1 << (UART0_TX_CH as u32)),
            ERROR: (1 << (UART0_TX_CH as u32))
        );
        write_reg!(dma, self.dma, CHCTRL_CH5_SRCADDR, tx.as_ptr() as u32);
        write_reg!(dma, self.dma, CHCTRL_CH5_TRANSIZE, tx.len() as u32);
        modify_reg!(dma, self.dma, CHCTRL_CH5_CTRL, ENABLE: Enable);
    }

    /// Stop UART0 DMA
    pub fn uart0_stop(&self) {
        modify_reg!(dma, self.dma, CHCTRL_CH4_CTRL, ENABLE: Disable);
        modify_reg!(dma, self.dma, CHCTRL_CH5_CTRL, ENABLE: Disable);
    }
}
