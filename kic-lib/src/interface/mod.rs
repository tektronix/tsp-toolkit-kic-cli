use crate::{
    error::Result,
    instrument::{
        info::{get_info, InstrumentInfo},
        Info,
    },
};
use std::{
    io::{Read, Write},
    net::TcpStream,
};

pub mod async_stream;
pub mod connection_addr;

/// Defines a marker trait that we will implement on each device interface
pub trait Interface: NonBlock + Read + Write {}

/// This device can be set to be non-blocking. This is a requirement of an Interface
pub trait NonBlock {
    /// Set the interface not to block on reads to allow for polling.
    ///
    /// # Errors
    /// There may be errors that occur from the associated physical interface
    /// (e.g. LAN, USB).
    fn set_nonblocking(&mut self, enable: bool) -> Result<()>;
}

impl NonBlock for TcpStream {
    fn set_nonblocking(&mut self, enable: bool) -> crate::error::Result<()> {
        Ok(Self::set_nonblocking(self, enable)?)
    }
}

impl Info for TcpStream {
    // write all methods for Info trait here
    fn info(&mut self) -> Result<InstrumentInfo> {
        get_info(self)
    }
}

impl Interface for TcpStream {}
