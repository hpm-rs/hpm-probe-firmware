use crate::app::Request;
use crate::DAP2_PACKET_SIZE;
use usb_device::class_prelude::*;
use usb_device::Result;

pub struct CmsisDapV2<'a, B: UsbBus> {
    interface: InterfaceNumber,
    name: StringIndex,
    read_ep: EndpointOut<'a, B>,
    write_ep: EndpointIn<'a, B>,
    trace_ep: EndpointIn<'a, B>,
    trace_busy: bool,
}

impl<B: UsbBus> CmsisDapV2<'_, B> {
    pub fn new(alloc: &UsbBusAllocator<B>) -> CmsisDapV2<B> {
        CmsisDapV2 {
            interface: alloc.interface(),
            name: alloc.string(),
            read_ep: alloc.bulk(DAP2_PACKET_SIZE),
            write_ep: alloc.bulk(DAP2_PACKET_SIZE),
            trace_ep: alloc.bulk(DAP2_PACKET_SIZE),
            trace_busy: false,
        }
    }

    pub fn process(&mut self) -> Option<Request> {
        let mut buf = [0u8; DAP2_PACKET_SIZE as usize];
        match self.read_ep.read(&mut buf) {
            Ok(size) if size > 0 => Some(Request::DAP2Command((buf, size))),
            _ => None,
        }
    }

    pub fn write_packet(&mut self, data: &[u8]) -> Result<()> {
        if data.len() > self.write_ep.max_packet_size() as usize {
            return Err(UsbError::BufferOverflow);
        }
        self.write_ep.write(data).map(|_| ())
    }

    pub fn trace_busy(&self) -> bool {
        self.trace_busy
    }

    pub fn trace_write(&mut self, data: &[u8]) -> Result<()> {
        if data.len() > self.trace_ep.max_packet_size() as usize {
            return Err(UsbError::BufferOverflow);
        }
        self.trace_ep.write(data).map(|_| ())?;
        self.trace_busy = true;
        Ok(())
    }
}

impl<B: UsbBus> UsbClass<B> for CmsisDapV2<'_, B> {
    fn get_configuration_descriptors(&self, writer: &mut DescriptorWriter) -> Result<()> {
        writer.interface_alt(self.interface, 0, 0xff, 0, 0, Some(self.name))?;

        writer.endpoint(&self.read_ep)?;
        writer.endpoint(&self.write_ep)?;
        writer.endpoint(&self.trace_ep)?;

        Ok(())
    }

    fn get_string(&self, index: StringIndex, _lang_id: u16) -> Option<&str> {
        if index == self.name {
            Some("HPM-probe CMSIS-DAP v2 Interface")
        } else {
            None
        }
    }

    fn reset(&mut self) {
        self.trace_busy = false;
    }

    fn endpoint_in_complete(&mut self, addr: EndpointAddress) {
        if addr == self.trace_ep.address() {
            self.trace_busy = false;
        }
    }
}
