use std::{hash::Hash, sync::mpsc::TryRecvError};

use kic_lib::{ki2600, model::ki3700, tti, versatest};

pub mod ethernet;
pub mod instrument_discovery;

#[cfg(not(feature = "visa"))]
pub mod process;

#[cfg(feature = "visa")]
pub mod visa;

/// A utility struct that, after initialized
pub struct DiscoveredPrinter {
    cancel_tx: std::sync::mpsc::Sender<()>,
}

impl DiscoveredPrinter {
    /// Starts a thread that will print each instrument to stdout one by one as it is
    /// discovered.
    pub fn start() -> (DiscoveredPrinter, std::sync::mpsc::Sender<String>) {
        let (tx, rx) = std::sync::mpsc::channel::<String>();
        let (cancel_tx, cancel_rx) = std::sync::mpsc::channel::<()>();
        let _jh = std::thread::spawn(move || loop {
            if cancel_rx.try_recv().is_ok() {
                return;
            }

            match rx.try_recv() {
                Ok(x) => println!("{x}"),
                Err(TryRecvError::Disconnected) => return,
                _ => continue,
            }
        });

        (Self { cancel_tx }, tx)
    }

    pub async fn stop(&self) {
        let _ = self.cancel_tx.send(());
    }
}

impl Drop for DiscoveredPrinter {
    fn drop(&mut self) {
        let _ = self.cancel_tx.send(());
    }
}

#[must_use]
pub fn model_category(in_str: &str) -> &'static str {
    if ki2600::Instrument::model_is(in_str)
        || ki3700::Instrument::model_is(in_str)
        || tti::Instrument::model_is(in_str)
    {
        "tti/26xx"
    } else if versatest::Instrument::model_is(in_str) {
        "versatest"
    } else {
        ""
    }
}

#[derive(Debug, Clone, Eq, PartialEq, PartialOrd, Hash, serde::Serialize, serde::Deserialize)]
enum IoType {
    Lan,
    Visa,
    Usb,
}
