use usb_device::control::{Recipient, RequestType};
use usb_device::Result;
use usb_device::{class_prelude::*, device};

#[allow(unused)]
mod request {
    pub const DFU_DETACH: u8 = 0;
    pub const DFU_DNLOAD: u8 = 1;
    pub const DFU_UPLOAD: u8 = 2;
    pub const DFU_GETSTATUS: u8 = 3;
    pub const DFU_CLRSTATUS: u8 = 4;
    pub const DFU_GETSTATE: u8 = 5;
    pub const DFU_ABORT: u8 = 6;
}

pub struct DfuRuntime {
    interface: InterfaceNumber,
    name: StringIndex,
}

impl DfuRuntime {
    pub fn new<B: UsbBus>(alloc: &UsbBusAllocator<B>) -> DfuRuntime {
        DfuRuntime {
            interface: alloc.interface(),
            name: alloc.string(),
        }
    }
}

impl<B: UsbBus> UsbClass<B> for DfuRuntime {
    fn get_configuration_descriptors(&self, writer: &mut DescriptorWriter) -> Result<()> {
        writer.interface_alt(
            self.interface,
            device::DEFAULT_ALTERNATE_SETTING,
            0xFE,
            1,
            1,
            Some(self.name),
        )?;

        // DFU Functional Descriptor
        writer.write(
            0x21, // Functional descriptor type
            &[
                0x0F, // bmAttributes
                0xFF, 0x00, // wDetachTimeOut
                0x08, 0x00, // wTransferSize
                0x00, 0x01, // bcdDFUVersion
            ],
        )?;

        Ok(())
    }

    fn get_string(&self, index: StringIndex, _lang_id: u16) -> Option<&str> {
        if index == self.name {
            Some("HS-Probe DFU")
        } else {
            None
        }
    }

    fn control_in(&mut self, xfer: ControlIn<B>) {
        let req = xfer.request();
        if !(req.request_type == RequestType::Class
            && req.recipient == Recipient::Interface
            && req.index == u8::from(self.interface) as u16)
        {
            return;
        }

        match req.request {
            request::DFU_GETSTATUS => {
                xfer.accept_with_static(&[0x00; 6]).ok();
            }
            _ => {
                xfer.reject().ok();
            }
        }
    }

    fn control_out(&mut self, xfer: ControlOut<B>) {
        let req = xfer.request();
        if !(req.request_type == RequestType::Class
            && req.recipient == Recipient::Interface
            && req.index == u8::from(self.interface) as u16)
        {
            return;
        }

        match req.request {
            request::DFU_DETACH => {
                // hs_probe_bsp::bootload::bootload();
            }
            _ => {
                xfer.reject().ok();
            }
        }
    }
}
