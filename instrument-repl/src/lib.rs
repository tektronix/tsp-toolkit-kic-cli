#![deny(
    clippy::undocumented_unsafe_blocks,
    clippy::pedantic,
    clippy::nursery,
    clippy::arithmetic_side_effects
)]
use std::env;

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod command;
pub mod error;
pub mod instrument;
pub mod repl;
mod resources;
mod state_machine;
pub mod tsp_error;

pub use error::InstrumentReplError;
pub use tsp_error::{InstrumentTime, TspError};
