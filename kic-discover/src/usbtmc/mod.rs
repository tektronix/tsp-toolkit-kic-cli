use serde::{Deserialize, Serialize};
use std::{collections::HashSet, hash::Hash, time::Duration};
use tmc::{list_instruments, Instrument, InstrumentHandle, TMCError, TMCResult};
use tsp_instrument::{
    instrument::info::{ConnectionAddr, InstrumentInfo},
    usbtmc::UsbtmcAddr,
};

use crate::{insert_disc_device, model_check, IoType};

pub struct Usbtmc {
    device: rusb::Device<rusb::Context>,
    #[allow(dead_code)]
    handle: Option<InstrumentHandle<rusb::Context>>,
    pub unique_string: String,
}

impl Usbtmc {
    pub fn new(device: rusb::Device<rusb::Context>) -> TMCResult<Self> {
        let vendor = device.device_descriptor()?.vendor_id();
        let product = device.device_descriptor()?.product_id();
        let address = device.address();
        Ok(Self {
            device,
            handle: None,
            unique_string: format!("{vendor:X}:{product:X}:{address}"),
        })
    }

    pub async fn usb_discover(
        _timeout: Option<Duration>,
    ) -> anyhow::Result<HashSet<InstrumentInfo>> {
        let context = match rusb::Context::new() {
            Ok(x) => x,
            Err(e) => {
                return Err(Box::new(e).into());
            }
        };
        let instruments = match list_instruments(context) {
            Ok(x) => x,
            Err(e) => {
                return Err(Box::new(e).into());
            }
        };
        let mut discovered_instrs: HashSet<InstrumentInfo> = HashSet::new();

        if instruments.is_empty() {
            eprintln!("No instruments found");
            //return Ok(());
        }

        // We allow the unused mut here because it is only unused in release mode.
        #[allow(unused_mut)]
        for mut instrument in instruments {
            #[cfg(debug_assertions)]
            eprintln!(
                "Found instrument: {}",
                instrument
                    .read_resource_string()
                    .unwrap_or_else(|_| String::from("[UNKNOWN]"))
            );
            let manufacturer = instrument
                .read_manufacturer_string()?
                .unwrap_or_else(|| String::from("NA"));
            let firmware_revision = match instrument.read_device_version()? {
                Some(version) => version.to_string(),
                None => String::from("NA"),
            };
            let model = String::from(model_lut(instrument.device_desc.product_id()));
            let serial_number = instrument
                .read_serial_number()?
                .unwrap_or_else(|| String::from("NA"))
                .clone();

            let tmc_instr: Result<Usbtmc, TMCError> = instrument.try_into();

            //ToDo: test versatest when it's discoverable
            let res = model_check(model.as_str());
            if manufacturer.to_ascii_lowercase().contains("keithley") && res.0 {
                if let Ok(mut instr) = tmc_instr {
                    let usb_info = UsbDeviceInfo {
                        io_type: IoType::Usb,
                        unique_string: instr.unique_string.clone(),
                        manufacturer,
                        model,
                        serial_number,
                        firmware_revision,
                        instr_categ: res.1.to_string(),
                    };
                    if let Ok(out_str) = serde_json::to_string(&usb_info) {
                        insert_disc_device(out_str.as_str())?;
                    }
                    let usbtmc_addr = UsbtmcAddr {
                        device: instr.device,
                        model: usb_info.model.clone(),
                        serial: usb_info.serial_number.clone(),
                    };
                    let disc_usb_inst = InstrumentInfo {
                        vendor: Some(usb_info.manufacturer),
                        model: Some(usb_info.model),
                        serial_number: Some(usb_info.serial_number.clone()),
                        firmware_rev: Some(usb_info.firmware_revision),
                        address: Some(ConnectionAddr::Usbtmc(usbtmc_addr.clone())),
                    };
                    discovered_instrs.insert(disc_usb_inst);
                }
            }
        }
        Ok(discovered_instrs)
    }
}

impl TryFrom<Instrument<rusb::Context>> for Usbtmc {
    type Error = TMCError;

    fn try_from(value: Instrument<rusb::Context>) -> Result<Self, Self::Error> {
        Usbtmc::new(value.device)
    }
}

const fn model_lut(pid: u16) -> &'static str {
    match pid {
        0x3706 => "3706",
        0xCA7C => "4210-CVU-ACU",
        0x707A => "707A",
        0x707B => "707B",
        0x2100 => "2100",
        0x2110 => "2110",
        0x3390 => "3390",
        0x488B => "K-USB-488B",
        0x2450 => "2450",
        0x2460 => "2460",
        0x2461 => "2461",
        0x1642 => "2461-SYS",
        0x2470 => "2470",
        0x2601 => "2601",
        0x26F1 => "2601B-PULSE",
        0x2602 => "2602B",
        0x2604 => "2604B",
        0x2611 => "2611B",
        0x2612 => "2612B",
        0x2614 => "2614B",
        0x2634 => "2634B",
        0x2635 => "2635B",
        0x2636 => "2636B",
        0x426C => "4200A-CVIV",
        0x6500 => "DMM6500",
        0x6510 => "DAQ6510",
        0x7500 => "DMM7500",
        0x7512 => "DMM7512",
        _ => "UNKNOWN",
    }
}

#[derive(Debug, Clone, Eq, PartialEq, PartialOrd, Hash, Serialize, Deserialize)]
pub struct UsbDeviceInfo {
    io_type: IoType,
    unique_string: String,
    manufacturer: String,
    model: String,
    serial_number: String,
    firmware_revision: String,
    instr_categ: String,
}
