use std::{collections::HashSet, hash::Hash, io::Error, sync::Mutex};

pub mod ethernet;
pub mod instrument_discovery;

#[macro_use]
extern crate lazy_static;

lazy_static! {
    static ref SUPPORTED_SET: HashSet<&'static str> = {
        HashSet::from([
            "2601",
            "2602",
            "2611",
            "2612",
            "2635",
            "2636",
            "2601A",
            "2602A",
            "2611A",
            "2612A",
            "2635A",
            "2636A",
            "2651A",
            "2657A",
            "2601B",
            "2601B-PULSE",
            "2602B",
            "2606B",
            "2611B",
            "2612B",
            "2635B",
            "2636B",
            "2604B",
            "2614B",
            "2634B",
            "2601B-L",
            "2602B-L",
            "2611B-L",
            "2612B-L",
            "2635B-L",
            "2636B-L",
            "2604B-L",
            "2614B-L",
            "2634B-L",
            "3706",
            "3706-SNFP",
            "3706-S",
            "3706-NFP",
            "3706A",
            "3706A-SNFP",
            "3706A-S",
            "3706A-NFP",
            "707B",
            "708B",
        ])
    };
    static ref SUPPORTED_NIMITZ_SET: HashSet<&'static str> = {
        HashSet::from([
            "2450", "2470", "DMM7510", "2460", "2461", "2461-SYS", "DMM7512", "DMM6500", "DAQ6510",
        ])
    };

    //TODO : Remove the TSP entry when LXI page for versatest is available
    static ref SUPPORTED_VERSATEST_SET: HashSet<&'static str> = {
         HashSet::from([
            "VERSATEST-300", "VERSATEST-600", "TSP",
         ])
    };
    pub static ref DISC_INSTRUMENTS: Mutex<HashSet<String>> = Mutex::new(HashSet::new());
}

#[must_use]
pub fn model_check(in_str: &str) -> (bool, &'static str) {
    if let Some(model_split) = in_str
        .to_ascii_uppercase()
        .as_str()
        .split_ascii_whitespace()
        .last()
    {
        if SUPPORTED_SET.contains(model_split) {
            return (true, "tti/26xx");
        } else if SUPPORTED_VERSATEST_SET.contains(in_str) {
            return (true, "versatest");
        }
        return (is_nimitz(in_str), "tti/26xx");
    }
    (false, "")
}

//Nimitz model is the set of instruments that were part of Nimitz program
//Nimitz is also known as the TTI platform
#[must_use]
pub fn is_nimitz(in_str: &str) -> bool {
    if let Some(model_split) = in_str
        .to_ascii_uppercase()
        .as_str()
        .split_ascii_whitespace()
        .last()
    {
        if SUPPORTED_NIMITZ_SET.contains(model_split) {
            return true;
        }
    }
    false
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
