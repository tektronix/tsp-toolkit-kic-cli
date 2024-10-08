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
use regex::Regex;
use std::{
    fmt::Display,
    fs::{self, File},
    io::{self, Read, Write},
    path::PathBuf,
    sync::mpsc::{channel, SendError, Sender, TryRecvError},
    thread::JoinHandle,
    time::Duration,
};
use tracing::{debug, error, info, instrument, trace, warn};

use tsp_toolkit_kic_lib::instrument::Instrument;

use crate::{
    command::Request,
    error::{InstrumentReplError, Result},
    instrument::{ParsedResponse, ResponseParser},
    resources::{KIC_COMMON_TSP, TSP_LINK_NODES_TSP},
    state_machine::ReadState,
    TspError,
};

pub struct Repl {
    inst: Box<dyn Instrument>,
    command: Command,
    lang_cong_file_path: String,
}

fn accumulate_and_search(accumulator: &mut String, buf: &[u8], needle: &str) -> bool {
    let first_null = buf.iter().position(|&x| x == b'\0').unwrap_or(buf.len());
    let buf = &buf[..first_null];
    if !buf.is_empty() {
        accumulator.push_str(&String::from_utf8_lossy(buf));
    }
    if accumulator.contains(needle) {
        info!("Successfully cleared instrument output queue");
        true
    } else {
        false
    }
}
/// Clear the output queue of the given TSP-enabled instrument.
///
/// # Errors
/// Errors in this function can range from [`std::io::Error`]s to being unable to
/// clear the output queue in the requested number of attempts.
#[instrument(skip(inst))]
pub fn clear_output_queue(
    inst: &mut Box<dyn Instrument>,
    max_attempts: usize,
    delay_between_attempts: Duration,
) -> Result<()> {
    let timestamp = Utc::now().to_string();

    info!("Clearing instrument output queue");
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
        if accumulate_and_search(&mut accumulate, &buf, &timestamp) {
            return Ok(());
        }
    }
    error!("Unable to clear instrument output queue");
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
            lang_cong_file_path: String::new(),
        }
    }

    fn clear_output_queue(
        &mut self,
        max_attempts: usize,
        delay_between_attempts: Duration,
    ) -> Result<()> {
        clear_output_queue(&mut self.inst, max_attempts, delay_between_attempts)
    }

    #[instrument(skip(self))]
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
            debug!("Handling data");
            let parser = ResponseParser::new(data);
            let mut get_error = false;
            for response in parser {
                *prev_state = *state;
                *state = Some(prev_state.unwrap_or_default().next_state(&response)?);

                match Self::state_action(*prev_state, *state) {
                    Action::Prompt => {
                        trace!("Set prompt = true");
                        prompt = true;
                    }
                    Action::GetError => {
                        trace!("set get_errors = true");
                        get_error = true;
                    }
                    Action::PrintText => {
                        trace!("Print data");
                        Self::print_data(*state, response)?;
                    }
                    Action::PrintError => {
                        trace!("Print error");
                        Self::print_data(*state, response)?;
                    }
                    Action::GetNodeDetails => {
                        trace!("Update node configuration file");
                        Self::update_node_config_json(&self.lang_cong_file_path, &response);
                    }

                    Action::None => {
                        trace!("No action required based on data");
                    }
                }
            }
            if get_error {
                let errors = self.get_errors()?;
                for e in errors {
                    error!("TSP error: {e}");
                    Self::print_data(*state, ParsedResponse::TspError(e.to_string()))?;
                }
                prompt = true;
                *state = Some(ReadState::DataReadEnd);
            }
            debug!("Data handling complete");
        }
        Ok(prompt)
    }

    /// Start the Repl
    ///
    /// # Errors
    /// There are many errors that can be returned from this function, they include but
    /// aren't limited to any errors possible from [`std::io::Read`] or [`std::io::Write`]
    #[allow(clippy::too_many_lines, clippy::cognitive_complexity)] //This is just going to be a long function
    #[instrument(skip(self))]
    pub fn start(&mut self) -> Result<()> {
        info!("Starting REPL");
        let mut prev_state: Option<ReadState> = None;
        let mut state: Option<ReadState> = None;
        self.inst.set_nonblocking(true)?;

        let (user_out, loop_in) = channel();

        let join = Self::init_user_input(user_out)?;

        self.clear_output_queue(5000, Duration::from_millis(1))?;

        debug!("Writing common script to instrument");
        self.inst.write_script(
            b"_kic_common",
            KIC_COMMON_TSP.to_string().as_bytes(),
            false,
            true,
        )?;
        debug!("Writing common script to instrument completed");

        self.inst.write_all(b"_KIC.prompts_enable(true)\n")?;
        let errors = self.get_errors()?;
        for e in errors {
            error!("TSP error: {e}");
            Self::print_data(None, ParsedResponse::TspError(e.to_string()))?;
        }

        let mut prompt = true;
        debug!("Starting user loop");
        'user_loop: loop {
            self.inst.set_nonblocking(true)?;
            std::thread::sleep(Duration::from_micros(1));
            let mut read_buf: Vec<u8> = vec![0; 1024];
            let read_size = self.inst.read(&mut read_buf)?;
            let read_buf: Vec<u8> = read_buf[..read_size].into();
            prompt = self.handle_data(&read_buf, prompt, &mut prev_state, &mut state)?;

            if prompt {
                prompt = false;
                Self::print_flush(&"\nTSP> ".blue())?;
            }
            match loop_in.try_recv() {
                Ok(msg) => {
                    debug!("User loop received request: {msg:?}");
                    match msg {
                        Request::Tsp(tsp) => {
                            self.inst.write_all(format!("{tsp}\n").as_bytes())?;
                            prev_state = None;
                        }
                        Request::GetError => {
                            let errors = self.get_errors()?;
                            for e in errors {
                                error!("TSP error: {e}");
                                Self::print_data(state, ParsedResponse::TspError(e.to_string()))?;
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

                            let re = Regex::new(r"[^A-Za-z\d_]");
                            match re {
                                Ok(re_res) => {
                                    let result = re_res.replace_all(name, "_");

                                    let script_name = format!("kic_{result}");

                                    self.inst.write_script(
                                        script_name.as_bytes(),
                                        contents.as_bytes(),
                                        false,
                                        true,
                                    )?;
                                }
                                Err(err_msg) => {
                                    unreachable!(
                                        "Issue with regex creation: {}",
                                        err_msg.to_string()
                                    );
                                }
                            }
                            prompt = false;
                        }
                        Request::TspLinkNodes { json_file } => {
                            self.set_lang_config_path(json_file.to_string_lossy().to_string());

                            self.inst.write_script(
                                b"TSP_LINK_NODES",
                                TSP_LINK_NODES_TSP.to_string().as_bytes(),
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
                            Self::print_flush(&"Flash update is in progress.\nClose the terminal and reconnect again once the instrument has restarted.".bright_yellow())?;
                            self.inst.flash_firmware(contents.as_ref(), slot)?;
                            // Flashing FW disables prompts before flashing but might
                            // lose runtime state, so we can't save the previous
                            // setting, so we just hardcode it to enabled here.
                            self.inst.write_all(b"localnode.prompts=1\n")?;
                        }
                        Request::Exit => {
                            info!("Exiting...");
                            break 'user_loop;
                        }
                        Request::Reset => {
                            self.inst.as_mut().reset()?;
                            prompt = true;
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
                        Request::InvalidInput(s) => {
                            prompt = true;
                            warn!("Invalid input: {s}");
                            Self::println_flush(&(s + "\n").red())?;
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
        let mut err: String = String::new();
        'error_loop: loop {
            std::thread::sleep(Duration::from_micros(1));
            let mut read_buf: Vec<u8> = vec![0; 1024];
            let _ = self.inst.read(&mut read_buf)?;
            if !(String::from_utf8_lossy(&read_buf).trim_end_matches(char::from(0))).is_empty() {
                err.push_str(String::from_utf8_lossy(&read_buf).trim_end_matches(char::from(0)));
                if err.contains(">DONE") {
                    break 'error_loop;
                }
            }
        }

        let parser = ResponseParser::new(err.as_bytes());
        for response in parser {
            if let ParsedResponse::TspError(e) = response {
                let x: TspError = serde_json::from_str(e.trim())?;
                errors.push(x);
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

    fn print_data(_state: Option<ReadState>, resp: ParsedResponse) -> Result<()> {
        match resp {
            ParsedResponse::TspError(e) => Self::print_flush(&(e + "\n").red()),
            ParsedResponse::Data(d) => Self::print_flush(&String::from_utf8_lossy(&d).to_string()),
            ParsedResponse::Prompt
            | ParsedResponse::PromptWithError
            | ParsedResponse::TspErrorStart
            | ParsedResponse::TspErrorEnd
            | ParsedResponse::ProgressIndicator
            | ParsedResponse::NodeStart
            | ParsedResponse::NodeEnd => Ok(()),
        }
    }

    fn update_node_config_json(file_path: &str, resp: &ParsedResponse) {
        if let ParsedResponse::Data(d) = &resp {
            if let Err(e) =
                Self::write_json_data(file_path.to_string(), String::from_utf8_lossy(d).as_ref())
            {
                eprintln!("Unable to write configuration: {e}");
            }
        }
    }

    fn write_json_data(file_path: String, input_line: &str) -> Result<()> {
        let path = PathBuf::from(file_path.clone());
        let Some(path) = path.parent() else {
            return Err(InstrumentReplError::IOError {
                source: std::io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "given path did not have a containing folder",
                ),
            });
        };

        if path.is_file() {
            return Err(InstrumentReplError::IOError {
                source: std::io::Error::new(
                    io::ErrorKind::NotADirectory,
                    "the parent folder is already a file",
                ),
            });
        }

        // If the path doesn't already exist, recursively create it.
        if !path.is_dir() {
            fs::create_dir_all(path)?;
        }

        if let Ok(mut file) = File::create(file_path) {
            // Convert the Lua string to JSON
            let json_value: serde_json::Value = serde_json::from_str(input_line.trim())?;

            // Convert the JSON value to a pretty-printed string
            let json_string = serde_json::to_string_pretty(&json_value)?;

            file.write_all(json_string.as_bytes())?;
        } else {
            return Err(InstrumentReplError::IOError {
                source: std::io::Error::new(io::ErrorKind::Other, "Failed to open file."),
            });
        }
        Ok(())
    }

    fn set_lang_config_path(&mut self, file_path: String) {
        self.lang_cong_file_path = file_path;
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
            Command::new(".script").about("Send a TSP script to the connected instrument")
                .help_template(SUBCMD_TEMPLATE)
                .disable_help_flag(true)
                .arg(
                    Arg::new("help").short('h').long("help").help("Print help").action(ArgAction::SetTrue)
                )
                .arg(
                    Arg::new("path")
                        .required_unless_present("help")
                        .help("Path to the TSP script file to be sent to the instrument")
                )
        )
        .subcommand(
            Command::new(".upgrade").about("Upgrade the firmware on the connected instrument")
                .help_template(SUBCMD_TEMPLATE)
                .disable_help_flag(true)
                .arg(
                    Arg::new("help").short('h').long("help").help("Print help").action(ArgAction::SetTrue)
                )
                .arg(
                    arg!(-s --slot <SLOT_NUM> "Collect information of a specific slot (if applicable) instead of the mainframe").value_parser(value_parser!(u16))
                )
                .arg(
                    Arg::new("path")
                        .required_unless_present("help")
                        .help("Path to the firmware file to be sent to the instrument")
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
                .about("Disconnect from instrument and close terminal")
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
        .subcommand(
            Command::new(".nodes").about("Fetch TSP-Link™ node details and update it to provided JSON file")
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
            Command::new(".reset")
                .help_template(SUBCMD_TEMPLATE)
                .about("Cancel any ongoing jobs and send *RST.")
                .disable_help_flag(true)
                .arg(
                    Arg::new("help").short('h').long("help").help("Print help").action(ArgAction::SetTrue)
                ),
        )
        .disable_help_flag(true)
    }

    #[allow(clippy::too_many_lines)] // This is a parser function, it is unavoidably long
    #[instrument]
    fn parse_user_commands(input: &str) -> Result<Request> {
        debug!("Parsing user input");
        if input.trim().is_empty() {
            return Ok(Request::None);
        }
        let path = PathBuf::from(input.trim());
        if path.is_file() {
            trace!("Detected file path: {path:?}");
            return Ok(Request::Script { file: path });
        }

        if !Self::starts_with_command(input) {
            return Ok(Request::Tsp(input.trim().to_string()));
        }

        let Some(cmd) = shlex::split(input.trim()) else {
            return Ok(Request::InvalidInput(format!(
                "Invalid command {}",
                input.trim()
            )));
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
                    let file = PathBuf::from(file);
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
            Some((".reset", flags)) => match flags.get_one::<bool>("help") {
                Some(help) if *help => Request::Help {
                    sub_cmd: Some(".reset".to_string()),
                },
                _ => Request::Reset,
            },
            Some((".nodes", flags)) => match flags.get_one::<bool>("help") {
                Some(help) if *help => Request::Help {
                    sub_cmd: Some(".nodes".to_string()),
                },
                _ => {
                    let Some(file) = flags.get_one::<String>("path") else {
                        return Err(InstrumentReplError::CommandError {
                            details: "expected file path, but none were provided".to_string(),
                        });
                    };
                    let json_file = PathBuf::from(file.clone());

                    Request::TspLinkNodes { json_file }
                }
            },
            Some((".upgrade", flags)) => match flags.get_one::<bool>("help") {
                Some(help) if *help => Request::Help {
                    sub_cmd: Some(".upgrade".to_string()),
                },
                _ => {
                    let Some(file) = flags.get_one::<String>("path") else {
                        return Err(InstrumentReplError::CommandError {
                            details: "expected file path, but none were provided".to_string(),
                        });
                    };
                    let file = file.clone();
                    let file = file.parse::<PathBuf>().unwrap(); //PathBuf::parse is infallible so unwrapping is OK here.

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

    /// Return `true` if input belong to cli subcommands
    fn starts_with_command(input: &str) -> bool {
        // Split the input string into words
        let words_in_input: Vec<&str> = input.split_whitespace().collect();

        // Check if there is at least one word in the input
        if let Some(first_word) = words_in_input.first() {
            return Self::cli()
                .get_subcommands()
                .any(|e| e.get_name() == *first_word);
        }

        false
    }
    /// Start a thread that blocks on user input lines, converts them to the proper request
    /// and `send()`s them on the `out` channel.
    ///
    /// # Return
    /// This function returns a join handle to the created user-input thread.
    ///
    /// # Errors
    /// This function can error if the thread couldn't be created.
    #[instrument]
    fn init_user_input(out: Sender<Request>) -> Result<JoinHandle<Result<()>>> {
        let jh = std::thread::Builder::new()
            .name("user_input".to_string())
            .spawn(
                #[allow(clippy::cognitive_complexity)]
                move || {
                    info!("Starting user input loop");
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
                    info!("Closing user input loop");
                    Ok(())
                },
            )?;
        Ok(jh)
    }
    #[allow(clippy::too_many_lines)]
    const fn state_action(prev_state: Option<ReadState>, state: Option<ReadState>) -> Action {
        match (prev_state, state) {
            (None, Some(state)) => match state {
                ReadState::Init | ReadState::DataReadEnd | ReadState::ErrorReadEnd => {
                    Action::Prompt
                }

                ReadState::DataReadEndPendingError => Action::GetError,

                ReadState::TextDataReadStart | ReadState::TextDataReadContinue => Action::PrintText,

                ReadState::ErrorReadContinue => Action::PrintError,
                ReadState::NodeDataReadStart | ReadState::NodeDataReadContinue => {
                    Action::GetNodeDetails
                }
                ReadState::NodeDataReadEnd => Action::Prompt,

                ReadState::ErrorReadStart | ReadState::FileLoading => Action::None,
            },

            (None | Some(_), None) => Action::None,

            (Some(prev_state), Some(state)) => match (prev_state, state) {
                (_, ReadState::ErrorReadContinue) => Action::PrintError,

                (_, ReadState::DataReadEndPendingError) => Action::GetError,

                //Action::GetNodeDetails
                (
                    ReadState::Init
                    | ReadState::TextDataReadStart
                    | ReadState::TextDataReadContinue
                    | ReadState::DataReadEnd
                    | ReadState::ErrorReadEnd
                    | ReadState::FileLoading
                    | ReadState::NodeDataReadStart,
                    ReadState::NodeDataReadContinue,
                ) => Action::GetNodeDetails,
                //Action::GetNodeDetails

                //Action::PrintText
                (
                    ReadState::Init
                    | ReadState::TextDataReadStart
                    | ReadState::TextDataReadContinue
                    | ReadState::DataReadEnd
                    | ReadState::ErrorReadEnd
                    | ReadState::FileLoading
                    | ReadState::NodeDataReadStart
                    | ReadState::NodeDataReadContinue
                    | ReadState::NodeDataReadEnd,
                    ReadState::TextDataReadStart | ReadState::TextDataReadContinue,
                ) => Action::PrintText,
                //Action::PrintText

                // Action::Prompt
                (
                    ReadState::TextDataReadStart
                    | ReadState::TextDataReadContinue
                    | ReadState::ErrorReadStart
                    | ReadState::ErrorReadContinue
                    | ReadState::FileLoading,
                    ReadState::Init,
                )
                | (
                    _,
                    ReadState::DataReadEnd | ReadState::ErrorReadEnd | ReadState::NodeDataReadEnd,
                ) => Action::Prompt,
                //Action::Prompt
                (
                    ReadState::Init | ReadState::DataReadEnd | ReadState::ErrorReadEnd,
                    ReadState::Init,
                )
                | (
                    ReadState::DataReadEndPendingError,
                    ReadState::Init
                    | ReadState::TextDataReadStart
                    | ReadState::TextDataReadContinue,
                )
                | (
                    ReadState::ErrorReadStart | ReadState::ErrorReadContinue,
                    ReadState::TextDataReadStart | ReadState::TextDataReadContinue,
                )
                | (_, ReadState::FileLoading | ReadState::ErrorReadStart | _) => Action::None,
            },
        }
    }
}

impl Drop for Repl {
    #[instrument(skip(self))]
    fn drop(&mut self) {
        trace!("Calling Repl::drop()");
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
    PrintError,
    GetNodeDetails,
    None,
}
