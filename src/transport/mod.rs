//! Abstract Device transport interface.
//! 
//! This module defines the `Transport` trait, which represents an abstraction of the transport layer for a device.
//! The transport layer can be implemented using various communication methods such as USB, serial port, or network.
//! 
//! The `Transport` trait provides methods for sending and receiving raw data, as well as higher-level methods for transferring commands and receiving responses.
//! 
//! # Examples
//! 
//! Implementing a custom transport:
//! 
//! ```rust
//! use std::time::Duration;
//! use anyhow::Result;
//! use crate::protocol::{Command, Response};
//! 
//! pub struct CustomTransport {
//!     // implementation details...
//! }
//! 
//! impl Transport for CustomTransport {
//!     fn send_raw(&mut self, raw: &[u8]) -> Result<()> {
//!         // implementation...
//!         # Ok(())
//!     }
//! 
//!     fn recv_raw(&mut self, timeout: Duration) -> Result<Vec<u8>> {
//!         // implementation...
//!         # Ok(vec![])
//!     }
//! }
//! ```
//! 
//! Using the `Transport` trait:
//! 
//! ```rust
//! use std::time::Duration;
//! use anyhow::Result;
//! use crate::protocol::{Command, Response};
//! 
//! fn send_command<T: Transport>(transport: &mut T, cmd: Command) -> Result<Response> {
//!     transport.transfer(cmd)
//! }
//! ```
//! 
//! # Implementations
//! 
//! The following transport implementations are provided:
//! 
//! - `UsbTransport`: A USB transport implementation.
mod usb;

const DEFAULT_TRANSPORT_TIMEOUT_MS: u64 = 1000;

/// Abstraction of the transport layer.
/// Might be a USB, a serial port, or Network.
pub trait Transport {
    /// Sends raw data over the transport.
    fn send_raw(&mut self, raw: &[u8]) -> Result<()>;

    /// Receives raw data from the transport with a specified timeout.
    fn recv_raw(&mut self, timeout: Duration) -> Result<Vec<u8>>;

    /// Transfers a command over the transport and returns the response.
    /// Uses the default transport timeout.
    fn transfer(&mut self, cmd: Command) -> Result<Response> {
        self.transfer_with_wait(cmd, Duration::from_millis(DEFAULT_TRANSPORT_TIMEOUT_MS))
    }

    /// Transfers a command over the transport and returns the response.
    /// Waits for the specified duration before receiving the response.
    fn transfer_with_wait(&mut self, cmd: Command, wait: Duration) -> Result<Response> {
        let req = &cmd.into_raw()?;
        log::debug!("=> {}   {}", hex::encode(&req[..3]), hex::encode(&req[3..]));
        self.send_raw(&req)?;
        sleep(Duration::from_micros(1)); // required for some Linux platform

        let resp = self.recv_raw(wait)?;
        anyhow::ensure!(req[0] == resp[0], "response command type mismatch");
        log::debug!("<= {} {}", hex::encode(&resp[..4]), hex::encode(&resp[4..]));
        Response::from_raw(&resp)
    }
}
