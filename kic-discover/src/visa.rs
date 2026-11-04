use std::{
    collections::{HashMap, HashSet},
    ffi::CString,
    net::IpAddr,
    time::{Duration, Instant},
};

use kic_lib::{
    instrument::info::InstrumentInfo, interface::connection_addr::ConnectionInfo, model::Model,
};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, trace};
use visa_rs::AsResourceManager;

use crate::{ethernet::LxiDeviceInfo, model_category, IoType};

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
pub async fn visa_discover(
    timeout: Option<Duration>,
    tx: crossbeam_channel::Sender<String>,
) -> anyhow::Result<HashSet<InstrumentInfo>> {
    let start = Instant::now();
    let mut discovered_instruments: HashSet<InstrumentInfo> = HashSet::new();
    let mut discovered_resources: HashMap<String, (Instant, InstrumentInfo)> = HashMap::new();

    loop {
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

            // Tektronix instruments currently only support HiSLIP/VXI-11 (TCPIP), GPIB, and USBTMC (USB) connections
            // KIC lacks support for TCPIP::*::SOCKET and *::INTFC connections
            let s = i.to_string();
            if !(s.contains("TCPIP") || s.contains("GPIB") || s.contains("USB"))
                || s.contains("SOCKET")
                || s.contains("INTFC")
                || discovered_resources
                    .get(&s)
                    .is_some_and(|(last_checked, _)| {
                        last_checked.elapsed() < Duration::from_secs(2)
                    })
            {
                continue;
            }

            let Ok(info) = s.parse::<ConnectionInfo>() else {
                continue;
            };
            // Since we are using reqwest::blocking::Client, we need to using
            // tokio::task::spawn_blocking (see
            // https://docs.rs/reqwest/0.12.22/reqwest/blocking/index.html for more information)
            let info: Option<InstrumentInfo> =
                tokio::task::spawn_blocking(move || info.ping().ok()).await?;

            if let Some(info) = info {
                let changed = discovered_resources
                    .get(&s)
                    .map_or(true, |(_, previous_info)| previous_info != &info);
                discovered_resources.insert(s.clone(), (Instant::now(), info.clone()));
                trace!("Got info: {info:?}");
                if !matches!(info.model, Model::Other(_)) {
                    if changed {
                        if let Ok(out_str) = serde_json::to_string(&VisaDeviceInfo {
                            io_type: IoType::Visa,
                            instr_address: s,
                            manufacturer: info.vendor.to_string(),
                            model: info.model.to_string(),
                            serial_number: info.serial_number.to_string(),
                            firmware_revision: info
                                .firmware_rev
                                .clone()
                                .unwrap_or("UNKNOWN".to_string()),
                            instr_categ: model_category(&info.model.to_string()).to_string(),
                        }) {
                            tx.send(out_str.to_string())?;
                        }
                    }
                    discovered_instruments.insert(info.clone());
                }
            }
        }
        // Continue discovering new instruments without continuously rescanning.
        if timeout.is_none() || (Instant::now() - start) >= timeout.unwrap() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
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
