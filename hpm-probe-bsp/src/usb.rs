use crate::ral::usb;
use hpm_usbd::BusAdapter;

pub struct UsbPeripherals<const N: u8> {
    pub usb: usb::Instance<N>,
}

unsafe impl<const N: u8> hpm_usbd::Peripherals for UsbPeripherals<N> {
    fn usb(&self) -> *const () {
        let rb: &usb::RegisterBlock = &self.usb;
        (rb as *const usb::RegisterBlock).cast()
    }
}

pub type UsbBusType = BusAdapter;
