use std::{collections::HashSet, time::Duration};

use kic_lib::instrument::info::InstrumentInfo;
use kic_lib::model::{Model, Vendor};

use crate::ethernet::LxiDeviceInfo;

#[derive(Debug)]
pub struct InstrumentDiscovery {
    timeout: Option<Duration>,
}

impl InstrumentDiscovery {
    #[must_use]
    pub const fn new(timeout: Duration) -> Self {
        Self {
            timeout: Some(timeout),
        }
    }

    /// Discover instruments on the network.
    ///
    /// # Errors
    /// If [`LxiDeviceInfo::discover`] fails, an error will be returned
    pub async fn lan_discover(&self) -> anyhow::Result<HashSet<InstrumentInfo>> {
        let mut discovery_results: HashSet<InstrumentInfo> = HashSet::new();

        match LxiDeviceInfo::discover(self.timeout).await {
            Ok(instrs) => {
                for inst in instrs {
                    discovery_results.insert(inst.into());
                }
            }
            Err(e) => {
                eprintln!("Unable to discover LXI devices: {e}"); //TODO add color
                return Err(e);
            }
        };
        Ok(discovery_results)
    }
}

impl From<LxiDeviceInfo> for InstrumentInfo {
    fn from(lxi_info: LxiDeviceInfo) -> Self {
        Self {
            vendor: lxi_info
                .manufacturer
                .parse::<Vendor>()
                .expect("should have parsed manufacturer"),
            model: lxi_info
                .model
                .parse::<Model>()
                .expect("should have parsed model"),
            serial_number: lxi_info.serial_number,
            firmware_rev: Some(lxi_info.firmware_revision),
        }
    }
}
