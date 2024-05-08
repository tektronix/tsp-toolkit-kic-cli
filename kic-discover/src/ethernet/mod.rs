use futures::future::join_all;
use futures_util::{pin_mut, stream::StreamExt};
use local_ip_address::list_afinet_netifas;
use mdns::{Record, RecordKind};
use minidom::Element;
use serde::{Deserialize, Serialize};
use std::hash::Hash;
use std::net::{IpAddr, Ipv4Addr};
use std::{collections::HashSet, time::Duration};
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};

use crate::{insert_disc_device, model_check, IoType};

pub const COMM_PORT: u16 = 5025;
pub const DST_PORT: u16 = 5030;
pub const SERVICE_NAMES: [&str; 3] = [
    "_scpi-raw._tcp.local",
    "_lxi._tcp.local",
    "_vxi-11._tcp.local",
    //"_scpi-telnet._tcp.local",
];

#[allow(clippy::unsafe_derive_deserialize)]
#[derive(Debug, Clone, Eq, PartialEq, PartialOrd, Hash, Serialize, Deserialize)]
pub struct LxiDeviceInfo {
    io_type: IoType,
    pub ip_addr: IpAddr,
    pub manufacturer: String,
    pub model: String,
    pub serial_number: String,
    pub firmware_revision: String,
    socket_port: String,
    instr_categ: String,
}

impl LxiDeviceInfo {
    async fn discover_devices(
        service_name: &str,
        interface_ip: Ipv4Addr,
        device_tx: UnboundedSender<Self>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // The hostname of the devices we are searching for.
        // Every lxi compliant instrument will respond to this service name.
        //Send out a broadcast every X milliseconds for LXI instruments over mDNS

        let stream =
            mdns::discover::interface(service_name, Duration::from_millis(500), interface_ip)?
                .listen();
        pin_mut!(stream);

        while let Some(Ok(response)) = stream.next().await {
            #[cfg(debug_assertions)]
            eprintln!("Found Instrument: {response:?}");
            let addr: Option<IpAddr> = response.records().find_map(Self::to_ip_addr);

            if let Some(addr) = addr {
                #[cfg(debug_assertions)]
                eprintln!("Querying for LXI identification XML page for {addr}");
                if let Some(xmlstr) = Self::query_lxi_xml(addr).await {
                    if let Some(instr) = Self::parse_lxi_xml(&xmlstr, addr) {
                        if let Ok(out_str) = serde_json::to_string(&instr) {
                            insert_disc_device(out_str.as_str())?;
                        }
                        // Send devices back as we discover them
                        device_tx.send(instr)?;
                    }
                }
            }
        }

        Ok(())
    }

    pub async fn query_lxi_xml(instr_addr: IpAddr) -> Option<String> {
        let uri = format!("http://{instr_addr}/lxi/identification");
        if let Ok(resp) = reqwest::get(uri).await {
            if let Ok(resp_text) = resp.text().await {
                return Some(resp_text);
            }
        }
        None
    }

    #[must_use]
    pub fn parse_lxi_xml(xml_data: &str, instr_addr: IpAddr) -> Option<Self> {
        const DEVICE_NS: &str = "http://www.lxistandard.org/InstrumentIdentification/1.0";
        if let Ok(root) = xml_data.parse::<Element>() {
            if root.is("LXIDevice", DEVICE_NS) {
                let manufacturer = root
                    .get_child("Manufacturer", DEVICE_NS)
                    .unwrap_or(&minidom::Element::bare("FirmwareRevision", DEVICE_NS))
                    .text();
                let model = root
                    .get_child("Model", DEVICE_NS)
                    .unwrap_or(&minidom::Element::bare("FirmwareRevision", DEVICE_NS))
                    .text();
                let serial_number = root
                    .get_child("SerialNumber", DEVICE_NS)
                    .unwrap_or(&minidom::Element::bare("FirmwareRevision", DEVICE_NS))
                    .text();
                let firmware_revision = root
                    .get_child("FirmwareRevision", DEVICE_NS)
                    .unwrap_or(&minidom::Element::bare("FirmwareRevision", DEVICE_NS))
                    .text();

                let s1: Vec<&str> = xml_data.split("::SOCKET").collect();
                let port_split: Vec<&str> = s1[0].split("::").collect();
                let socket_port = if port_split.is_empty() {
                    port_split[port_split.len().saturating_sub(1)].to_string()
                } else {
                    "5025".to_string()
                };

                //ToDo: test versatest when it's discoverable
                let res = model_check(model.as_str());

                if manufacturer.to_ascii_lowercase().contains("keithley") && res.0 {
                    let device = Self {
                        io_type: IoType::Lan,
                        ip_addr: instr_addr,
                        manufacturer,
                        model,
                        serial_number,
                        firmware_revision,
                        socket_port,
                        instr_categ: res.1.to_string(),
                    };
                    //println!("{:?}", device);
                    return Some(device);
                }
            }
        }
        None
    }

    fn to_ip_addr(record: &Record) -> Option<IpAddr> {
        match record.kind {
            //A refers to Ipv4 address and AAAA refers to Ipv6 address
            RecordKind::A(addr) => Some(addr.into()),
            RecordKind::AAAA(addr) => Some(addr.into()),
            _ => None,
        }
    }
}

impl LxiDeviceInfo {
    ///Discover LXI devices
    ///
    ///# Errors
    ///Possible errors include but are not limited to those generated by trying
    ///to gather the network interface IPs to iterate over for our search.
    pub async fn discover(timeout: Option<Duration>) -> anyhow::Result<HashSet<Self>> {
        let timeout = timeout.unwrap_or(Duration::new(5, 0));

        let mut discover_futures = Vec::new();

        let interfaces = match list_afinet_netifas() {
            Ok(ips) => ips,
            Err(e) => return Err(Box::new(e).into()),
        };

        let (device_tx, mut device_rx) = unbounded_channel();

        'interface_loop: for (name, ip) in interfaces {
            for service_name in SERVICE_NAMES {
                #[cfg(debug_assertions)]
                eprintln!("Looking for {service_name} on {name} ({ip})");
                if let IpAddr::V4(ip) = ip {
                    discover_futures.push(Self::discover_devices(
                        service_name,
                        ip,
                        device_tx.clone(),
                    ));
                } else {
                    continue 'interface_loop;
                }
            }
        }

        let mut devices: HashSet<Self> = HashSet::new();

        // ignore the error from the timeout since that is our method of stopping execution of the futures
        // This if statement prevents a must_use warning
        let _ = tokio::time::timeout(timeout, join_all(discover_futures)).await;
        // need to drop the last sender or else the while loop will spin forever
        // The other Senders were cloned from this one
        drop(device_tx);
        while let Some(device) = device_rx.recv().await {
            devices.insert(device);
        }

        Ok(devices)
    }
}
