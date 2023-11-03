use std::fmt::Display;
use std::net::SocketAddr;

use crate::interface::usbtmc::UsbtmcAddr;

/// A generic connection address that covers all the different connection types.
/// Each device interface type will also have a [`TryFrom`] impl that converts from
/// this enum to itself. [`From`] is **not** implemented because the conversion could
/// fail.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ConnectionAddr {
    /// A LAN connection is created with a [`SocketAddr`], which includes an IpAddr and
    /// a port for the connection.
    Lan(SocketAddr),

    /// A USBTMC connection is created with a [`UsbtmcAddr`].
    Usbtmc(UsbtmcAddr),
    //Add other device interface types here
    Unknown,
}

impl Display for ConnectionAddr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Lan(lan_info) => lan_info.to_string(),
            Self::Usbtmc(usb_info) => usb_info.to_string(),
            Self::Unknown => "<UNKNOWN>".to_string(),
        };
        write!(f, "{s}")
    }
}
