use crate::{
    instrument::{info::get_info, Instrument},
    InstrumentError, Interface,
};

pub mod ki2600;
pub mod tti;
pub mod versatest;

impl TryFrom<Box<dyn Interface>> for Box<dyn Instrument> {
    type Error = InstrumentError;

    fn try_from(mut interface: Box<dyn Interface>) -> std::result::Result<Self, Self::Error> {
        let info = get_info(interface.as_mut())?;
        if tti::Instrument::is(&info) {
            Ok(Box::new(tti::Instrument::new(interface)))
        } else if ki2600::Instrument::is(&info) {
            Ok(Box::new(ki2600::Instrument::new(interface)))
        } else if versatest::Instrument::is(&info) {
            Ok(Box::new(versatest::Instrument::new(interface)))
        } else {
            Err(InstrumentError::InstrumentError {
                error: "unable to determine instrument type".to_string(),
            })
        }
    }
}
