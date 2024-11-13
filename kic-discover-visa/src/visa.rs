use std::{collections::HashSet, ffi::CString, time::Duration};

use serde::{Deserialize, Serialize};
use tracing::{error, trace};
use tsp_toolkit_kic_lib::{
    instrument::{info::InstrumentInfo, Instrument},
    interface::connection_addr::ConnectionAddr,
    protocol::Protocol,
};
use visa_rs::AsResourceManager;

use crate::{insert_disc_device, model_check, IoType};

#[tracing::instrument]
pub async fn visa_discover(timeout: Option<Duration>) -> anyhow::Result<HashSet<InstrumentInfo>> {
    let mut discovered_instruments: HashSet<InstrumentInfo> = HashSet::new();

    let Ok(rm) = visa_rs::DefaultRM::new() else {
        error!("Unable to get VISA Default Resource Manager");
        return Ok(discovered_instruments);
    };
    let instruments = match rm.find_res_list(&CString::new("?*")?.into()) {
        Ok(x) => x,
        Err(e) => {
            trace!("No VISA instruments found: {e}");
            return Ok(discovered_instruments);
        }
    };
    trace!("discovered: {instruments:?}");

    for i in instruments {
        let Ok(i) = i else {
            continue;
        };
        if i.to_string().contains("SOCKET") || i.to_string().contains("INTFC") {
            continue;
        }
        trace!("Connecting to {i:?} to get info");
        let Ok(interface) = Protocol::try_from_visa(i.to_string()) else {
            trace!("Resource {i} no longer available, skipping.");
            continue;
        };
        let mut connected: Box<dyn Instrument> = match interface.try_into() {
            Ok(c) => c,
            Err(_) => {
                trace!("Resource {i} no longer available, skipping.");
                continue;
            }
        };
        //let Ok(mut connected) = rm.open(&i, AccessMode::NO_LOCK, visa_rs::TIMEOUT_IMMEDIATE) else {
        //    trace!("Resource {i} no longer available, skipping.");
        //    continue;
        //};

        trace!("Getting info from {:?}", i);
        let Ok(mut info) = connected.info() else {
            trace!("Unable to write to {i}, skipping");
            drop(connected);
            continue;
        };
        trace!("Dropping instrument");
        drop(connected);

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
