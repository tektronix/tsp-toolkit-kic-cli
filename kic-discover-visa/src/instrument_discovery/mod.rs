use std::net::SocketAddr;
use std::{collections::HashSet, time::Duration};

use tsp_toolkit_kic_lib::{
    instrument::info::InstrumentInfo, interface::connection_addr::ConnectionAddr,
};

use crate::ethernet::{LxiDeviceInfo, COMM_PORT};
use crate::visa::visa_discover;

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

    // pub async fn discover<T>(&self) -> anyhow::Result<HashSet<DiscoveryInfo>>
    // where
    //     T: Discover,
    // {
    //     let mut discovery_results: HashSet<DiscoveryInfo> = HashSet::new();
    //     match T::discover(self.timeout).await {
    //         Ok(instrs) => {
    //             for inst in instrs {
    //                 discovery_results.insert(inst);
    //             }
    //         }
    //         Err(e) => {
    //             eprintln!("Unable to discover LXI devices: {e}"); //TODO add color
    //             return Err(e);
    //         }
    //     };
    //     Ok(discovery_results)
    // }

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

    pub async fn visa_discover(&self) -> anyhow::Result<HashSet<InstrumentInfo>> {
        visa_discover(self.timeout).await
    }
}

impl From<LxiDeviceInfo> for InstrumentInfo {
    fn from(lxi_info: LxiDeviceInfo) -> Self {
        Self {
            vendor: Some(lxi_info.manufacturer),
            model: Some(lxi_info.model),
            serial_number: Some(lxi_info.serial_number),
            firmware_rev: Some(lxi_info.firmware_revision),
            address: Some(ConnectionAddr::Lan(SocketAddr::new(
                lxi_info.instr_address,
                COMM_PORT,
            ))),
        }
    }
}
