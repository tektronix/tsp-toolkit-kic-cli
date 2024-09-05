use std::{collections::HashSet, ffi::CString, time::Duration};

use serde::{Deserialize, Serialize};
use tracing::trace;
use tsp_toolkit_kic_lib::{
    instrument::info::{get_info, InstrumentInfo},
    interface::connection_addr::ConnectionAddr,
};
use visa_rs::{flags::AccessMode, AsResourceManager};

use crate::{insert_disc_device, model_check, IoType};

#[tracing::instrument]
pub async fn visa_discover(timeout: Option<Duration>) -> anyhow::Result<HashSet<InstrumentInfo>> {
    let mut discovered_instruments: HashSet<InstrumentInfo> = HashSet::new();

    let rm = visa_rs::DefaultRM::new()?;
    let instruments = rm.find_res_list(&CString::new("?*")?.into())?;
    trace!("discovered: {instruments:?}");

    for i in instruments {
        let i = i?;
        if i.to_string().contains("SOCKET") {
            continue;
        }
        trace!("Connecting to {i:?} to get info");
        let mut connected = rm.open(&i, AccessMode::NO_LOCK, visa_rs::TIMEOUT_IMMEDIATE)?;
        trace!("Getting info from {connected:?}");
        let mut info = get_info(&mut connected)?;
        info.address = Some(ConnectionAddr::Visa(i.clone()));
        trace!("Got info: {info:?}");
        let res = model_check(info.clone().model.unwrap_or("".to_string()).as_str());
        if res.0 {
            if let Ok(out_str) = serde_json::to_string(&VisaDeviceInfo {
                io_type: IoType::Visa,
                instr_address: i.to_string(),
                manufacturer: "Keithley Instruments".to_string(),
                model: info.clone().model.unwrap_or("UNKNOWN".to_string()),
                serial_number: info.clone().serial_number.unwrap_or("UNKNOWN".to_string()),
                firmware_revision: info.clone().firmware_rev.unwrap_or("UNKNOWN".to_string()),
                instr_categ: model_check(info.clone().model.unwrap_or("".to_string()).as_str())
                    .1
                    .to_string(),
            }) {
                insert_disc_device(out_str.as_str())?;
            }
            discovered_instruments.insert(info);
        }
    }
    Ok(discovered_instruments)
}

#[derive(Debug, Clone, Eq, PartialEq, PartialOrd, Hash, Serialize, Deserialize)]
pub struct VisaDeviceInfo {
    io_type: IoType,
    instr_address: String,
    manufacturer: String,
    model: String,
    serial_number: String,
    firmware_revision: String,
    instr_categ: String,
}
