#![feature(rustdoc_missing_doc_code_examples, stmt_expr_attributes, io_error_more)]
#![deny(
    clippy::undocumented_unsafe_blocks,
    clippy::pedantic,
    clippy::nursery,
    clippy::arithmetic_side_effects
)]
#![feature(assert_matches)]
use std::env;

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod command;
pub mod error;
pub mod instrument;
pub mod repl;
mod resources;
mod state_machine;
pub mod tsp_error;

pub use error::InstrumentReplError;
pub use tsp_error::{InstrumentTime, TspError};

pub mod new {

    pub mod ui {
        use colored::Colorize;
        use indicatif::{ProgressBar, ProgressStyle};
        use std::{
            env,
            path::PathBuf,
            sync::mpsc::{Receiver, Sender, TryRecvError},
            thread::JoinHandle,
        };
        use tsp_toolkit_kic_lib::new::instrument::event::Event as InstrEvent;

        use super::repl::Event as ReplEvent;
        use crate::error::InstrumentReplError;

        enum State {
            Idle,
            Progress { pb: ProgressBar },
        }

        pub(crate) enum Event {
            Exit,
            Script {
                path: PathBuf,
                save: bool,
                run: bool,
            },
            Upgrade {
                path: PathBuf,
            },
            Info,
            Abort,
            Tsp(Vec<u8>),
        }

        pub(crate) struct Ui {
            #[allow(clippy::struct_field_names)]
            ui_tx: Sender<Event>,
            repl_rx: Receiver<ReplEvent>,
            inst_rx: Receiver<InstrEvent>,
            ansi_enabled: bool,
            state: State,
        }

        impl Ui {
            pub(crate) fn new(
                ui_tx: Sender<Event>,
                repl_rx: Receiver<ReplEvent>,
                inst_rx: Receiver<InstrEvent>,
            ) -> Self {
                let ansi_enabled = env::var("NO_COLOR").is_ok();

                Ui {
                    ui_tx,
                    repl_rx,
                    inst_rx,
                    ansi_enabled,
                    state: State::Idle,
                }
            }

            fn add_progress_bar(
                &mut self,
                progress_msg: String,
                finished_message: String,
                len: usize,
            ) {
                let pb = ProgressBar::new(len.try_into().unwrap())
                    .with_style(
                        ProgressStyle::with_template(
                            "{spinner:.green} [{elapsed_precise}] [{wide_bar}] {bytes}/{total_bytes} ({eta})"
                        ).expect("style of progress bar should be set")
                    )
                    .with_finish(indicatif::ProgressFinish::WithMessage(finished_message.into()))
                    .with_message(progress_msg);
                self.state = State::Progress { pb }
            }

            fn handle_repl_events(&self) -> core::result::Result<(), TryRecvError> {
                if matches!(self.state, State::Idle) {
                    match self.repl_rx.try_recv() {
                        Ok(event) => match event {
                            ReplEvent::Prompt => print!("\n{}", "TSP> ".blue()),
                            ReplEvent::TspError(tsp_error) => {
                                println!("{}", tsp_error.to_string().red());
                            }
                            ReplEvent::Data(vec) => {
                                print!("{}", String::from_utf8_lossy(&vec).normal());
                            }
                        },
                        Err(e) => return Err(e),
                    }
                }

                Ok(())
            }

            fn handle_instrument_events(&mut self) -> core::result::Result<(), TryRecvError> {
                match self.state {
                    State::Idle => match self.inst_rx.try_recv() {
                        Ok(event) => match event {
                            InstrEvent::Connected(instrument_info) => {
                                println!("{instrument_info}");
                            }
                            InstrEvent::WriteProgress(progress) => {
                                self.add_progress_bar(
                                    "Writing Command".to_string(),
                                    "Command Written".to_string(),
                                    progress.total,
                                );
                                if let State::Progress { ref pb } = self.state {
                                    pb.set_position(progress.written.try_into().unwrap());
                                    pb.tick();
                                }
                            }
                            InstrEvent::FwProgress(progress) => {
                                self.add_progress_bar(
                                    "Loading Firmware".to_string(),
                                    "Firmware Loading Complete".to_string(),
                                    progress.total,
                                );
                                if let State::Progress { ref pb } = self.state {
                                    pb.set_position(progress.written.try_into().unwrap());
                                    pb.tick();
                                }
                            }
                            InstrEvent::ScriptProgress(progress) => {
                                self.add_progress_bar(
                                    "Loading Script".to_string(),
                                    "Script Loading Complete".to_string(),
                                    progress.total,
                                );
                                if let State::Progress { ref pb } = self.state {
                                    pb.set_position(progress.written.try_into().unwrap());
                                    pb.tick();
                                }
                            }
                            InstrEvent::FwComplete
                            | InstrEvent::ScriptComplete
                            | InstrEvent::WriteComplete => {
                                if let State::Progress { ref pb } = self.state {
                                    pb.finish();
                                    self.state = State::Idle;
                                }
                            }
                        },
                        Err(e) => return Err(e),
                    },
                    State::Progress { ref pb } => match self.inst_rx.try_recv() {
                        Ok(event) => match event {
                            InstrEvent::WriteProgress(progress)
                            | InstrEvent::ScriptProgress(progress)
                            | InstrEvent::FwProgress(progress) => {
                                pb.inc(
                                    progress
                                        .written
                                        .saturating_sub(pb.position().try_into().unwrap())
                                        .try_into()
                                        .unwrap(),
                                );
                                pb.tick();
                            }
                            InstrEvent::FwComplete
                            | InstrEvent::ScriptComplete
                            | InstrEvent::WriteComplete => {
                                pb.finish();
                            }
                            // Nothing else supported while in `Progress` state
                            InstrEvent::Connected(_) => {}
                        },
                        Err(e) => return Err(e),
                    },
                }

                Ok(())
            }

            pub(crate) fn start(
                mut ui: Self,
            ) -> core::result::Result<JoinHandle<()>, InstrumentReplError> {
                Ok(std::thread::Builder::new()
                    .name("UI Thread".to_string())
                    .spawn(move || 'ui_loop: loop {
                        match ui.handle_repl_events() {
                            Ok(()) | Err(TryRecvError::Empty) => {}
                            Err(TryRecvError::Disconnected) => break 'ui_loop,
                        }
                        match ui.handle_instrument_events() {
                            Ok(()) | Err(TryRecvError::Empty) => {}
                            Err(TryRecvError::Disconnected) => break 'ui_loop,
                        }
                    })?)
            }
        }
    }

    pub mod repl {
        use std::{io::Write, sync::mpsc, thread::JoinHandle};

        use tsp_toolkit_kic_lib::new::instrument::Instrument;

        use super::ui::{Event as UiEvent, Ui};

        use crate::{tsp_error, InstrumentReplError};

        pub enum Event {
            Prompt,
            TspError(tsp_error::TspError),
            Data(Vec<u8>),
        }
        pub struct Repl {
            inst: Instrument,
            ui: JoinHandle<()>,
            ui_rx: mpsc::Receiver<UiEvent>,
            repl_tx: mpsc::Sender<Event>,
        }

        impl Repl {
            pub fn new(mut instrument: Instrument) -> Result<Self, InstrumentReplError> {
                let (repl_tx, repl_rx) = mpsc::channel();
                let (ui_tx, ui_rx) = mpsc::channel();
                let (inst_tx, inst_rx) = mpsc::channel();
                let ui = Ui::new(ui_tx, repl_rx, inst_rx);

                instrument.subscribe_events(inst_tx);

                Ok(Self {
                    inst: instrument,
                    ui: Ui::start(ui)?,
                    ui_rx,
                    repl_tx,
                })
            }

            pub fn start(mut self) -> crate::error::Result<()> {
                'repl: loop {
                    match self.ui_rx.try_recv() {
                        Ok(m) => match m {
                            UiEvent::Exit => todo!(),
                            UiEvent::Script { .. } => todo!(),
                            UiEvent::Upgrade { .. } => todo!(),
                            UiEvent::Info => todo!(),
                            UiEvent::Abort => todo!(),
                            UiEvent::Tsp(t) => {
                                self.inst.write_all(&t)?;
                            }
                        },
                        Err(mpsc::TryRecvError::Empty) => {}
                        Err(mpsc::TryRecvError::Disconnected) => break 'repl,
                    }
                }

                drop(self.repl_tx);
                let _ = self.ui.join();
                Ok(())
            }
        }
    }
}
