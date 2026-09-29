//! libusb transport for TI CC Debugger and SmartRF04EB devices.
//!
//! The device IDs and transfer flow are ported from Breezio-neo's xzg_cli.

use std::time::Duration;

use rusb::{Context, Device, DeviceHandle, TransferType, UsbContext};

use crate::error::{Error, Result};

pub const VID_CC_DEBUGGER: u16 = 0x0451;
pub const PID_CC_DEBUGGER: u16 = 0x16A2;
pub const VID_SMARTRF04EB: u16 = 0x11A0;
pub const PID_SMARTRF04EB: u16 = 0xEB20;

pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

const SUPPORTED_DEVICES: [(u16, u16, &str); 2] = [
    (VID_CC_DEBUGGER, PID_CC_DEBUGGER, "CC Debugger"),
    (VID_SMARTRF04EB, PID_SMARTRF04EB, "SmartRF04EB"),
];

pub trait UsbTransport {
    fn programmer(&self) -> &'static str;
    fn control_out(&mut self, request: u8, value: u16, index: u16, data: &[u8]) -> Result<()>;
    fn control_in(&mut self, request: u8, value: u16, index: u16, length: usize)
    -> Result<Vec<u8>>;
    fn bulk_out(&mut self, data: &[u8]) -> Result<()>;
    fn bulk_in(&mut self, length: usize) -> Result<Vec<u8>>;
}

pub struct RusbTransport {
    // Keep the handle ahead of the context so the handle is dropped first.
    handle: DeviceHandle<Context>,
    _context: Context,
    interface_number: u8,
    endpoint_in: u8,
    endpoint_out: u8,
    programmer: &'static str,
}

impl RusbTransport {
    pub fn connect() -> Result<Self> {
        let context = Context::new()?;
        let devices = context.devices()?;
        let mut candidates: Vec<(Device<Context>, u16, u16)> = Vec::new();
        for device in devices.iter() {
            let descriptor = device.device_descriptor()?;
            candidates.push((device, descriptor.vendor_id(), descriptor.product_id()));
        }
        let available: Vec<_> = candidates
            .iter()
            .map(|(_, vendor_id, product_id)| (*vendor_id, *product_id))
            .collect();
        let (vendor_id, product_id, programmer) =
            select_supported_device(&available).ok_or(Error::NoDevice)?;
        let (device, _, _) = candidates
            .into_iter()
            .find(|(_, candidate_vid, candidate_pid)| {
                *candidate_vid == vendor_id && *candidate_pid == product_id
            })
            .ok_or(Error::NoDevice)?;
        let handle = device.open()?;
        match handle.set_active_configuration(1) {
            Ok(()) | Err(rusb::Error::Busy) => {}
            Err(error) => return Err(error.into()),
        }

        let (interface_number, endpoint_in, endpoint_out) = discover_device_endpoints(&handle)?;
        handle.claim_interface(interface_number)?;

        eprintln!(
            "Connected to {programmer}; bulk IN=0x{endpoint_in:02x}, OUT=0x{endpoint_out:02x}"
        );

        Ok(Self {
            handle,
            _context: context,
            interface_number,
            endpoint_in,
            endpoint_out,
            programmer,
        })
    }
}

impl Drop for RusbTransport {
    fn drop(&mut self) {
        let _ = self.handle.release_interface(self.interface_number);
    }
}

fn discover_device_endpoints(handle: &DeviceHandle<Context>) -> Result<(u8, u8, u8)> {
    let config = handle.device().active_config_descriptor()?;
    for interface in config.interfaces() {
        for alternate in interface.descriptors() {
            let endpoints: Vec<_> = alternate
                .endpoint_descriptors()
                .map(|endpoint| {
                    let direction_in = endpoint.direction() == rusb::Direction::In;
                    let transfer_type = if endpoint.transfer_type() == TransferType::Bulk {
                        0x02
                    } else {
                        0
                    };
                    (endpoint.address(), transfer_type, direction_in)
                })
                .collect();
            if let Some((endpoint_in, endpoint_out)) = discover_bulk_endpoints(&endpoints)? {
                return Ok((alternate.interface_number(), endpoint_in, endpoint_out));
            }
        }
    }
    Err(Error::protocol(
        "USB device has no interface with bulk IN and OUT endpoints",
    ))
}

fn discover_bulk_endpoints(endpoints: &[(u8, u8, bool)]) -> Result<Option<(u8, u8)>> {
    let mut endpoint_in = None;
    let mut endpoint_out = None;
    for &(address, transfer_type, direction_in) in endpoints {
        if transfer_type != 0x02 {
            continue;
        }
        if direction_in {
            endpoint_in = Some(address);
        } else {
            endpoint_out = Some(address);
        }
    }
    match (endpoint_in, endpoint_out) {
        (Some(input), Some(output)) => Ok(Some((input, output))),
        _ => Ok(None),
    }
}

pub fn select_supported_device(available: &[(u16, u16)]) -> Option<(u16, u16, &'static str)> {
    SUPPORTED_DEVICES
        .iter()
        .find(|(vid, pid, _)| available.contains(&(*vid, *pid)))
        .map(|(vid, pid, name)| (*vid, *pid, *name))
}

pub fn check_transfer_length(
    operation: &'static str,
    expected: usize,
    actual: usize,
) -> Result<()> {
    if actual == expected {
        Ok(())
    } else {
        Err(Error::ShortTransfer {
            operation,
            expected,
            actual,
        })
    }
}

impl UsbTransport for RusbTransport {
    fn programmer(&self) -> &'static str {
        self.programmer
    }

    fn control_out(&mut self, request: u8, value: u16, index: u16, data: &[u8]) -> Result<()> {
        let actual =
            self.handle
                .write_control(0x40, request, value, index, data, DEFAULT_TIMEOUT)?;
        check_transfer_length("USB control OUT", data.len(), actual)
    }

    fn control_in(
        &mut self,
        request: u8,
        value: u16,
        index: u16,
        length: usize,
    ) -> Result<Vec<u8>> {
        let mut buffer = vec![0; length];
        let actual =
            self.handle
                .read_control(0xC0, request, value, index, &mut buffer, DEFAULT_TIMEOUT)?;
        buffer.truncate(actual);
        Ok(buffer)
    }

    fn bulk_out(&mut self, data: &[u8]) -> Result<()> {
        let actual = self
            .handle
            .write_bulk(self.endpoint_out, data, DEFAULT_TIMEOUT)?;
        check_transfer_length("USB bulk OUT", data.len(), actual)
    }

    fn bulk_in(&mut self, length: usize) -> Result<Vec<u8>> {
        let mut buffer = vec![0; length];
        let actual = self
            .handle
            .read_bulk(self.endpoint_in, &mut buffer, DEFAULT_TIMEOUT)?;
        check_transfer_length("USB bulk IN", length, actual)?;
        Ok(buffer)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        PID_CC_DEBUGGER, PID_SMARTRF04EB, VID_CC_DEBUGGER, VID_SMARTRF04EB, check_transfer_length,
        discover_bulk_endpoints, select_supported_device,
    };
    use crate::error::Error;

    #[test]
    fn selects_supported_device_with_cc_debugger_priority() {
        assert_eq!(
            select_supported_device(&[
                (VID_SMARTRF04EB, PID_SMARTRF04EB),
                (VID_CC_DEBUGGER, PID_CC_DEBUGGER)
            ]),
            Some((VID_CC_DEBUGGER, PID_CC_DEBUGGER, "CC Debugger"))
        );
        assert_eq!(select_supported_device(&[]), None);
    }

    #[test]
    fn finds_bulk_in_and_out_endpoints() {
        assert_eq!(
            discover_bulk_endpoints(&[(0x81, 0, true), (0x82, 0x02, true), (0x03, 0x02, false),])
                .unwrap(),
            Some((0x82, 0x03))
        );
        assert_eq!(
            discover_bulk_endpoints(&[(0x81, 0x02, true)]).unwrap(),
            None
        );
    }

    #[test]
    fn rejects_short_usb_transfers() {
        let error = check_transfer_length("USB bulk IN", 8, 7).unwrap_err();
        assert!(matches!(
            error,
            Error::ShortTransfer {
                expected: 8,
                actual: 7,
                ..
            }
        ));
        check_transfer_length("USB bulk OUT", 0, 0).unwrap();
    }
}
