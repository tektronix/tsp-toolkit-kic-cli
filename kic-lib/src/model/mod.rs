use std::{fmt::Display, str::FromStr};

use tracing::{instrument, trace};

use crate::{
    instrument::{authenticate::Authentication, Instrument},
    interface::connection_addr::ConnectionInfo,
    protocol::Protocol,
    InstrumentError,
};

pub mod ki2600;
pub mod ki3700;
pub mod tti;
pub mod versatest;

#[must_use]
pub fn is_supported(model: impl AsRef<str>) -> bool {
    self::ki2600::Instrument::model_is(&model)
        || self::ki3700::Instrument::model_is(&model)
        || self::tti::Instrument::model_is(&model)
        || self::versatest::Instrument::model_is(&model)
}

/// Connect to an instrument given the instrument's connection information and authentication
/// info.
///
/// # Errors
/// Errors may occur when getting the model (for LAN-based connections, this would
/// likely be a [`reqwest`] error from trying to fetch the LXI Identification page).
/// IO errors or parsing errors are possible. There could be errors in establishing the
/// connection as well.
#[instrument(skip(conn, auth))]
pub fn connect_to(
    conn: &ConnectionInfo,
    auth: Authentication,
) -> Result<Box<dyn Instrument>, InstrumentError> {
    trace!("Connecting to {conn}");
    let model: Model = conn.get_model()?;

    Ok(if model.is_2600() {
        Box::new(ki2600::Instrument::connect(conn, auth)?)
    } else if model.is_3700_70x() {
        Box::new(ki3700::Instrument::connect(conn, auth)?)
    } else if model.is_tti() {
        Box::new(tti::Instrument::connect(conn, auth)?)
    } else if model.is_mp() {
        Box::new(versatest::Instrument::connect(conn, auth)?)
    } else {
        trace!("Unable to determine instrument model, defaulting to MP5000 series connection procedure.");
        Box::new(versatest::Instrument::connect(conn, auth)?)
    })
}

/// Connect to an instrument given the instrument's connection information and authentication
/// info.
///
/// # Errors
/// Errors may occur when getting the model (for LAN-based connections, this would
/// likely be a [`reqwest`] error from trying to fetch the LXI Identification page).
/// IO errors or parsing errors are possible. There could be errors in establishing the
/// connection as well.
pub fn connect_protocol(
    conn: &ConnectionInfo,
    proto: Protocol,
    auth: Authentication,
) -> Result<Box<dyn Instrument>, InstrumentError> {
    let model: Model = conn.get_model()?;

    Ok(if model.is_2600() {
        Box::new(ki2600::Instrument::new(proto, auth))
    } else if model.is_3700_70x() {
        Box::new(ki3700::Instrument::new(proto, auth))
    } else if model.is_tti() {
        Box::new(tti::Instrument::new(proto, auth))
    } else if model.is_mp() {
        Box::new(versatest::Instrument::new(proto, auth))
    } else {
        trace!("Unable to determine instrument model, defaulting to MP5000 series connection procedure.");
        Box::new(versatest::Instrument::new(proto, auth))
    })
}

//impl TryFrom<Protocol> for Box<dyn Instrument> {
//    type Error = InstrumentError;
//
//    fn try_from(mut proto: Protocol) -> Result<Self, Self::Error> {
//        let info = proto.info()?;
//        let auth = Box::new(Authenticate {});
//        if tti::Instrument::is(&info) {
//            let mut ins = Box::new(tti::Instrument::new(proto, auth));
//            ins.as_mut().add_info(info);
//            Ok(ins)
//        } else if ki2600::Instrument::is(&info) {
//            let mut ins = Box::new(ki2600::Instrument::new(proto, auth));
//            ins.as_mut().add_info(info);
//            Ok(ins)
//        } else if ki3700::Instrument::is(&info) {
//            let mut ins = Box::new(ki3700::Instrument::new(proto, auth));
//            ins.as_mut().add_info(info);
//            Ok(ins)
//        } else if versatest::Instrument::is(&info) {
//            let mut ins = Box::new(versatest::Instrument::new(proto, auth));
//            ins.as_mut().add_info(info);
//            Ok(ins)
//        } else {
//            Err(InstrumentError::InstrumentError {
//                error: "unable to determine instrument type".to_string(),
//            })
//        }
//    }
//}
//
//impl TryFrom<Box<dyn Interface>> for Box<dyn Instrument> {
//    type Error = InstrumentError;
//
//    fn try_from(mut interface: Box<dyn Interface>) -> std::result::Result<Self, Self::Error> {
//        let info = interface.as_mut().info()?;
//        let auth = Box::new(Authenticate {});
//        if tti::Instrument::is(&info) {
//            let mut ins = Box::new(tti::Instrument::new(
//                protocol::Protocol::Raw(interface),
//                auth,
//            ));
//            ins.as_mut().add_info(info);
//            Ok(ins)
//        } else if ki2600::Instrument::is(&info) {
//            let mut ins = Box::new(ki2600::Instrument::new(
//                protocol::Protocol::Raw(interface),
//                auth,
//            ));
//            ins.as_mut().add_info(info);
//            Ok(ins)
//        } else if ki3700::Instrument::is(&info) {
//            let mut ins = Box::new(ki3700::Instrument::new(
//                protocol::Protocol::Raw(interface),
//                auth,
//            ));
//            ins.as_mut().add_info(info);
//            Ok(ins)
//        } else if versatest::Instrument::is(&info) {
//            let mut ins = Box::new(versatest::Instrument::new(
//                protocol::Protocol::Raw(interface),
//                auth,
//            ));
//            ins.as_mut().add_info(info);
//            Ok(ins)
//        } else {
//            Err(InstrumentError::InstrumentError {
//                error: "unable to determine instrument type".to_string(),
//            })
//        }
//    }
//}

const TEKTRONIX_VID: u16 = 0x0699u16;
const KEITHLEY_VID: u16 = 0x05E6u16;

#[repr(u16)]
#[derive(Clone, Debug, PartialEq, Hash, Eq, Default, serde::Serialize)]
pub enum Vendor {
    #[serde(rename = "TEKTRONIX")]
    Tektronix = TEKTRONIX_VID,

    #[default]
    #[serde(rename = "Keithley Instruments")]
    Keithley = KEITHLEY_VID,
}

impl TryFrom<u16> for Vendor {
    type Error = InstrumentError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            TEKTRONIX_VID => Ok(Self::Tektronix),
            KEITHLEY_VID => Ok(Self::Keithley),
            _ => Err(Self::Error::UnknownVendor(format!(
                "the vendor ID '{value}' is not recognized"
            ))),
        }
    }
}

impl Display for Vendor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Tektronix => "TEKTRONIX",
                Self::Keithley => "Keithley Instruments",
            }
        )
    }
}

impl FromStr for Vendor {
    type Err = InstrumentError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if s.to_uppercase().contains("TEKTRONIX") {
            Ok(Self::Tektronix)
        } else if s.to_uppercase().contains("KEITHLEY") {
            Ok(Self::Keithley)
        } else {
            Err(InstrumentError::UnknownVendor(format!(
                "did not recognize vendor '{s}'"
            )))
        }
    }
}

/// This macro is intended only to define instrument models. It should not be used for
/// any other purpose and must not be made public in any way.
///
/// This macro generates an enum with a list of variants that represent all the models
/// this library supports (as well as an `Other` variant which allows the use of unsupported
/// instruments on a best-guess basis). It also implements [`std::str::FromStr`],
/// [`std::fmt::Display`], and [`std::convert::From<u16>`] (for USB product IDs) for
/// each model provided.
///
/// # Syntax
/// The syntax is as follows:
/// ```ignore
/// define_models! {
///     <VIS> enum Models[<ERROR-TYPE>, <FAMILY-TYPE>] {
///         <VARIANT-NAME>[<u16-PID] <- [<STRING-REPRESENTATION>,<OTHER-STRING-REPS>...] #<FAMILY-GROUP,
///         <VARIANT-NAME> <- [<STRING-REPRESENTATION>,<OTHER-STRING-REPS>...] #<FAMILY-GROUP,
///     }
/// }
/// ```
/// # Example
///
/// ```ignore
/// # #[derive(Debug, Clone, thiserror::Error)]
/// # pub enum CarError {
/// #   #[error("Error: {0}")]
/// #   A(String),
/// # }
///
/// pub enum Family {
///     A,
///     B,
///     C,
/// }
///
/// define_models! {
///     pub enum Cars[CarError, Family] {
///         ModelT <- ["Ford Model T", "Model T", "Mr. T"] #Family::A,
///         _300[0x0300] <- ["Chrysler 300", "300"] #Family::B,
///         M3[0x0003] <- ["BMW M3"] #Family::C,
///     }
/// }
/// ```
macro_rules! define_models {
    (
        pub enum $name:ident[$error:path, $fam:path] {
            $(
                $variant:ident$([$pid:expr])? <- [$string_val:literal$(,$alt_string_val:expr),* $(,)?] #$family:path
            ),+ $(,)?
        }
    ) => {
        #[derive(Debug, Hash, PartialEq, Eq, Clone, serde::Serialize)]
        pub enum $name {
            $(#[serde(rename=$string_val)]$variant),+,
            #[serde(rename="Unknown Model")]
            Other(String),
        }

        impl Default for $name {
            fn default() -> Self {Self::Other(String::default())}
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", match self {
                    $(
                        $name::$variant => $string_val
                    ),+,
                    $name::Other(name) => name,
                })
            }
        }

        impl std::str::FromStr for $name {
            type Err = $error;
            fn from_str(val: &str) -> Result<Self, Self::Err> {
                match val {
                    $(
                        $string_val$( | $alt_string_val)* => Ok($name::$variant)
                    ),+,
                    _ => Ok($name::Other(val.to_string()))
                }
            }
        }

        impl $name {
            #[must_use]
            pub const fn family(&self) -> Option<$fam> {
                match self {
                    $(
                        $name::$variant => Some($family)
                    ),+,
                    $name::Other(_) => None,
                }
            }

            #[must_use]
            pub fn from_pid(pid: u16) -> Self {
                $(
                    define_models!(@match_pid $name, $variant, pid, $($pid)?);
                )*
                $name::Other(format!("PID: {pid:#X}"))
            }
        }
    };
    (@match_pid $name:ident, $variant:ident, $input:ident, $pid:literal) => {
        if $input == $pid {
            return $name::$variant;
        }
    };
    (@match_pid $name:ident, $variant:ident, $input:ident, ) => { };
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Family {
    _26xx,
    _3700,
    Tti,
    ModularPlatform,
}

define_models! {
    pub enum Model[InstrumentError, Family] {
        //2600
        _2601A <- ["2601"] #Family::_26xx,
        _2602A <- ["2602"] #Family::_26xx,
        _2611A <- ["2611"] #Family::_26xx,
        _2612A <- ["2612"] #Family::_26xx,
        _2635A <- ["2635"] #Family::_26xx,
        _2636A <- ["2636"] #Family::_26xx,
        _2651A <- ["2651"] #Family::_26xx,
        _2657A <- ["2657"] #Family::_26xx,
        _2601B[0x2601] <- ["2601B"] #Family::_26xx,
        _2601BPulse[0x26F1] <- ["2601B-PULSE"] #Family::_26xx,
        _2602B[0x2602] <- ["2602B"] #Family::_26xx,
        _2606B[0x2606] <- ["2606B"] #Family::_26xx,
        _2611B[0x2611] <- ["2611B"] #Family::_26xx,
        _2612B[0x2612] <- ["2612B"] #Family::_26xx,
        _2635B[0x2635] <- ["2635B"] #Family::_26xx,
        _2636B[0x2636] <- ["2636B"] #Family::_26xx,
        _2604B[0x2604] <- ["2604B"] #Family::_26xx,
        _2614B[0x2614] <- ["2614B"] #Family::_26xx,
        _2634B[0x2634] <- ["2634B"] #Family::_26xx,
        _2601BL <- ["2601B-L"] #Family::_26xx,
        _2602BL <- ["2602B-L"] #Family::_26xx,
        _2611BL <- ["2611B-L"] #Family::_26xx,
        _2612BL <- ["2612B-L"] #Family::_26xx,
        _2635BL <- ["2635B-L"] #Family::_26xx,
        _2636BL <- ["2636B-L"] #Family::_26xx,
        _2604BL <- ["2604B-L"] #Family::_26xx,
        _2614BL <- ["2614B-L"] #Family::_26xx,
        _2634BL <- ["2634B-L"] #Family::_26xx,

        // 3706 or 70xB
        _3706 <- ["3706"] #Family::_3700,
        _3706S <- ["3706-S"] #Family::_3700,
        _3706SNFP <- ["3706-SNFP"] #Family::_3700,
        _3706NFP <- ["3706-NFP"] #Family::_3700,
        _3706A[0x3706] <- ["3706A"] #Family::_3700,
        _3706AS <- ["3706A-S"] #Family::_3700,
        _3706ASNFP <- ["3706A-SNFP"] #Family::_3700,
        _3706ANFP <- ["3706A-NFP"] #Family::_3700,
        _707B[0x707B] <- ["707B"] #Family::_3700,
        _708B[0x708B] <- ["708B"] #Family::_3700,
        _5880Sru <- ["5880_SRU"] #Family::_3700,
        _5881Sru <- ["5881_SRU"] #Family::_3700,

        // TTI
        _2450[0x2450] <- ["2450"] #Family::Tti,
        _2470[0x2470] <- ["2470"] #Family::Tti,
        _2460[0x2460] <- ["2460"] #Family::Tti,
        _2461[0x2461] <- ["2461"] #Family::Tti,
        _2461Sys[0x1642] <- ["2461-SYS"] #Family::Tti,
        DMM7500[0x7500] <- ["DMM7500"] #Family::Tti,
        DMM7510[0x7510] <- ["DMM7510"] #Family::Tti,
        DMM7512[0x7512] <- ["DMM7512"] #Family::Tti,
        DMM6500[0x6500] <- ["DMM6500"] #Family::Tti,
        DAQ6510[0x6510] <- ["DAQ6510"] #Family::Tti,

        // Modular Platform
        MP5103[0x5103] <- ["MP5103"] #Family::ModularPlatform,
        TSPop <- ["TSPop", "TSP"] #Family::ModularPlatform,
    }
}

impl Model {
    #[must_use]
    pub const fn is_tti(&self) -> bool {
        matches!(self.family(), Some(Family::Tti))
    }

    #[must_use]
    pub const fn is_mp(&self) -> bool {
        matches!(self.family(), Some(Family::ModularPlatform))
    }

    #[must_use]
    pub const fn is_3700_70x(&self) -> bool {
        matches!(self.family(), Some(Family::_3700))
    }

    #[must_use]
    pub const fn is_2600(&self) -> bool {
        matches!(self.family(), Some(Family::_26xx))
    }

    #[must_use]
    pub const fn is_other(&self) -> bool {
        self.family().is_none()
    }
}
