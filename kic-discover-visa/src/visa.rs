use std::{collections::HashSet, ffi::CString, time::Duration};

use tracing::trace;
use tsp_toolkit_kic_lib::{
    instrument::info::{get_info, InstrumentInfo},
    interface::connection_addr::ConnectionAddr,
};
use visa_rs::{flags::AccessMode, AsResourceManager};

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
        info.address = Some(ConnectionAddr::Visa(i));
        trace!("Got info: {info:?}");
        discovered_instruments.insert(info);
    }
    Ok(discovered_instruments)
}
