//! Error types shared by the HEX parser and USB protocol layers.

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("USB error: {0}")]
    Usb(#[from] rusb::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("no TI CC Debugger or compatible SmartRF04EB found via libusb")]
    NoDevice,
    #[error("Intel HEX error: {0}")]
    Hex(String),
    #[error("CC2530 protocol error: {0}")]
    Protocol(String),
    #[error("{operation} short transfer: {actual}/{expected} bytes")]
    ShortTransfer {
        operation: &'static str,
        expected: usize,
        actual: usize,
    },
}

impl Error {
    pub fn hex(message: impl Into<String>) -> Self {
        Self::Hex(message.into())
    }

    pub fn protocol(message: impl Into<String>) -> Self {
        Self::Protocol(message.into())
    }
}
