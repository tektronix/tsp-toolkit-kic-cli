use std::net::SocketAddr;
use std::{collections::HashSet, time::Duration};

use tsp_toolkit_kic_lib::instrument::info::{ConnectionAddr, InstrumentInfo};

use crate::ethernet::{LxiDeviceInfo, COMM_PORT};
use crate::usbtmc::Usbtmc;

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

    /// Discover instruments over USB
    ///
    /// # Errors
    /// If [`Usbtmc::usb_discover`] fails, and error will be returned.
    pub async fn usb_discover(&self) -> anyhow::Result<HashSet<InstrumentInfo>> {
        let mut discovery_results: HashSet<InstrumentInfo> = HashSet::new();

        match Usbtmc::usb_discover(self.timeout).await {
            Ok(instrs) => {
                for inst in instrs {
                    discovery_results.insert(inst);
                }
            }
            Err(e) => {
                eprintln!("Unable to discover USB devices: {e}"); //TODO add color
                return Err(e);
            }
        }
        Ok(discovery_results)
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
                lxi_info.ip_addr,
                COMM_PORT,
            ))),
        }
    }
}
