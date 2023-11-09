//! A REPL for communicating with an instrument will need to consist of, at minimum, 2
//! parts (threads):
//! 1. stdin processing
//! 2. stdout printing
//! 3. instrument read and write (handled by `ConnectAsync`)
//!     this is done by checking the `read_into` mpsc channel receiver and then checking
//!     the instrument communication line for data coming back from the instrument.

use chrono::Utc;
use clap::{arg, value_parser, Arg, ArgAction, Command};
use colored::Colorize;
use std::{
    fmt::Display,
    fs::File,
    io::{Read, Write},
    path::PathBuf,
    sync::mpsc::{channel, SendError, Sender, TryRecvError},
    thread::JoinHandle,
    time::Duration,
};

use tsp_instrument::instrument::Instrument;

use crate::{
    command::Request,
    error::{InstrumentReplError, Result},
    instrument::{ParsedResponse, ResponseParser},
    resources::KIC_COMMON_TSP,
    state_machine::ReadState,
    TspError,
};

pub struct Repl {
    inst: Box<dyn Instrument>,
    command: Command,
}
/// Clear the output queue of the given TSP-enabled instrument.
///
/// # Errors
/// Errors in this function can range from [`std::io::Error`]s to being unable to
/// clear the output queue in the requested number of attempts.
pub fn clear_output_queue(
    inst: &mut Box<dyn Instrument>,
    max_attempts: usize,
    delay_between_attempts: Duration,
) -> Result<()> {
    let timestamp = Utc::now().to_string();

    inst.write_all(format!("print(\"{timestamp}\")\n").as_bytes())?;

    inst.set_nonblocking(true)?;

    let mut accumulate = String::new();
    for _ in 0..max_attempts {
        std::thread::sleep(delay_between_attempts);
        let mut buf: Vec<u8> = vec![0u8; 512];
        match inst.read(&mut buf) {
            Ok(_) => Ok(()),
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(delay_between_attempts);
                continue;
            }
            Err(e) => Err(e),
        }?;
        let first_null = buf.iter().position(|&x| x == b'\0').unwrap_or(buf.len());
        let buf = &buf[..first_null];
        if !buf.is_empty() {
            accumulate = format!("{accumulate}{}", String::from_utf8_lossy(buf));
        }
        if accumulate.contains(&timestamp) {
            return Ok(());
        }
    }
    Err(InstrumentReplError::Other(
        "unable to clear instrument output queue".to_string(),
    ))
}

impl Repl {
    #[must_use]
    pub fn new(inst: Box<dyn Instrument>) -> Self {
        Self {
            inst,
            command: Self::cli(),
        }
    }

    fn clear_output_queue(
        &mut self,
        max_attempts: usize,
        delay_between_attempts: Duration,
    ) -> Result<()> {
        clear_output_queue(&mut self.inst, max_attempts, delay_between_attempts)
    }

    fn handle_data(
        &mut self,
        data: &[u8],
        mut prompt: bool,
        prev_state: &mut Option<ReadState>,
        state: &mut Option<ReadState>,
    ) -> Result<bool> {
        if !String::from_utf8_lossy(data)
            .trim_end_matches(char::from(0))
            .is_empty()
        {
            let parser = ResponseParser::new(data);
            let mut get_error = false;
            for response in parser {
                *prev_state = *state;
                *state = Some(prev_state.unwrap_or_default().next_state(&response)?);

                match Self::state_action(*prev_state, *state) {
                    Action::Prompt => prompt = true,
                    Action::GetError => {
                        get_error = true;
                    }
                    Action::PrintText => Self::print_data(response)?,
                    Action::PrintHex => Self::print_data(response)?,
                    Action::PrintError => Self::print_data(response)?,
                    Action::None => {}
                }
            }
            if get_error {
                let errors = self.get_errors()?;
                for e in errors {
                    Self::print_data(ParsedResponse::TspError(e.to_string()))?;
                }
                prompt = true;
                *state = Some(ReadState::DataReadEnd);
            }
        }
        Ok(prompt)
    }

    /// Start the Repl
    ///
    /// # Errors
    /// There are many errors that can be returned from this function, they include but
    /// aren't limited to any errors possible from [`std::io::Read`] or [`std::io::Write`]
    #[allow(clippy::too_many_lines)] //This is just going to be a long function
    pub fn start(&mut self) -> Result<()> {
        let mut prev_state: Option<ReadState> = None;
        let mut state: Option<ReadState> = None;
        self.inst.set_nonblocking(true)?;

        let (user_out, loop_in) = channel();

        let join = Self::init_user_input(user_out)?;

        self.clear_output_queue(5000, Duration::from_millis(1))?;

        self.inst.write_script(
            b"_kic_common",
            KIC_COMMON_TSP.to_string().as_bytes(),
            false,
            true,
        )?;

        self.inst.write_all(b"_KIC.prompts_enable(true)\n")?;
        let errors = self.get_errors()?;
        for e in errors {
            Self::print_data(ParsedResponse::TspError(e.to_string()))?;
        }

        let mut prompt = true;
        'user_loop: loop {
            self.inst.set_nonblocking(true)?;
            std::thread::sleep(Duration::from_micros(1));
            let mut read_buf: Vec<u8> = vec![0; 1024];
            let read_size = self.inst.read(&mut read_buf)?;
            let read_buf: Vec<u8> = read_buf[..read_size].into();
            prompt = self.handle_data(&read_buf, prompt, &mut prev_state, &mut state)?;

            // Only request error messages if an error is indicated.
            if prompt {
                prompt = false;
                Self::print_flush(&"\nTSP> ".blue())?;
            }
            match loop_in.try_recv() {
                Ok(msg) => {
                    match msg {
                        Request::Tsp(tsp) => {
                            self.inst.write_all(format!("{tsp}\n").as_bytes())?;
                            prev_state = None;
                        }
                        Request::GetError => {
                            let errors = self.get_errors()?;
                            for e in errors {
                                Self::print_data(ParsedResponse::TspError(e.to_string()))?;
                            }
                            prompt = true;
                        }
                        Request::Script { file } => {
                            let mut contents = String::new();
                            let _ = File::open(&file)?.read_to_string(&mut contents)?;
                            let Some(name) = &file.file_stem() else {
                                return Err(InstrumentReplError::CommandError {
                                    details: "requested script file had no stem".to_string(),
                                });
                            };
                            let Some(name) = name.to_str() else {
                                unreachable!("Could not convert OsStr to &str");
                            };
                            self.inst.write_script(
                                name.as_bytes(),
                                contents.as_bytes(),
                                false,
                                true,
                            )?;
                            prompt = true;
                        }
                        Request::Info { .. } => {
                            Self::println_flush(&self.inst.info()?.to_string().normal())?;
                            prompt = true;
                        }
                        Request::Update { file, slot } => {
                            let mut contents: Vec<u8> = Vec::new();
                            let _ = File::open(&file)?.read_to_end(&mut contents)?;
                            self.inst.flash_firmware(contents.as_ref(), slot)?;
                            // Flashing FW disables prompts before flashing but might
                            // lose runtime state, so we can't save the previous
                            // setting, so we just hardcode it to enabled here.
                            self.inst.write_all(b"localnode.prompts=1\n")?;
                        }
                        Request::Exit => {
                            break 'user_loop;
                        }
                        Request::Help { sub_cmd } => {
                            prompt = true;
                            if let Some(sub_cmd) = sub_cmd {
                                if let Some(mut sub) = self
                                    .command
                                    .get_subcommands()
                                    .find(|e| e.get_name() == sub_cmd)
                                    .map(Command::to_owned)
                                {
                                    sub.print_help()?;
                                    continue 'user_loop;
                                };
                            };
                            self.command.print_help()?;
                        }
                        Request::Usage(s) => {
                            prompt = true;
                            Self::println_flush(&s)?;
                        }
                        Request::None => {
                            prompt = true;
                        }
                    };
                }
                Err(TryRecvError::Disconnected) => break 'user_loop,
                Err(TryRecvError::Empty) => {}
            }
        }
        drop(loop_in);
        let _ = join.join();
        Ok(())
    }

    fn get_errors(&mut self) -> Result<Vec<TspError>> {
        self.inst.write_all(b"print(_KIC.error_message())\n")?;

        let mut errors: Vec<TspError> = Vec::new();

        'error_loop: loop {
            std::thread::sleep(Duration::from_micros(1));
            let mut read_buf: Vec<u8> = vec![0; 1024];
            let _ = self.inst.read(&mut read_buf)?;

            if !(String::from_utf8_lossy(&read_buf).trim_end_matches(char::from(0))).is_empty() {
                let parser = ResponseParser::new(&read_buf);
                for response in parser {
                    match response {
                        ParsedResponse::TspError(e) => {
                            errors.push(serde_json::from_str(e.trim())?);
                        }
                        ParsedResponse::TspErrorEnd => break 'error_loop,
                        _ => {}
                    }
                }
            }
        }

        self.inst.set_nonblocking(true)?;
        Ok(errors)
    }

    fn print_flush<D: Display>(string: &D) -> Result<()> {
        print!("{string}");
        std::io::stdout().flush()?;
        Ok(())
    }

    fn println_flush<D: Display>(string: &D) -> Result<()> {
        println!("{string}");
        std::io::stdout().flush()?;
        Ok(())
    }

    fn print_data(resp: ParsedResponse) -> Result<()> {
        match resp {
            ParsedResponse::TspError(e) => Self::print_flush(&(e + "\n").red()),
            ParsedResponse::Data(d) => Self::print_flush(&String::from_utf8_lossy(&d).to_string()),
            ParsedResponse::BinaryData(b) => Self::print_flush(&format!("{:X?}", &b)),
            ParsedResponse::Prompt
            | ParsedResponse::PromptWithError
            | ParsedResponse::TspErrorStart
            | ParsedResponse::TspErrorEnd
            | ParsedResponse::ProgressIndicator => Ok(()),
        }
    }

    #[allow(clippy::cognitive_complexity)]
    fn cli() -> Command {
        const CMD_TEMPLATE: &str = "\
            {all-args}
        ";
        const SUBCMD_TEMPLATE: &str = "\
            {about-with-newline}\n\
            {usage-heading}\n   {usage}\n\
            \n\
            {all-args}{after-help}\
        ";
        Command::new("repl")
        .multicall(true)
        .disable_help_subcommand(true)
        .allow_external_subcommands(true)
        .subcommand_required(false)
        .help_template(CMD_TEMPLATE)
        .subcommand(
            Command::new(".script").about("Send a script to the connected instrument")
                .help_template(SUBCMD_TEMPLATE)
                .disable_help_flag(true)
                .arg(
                    Arg::new("help").short('h').long("help").help("Print help").action(ArgAction::SetTrue)
                )
                .arg(
                    Arg::new("path").required_unless_present("help")
                )
        )
        .subcommand(
            Command::new(".update").about("Upgrade the firmware on the connected instrument")
                .help_template(SUBCMD_TEMPLATE)
                .disable_help_flag(true)
                .arg(
                    Arg::new("help").short('h').long("help").help("Print help").action(ArgAction::SetTrue)
                )
                .arg(
                    arg!(-s --slot <SLOT_NUM> "Collect information of a specific slot (if applicable) instead of the mainframe").value_parser(value_parser!(u16))
                )
                .arg(
                    Arg::new("path").required_unless_present("help")
                )
        )
        .subcommand(
            Command::new(".help")
                .about("Display help text")
                .help_template(SUBCMD_TEMPLATE)
                .allow_external_subcommands(true),
        )
        .subcommand(
            Command::new(".exit")
                .alias(".quit").help_template(SUBCMD_TEMPLATE)
                .about("Exit the application")
                .disable_help_flag(true)
                .arg(
                    Arg::new("help").short('h').long("help").help("Print help").action(ArgAction::SetTrue)
                ),
        )
        .subcommand(
            Command::new(".info").about("Show the current instrument information.")
                .disable_help_flag(true)
                .help_template(SUBCMD_TEMPLATE)
                .arg(
                    Arg::new("help").short('h').long("help").help("Print help").action(ArgAction::SetTrue)
                )
                .arg(
                    arg!(-s --slot <SLOT_NUM> "Collect information of a specific slot (if applicable) instead of the mainframe").value_parser(value_parser!(usize))
                )
        )
        .disable_help_flag(true)
    }

    #[allow(clippy::too_many_lines)] // This is a parser function, it is unavoidably long
    fn parse_user_commands(input: &str) -> Result<Request> {
        if input.trim().is_empty() {
            return Ok(Request::None);
        }
        if let Ok(path) = PathBuf::try_from(input.trim()) {
            if path.is_file() {
                return Ok(Request::Script { file: path });
            }
        }
        let Some(cmd) = shlex::split(input.trim()) else {
            return Err(crate::InstrumentReplError::CommandError {
                details: "invalid quoting".to_string(),
            });
        };
        let cli = Self::cli();

        let matches = cli.try_get_matches_from(cmd);

        if let Err(e) = matches {
            return Ok(Request::Usage(e.to_string()));
        };

        let matches = matches.unwrap();

        // Send the correct Request based on the input
        Ok(match matches.subcommand() {
            Some((".help", flags)) => match flags.subcommand() {
                Some((cmd, _)) => Request::Help {
                    sub_cmd: Some(cmd.to_string()),
                },
                None => Request::Help { sub_cmd: None },
            },
            Some((".exit", flags)) => match flags.get_one::<bool>("help") {
                Some(help) if *help => Request::Help {
                    sub_cmd: Some(".exit".to_string()),
                },
                _ => Request::Exit,
            },
            Some((".info", flags)) => match flags.get_one::<bool>("help") {
                Some(help) if *help => Request::Help {
                    sub_cmd: Some(".info".to_string()),
                },
                _ => {
                    let slot = flags.get_one::<usize>("slot").copied();
                    Request::Info {
                        //TODO
                        slot,
                    }
                }
            },
            Some((".script", flags)) => match flags.get_one::<bool>("help") {
                Some(help) if *help => Request::Help {
                    sub_cmd: Some(".script".to_string()),
                },
                _ => {
                    let Some(file) = flags.get_one::<String>("path") else {
                        return Err(InstrumentReplError::CommandError {
                            details: "expected file path, but none were provided".to_string(),
                        });
                    };
                    let file = file.clone();
                    let Ok(file) = PathBuf::try_from(file.clone()) else {
                        return Ok(Request::Usage(
                            InstrumentReplError::CommandError {
                                details: format!(
                                    "expected file path, but unable to parse from \"{file}\""
                                ),
                            }
                            .to_string(),
                        ));
                    };
                    if !file.is_file() {
                        return Ok(Request::Usage(
                            InstrumentReplError::Other(format!(
                                "unable to find file \"{}\"",
                                file.to_string_lossy()
                            ))
                            .to_string(),
                        ));
                    }
                    Request::Script { file }
                }
            },
            Some((".update", flags)) => match flags.get_one::<bool>("help") {
                Some(help) if *help => Request::Help {
                    sub_cmd: Some(".update".to_string()),
                },
                _ => {
                    let Some(file) = flags.get_one::<String>("path") else {
                        return Err(InstrumentReplError::CommandError {
                            details: "expected file path, but none were provided".to_string(),
                        });
                    };
                    let file = file.clone();
                    let Ok(file) = file.parse::<PathBuf>() else {
                        return Ok(Request::Usage(
                            InstrumentReplError::CommandError {
                                details: format!(
                                    "expected file path, but unable to parse from \"{file}\""
                                ),
                            }
                            .to_string(),
                        ));
                    };

                    if !file.is_file() {
                        return Ok(Request::Usage(
                            InstrumentReplError::Other(format!(
                                "unable to find file \"{}\"",
                                file.to_string_lossy()
                            ))
                            .to_string(),
                        ));
                    }

                    let slot = flags.get_one::<u16>("slot").copied();
                    Request::Update { file, slot }
                }
            },
            _ => Request::Tsp(input.trim().to_string()),
        })
    }

    /// Start a thread that blocks on user input lines, converts them to the proper request
    /// and `send()`s them on the `out` channel.
    ///
    /// # Return
    /// This function returns a join handle to the created user-input thread.
    ///
    /// # Errors
    /// This function can error if the thread couldn't be created.
    fn init_user_input(out: Sender<Request>) -> Result<JoinHandle<Result<()>>> {
        let jh = std::thread::Builder::new()
            .name("user_input".to_string())
            .spawn(move || {
                'input_loop: loop {
                    // break the loop if told to exit
                    // NOTE: It is possible that we could get stuck on the readline below
                    //       if the caller of this function doesn't close the Sender or send
                    //       a message quickly enough.
                    let mut input = String::new();
                    let _ = std::io::stdin().read_line(&mut input)?;
                    let req = Self::parse_user_commands(&input)?;
                    match out.send(req.clone()) {
                        Ok(()) => {}
                        Err(SendError(_)) => break 'input_loop,
                    }
                    // This `if` statement seeks to fix the NOTE above about not exiting.
                    // It feels a little awkward, but should be effective.
                    if req == Request::Exit {
                        break 'input_loop;
                    }
                }
                Ok(())
            })?;
        Ok(jh)
    }

    const fn state_action(prev_state: Option<ReadState>, state: Option<ReadState>) -> Action {
        match (prev_state, state) {
            (None, Some(state)) => match state {
                ReadState::Init | ReadState::DataReadEnd | ReadState::ErrorReadEnd => {
                    Action::Prompt
                }

                ReadState::DataReadEndPendingError => Action::GetError,

                ReadState::TextDataReadStart | ReadState::TextDataReadContinue => Action::PrintText,

                ReadState::BinaryDataReadStart | ReadState::BinaryDataReadContinue => {
                    Action::PrintHex
                }

                ReadState::ErrorReadContinue => Action::PrintError,

                ReadState::ErrorReadStart | ReadState::FileLoading => Action::None,
            },

            (None | Some(_), None) => Action::None,

            (Some(prev_state), Some(state)) => match (prev_state, state) {
                (_, ReadState::ErrorReadContinue) => Action::PrintError,

                (_, ReadState::DataReadEndPendingError) => Action::GetError,

                //Action::PrintText
                (
                    ReadState::Init
                    | ReadState::TextDataReadStart
                    | ReadState::TextDataReadContinue
                    | ReadState::DataReadEnd
                    | ReadState::ErrorReadEnd
                    | ReadState::FileLoading,
                    ReadState::TextDataReadStart | ReadState::TextDataReadContinue,
                )
                | (
                    ReadState::TextDataReadStart | ReadState::TextDataReadContinue,
                    ReadState::BinaryDataReadStart | ReadState::BinaryDataReadContinue,
                ) => Action::PrintText,
                //Action::PrintText

                // Action::PrintHex
                (
                    ReadState::Init
                    | ReadState::BinaryDataReadStart
                    | ReadState::BinaryDataReadContinue
                    | ReadState::DataReadEnd
                    | ReadState::ErrorReadEnd
                    | ReadState::FileLoading,
                    ReadState::BinaryDataReadStart | ReadState::BinaryDataReadContinue,
                )
                | (
                    ReadState::BinaryDataReadStart | ReadState::BinaryDataReadContinue,
                    ReadState::TextDataReadStart | ReadState::TextDataReadContinue,
                ) => Action::PrintHex,
                // Action::PrintHex

                // Action::None
                (
                    ReadState::Init | ReadState::DataReadEnd | ReadState::ErrorReadEnd,
                    ReadState::Init,
                )
                | (
                    ReadState::DataReadEndPendingError,
                    ReadState::Init
                    | ReadState::TextDataReadStart
                    | ReadState::TextDataReadContinue
                    | ReadState::BinaryDataReadStart
                    | ReadState::BinaryDataReadContinue
                    | ReadState::DataReadEnd,
                )
                | (
                    ReadState::ErrorReadStart | ReadState::ErrorReadContinue,
                    ReadState::TextDataReadStart
                    | ReadState::TextDataReadContinue
                    | ReadState::BinaryDataReadStart
                    | ReadState::BinaryDataReadContinue,
                )
                | (ReadState::ErrorReadStart, ReadState::DataReadEnd)
                | (_, ReadState::FileLoading | ReadState::ErrorReadStart) => Action::None,
                // Action::None

                // Action::Prompt
                (
                    ReadState::TextDataReadStart
                    | ReadState::TextDataReadContinue
                    | ReadState::BinaryDataReadStart
                    | ReadState::BinaryDataReadContinue
                    | ReadState::ErrorReadStart
                    | ReadState::ErrorReadContinue
                    | ReadState::FileLoading,
                    ReadState::Init,
                )
                | (_, ReadState::DataReadEnd | ReadState::ErrorReadEnd) => Action::Prompt,
                //Action::Prompt
            },
        }
    }
}

impl Drop for Repl {
    fn drop(&mut self) {
        let _ = self
            .inst
            .write_all(b"if (_KIC ~= nil and _KIC['cleanup'] ~= nil) then _KIC.cleanup() end\n");
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Action {
    Prompt,
    GetError,
    PrintText,
    PrintHex,
    PrintError,
    None,
}
