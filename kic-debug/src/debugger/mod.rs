use clap::{arg, value_parser, Command};
use colored::Colorize;
use kic_lib::instrument::{clear_output_queue, Instrument};
use serde::{Deserialize, Serialize};
use std::{
    fmt::Display,
    fs,
    io::{Error, Write},
    path::{Path, PathBuf},
    sync::mpsc::{channel, SendError, Sender, TryRecvError},
    thread::{self, JoinHandle},
    time::Duration,
};
pub mod breakpoint;
pub mod variable;
pub mod watchpoint;
use self::{breakpoint::Breakpoint, variable::VariableInfo, watchpoint::WatchpointInfo};
pub use crate::resources::{KIDEBUGGER_TSP, TSPDBG_TSP};
use crate::{
    command::Request,
    error::{DebugError, Result},
};
use regex::Regex;

#[derive(Serialize, Deserialize, Debug)]
pub struct DebugInfo {
    #[serde(rename = "FileName")]
    pub file_name: String,
    #[serde(rename = "BreakPoints")]
    pub break_points: Vec<Breakpoint>,
}
pub struct Debugger {
    instrument: Box<dyn Instrument>, // reference of the instrument
    debuggee_file_name: Option<String>,
    debuggee_file_path: Option<PathBuf>,
    breakpoints: Vec<Breakpoint>,
}

impl Debugger {
    /// Create a new debugger instance
    /// * `inst` - A mutable reference of the instrument
    #[must_use]
    pub fn new(inst: Box<dyn Instrument>) -> Self {
        Self {
            instrument: inst,
            debuggee_file_name: None,
            debuggee_file_path: None,
            breakpoints: Default::default(),
        }
    }

    // Funtion to handle all the special characters in the tsp script
    // * `script_name` - A String holds file name
    fn format_scriptname(mut script_name: String) -> String {
        let format = Regex::new(r"[\p{P}\p{S}]+").unwrap();
        script_name = format.replace_all(&script_name, "_").to_string(); //replace all special characters not supported by lua
        script_name = script_name
            .trim_start_matches('_')
            .to_string() //file_name should not start with underscore
            .trim_end_matches('.')
            .to_string() // file_name should not end with
            .replace(' ', "_"); //should not have spaces
        script_name
    }

    fn print_flush<D: Display>(string: &D) -> Result<()> {
        print!("{string}");
        let pr = std::io::stdout().flush();
        match pr {
            Ok(_) => {}
            Err(e) => {
                eprintln!("Error: {e:?}");
            }
        }
        Ok(())
    }

    fn println_flush<D: Display>(string: &D) {
        println!("{string}");
        let pr = std::io::stdout().flush();
        match pr {
            Ok(_) => {}
            Err(e) => {
                eprintln!("Error {e:?}");
            }
        }
    }

    /// Start debug session
    /// * `file_name` - A String holds file name with extension. x "callStacks.tsp"
    /// * `file_content` - A String holds file content
    /// * `breakpoints` - A Vector of Breakpoints. Ex [{\"LineNumber\":2,\"Enable\":true,\"Condition\":\"\"}]
    /// # Errors
    /// IO Errors from writing to the instrument may occur
    pub fn start_debugger(
        &mut self,
        file_name: &str,
        file_content: &str,
        breakpoints: Vec<Breakpoint>,
    ) -> Result<()> {
        self.load_debugger_files()?;
        self.clear_debugger_file_sources()?;

        self.clear_breakpoints()?;

        for item in breakpoints {
            self.set_breakpoint(&item)?;
        }

        // to remove extension from file name
        let file_path = Path::new(&file_name);
        let file = file_path.file_stem();
        let Some(file_name_str) = file else {
            return Err(DebugError::Other("Invalid File Name".to_string()));
        };
        let file_name = Self::format_scriptname(file_name_str.to_string_lossy().to_string());
        let mut script_name = format!("kic_{file_name}");
        self.debuggee_file_name = Some(script_name.clone());
        script_name.truncate(31);
        // script_name.truncate(255);
        self.instrument.write_script(
            script_name.clone().as_bytes(),
            file_content.as_bytes(),
            false,
            false,
        )?;

        self.instrument.write_all(
            format!(
                "kiExecuteWithDebugger({script_name}.source,\"debug_{script_name}\",\"xml\")\n",
            )
            .as_bytes(),
        )?;
        Ok(())
    }

    /// Load debugger files to the instrument.
    fn load_debugger_files(&mut self) -> Result<()> {
        let tspdbg = TSPDBG_TSP.decrypt()?;
        self.instrument
            .write_script(b"tspdbg", tspdbg.to_string().as_bytes(), false, true)?;

        let ki_debugger = KIDEBUGGER_TSP.decrypt()?;
        self.instrument.write_script(
            b"kiDebugger",
            ki_debugger.to_string().as_bytes(),
            false,
            true,
        )?;
        Ok(())
    }

    fn clear_debugger_file_sources(&mut self) -> Result<()> {
        self.instrument
            .write_all(b"getmetatable(kiDebugger).Objects.source = nil\n")?;
        Ok(())
    }

    /// Send the `KiSetWatchpoint` command to the on-instrument debugger
    /// * Arguments
    ///   `watch_point` - A WatchpointInfo struct holds watchpoint information
    pub fn set_watchpoint(&mut self, watch_point: WatchpointInfo) -> Result<()> {
        let mut enable_val = 1;
        if !watch_point.enable {
            enable_val = 0;
        }
        // watch expressions need to be double-escaped because they will be executed as a string in Lua.
        let expression = watch_point.expression.replace('\"', "\\\"");
        self.instrument
            .write_all(format!("kiSetWatchpoint(\"{expression}\",{enable_val})\n").as_bytes())?;

        Ok(())
    }

    /// Set a breakpoint at the given line number
    /// * Arguments
    /// * `break_point` - A Breakpoint struct holds breakpoint data
    /// # Errors
    /// IO Errors from writing to the instrument may occur
    pub fn set_breakpoint(&mut self, break_point: &Breakpoint) -> Result<()> {
        let enable_val: u8 = break_point.enable.into();

        self.instrument.write_all(
            format!(
                "kiSetBreakpoint({0},{1},false)\n",
                break_point.line_number, enable_val
            )
            .as_bytes(),
        )?;
        self.breakpoints.push(break_point.clone());

        Ok(())
    }

    /// Convert &str variable_data to VariableInfo struct, parse it and
    /// call the appropriate kiDebugger variable type setter
    /// * Arguments
    /// * `var_info` - A VariableInfo struct holds variable information
    ///
    /// * Example {"StackLevel":2,"ArgumentList":["x", "y", "z"],"Value":"7","Scope":"locals"}
    /// * `var_info` : `{"StackLevel":0,"ArgumentList":["newTab", "tab", "x"],"Value":"7","Scope":"upvalues"}`
    /// * possible values of Scope are "locals", "upvalues", "globals"
    pub fn set_variable(&mut self, var_info: VariableInfo) -> Result<()> {
        let mut el: String;
        let mut index = 0;
        let mut arg_list: String = "".to_string();
        while index < var_info.argument_list.len() {
            el = var_info.argument_list[index].to_owned();
            arg_list = format!("{arg_list},{el}");
            index += 1;
        }
        arg_list = arg_list.trim_start_matches([',', ' ']).to_string();

        let level = var_info.stack_level.to_string();
        let mut value = var_info.value.to_string();
        value = value.replace('\"', "\\\"");
        if var_info.scope_type == "locals" {
            self.instrument.write_all(
                format!("kiSetLocalVariable({level},\"{value}\",{arg_list})\n").as_bytes(),
            )?;
        }
        if var_info.scope_type == "upvalues" {
            self.instrument.write_all(
                format!("kiSetUpVariable({level},\"{value}\",{arg_list})\n").as_bytes(),
            )?;
        }
        if var_info.scope_type == "globals" {
            self.instrument.write_all(
                format!("kiSetGlobalVariable({level},\"{value}\", {arg_list})\n").as_bytes(),
            )?;
        }
        Ok(())
    }

    /// Send the `kiClearBreakpoints()` command to the instrument
    /// which will remove all breakpoints
    /// # Errors
    /// IO Errors from writing to the instrument may occur
    pub fn clear_breakpoints(&mut self) -> Result<()> {
        self.breakpoints.clear();
        self.instrument.write_all(b"kiClearBreakpoints()\n")?;

        Ok(())
    }

    /// Sends `kiRun` command to the instrument
    /// which will continue execution of the debuggee script
    /// from the current line
    /// # Errors
    /// IO Errors from writing to the instrument may occur
    pub fn continue_debugging(&mut self) -> Result<()> {
        self.instrument.write_all(b"kiRun\n")?;

        Ok(())
    }

    /// Send `kiStepOver` command to the instrument
    /// which will step over on the current line
    /// # Errors
    /// IO Errors from writing to the instrument may occur
    pub fn stepover_debugging(&mut self) -> Result<()> {
        self.instrument.write_all(b"kiStepOver\n")?;

        Ok(())
    }

    /// Send the `kiStepIn` command to the instrument
    /// which will step into any function calls on the current line
    /// # Errors
    /// IO Errors from writing to the instrument may occur
    pub fn stepin_debugging(&mut self) -> Result<()> {
        self.instrument.write_all(b"kiStepIn\n")?;

        Ok(())
    }

    /// Send the `kiStepOut` command to the instrument
    /// which will step out of current execution context to one level up in the call stack
    /// # Errors
    /// IO Errors from writing to the instrument may occur
    pub fn stepout_debugging(&mut self) -> Result<()> {
        self.instrument.write_all(b"kiStepOut\n")?;

        Ok(())
    }

    /// Terminate tsp debugger and returns Instrument
    fn exit_debugger(&mut self) -> Result<()> {
        // If the session in progress, abort will terminate it,
        // if session already ended, abort will not do anything
        self.instrument.write_all(b"abort\n")?;
        self.instrument.write_all(b"kiDebugger = nil\n")?;

        if let Some(debug_file_name) = self.debuggee_file_name.to_owned() {
            self.instrument
                .write_all(format!("{debug_file_name} = nil\n").as_bytes())?;
            self.instrument
                .write_all(format!("script.delete(\"{debug_file_name}\")\n").as_bytes())?;

            self.debuggee_file_name = None;
        }

        // kiDebugger global functions.
        let ki_debugger_global_functions = &[
            "kiClearBreakpoints",
            "kiSetBreakpoint",
            "kiSetWatchpoint",
            "kiClearWatchpoints",
            "kiClearWatchpoint",
            "kiExecuteWithDebugger",
            "kiSetUpVariable",
            "kiSetLocalVariable",
            "kiSetGlobalVariable",
        ];

        for func in ki_debugger_global_functions {
            self.instrument
                .write_all(format!("{func} = nil\n").as_bytes())?;
        }

        self.instrument
            .write_all(b"script.delete(\"kiDebugger\")\n")?;

        Ok(())
    }

    /// Start the Repl
    ///
    /// # Errors
    /// There are many errors that can be returned from this function, they include but
    /// aren't limited to any errors possible from [`std::io::Read`] or [`std::io::Write`]
    #[allow(clippy::too_many_lines)] //This is just going to be a long function
    pub fn start(&mut self) -> Result<()> {
        // let mut prev_state: Option<ReadState> = None;
        // let mut state: Option<ReadState> = None;
        self.instrument.set_nonblocking(true)?;

        let (user_out, loop_in) = channel();

        let join = Self::init_user_input(user_out)?;

        self.instrument.write_all(b"localnode.prompts = 0\n")?;

        Self::print_flush(&"\nTSP> ".blue())?;
        'user_loop: loop {
            self.instrument.set_nonblocking(true)?;
            thread::sleep(Duration::from_millis(1));
            let mut read_buf: Vec<u8> = vec![0; 1024];
            let read_size = match self.instrument.read(&mut read_buf) {
                Ok(read_size) => read_size,
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => 0,
                Err(e) => return Err(e.into()),
            };
            let read_buf: Vec<u8> = read_buf[..read_size].into();
            if !String::from_utf8_lossy(&read_buf)
                .trim_end_matches(char::from(0))
                .is_empty()
            {
                Self::print_flush(&String::from_utf8_lossy(&read_buf))?;
            }

            match loop_in.try_recv() {
                Ok(req) => match req {
                    Request::BreakPoint { breakpoint_info } => {
                        self.set_breakpoint(&breakpoint_info)?;
                    }
                    Request::Watchpoint { watchpoint_info } => {
                        self.set_watchpoint(watchpoint_info)?;
                    }
                    Request::Variable { vairable_info } => {
                        self.set_variable(vairable_info)?;
                    }
                    Request::StartDebugger {
                        file_path,
                        break_points,
                    } => {
                        let file_path = Path::new(
                            (file_path)
                                .trim()
                                .trim_end_matches(['\'', '"'])
                                .trim_start_matches(['\'', '"']),
                        );

                        if let Ok(_file) = fs::File::open(file_path) {
                            self.debuggee_file_path = Some(file_path.to_path_buf());
                            let file_contents = fs::read_to_string(file_path)?;
                            let script_name = file_path
                                .file_stem()
                                .unwrap()
                                .to_os_string()
                                .into_string()
                                .unwrap()
                                .replace(' ', "_");
                            self.start_debugger(&script_name, &file_contents, break_points)?;
                        } else {
                            return Err(DebugError::IOError {
                                source: Error::new(
                                    std::io::ErrorKind::NotFound,
                                    "Error: Could not locate file".to_string(),
                                ),
                            });
                        }
                    }
                    Request::Run => {
                        self.continue_debugging()?;
                    }
                    Request::StepOver => {
                        self.stepover_debugging()?;
                    }
                    Request::ClearBreakPoints => {
                        self.clear_breakpoints()?;
                    }
                    Request::StepIn => {
                        self.stepin_debugging()?;
                    }
                    Request::StepOut => {
                        self.stepout_debugging()?;
                    }
                    Request::Exit => {
                        clear_output_queue(&mut *self.instrument, 5, Duration::from_millis(100))?;
                        break 'user_loop;
                    }
                    Request::Restart => {
                        eprintln!("RESTART RECV'D");
                        self.instrument.write_all(b"abort\n")?;
                        self.instrument.write_all(b"*RST\n")?;
                        std::thread::sleep(Duration::from_millis(100));
                        let orig_file_name = self
                            .debuggee_file_name
                            .clone()
                            .expect("should have file name in Debugger App");
                        let orig_file_path = self
                            .debuggee_file_path
                            .clone()
                            .expect("should have file path in Debugger App");
                        let orig_breakpoints = self.breakpoints.clone();
                        if let Ok(_file) = fs::File::open(&orig_file_path) {
                            let file_contents = fs::read_to_string(&orig_file_path)?;
                            self.start_debugger(&orig_file_name, &file_contents, orig_breakpoints)?;
                        }
                    }
                    Request::GetError(error) => {
                        Self::println_flush(&format!("Error: {error:?}"));
                    }

                    Request::Tsp(tsp) => {
                        self.instrument.write_all(format!("{tsp}\n").as_bytes())?;
                    }
                    _ => {}
                },
                Err(TryRecvError::Disconnected) => break 'user_loop,
                Err(TryRecvError::Empty) => {}
            }
        }
        drop(loop_in);
        let _ = join.join();
        Ok(())
    }

    /// Command Line Interface
    #[allow(clippy::cognitive_complexity)]
    fn cli() -> Command {
        Command::new("kic-debug")
            .multicall(true)
            .disable_help_subcommand(true)
            .allow_external_subcommands(true)
            .subcommand_required(false)
            .subcommand(
                Command::new(".debug")
                    .about("initialize debugger")
                    .allow_external_subcommands(true)
                    .subcommand_required(false)
                    .disable_help_flag(true)
                    .subcommand(
                        Command::new("run")
                            .about("Continue to next breakpont")
                            .disable_help_flag(true),
                    )
                    .subcommand(
                        Command::new("stepOver")
                            .about("Step-over")
                            .disable_help_flag(true),
                    )
                    .subcommand(
                        Command::new("stepIn")
                            .about("Step-in")
                            .disable_help_flag(true),
                    )
                    .subcommand(
                        Command::new("stepOut")
                            .about("Step-out")
                            .disable_help_flag(true),
                    )
                    .subcommand(
                        Command::new("exit")
                            .about("exit debugger and disconnect instrument")
                            .disable_help_flag(true),
                    )
                    .subcommand(
                        Command::new("clearBreakpoints")
                            .about("clear all breakpoints")
                            .disable_help_flag(true),
                    )
                    .subcommand(
                        Command::new("setBreakpoint")
                            .about("set breakpoint")
                            .disable_help_flag(true)
                            .arg(arg!([Breakpoint]).value_parser(value_parser!(String)))
                            .disable_help_flag(true),
                    )
                    .subcommand(
                        Command::new("setWatchpoint")
                            .about("set watchpoint")
                            .disable_help_flag(true)
                            .arg(arg!([Watchpoint]).value_parser(value_parser!(String))),
                    )
                    .subcommand(
                        Command::new("setVariable")
                            .about("set variable")
                            .disable_help_flag(true)
                            .arg(arg!([Variable]).value_parser(value_parser!(String)))
                            .disable_help_flag(true),
                    )
                    .subcommand(
                        Command::new("restart")
                            .about("restart the debugger")
                            .disable_help_flag(true),
                    ),
            )
            .disable_help_flag(true)
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

    /// Parse user command
    /// * `input` - A &str holds user input from the terminal
    /// * Return - A Result of Request
    ///
    /// # Errors
    /// Returns a [`DebugError::CommandError`] if [`shlex`] could not properly split the input
    #[allow(clippy::too_many_lines)]
    pub fn parse_user_commands(input: &str) -> Result<Request> {
        if input.trim().is_empty() {
            return Ok(Request::None);
        }
        let Some(cmd) = shlex::split(input.trim()) else {
            return Err(DebugError::CommandError {
                details: "Error: Invalid quoting".to_string(),
            });
        };

        let cli = Self::cli();

        let matches = cli.try_get_matches_from(cmd);

        if let Err(e) = matches {
            return Ok(Request::Usage(e.to_string()));
        };

        let arg_matches = matches;
        let matches = match arg_matches {
            Ok(m) => m,
            Err(e) => {
                return Ok(Request::Usage(e.to_string()));
            }
        };

        let matches = matches.subcommand();

        match matches {
            Some((".debug", flag)) => match flag.subcommand() {
                Some(("run", _)) => Ok(Request::Run),
                Some(("stepOver", _)) => Ok(Request::StepOver),
                Some(("stepIn", _)) => Ok(Request::StepIn),
                Some(("stepOut", _)) => Ok(Request::StepOut),
                Some(("exit", _)) => Ok(Request::Exit),
                Some(("clearBreakpoints", _)) => Ok(Request::ClearBreakPoints),
                Some(("restart", _)) => Ok(Request::Restart),
                Some(("setBreakpoint", flag)) => {
                    let breakpoint_info = flag.get_one::<String>("Breakpoint"); //matches.get_one::<PathBuf>("config")
                    match breakpoint_info {
                        Some(bpoint) => {
                            let bp: std::result::Result<Breakpoint, serde_json::Error> =
                                serde_json::from_str(bpoint.as_str()); // need to do it
                            match bp {
                                Ok(bp) => Ok(Request::BreakPoint {
                                    breakpoint_info: bp,
                                }),
                                Err(e) => {
                                    Self::println_flush(&format!("serde error: {e:?}"));
                                    Ok(Request::GetError(e.to_string()))
                                }
                            }
                        }
                        _ => Ok(Request::GetError(
                            "Error: Could not find setBreakpoint command argrument".to_string(),
                        )),
                    }
                }
                Some(("setWatchpoint", flag)) => {
                    let watchpoint_info = flag.get_one::<String>("Watchpoint");
                    match watchpoint_info {
                        Some(wpoint) => {
                            let wp: std::result::Result<WatchpointInfo, serde_json::Error> =
                                serde_json::from_str(wpoint.as_str()); // need to do it
                            match wp {
                                Ok(wp) => Ok(Request::Watchpoint {
                                    watchpoint_info: wp,
                                }),
                                Err(e) => Ok(Request::GetError(e.to_string())),
                            }
                        }
                        _ => Ok(Request::GetError(
                            "Error: Could not find setWatchpoint command argrument".to_string(),
                        )),
                    }
                }
                Some(("setVariable", flag)) => {
                    let variable_info = flag.get_one::<String>("Variable");
                    match variable_info {
                        Some(vpoint) => {
                            let vp: std::result::Result<VariableInfo, serde_json::Error> =
                                serde_json::from_str(vpoint.as_str()); // need to do it
                            match vp {
                                Ok(vp) => Ok(Request::Variable { vairable_info: vp }),
                                Err(e) => Ok(Request::GetError(e.to_string())),
                            }
                        }
                        _ => Ok(Request::GetError(
                            "Error: Could not find setVariable command argrument".to_string(),
                        )),
                    }
                }
                _ => {
                    match flag.subcommand() {
                        Some(sub) => {
                            let debug_info = sub.0;
                            let di: std::result::Result<DebugInfo, serde_json::Error> =
                                serde_json::from_str(debug_info); // need to do it.

                            match di {
                                Ok(di) => Ok(Request::StartDebugger {
                                    file_path: di.file_name,
                                    break_points: di.break_points,
                                }),
                                Err(e) => Ok(Request::GetError(e.to_string())),
                            }
                        }

                        _ => Ok(Request::GetError(
                            "Error: Could not find debug command argrument".to_string(),
                        )),
                    }
                }
            },
            _ => Ok(Request::Tsp(input.trim().to_string())),
        }
    }
}

impl Drop for Debugger {
    fn drop(&mut self) {
        // if we get an Err(...) back, just ignore it.
        // We can't do anything about it in the drop() anyway.
        let _ = self.exit_debugger();
    }
}

//#[cfg(test)]
//mod debugger_test {
//    use super::breakpoint::Breakpoint;
//    //use super::DebugInfo;
//    use crate::debugger::Debugger;
//    //pub use crate::resources::{KIDEBUGGER_TSP, TSPDBG_TSP};
//    use mockall::{mock, Sequence};
//    use std::io::{Read, Write};
//    use kic_lib::instrument::authenticate::Authentication;
//    use kic_lib::instrument::Info;
//    use kic_lib::interface;
//    use kic_lib::interface::NonBlock;
//    use kic_lib::model::ki2600;
//
//    // use kic_lib::device_interface::Interface::MockInterface;
//    #[test]
//    fn test_new() {
//        let mut interface = MockInterface::new();
//        let auth = MockAuthenticate::new();
//        let mut seq = Sequence::new();
//
//        interface
//            .expect_write()
//            .times(..)
//            .withf(|x| x == b"abort\n")
//            .returning(|x| Ok(x.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiDebugger = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiClearBreakpoints = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiSetBreakpoint = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiSetWatchpoint = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiClearWatchpoints = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiClearWatchpoint = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiExecuteWithDebugger = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiSetUpVariable = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiSetLocalVariable = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiSetGlobalVariable = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"script.delete(\"kiDebugger\")\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(..)
//            .withf(|buf: &[u8]| buf == b"password\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(..)
//            .withf(|buf: &[u8]| buf == b"abort\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        let instrument = ki2600::Instrument::new(
//            kic_lib::protocol::Protocol::Raw(Box::new(interface)),
//            Box::new(auth),
//        );
//        let debugger = Debugger::new(Box::new(instrument));
//        assert_eq!(debugger.debuggee_file_name, None);
//    }
//
//    #[test]
//    fn test_set_breakpoint() {
//        let mut interface = MockInterface::new();
//        let auth = MockAuthenticate::new();
//        let mut seq = Sequence::new();
//        interface
//            .expect_write()
//            .times(..)
//            .withf(|x| x == b"abort\n")
//            .returning(|x| Ok(x.len()));
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiSetBreakpoint(10,1,false)\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiDebugger = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiClearBreakpoints = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiSetBreakpoint = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiSetWatchpoint = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiClearWatchpoints = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiClearWatchpoint = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiExecuteWithDebugger = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiSetUpVariable = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiSetLocalVariable = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiSetGlobalVariable = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"script.delete(\"kiDebugger\")\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(..)
//            .withf(|buf: &[u8]| buf == b"password\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(..)
//            .withf(|buf: &[u8]| buf == b"abort\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        let instrument = ki2600::Instrument::new(
//            kic_lib::protocol::Protocol::Raw(Box::new(interface)),
//            Box::new(auth),
//        );
//        let mut debugger = Debugger::new(Box::new(instrument));
//        let breakpoint = Breakpoint {
//            line_number: 10,
//            enable: true,
//            condition: String::new(),
//        };
//        debugger.set_breakpoint(&breakpoint).unwrap();
//    }
//
//    #[test]
//    fn test_clear_breakpoints() {
//        let mut interface = MockInterface::new();
//        let auth = MockAuthenticate::new();
//        let mut seq = Sequence::new();
//        interface
//            .expect_write()
//            .times(..)
//            .withf(|x| x == b"abort\n")
//            .returning(|x| Ok(x.len()));
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiClearBreakpoints()\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiDebugger = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiClearBreakpoints = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiSetBreakpoint = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiSetWatchpoint = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiClearWatchpoints = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiClearWatchpoint = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiExecuteWithDebugger = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiSetUpVariable = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiSetLocalVariable = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiSetGlobalVariable = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"script.delete(\"kiDebugger\")\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(..)
//            .withf(|buf: &[u8]| buf == b"password\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(..)
//            .withf(|buf: &[u8]| buf == b"abort\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        let instrument = ki2600::Instrument::new(
//            kic_lib::protocol::Protocol::Raw(Box::new(interface)),
//            Box::new(auth),
//        );
//        let mut debugger = Debugger::new(Box::new(instrument));
//
//        debugger.clear_breakpoints().unwrap();
//    }
//
//    // #[test]
//    //    fn test_start_debugger() {
//    //        let mut interface = MockInterface::new();
//    //        let auth = MockAuthenticate::new();
//    //        let mut seq = Sequence::new();
//    //        let resource = KIDEBUGGER_TSP.decrypt().unwrap();
//    //        // kiDebugger=nil
//    //        // expect_flush
//    //        // loadscript kiDebugger
//    //        // expect_flush
//    //        // resource.to_string().lines().count()
//    //        // endscript
//    //        // expect_flush
//    //        // kiDebugger.run()
//    //        // expect_flush
//    //        interface
//    //            .expect_write()
//    //            .times(..)
//    //            .withf(|x| x == b"abort\n")
//    //            .returning(|x| Ok(x.len()));
//    //        interface.expect_flush().times(..).returning(|| Ok(()));
//    //        interface
//    //            .expect_write()
//    //            .times(resource.to_string().lines().count() + 6)
//    //            .in_sequence(&mut seq)
//    //            .returning(|buf: &[u8]| Ok(buf.len()));
//    //
//    //        let resource = TSPDBG_TSP.decrypt().unwrap();
//    //        // tspdbg=nil
//    //        // loadscript tspdbg
//    //        // resource.to_string().lines().count()
//    //        // endscript
//    //        // tspdbg.run()
//    //        interface
//    //            .expect_write()
//    //            .times(resource.to_string().lines().count() + 6)
//    //            .in_sequence(&mut seq)
//    //            .returning(|buf: &[u8]| Ok(buf.len()));
//    //
//    //        interface
//    //            .expect_write()
//    //            .times(1)
//    //            .in_sequence(&mut seq)
//    //            .withf(|buf: &[u8]| buf == b"kiClearBreakpoints()\n")
//    //            .returning(|buf: &[u8]| Ok(buf.len()));
//    //
//    //        interface
//    //            .expect_write()
//    //            .times(1)
//    //            .in_sequence(&mut seq)
//    //            .withf(|buf: &[u8]| buf == b"kiSetBreakpoint(34,1,false)\n")
//    //            .returning(|buf: &[u8]| Ok(buf.len()));
//    //
//    //        interface
//    //            .expect_write()
//    //            .times(1)
//    //            .in_sequence(&mut seq)
//    //            .withf(|buf: &[u8]| buf == b"kiSetBreakpoint(17,1,false)\n")
//    //            .returning(|buf: &[u8]| Ok(buf.len()));
//    //
//    //        interface
//    //            .expect_write()
//    //            .times(1)
//    //            .in_sequence(&mut seq)
//    //            .withf(|buf: &[u8]| buf == b"kic_callStacks=nil\n")
//    //            .returning(|buf: &[u8]| Ok(buf.len()));
//    //
//    //        interface
//    //            .expect_write()
//    //            .times(1)
//    //            .in_sequence(&mut seq)
//    //            .withf(|buf: &[u8]| buf == b"loadscript kic_callStacks\n")
//    //            .returning(|buf: &[u8]| Ok(buf.len()));
//    //
//    //        interface
//    //            .expect_write()
//    //            .times(1)
//    //            .in_sequence(&mut seq)
//    //            .withf(|buf: &[u8]| buf == b"line1\n")
//    //            .returning(|buf: &[u8]| Ok(buf.len()));
//    //
//    //        interface
//    //            .expect_write()
//    //            .times(1)
//    //            .in_sequence(&mut seq)
//    //            .withf(|buf: &[u8]| buf == b"line2\n")
//    //            .returning(|buf: &[u8]| Ok(buf.len()));
//    //
//    //        interface
//    //            .expect_write()
//    //            .times(1)
//    //            .in_sequence(&mut seq)
//    //            .withf(|buf: &[u8]| buf == b"line3\n")
//    //            .returning(|buf: &[u8]| Ok(buf.len()));
//    //
//    //        interface
//    //            .expect_write()
//    //            .times(1)
//    //            .in_sequence(&mut seq)
//    //            .withf(|buf: &[u8]| buf == b"\nendscript\n")
//    //            .returning(|buf: &[u8]| Ok(buf.len()));
//    //
//    //        interface
//    //                    .expect_write()
//    //                    .times(1)
//    //                    .in_sequence(&mut seq)
//    //                    .withf(|buf: &[u8]|  buf == b"kiExecuteWithDebugger(kic_callStacks.source,\"debug_kic_callStacks\",\"xml\")\n")
//    //                    .returning(|buf: &[u8]| Ok(buf.len()));
//    //
//    //        // connect.expect_write().times(1).returning(|buf: &[u8]| Ok(buf.len()));
//    //        let instrument = ki2600::Instrument::new(
//    //            kic_lib::protocol::Protocol::Raw(Box::new(interface)),
//    //            Box::new(auth),
//    //        );
//    //        let mut debugger = Debugger::new(Box::new(instrument));
//    //        let file_content = "line1\nline2\nline3";
//    //        let input = "{\"FileName\":\"callStacks.tsp\",\"BreakPoints\":[{\"LineNumber\":34,\"Enable\":true,\"Condition\":\"\"},{\"LineNumber\":17,\"Enable\":true,\"Condition\":\"\"}]}";
//    //        let debug_data: DebugInfo = serde_json::from_str(input).unwrap();
//    //        debugger
//    //            .start_debugger(
//    //                &debug_data.file_name,
//    //                file_content,
//    //                debug_data.break_points,
//    //            )
//    //            .unwrap();
//    //    }
//    //
//    //    // #[test]
//    //    fn test_start_debugger_error() {
//    //        let mut interface = MockInterface::new();
//    //        let auth = MockAuthenticate::new();
//    //        let mut seq = Sequence::new();
//    //        let resource = KIDEBUGGER_TSP.decrypt().unwrap();
//    //        // kiDebugger=nil
//    //        // loadscript kiDebugger
//    //        // resource.to_string().lines().count()
//    //        // endscript
//    //        // kiDebugger.run()
//    //        interface.expect_flush().times(..).returning(|| Ok(()));
//    //        interface
//    //            .expect_write()
//    //            .times(..)
//    //            .withf(|x| x == b"abort\n")
//    //            .returning(|x| Ok(x.len()));
//    //        interface
//    //            .expect_write()
//    //            .times(resource.to_string().lines().count() + 4)
//    //            .in_sequence(&mut seq)
//    //            .returning(|buf: &[u8]| Ok(buf.len()));
//    //
//    //        let resource = TSPDBG_TSP.decrypt().unwrap();
//    //        // tspdbg=nil
//    //        // loadscript tspdbg
//    //        // resource.to_string().lines().count()
//    //        // endscript
//    //        // tspdbg.run()
//    //        interface
//    //            .expect_write()
//    //            .times(resource.to_string().lines().count() + 4)
//    //            .in_sequence(&mut seq)
//    //            .returning(|buf: &[u8]| Ok(buf.len()));
//    //
//    //        interface
//    //            .expect_write()
//    //            .times(1)
//    //            .in_sequence(&mut seq)
//    //            .withf(|buf: &[u8]| buf == b"kiClearBreakpoints()\n")
//    //            .returning(|buf: &[u8]| Ok(buf.len()));
//    //
//    //        interface
//    //            .expect_write()
//    //            .times(1)
//    //            .in_sequence(&mut seq)
//    //            .withf(|buf: &[u8]| buf == b"kiSetBreakpoint(34,1,false)\n")
//    //            .returning(|buf: &[u8]| Ok(buf.len()));
//    //
//    //        interface
//    //            .expect_write()
//    //            .times(1)
//    //            .in_sequence(&mut seq)
//    //            .withf(|buf: &[u8]| buf == b"kiSetBreakpoint(17,1,false)\n")
//    //            .returning(|buf: &[u8]| Ok(buf.len()));
//    //
//    //        interface
//    //            .expect_write()
//    //            .times(1)
//    //            .in_sequence(&mut seq)
//    //            .withf(|buf: &[u8]| buf == b"kic_callStacks=nil\n")
//    //            .returning(|buf: &[u8]| Ok(buf.len()));
//    //
//    //        interface
//    //            .expect_write()
//    //            .times(1)
//    //            .in_sequence(&mut seq)
//    //            .withf(|buf: &[u8]| buf == b"loadscript kic_callStacks\n")
//    //            .returning(|buf: &[u8]| Ok(buf.len()));
//    //
//    //        interface
//    //            .expect_write()
//    //            .times(1)
//    //            .in_sequence(&mut seq)
//    //            .withf(|buf: &[u8]| buf == b"line1\n")
//    //            .returning(|buf: &[u8]| Ok(buf.len()));
//    //
//    //        interface
//    //            .expect_write()
//    //            .times(1)
//    //            .in_sequence(&mut seq)
//    //            .withf(|buf: &[u8]| buf == b"line2\n")
//    //            .returning(|buf: &[u8]| Ok(buf.len()));
//    //
//    //        interface
//    //            .expect_write()
//    //            .times(1)
//    //            .in_sequence(&mut seq)
//    //            .withf(|buf: &[u8]| buf == b"line3\n")
//    //            .returning(|buf: &[u8]| Ok(buf.len()));
//    //
//    //        interface
//    //            .expect_write()
//    //            .times(1)
//    //            .in_sequence(&mut seq)
//    //            .withf(|buf: &[u8]| buf == b"\nendscript\n")
//    //            .returning(|buf: &[u8]| Ok(buf.len()));
//    //
//    //        interface
//    //            .expect_write()
//    //            .times(1)
//    //            .in_sequence(&mut seq)
//    //            .withf(|buf: &[u8]|  buf == b"kiExecuteWithDebugger(kic_callStacks.source,\"debug_kic_callStacks\",\"xml\")\n")
//    //            .returning(|buf: &[u8]| Ok(buf.len()));
//    //
//    //        // connect.expect_write().times(1).returning(|buf: &[u8]| Ok(buf.len()));
//    //        let instrument = ki2600::Instrument::new(
//    //            kic_lib::protocol::Protocol::Raw(Box::new(interface)),
//    //            Box::new(auth),
//    //        );
//    //        let mut debugger = Debugger::new(Box::new(instrument));
//    //        let file_content = "line1\nline2\nline3";
//    //        let input = "{\"FileName\":\"callStacks.tsp\",\"BreakPoints\":[{\"LineNumber\":34,\"Enable\":true,\"Condition\":\"\"},{\"LineNumber\":17,\"Enable\":true,\"Condition\":\"\"}]}";
//    //        let debug_data: DebugInfo = serde_json::from_str(input).unwrap();
//    //        assert!(debugger
//    //            .start_debugger(
//    //                &debug_data.file_name,
//    //                file_content,
//    //                debug_data.break_points
//    //            )
//    //            .is_err());
//    //    }
//
//    #[test]
//    fn test_continue_debugging() {
//        let mut interface = MockInterface::new();
//        let auth = MockAuthenticate::new();
//        let mut seq = Sequence::new();
//        interface
//            .expect_write()
//            .times(..)
//            .withf(|x| x == b"abort\n")
//            .returning(|x| Ok(x.len()));
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiRun\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiDebugger = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiClearBreakpoints = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiSetBreakpoint = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiSetWatchpoint = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiClearWatchpoints = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiClearWatchpoint = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiExecuteWithDebugger = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiSetUpVariable = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiSetLocalVariable = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiSetGlobalVariable = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"script.delete(\"kiDebugger\")\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(..)
//            .withf(|buf: &[u8]| buf == b"password\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(..)
//            .withf(|buf: &[u8]| buf == b"abort\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        let instrument = ki2600::Instrument::new(
//            kic_lib::protocol::Protocol::Raw(Box::new(interface)),
//            Box::new(auth),
//        );
//        let mut debugger = Debugger::new(Box::new(instrument));
//        debugger.continue_debugging().unwrap();
//    }
//
//    #[test]
//    fn test_stepin_debugging() {
//        let mut interface = MockInterface::new();
//        let auth = MockAuthenticate::new();
//        let mut seq = Sequence::new();
//        interface
//            .expect_write()
//            .times(..)
//            .withf(|x| x == b"abort\n")
//            .returning(|x| Ok(x.len()));
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiStepIn\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiDebugger = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiClearBreakpoints = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiSetBreakpoint = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiSetWatchpoint = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiClearWatchpoints = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiClearWatchpoint = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiExecuteWithDebugger = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiSetUpVariable = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiSetLocalVariable = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiSetGlobalVariable = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"script.delete(\"kiDebugger\")\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(..)
//            .withf(|buf: &[u8]| buf == b"password\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(..)
//            .withf(|buf: &[u8]| buf == b"abort\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        let instrument = ki2600::Instrument::new(
//            kic_lib::protocol::Protocol::Raw(Box::new(interface)),
//            Box::new(auth),
//        );
//        let mut debugger = Debugger::new(Box::new(instrument));
//        debugger.stepin_debugging().unwrap();
//    }
//
//    #[test]
//    fn test_stepout_debugging() {
//        let mut interface = MockInterface::new();
//        let auth = MockAuthenticate::new();
//        let mut seq = Sequence::new();
//        interface
//            .expect_write()
//            .times(..)
//            .withf(|x| x == b"abort\n")
//            .returning(|x| Ok(x.len()));
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiStepOut\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiDebugger = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiClearBreakpoints = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiSetBreakpoint = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiSetWatchpoint = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiClearWatchpoints = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiClearWatchpoint = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiExecuteWithDebugger = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiSetUpVariable = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiSetLocalVariable = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiSetGlobalVariable = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"script.delete(\"kiDebugger\")\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(..)
//            .withf(|buf: &[u8]| buf == b"password\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(..)
//            .withf(|buf: &[u8]| buf == b"abort\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        // let (interface, _) = test_exit(interface, seq);
//
//        let instrument = ki2600::Instrument::new(
//            kic_lib::protocol::Protocol::Raw(Box::new(interface)),
//            Box::new(auth),
//        );
//        {
//            let mut debugger = Debugger::new(Box::new(instrument));
//            debugger.stepout_debugging().unwrap();
//        }
//    }
//
//    #[test]
//    fn test_stepover_debugging() {
//        let mut interface = MockInterface::new();
//        let auth = MockAuthenticate::new();
//        let mut seq = Sequence::new();
//        interface
//            .expect_write()
//            .times(..)
//            .withf(|x| x == b"abort\n")
//            .returning(|x| Ok(x.len()));
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiStepOver\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiDebugger = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiClearBreakpoints = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiSetBreakpoint = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiSetWatchpoint = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiClearWatchpoints = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiClearWatchpoint = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiExecuteWithDebugger = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiSetUpVariable = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiSetLocalVariable = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiSetGlobalVariable = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"script.delete(\"kiDebugger\")\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(..)
//            .withf(|buf: &[u8]| buf == b"password\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(..)
//            .withf(|buf: &[u8]| buf == b"abort\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        let instrument = ki2600::Instrument::new(
//            kic_lib::protocol::Protocol::Raw(Box::new(interface)),
//            Box::new(auth),
//        );
//        let mut debugger = Debugger::new(Box::new(instrument));
//        debugger.stepover_debugging().unwrap();
//    }
//
//    #[test]
//    fn test_exit_debugger() {
//        let mut interface = MockInterface::new();
//        let auth = MockAuthenticate::new();
//        let debug_file_name = Some(String::from("kic_test_file"));
//        let test_file_name = debug_file_name.clone();
//        let mut seq = Sequence::new();
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"abort\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"kiDebugger = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(move |buf: &[u8]| buf == b"kic_test_file = nil\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"script.delete(\"kic_test_file\")\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        let ki_debugger_global_functions = &[
//            "kiClearBreakpoints",
//            "kiSetBreakpoint",
//            "kiSetWatchpoint",
//            "kiClearWatchpoints",
//            "kiClearWatchpoint",
//            "kiExecuteWithDebugger",
//            "kiSetUpVariable",
//            "kiSetLocalVariable",
//            "kiSetGlobalVariable",
//        ];
//
//        for func in ki_debugger_global_functions {
//            interface
//                .expect_write()
//                .times(1)
//                .in_sequence(&mut seq)
//                .withf(move |buf: &[u8]| buf == format!("{func} = nil\n").as_bytes())
//                .returning(|buf: &[u8]| Ok(buf.len()));
//        }
//
//        interface
//            .expect_write()
//            .times(1)
//            .in_sequence(&mut seq)
//            .withf(|buf: &[u8]| buf == b"script.delete(\"kiDebugger\")\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(..)
//            .withf(|buf: &[u8]| buf == b"password\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        interface
//            .expect_write()
//            .times(..)
//            .withf(|buf: &[u8]| buf == b"abort\n")
//            .returning(|buf: &[u8]| Ok(buf.len()));
//
//        let instrument = ki2600::Instrument::new(
//            kic_lib::protocol::Protocol::Raw(Box::new(interface)),
//            Box::new(auth),
//        );
//        // debugger gets dropped when goes out of scope.
//        {
//            let mut debugger = Debugger::new(Box::new(instrument));
//            debugger.debuggee_file_name = test_file_name;
//            assert_eq!(debugger.debuggee_file_name, debug_file_name);
//        }
//    }
//
//    mock! {
//       Interface {}
//
//       impl interface::Interface for Interface {}
//
//
//       impl Read for Interface {
//           fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize>;
//       }
//
//       impl Write for Interface {
//           fn write(&mut self, buf: &[u8]) -> std::io::Result<usize>;
//
//           fn flush(&mut self) -> std::io::Result<()>;
//       }
//
//       impl NonBlock for Interface {
//           fn set_nonblocking(&mut self, enable: bool) -> Result<(), kic_lib::InstrumentError>;
//       }
//
//       impl Info for Interface {}
//    }
//
//    mock! {
//
//        Authenticate {}
//
//        impl Authentication for Authenticate {
//            fn read_password(&self) -> std::io::Result<String>;
//        }
//
//    }
//}
