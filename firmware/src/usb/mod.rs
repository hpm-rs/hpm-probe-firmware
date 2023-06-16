use crate::app::Request;
use crate::bsp::clock::Clocks;
use crate::bsp::usb::{UsbBusType, UsbPeripherals};
use crate::ral::usb;
use crate::VCP_PACKET_SIZE;
use hpm_usbd::{EndpointMemory, EndpointState, Speed};
use usb_device::bus::UsbBusAllocator;
use usb_device::prelude::*;
use usbd_serial::{LineCoding, SerialPort};

mod dap_v1;
mod dap_v2;
mod dfu;
mod winusb;

use dap_v1::CmsisDapV1;
use dap_v2::CmsisDapV2;
use dfu::DfuRuntime;
use winusb::MicrosoftDescriptors;

struct UninitializedUSB<const N: u8> {
    usb: usb::Instance<N>,
}

struct InitializedUSB {
    device: UsbDevice<'static, UsbBusType>,
    device_state: UsbDeviceState,
    winusb: MicrosoftDescriptors,
    dap_v1: CmsisDapV1<'static, UsbBusType>,
    dap_v2: CmsisDapV2<'static, UsbBusType>,
    serial: SerialPort<'static, UsbBusType>,
    dfu: DfuRuntime,
}

#[allow(clippy::large_enum_variant)]
enum State<const N: u8> {
    Uninitialized(UninitializedUSB<N>),
    Initialized(InitializedUSB),
    Initializing,
}

impl<const N: u8> State<N> {
    pub fn as_initialized(&self) -> &InitializedUSB {
        if let State::Initialized(initialized) = self {
            initialized
        } else {
            panic!("USB is not initialized yet");
        }
    }

    pub fn as_initialized_mut(&mut self) -> &mut InitializedUSB {
        if let State::Initialized(initialized) = self {
            initialized
        } else {
            panic!("USB is not initialized yet");
        }
    }
}

static EP_MEMORY: EndpointMemory<4096> = EndpointMemory::new();
static EP_STATE: EndpointState = EndpointState::max_endpoints();
static mut USB_BUS: Option<UsbBusAllocator<UsbBusType>> = None;

/// USB stack interface
#[allow(clippy::upper_case_acronyms)]
pub struct USB<const N: u8> {
    state: State<N>,
}

impl<const N: u8> USB<N> {
    /// Create a new USB object from the peripheral instance
    pub fn new(usb0: usb::Instance<N>) -> Self {
        let usb = UninitializedUSB { usb: usb0 };
        USB {
            state: State::Uninitialized(usb),
        }
    }

    /// Initialise the USB peripheral ready to start processing packets
    pub fn setup(&mut self, clocks: &Clocks, serial_string: &'static str) {
        let state = core::mem::replace(&mut self.state, State::Initializing);
        if let State::Uninitialized(usb) = state {
            unsafe {
                let usb = UsbPeripherals { usb: usb.usb };

                let bus_adapter = UsbBusType::with_speed(usb, &EP_MEMORY, &EP_STATE, Speed::High);
                let usb_bus = usb_device::bus::UsbBusAllocator::new(bus_adapter);
                USB_BUS = Some(usb_bus);
                let usb_bus = USB_BUS.as_ref().unwrap();

                let winusb = MicrosoftDescriptors;

                // Order of these calls is important, if the interface numbers for CmsisDapV2 or DfuRuntime change,
                // definitions in winusb.rs (DAP_V2_INTERFACE, DFU_INTERFACE) have to be adapted!
                let serial = SerialPort::new(usb_bus);
                let dap_v1 = CmsisDapV1::new(usb_bus);
                let dap_v2 = CmsisDapV2::new(usb_bus);
                let dfu = DfuRuntime::new(usb_bus);

                let device = UsbDeviceBuilder::new(usb_bus, UsbVidPid(0x1209, 0x4853))
                    .manufacturer("Probe-rs development team")
                    .product("HS-Probe with CMSIS-DAP Support")
                    .serial_number(serial_string)
                    .composite_with_iads()
                    .max_packet_size_0(64)
                    .max_power(500)
                    .device_release(0x11)
                    .build();
                let device_state = device.state();

                let usb = InitializedUSB {
                    device,
                    device_state,
                    winusb,
                    dap_v1,
                    dap_v2,
                    serial,
                    dfu,
                };
                self.state = State::Initialized(usb)
            }
        } else {
            panic!("Invalid state");
        }
    }

    /// Process a pending USB interrupt.
    ///
    /// Call this function when a USB interrupt occurs.
    ///
    /// Returns Some(Request) if a new request has been received
    /// from the host.
    ///
    /// This function will clear the interrupt bits of all interrupts
    /// it processes; if any are unprocessed the USB interrupt keeps
    /// triggering until all are processed.
    pub fn interrupt(&mut self, vcp_idle: bool) -> Option<Request> {
        let usb = self.state.as_initialized_mut();
        if usb.device.poll(&mut [
            &mut usb.winusb,
            &mut usb.serial,
            // &mut usb.dap_v1,
            &mut usb.dap_v2,
            // &mut usb.dfu,
        ]) {
            let old_state = usb.device_state;
            let new_state = usb.device.state();
            usb.device_state = new_state;
            if (old_state != new_state) && (new_state != UsbDeviceState::Configured) {
                return Some(Request::Suspend);
            } else if (old_state != new_state) && (new_state == UsbDeviceState::Configured) {
                usb.device.bus().configure();
            }

            // let r = usb.dap_v1.process();
            // if r.is_some() {
            //     return r;
            // }

            let r = usb.dap_v2.process();
            if r.is_some() {
                return r;
            }

            if vcp_idle {
                let mut buf = [0; VCP_PACKET_SIZE as usize];
                let serialdata = usb.serial.read(&mut buf);
                match serialdata {
                    Ok(x) => {
                        return Some(Request::VCPPacket((buf, x)));
                    }
                    // discard error?
                    Err(_e) => (),
                }
            }
        }
        None
    }

    /// Transmit a DAP report back over the DAPv1 HID interface
    pub fn dap1_reply(&mut self, data: &[u8]) {
        let usb = self.state.as_initialized_mut();
        usb.dap_v1
            .write_packet(data)
            .expect("DAPv1 EP write failed");
    }

    /// Transmit a DAP report back over the DAPv2 bulk interface
    pub fn dap2_reply(&mut self, data: &[u8]) {
        let usb = self.state.as_initialized_mut();
        usb.dap_v2
            .write_packet(data)
            .expect("DAPv2 EP write failed");
    }

    /// Check if SWO endpoint is currently busy transmitting data
    pub fn dap2_swo_is_busy(&self) -> bool {
        let usb = self.state.as_initialized();
        usb.dap_v2.trace_busy()
    }

    /// Transmit SWO streaming data back over the DAPv2 bulk interface
    pub fn dap2_stream_swo(&mut self, data: &[u8]) {
        let usb = self.state.as_initialized_mut();
        usb.dap_v2.trace_write(data).expect("trace EP write failed");
    }

    /// Grab the current LineCoding (UART parameters) from the CDC-ACM stack
    pub fn serial_line_encoding(&self) -> &LineCoding {
        let usb = self.state.as_initialized();
        usb.serial.line_coding()
    }

    /// Return UART data to host trough USB
    pub fn serial_return(&mut self, data: &[u8]) {
        let usb = self.state.as_initialized_mut();
        usb.serial.write(data).expect("Serial EP write failed");
    }
}
