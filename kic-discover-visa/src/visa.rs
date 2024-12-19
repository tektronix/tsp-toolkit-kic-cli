use std::{collections::HashSet, ffi::CString, net::IpAddr, time::Duration};

use serde::{Deserialize, Serialize};
use tracing::{error, trace};
use tsp_toolkit_kic_lib::{
    instrument::{info::InstrumentInfo, Instrument},
    interface::connection_addr::ConnectionAddr,
    model::is_supported,
    protocol::Protocol,
};
use visa_rs::AsResourceManager;

use crate::{ethernet::LxiDeviceInfo, insert_disc_device, model_category, IoType};

/// Extract the IP address from the resource string and then get the [`LxiDeviceInfo`]
/// which can be converted to [`InstrumentInfo`].
/// Returns [`None`] in all error cases
pub async fn visa_tcpip_info(rsc: String) -> Option<InstrumentInfo> {
    let [_, ip_addr, ..] = rsc.split("::").collect::<Vec<&str>>()[..] else {
        return None;
    };
    let instr_addr: IpAddr = ip_addr.parse().ok()?;
    let lxi_xml = LxiDeviceInfo::query_lxi_xml(instr_addr).await?;
    Some(LxiDeviceInfo::parse_lxi_xml(&lxi_xml, instr_addr)?.into())
}

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

        let info = if i.to_string().starts_with("TCPIP") {
            trace!("Getting info from LXI page");
            visa_tcpip_info(i.to_string()).await
        } else {
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

            trace!("Getting info from {:?}", i);
            connected.info().ok()
        };

        if let Some(mut info) = info {
            info.address = Some(ConnectionAddr::Visa(i.clone()));
            trace!("Got info: {info:?}");
            if is_supported(info.clone().model.unwrap_or_default()) {
                if let Ok(out_str) = serde_json::to_string(&VisaDeviceInfo {
                    io_type: IoType::Visa,
                    instr_address: i.to_string(),
                    manufacturer: "Keithley Instruments".to_string(),
                    model: info.clone().model.unwrap_or("UNKNOWN".to_string()),
                    serial_number: info.clone().serial_number.unwrap_or("UNKNOWN".to_string()),
                    firmware_revision: info.clone().firmware_rev.unwrap_or("UNKNOWN".to_string()),
                    instr_categ: model_category(&info.clone().model.unwrap_or("".to_string()))
                        .to_string(),
                }) {
                    insert_disc_device(out_str.as_str())?;
                }
                discovered_instruments.insert(info);
            }
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
