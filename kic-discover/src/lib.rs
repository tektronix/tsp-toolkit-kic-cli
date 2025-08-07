use std::{collections::HashSet, hash::Hash, io::Error, sync::Mutex};

use kic_lib::{ki2600, model::ki3700, tti, versatest};

pub mod ethernet;
pub mod instrument_discovery;

#[macro_use]
extern crate lazy_static;

lazy_static! {
    pub static ref DISC_INSTRUMENTS: Mutex<HashSet<String>> = Mutex::new(HashSet::new());
}

#[must_use]
pub fn model_category(in_str: &str) -> &'static str {
    if ki2600::Instrument::model_is(in_str)
        || ki3700::Instrument::model_is(in_str)
        || tti::Instrument::model_is(in_str)
    {
        "tti/26xx"
    } else if versatest::Instrument::model_is(in_str) {
        "versatest"
    } else {
        ""
    }
}

/// Insert a discovered device into our map of instruments
///
/// # Errors
/// If we fail to lock the `DISC_INSTRUMENTS` variable, a [`std::io::Error`]
/// with [`std::io::ErrorKind::PermissionDenied`] will be returned.
pub fn insert_disc_device(device: &str) -> Result<(), Error> {
    DISC_INSTRUMENTS
        .lock()
        .map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "failed to acquire".to_string(),
            )
        })?
        .insert(device.to_string());
    Ok(())
}

#[derive(Debug, Clone, Eq, PartialEq, PartialOrd, Hash, serde::Serialize, serde::Deserialize)]
enum IoType {
    Lan,
}
