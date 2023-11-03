#![feature(lint_reasons, rustdoc_missing_doc_code_examples)]
#![deny(
    clippy::undocumented_unsafe_blocks,
    clippy::pedantic,
    clippy::nursery,
    clippy::arithmetic_side_effects
)]
#![feature(assert_matches)]
//#![warn(missing_docs, rustdoc::all)]
//#![allow(rustdoc::missing_doc_code_examples)]
#![doc(html_logo_url = "../../../ki-comms_doc_icon.png")]

//! The TSP Instrument crate defines the necessary components to enable communication
//! with an instrument at various levels of abstraction. The aim of the library is to
//! interact correctly, efficiently and reliably with Keithley instruments via multiple
//! connection interfaces. Right now LAN and USBTMC are implemented, and VISA is
//! planned

//pub mod connect;
pub mod error;
pub mod instrument;
pub mod interface;
pub mod model;

#[cfg(test)]
pub(crate) mod test_util;

//pub use connect::{AsyncToSync, Connect, ConnectAsync, ConnectionAddr, Disconnect};
pub use error::InstrumentError;
pub use instrument::firmware::Flash;
pub use interface::{connection_addr::ConnectionAddr, usbtmc, Interface};
pub use model::{ki2600, tti, versatest};
