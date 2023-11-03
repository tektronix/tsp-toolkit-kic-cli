use std::env;
const VERSION: &str = env!("CARGO_PKG_VERSION");
pub mod command;
pub mod debugger;
pub mod error;
pub mod resources;
