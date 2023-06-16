use crate::dap::DAPVersion;
// use crate::vcp::VcpConfig;
use crate::{DAP1_PACKET_SIZE, DAP2_PACKET_SIZE, VCP_PACKET_SIZE};
use hpm_probe_bsp as bsp;

#[allow(clippy::large_enum_variant)]
pub enum Request {
    Suspend,
    DAP1Command(([u8; DAP1_PACKET_SIZE as usize], usize)),
    DAP2Command(([u8; DAP2_PACKET_SIZE as usize], usize)),
    VCPPacket(([u8; VCP_PACKET_SIZE as usize], usize)),
}

pub struct App<'a> {
    clocks: &'a bsp::clock::Clocks,
    dma: &'a bsp::dma::DMA,
    pins: &'a bsp::gpio::Pins<'a>,
    swd_spi: &'a bsp::spi::Spi<1>,
    jtag_spi: &'a bsp::spi::Spi<2>,
    usb: &'a mut crate::usb::USB<0>,
    dap: &'a mut crate::dap::DAP<'a>,
    // vcp: &'a mut crate::vcp::VCP<'a>,
    delay: &'a bsp::delay::Delay,
    resp_buf: [u8; DAP2_PACKET_SIZE as usize],
    // vcp_config: VcpConfig,
}

impl<'a> App<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        clocks: &'a bsp::clock::Clocks,
        dma: &'a bsp::dma::DMA,
        pins: &'a bsp::gpio::Pins<'a>,
        swd_spi: &'a bsp::spi::Spi<1>,
        jtag_spi: &'a bsp::spi::Spi<2>,
        usb: &'a mut crate::usb::USB<0>,
        dap: &'a mut crate::dap::DAP<'a>,
        // vcp: &'a mut crate::vcp::VCP<'a>,
        delay: &'a bsp::delay::Delay,
    ) -> Self {
        App {
            clocks,
            dma,
            pins,
            swd_spi,
            jtag_spi,
            usb,
            dap,
            // vcp,
            delay,
            resp_buf: [0; DAP2_PACKET_SIZE as usize],
            // vcp_config: VcpConfig::default(),
        }
    }

    /// Unsafety: this function should be called from the main context.
    /// No other contexts should be active at the same time.
    pub unsafe fn setup(&mut self, serial: &'static str) {
        self.delay.set_base_clock(&self.clocks);

        // Configure DMA for SPI1, SPI2, USART1 and USART2 transfers
        self.dma.setup();

        // Configure GPIOs
        self.pins.setup();
        self.pins.high_impedance_mode();

        loop {
            self.pins.xpi0_sclk.toggle();
            // self.pins.xpi0_cs.toggle();
            // self.pins.xpi0_d0.toggle();
            // self.pins.xpi0_d1.toggle();
            // self.pins.xpi0_d2.toggle();
            // self.pins.xpi0_d3.toggle();
            self.delay.delay_us(1000);
        }

        self.swd_spi.set_base_clock(&self.clocks);
        self.swd_spi.disable();

        self.jtag_spi.set_base_clock(&self.clocks);
        self.jtag_spi.disable();

        // Configure DAP timing information
        self.dap.setup(&self.clocks);

        // Configure VCP clocks & pins
        // self.vcp.setup(&clocks);

        // Configure USB peripheral and connect to host
        self.usb.setup(&self.clocks, serial);

        self.pins.led_red.set_low();
        // self.pins.t5v_en.set_high();

        log::info!("system setup finished");
    }

    pub fn poll(&mut self) {
        // we need to inform the usb mod if we would be ready to receive
        // new acm data would there be some available.
        if let Some(req) = self.usb.interrupt(false) {
            self.process_request(req);
        }

        if self.dap.is_swo_streaming() && !self.usb.dap2_swo_is_busy() {
            // Poll for new UART data when streaming is enabled and
            // the SWO endpoint is ready to transmit more data.
            let len = self.dap.read_swo(&mut self.resp_buf);

            if len > 0 {
                self.usb.dap2_stream_swo(&self.resp_buf[0..len]);
            }
        }

        // Compare potentially new line encoding for vcp
        // There is probably a better way to do it but i could
        // not find a way to be informed of a new encoding by the
        // acm usb stack.
        // let new_line_coding = self.usb.serial_line_encoding();
        // let config = VcpConfig {
        //     stop_bits: new_line_coding.stop_bits(),
        //     data_bits: new_line_coding.data_bits(),
        //     parity_type: new_line_coding.parity_type(),
        //     data_rate: new_line_coding.data_rate(),
        // };
        // if config != self.vcp_config {
        //     self.vcp_config = config;
        //     self.vcp.stop();
        //     self.vcp.set_config(self.vcp_config);
        //     self.vcp.start();
        // }
        //
        // // check if there are bytes available in the uart rx buffer
        // let vcp_rx_len = self.vcp.rx_bytes_available();
        // if vcp_rx_len > 0 {
        //     // read them and get potentially new length of bytes
        //     let len = self.vcp.read(&mut self.resp_buf);
        //     // transfer those bytes to the usb host
        //     self.usb.serial_return(&self.resp_buf[0..len]);
        // }
    }

    fn process_request(&mut self, req: Request) {
        match req {
            Request::DAP1Command((report, n)) => {
                let len = self.dap.process_command(
                    &report[..n],
                    &mut self.resp_buf[..DAP1_PACKET_SIZE as usize],
                    DAPVersion::V1,
                );

                if len > 0 {
                    self.usb.dap1_reply(&self.resp_buf[..len]);
                }
            }
            Request::DAP2Command((report, n)) => {
                let len =
                    self.dap
                        .process_command(&report[..n], &mut self.resp_buf, DAPVersion::V2);

                if len > 0 {
                    self.usb.dap2_reply(&self.resp_buf[..len]);
                }
            }
            Request::VCPPacket((buffer, n)) => {
                // self.vcp.write(&buffer[0..n], n);
            }
            Request::Suspend => {
                self.pins.high_impedance_mode();
                self.pins.led_red.set_high();
                self.pins.led_blue.set_high();
                self.pins.led_green.set_high();
                self.swd_spi.disable();
                self.jtag_spi.disable();
            }
        }
    }
}
