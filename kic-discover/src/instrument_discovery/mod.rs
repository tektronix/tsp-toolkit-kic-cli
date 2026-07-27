#[cfg(feature = "visa")]
use std::collections::HashSet;
use std::time::Duration;

use kic_lib::instrument::info::InstrumentInfo;
use kic_lib::model::{Model, Vendor};

use crate::ethernet::LxiDeviceInfo;

#[cfg(feature = "visa")]
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

    /// Discover instruments on the network.
    ///
    /// # Errors
    /// If [`LxiDeviceInfo::discover`] fails, an error will be returned
    pub async fn lan_discover(&self, tx: std::sync::mpsc::Sender<String>) -> anyhow::Result<()> {
        LxiDeviceInfo::discover(self.timeout, tx).await?;
        Ok(())
    }

    #[cfg(feature = "visa")]
    pub async fn visa_discover(
        &self,
        tx: std::sync::mpsc::Sender<String>,
    ) -> anyhow::Result<HashSet<InstrumentInfo>> {
        visa_discover(self.timeout, tx.clone()).await
    }
}

impl From<LxiDeviceInfo> for InstrumentInfo {
    fn from(lxi_info: LxiDeviceInfo) -> Self {
        let vendor = lxi_info
            .manufacturer
            .parse::<Vendor>()
            .expect("should have parsed manufacturer");
        let model = lxi_info
            .model
            .parse::<Model>()
            .expect("should have parsed model");
        let serial_number = lxi_info.serial_number;
        let firmware_rev = Some(lxi_info.firmware_revision);
        Self {
            orig_idn: format!(
                "{vendor},MODEL {model},{serial_number},{}",
                firmware_rev.clone().unwrap_or("UNKNOWN".to_string())
            ),
            vendor,
            model,
            serial_number,
            firmware_rev,
        }
    }
}
