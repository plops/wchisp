//! USB Transportation.
//!
//! This module provides USB transportation for communication with WCH ISP USB devices.
//! It includes functionality for scanning and opening USB devices, as well as sending and receiving raw data.
//!
//! # Examples
//!
//! Scanning for USB devices:
//!
//! ```
//! use wchisp::transport::usb::UsbTransport;
//!
//! let num_devices = UsbTransport::scan_devices().unwrap();
//! println!("Found {} WCH ISP USB devices", num_devices);
//! ```
//!
//! Opening a specific USB device:
//!
//! ```
//! use wchisp::transport::usb::UsbTransport;
//!
//! let transport = UsbTransport::open_nth(0).unwrap();
//! ```
//!
//! Sending and receiving raw data:
//!
//! ```
//! use wchisp::transport::usb::UsbTransport;
//! use std::time::Duration;
//!
//! let mut transport = UsbTransport::open_any().unwrap();
//!
//! let data = vec![0x01, 0x02, 0x03];
//! transport.send_raw(&data).unwrap();
//!
//! let timeout = Duration::from_secs(1);
//! let received_data = transport.recv_raw(timeout).unwrap();
//! println!("Received data: {:?}", received_data);
//! ```
//!
//! # Notes
//!
//! - This module requires the `rusb` and `anyhow` crates.
//! - USB devices must have the vendor ID `0x4348` and product ID `0x55e0` to be recognized.
//! - Endpoint addresses `0x02` and `0x82` are used for data transfer.
//! - The USB timeout is set to 5000 milliseconds.
//!
//! For more information, refer to the [README.md](https://github.com/martin/wchisp/blob/main/README.md) file.
//!
//! # Safety
//!
//! - The USB device handle is automatically released when the `UsbTransport` object is dropped.
//! - Communication errors are ignored when releasing the interface.
//! - The USB device is not reset when the `UsbTransport` object is dropped.
//!
//! # Errors
//!
//! This module can return errors in the `anyhow::Result` type. Possible errors include:
//!
//! - Failed to initialize the USB context.
//! - Failed to find a WCH ISP USB device.
//! - Failed to open the USB device.
//! - Failed to claim the USB interface.
//! - USB endpoints not found.
//! - Failed to send or receive data over USB.
//!
//! # References
//!
//! - [rusb crate documentation](https://docs.rs/rusb)
//! - [anyhow crate documentation](https://docs.rs/anyhow)
//! - [Zadig USB driver installation](https://zadig.akeo.ie)
//! - [README.md file](https://github.com/martin/wchisp/blob/main/README.md)
//!
//! # See Also
//!
//! - [wchisp crate documentation](https://docs.rs/wchisp)
//! - [Transport trait documentation](https://docs.rs/wchisp/0.1.0/wchisp/transport/trait.Transport.html)
//!
//! USB Transportation.
use std::time::Duration;

use anyhow::Result;
use rusb::{Context, DeviceHandle, UsbContext};

use super::Transport;

const ENDPOINT_OUT: u8 = 0x02;
const ENDPOINT_IN: u8 = 0x82;

const USB_TIMEOUT_MS: u64 = 5000;

pub struct UsbTransport {
    device_handle: DeviceHandle<rusb::Context>,
}

impl UsbTransport {
    // Count the number of USB devices that match the vendor and product ID.
    pub fn scan_devices() -> Result<usize> {
        let context = Context::new()?;

        let n = context
            .devices()?
            .iter()
            .filter(|device| {
                device
                    .device_descriptor()
                    .map(|desc| desc.vendor_id() == 0x4348 && desc.product_id() == 0x55e0)
                    .unwrap_or(false)
            })
            .enumerate()
            .map(|(i, device)| {
                log::debug!("Found WCH ISP USB device #{}: [{:?}]", i, device);
            })
            .count();
        Ok(n)
    }
    // Attempt to open the nth available device, retrieve devices configuration parameters,
    // checks first interface and its first descriptor for the required endpoints, sets
    // the active configuration and claims the interface.
    pub fn open_nth(nth: usize) -> Result<UsbTransport> {
        let context = Context::new()?;

        let device = context
            .devices()?
            .iter()
            .filter(|device| {
                device
                    .device_descriptor()
                    .map(|desc| desc.vendor_id() == 0x4348 && desc.product_id() == 0x55e0)
                    .unwrap_or(false)
            })
            .nth(nth)
            .ok_or(anyhow::format_err!(
                "No WCH ISP USB device found(4348:55e0 device not found at index #{})",
                nth
            ))?;
        log::debug!("Found USB Device {:?}", device);

        let mut device_handle = match device.open() {
            Ok(handle) => handle,
            #[cfg(target_os = "windows")]
            Err(rusb::Error::NotSupported) => {
                log::error!("Failed to open USB device: {:?}", device);
                log::warn!("It's likely no WinUSB/LibUSB drivers installed. Please install it from Zadig. See also: https://zadig.akeo.ie");
                anyhow::bail!("Failed to open USB device on Windows");
            }
            #[cfg(target_os = "linux")]
            Err(rusb::Error::Access) => {
                log::error!("Failed to open USB device: {:?}", device);
                log::warn!("It's likely the udev rules is not installed properly. Please refer to README.md for more details.");
                anyhow::bail!("Failed to open USB device on Linux due to no enough permission");
            }
            Err(e) => {
                log::error!("Failed to open USB device: {}", e);
                anyhow::bail!("Failed to open USB device");
            }
        };

        let config = device.config_descriptor(0)?;

        let mut endpoint_out_found = false;
        let mut endpoint_in_found = false;
        if let Some(intf) = config.interfaces().next() {
            if let Some(desc) = intf.descriptors().next() {
                for endpoint in desc.endpoint_descriptors() {
                    if endpoint.address() == ENDPOINT_OUT {
                        endpoint_out_found = true;
                    }
                    if endpoint.address() == ENDPOINT_IN {
                        endpoint_in_found = true;
                    }
                }
            }
        }

        if !(endpoint_out_found && endpoint_in_found) {
            anyhow::bail!("USB Endpoints not found");
        }

        device_handle.set_active_configuration(1)?;
        let _config = device.active_config_descriptor()?;
        let _descriptor = device.device_descriptor()?;

        device_handle.claim_interface(0)?;

        Ok(UsbTransport { device_handle })
    }

    // Convenience function to open the first available device
    pub fn open_any() -> Result<UsbTransport> {
        Self::open_nth(0)
    }
}

impl Drop for UsbTransport {
    fn drop(&mut self) {
        // ignore any communication error
        let _ = self.device_handle.release_interface(0);
        // self.device_handle.reset().unwrap();
    }
}

impl Transport for UsbTransport {
    fn send_raw(&mut self, raw: &[u8]) -> Result<()> {
        self.device_handle
            .write_bulk(ENDPOINT_OUT, raw, Duration::from_millis(USB_TIMEOUT_MS))?;
        Ok(())
    }

    fn recv_raw(&mut self, timeout: Duration) -> Result<Vec<u8>> {
        let mut buf = [0u8; 64];
        let nread = self
            .device_handle
            .read_bulk(ENDPOINT_IN, &mut buf, timeout)?;
        Ok(buf[..nread].to_vec())
    }
}
