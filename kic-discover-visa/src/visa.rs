use std::{collections::HashSet, ffi::CString, net::IpAddr, time::Duration};

use serde::{Deserialize, Serialize};
use tracing::{debug, error, trace};
use tsp_toolkit_kic_lib::{
    instrument::info::InstrumentInfo, interface::connection_addr::ConnectionInfo, model::Model,
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
            debug!("No VISA instruments found: {e}");
            return Ok(discovered_instruments);
        }
    };
    debug!("discovered: {instruments:?}");

    for i in instruments {
        let Ok(i) = i else {
            continue;
        };

        if i.to_string().contains("SOCKET") || i.to_string().contains("INTFC") {
            continue;
        }

        let info = i.to_string().parse::<ConnectionInfo>()?;
        let info = info.get_info()?;

        trace!("Got info: {info:?}");
        if !matches!(info.model, Model::Other(_)) {
            if let Ok(out_str) = serde_json::to_string(&VisaDeviceInfo {
                io_type: IoType::Visa,
                instr_address: i.to_string(),
                manufacturer: info.vendor.to_string(),
                model: info.model.to_string(),
                serial_number: info.serial_number.to_string(),
                firmware_revision: info.firmware_rev.clone().unwrap_or("UNKNOWN".to_string()),
                instr_categ: model_category(&info.model.to_string()).to_string(),
            }) {
                insert_disc_device(out_str.as_str())?;
            }
            discovered_instruments.insert(info.clone());
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
