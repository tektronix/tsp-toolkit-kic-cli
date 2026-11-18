use std::hash::Hash;

use crossbeam_channel::{select_biased, unbounded, Receiver, Sender};

use kic_lib::{ki2600, model::ki3700, tti, versatest};

pub mod ethernet;
pub mod instrument_discovery;

#[cfg(not(feature = "visa"))]
pub mod process;

#[cfg(feature = "visa")]
pub mod visa;

/// A utility struct that, after initialized
pub struct DiscoveredPrinter {
    cancel_tx: Sender<()>,
}

impl DiscoveredPrinter {
    /// Starts a thread that will print each instrument to stdout one by one as it is
    /// discovered.
    #[must_use]
    pub fn start() -> (Self, Sender<String>) {
        let (tx, rx) = unbounded::<String>();
        let (cancel_tx, cancel_rx) = unbounded::<()>();
        let _jh = std::thread::spawn(move || {
            receive_discovered(&rx, &cancel_rx, |message| println!("{message}"));
        });

        (Self { cancel_tx }, tx)
    }

    pub fn stop(&self) {
        let _ = self.cancel_tx.send(());
    }
}

impl Drop for DiscoveredPrinter {
    fn drop(&mut self) {
        let _ = self.cancel_tx.send(());
    }
}

fn receive_discovered(
    rx: &Receiver<String>,
    cancel_rx: &Receiver<()>,
    mut print: impl FnMut(String),
) {
    loop {
        select_biased! {
            recv(cancel_rx) -> _ => return,
            recv(rx) -> message => match message {
                Ok(message) => print(message),
                Err(_) => return,
            },
        }
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

#[cfg(test)]
mod tests {
    use super::{receive_discovered, unbounded, DiscoveredPrinter};
    use std::time::Duration;

    #[test]
    fn printer_preserves_messages_and_drains_on_disconnection() {
        let (tx, rx) = unbounded();
        let (_cancel_tx, cancel_rx) = unbounded();
        let cloned_tx = tx.clone();
        tx.send("first".to_string()).unwrap();
        cloned_tx.send("second".to_string()).unwrap();
        tx.send("third".to_string()).unwrap();
        drop(cloned_tx);
        drop(tx);

        let mut messages = Vec::new();
        receive_discovered(&rx, &cancel_rx, |message| messages.push(message));

        assert_eq!(messages, ["first", "second", "third"]);
    }

    #[test]
    fn cancellation_takes_priority_over_queued_messages() {
        let (tx, rx) = unbounded();
        let (cancel_tx, cancel_rx) = unbounded();
        tx.send("queued".to_string()).unwrap();
        cancel_tx.send(()).unwrap();

        receive_discovered(&rx, &cancel_rx, |_| {
            panic!("cancelled printer must not print queued messages");
        });

        assert_eq!(rx.try_recv().unwrap(), "queued");
    }

    #[test]
    fn idle_printer_wakes_for_messages() {
        let (tx, rx) = unbounded();
        let (_cancel_tx, cancel_rx) = unbounded();
        let (output_tx, output_rx) = unbounded();
        let worker = std::thread::spawn(move || {
            receive_discovered(&rx, &cancel_rx, |message| output_tx.send(message).unwrap());
        });

        tx.send("discovered".to_string()).unwrap();
        assert_eq!(
            output_rx.recv_timeout(Duration::from_secs(2)).unwrap(),
            "discovered"
        );
        drop(tx);
        worker.join().unwrap();
    }

    fn assert_idle_printer_stops(
        cancel: impl FnOnce(DiscoveredPrinter) -> Option<DiscoveredPrinter>,
    ) {
        let (_tx, rx) = unbounded();
        let (cancel_tx, cancel_rx) = unbounded();
        let (ready_tx, ready_rx) = unbounded();
        let (finished_tx, finished_rx) = unbounded();
        let printer = DiscoveredPrinter { cancel_tx };
        let worker = std::thread::spawn(move || {
            ready_tx.send(()).unwrap();
            receive_discovered(&rx, &cancel_rx, |_| {
                panic!("idle printer must not receive a message");
            });
            finished_tx.send(()).unwrap();
        });

        ready_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        let _printer = cancel(printer);
        finished_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        worker.join().unwrap();
    }

    #[test]
    fn stop_wakes_idle_printer() {
        assert_idle_printer_stops(|printer| {
            printer.stop();
            Some(printer)
        });
    }

    #[test]
    fn drop_wakes_idle_printer() {
        assert_idle_printer_stops(|printer| {
            drop(printer);
            None
        });
    }
}
