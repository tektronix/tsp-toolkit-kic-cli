#![feature(rustdoc_missing_doc_code_examples, stmt_expr_attributes)]
#![doc(html_logo_url = "../../../ki-comms_doc_icon.png")]

//! The `kic` executable is a command-line tool that will allow a user to interact with
//! an instrument over all the media provided by the [`tsp-instrument`] crate.
//! This is done via an easy to understand command-line interface and, when
//! interactively connected to an instrument, with a REPL

mod error;
use crate::error::KicError;

mod process;
use crate::process::Process;

use anyhow::Context;
use clap::{
    arg, builder::PathBufValueParser, command, value_parser, Arg, ArgAction, ArgMatches, Args,
    Command, Subcommand,
};
use colored::Colorize;
use instrument_repl::repl::{self};
use regex::Regex;
use std::{
    collections::HashMap,
    env::set_var,
    fs::OpenOptions,
    io::{stdin, Read, Write},
    net::{IpAddr, SocketAddr, TcpStream},
    path::PathBuf,
    process::exit,
    sync::Mutex,
    thread,
    time::Duration,
};
use tracing::{debug, error, info, instrument, level_filters::LevelFilter, trace, warn};
use tracing_subscriber::{layer::SubscriberExt, Layer, Registry};

use tsp_toolkit_kic_lib::{
    instrument::{authenticate::Authentication, read_until, Instrument, State},
    model::{connect_protocol, connect_to},
    protocol::Protocol,
    ConnectionInfo,
};

#[derive(Debug, Subcommand)]
enum TerminateType {
    /// Perform the given action over a LAN connection.
    Lan(LanTerminateArgs),
}

#[derive(Debug, Args)]
struct LanTerminateArgs {
    /// The port to which to connect in order to terminate all other connections to the
    /// instrument
    #[arg(long, short = 'p', default_value = "5030")]
    port: Option<u16>,

    /// The IP address of the instrument to connect to.
    ip_addr: IpAddr,
}

// hack to make sure we rebuild if either Cargo.toml changes, since `clap` gets
// information from there.
#[cfg(not(debug_assertions))]
const _: &str = include_str!("../Cargo.toml");
#[cfg(not(debug_assertions))]
const _: &str = include_str!("../../Cargo.toml");

fn add_connection_subcommands(
    command: impl Into<Command>,
    additional_args: impl IntoIterator<Item = Arg>,
) -> Command {
    let mut command: Command = command.into();

    command = command.arg(
        Arg::new("addr")
            .help("The IP address or VISA resource string (requires VISA driver) to connect to")
            .required(true)
            .value_parser(value_parser!(ConnectionInfo)),
    ).arg(
        Arg::new("keyring")
           .help("Attempt to look up the credentials for this instrument using the provided id in the system keyring")
            .required(false)
            .long("keyring")
            .value_parser(value_parser!(String)),
    ).arg(
        Arg::new("password")
            .help("Use the provided password to authenticate with the instrument.")
            .required(false)
            .long("password")
            .value_parser(value_parser!(String)),
    ).arg(
        Arg::new("username")
            .help("Use the provided username to authenticate with the instrument.")
            .required(false)
            .long("username")
            .value_parser(value_parser!(String)),
    );

    for arg in additional_args {
        command = command.arg(arg.clone());
    }

    command
}

#[must_use]
fn cmds() -> Command {
    command!()
        .propagate_version(true)
        .subcommand_required(true)
        .allow_external_subcommands(true)
        .arg(
            Arg::new("log-file")
            .short('l')
            .long("log-file")
            .required(false)
            .help("Log to the given log file path. Can be used in conjunction with `--log-socket` and `--verbose`.")
            .global(true)
            .value_parser(PathBufValueParser::new()),
        )
        .arg(
            Arg::new("log-socket")
            .short('t')
            .long("log-socket")
            .required(false)
            .help("Log to the given socket (in IPv4 or IPv6 format with port number). Can be used in conjunction with `--log-file` and `--verbose`.")
            .global(true)
            .value_parser(clap::value_parser!(SocketAddr)),
        )
        .arg(
            Arg::new("verbose")
            .short('v')
            .long("verbose")
            .required(false)
            .help("Enable logging to stderr. Can be used in conjunction with `--log-file` and `--verbose`.")
            .global(true)
            .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("no-color")
                .short('n')
                .long("no-color")
                .help("Turn off ANSI color and formatting codes")
                .global(true)
                .action(ArgAction::SetTrue),
        )
        // This is mostly for subcommands, but is left here as an example.
        // We want to find all `kic-*` applications and run it with this option in order to add the sub command here.
        .subcommand(Command::new("print-description").hide(true))
        .subcommand({
            let cmd = Command::new("connect")
                .about("Connect to an instrument over one of the provided interfaces");
            add_connection_subcommands(cmd, [
                Arg::new("dump-output")
                    .short('o')
                    .long("dump-output")
                    .help("Display the contents of the file before the first prompt")
                    .hide(true)
                    .hide_long_help(true)
                    .value_parser(PathBufValueParser::new()),
            ])
        })
        .subcommand({
            let cmd = Command::new("reset")
                .about("Connect to an instrument, cancel any ongoing jobs, send *RST then exit.");
            add_connection_subcommands(cmd, [])
        })
        .subcommand({
            let cmd = Command::new("info")
                .about("Get the IDN information about an instrument.");
            add_connection_subcommands(cmd, [
                Arg::new("json")
                    .help("Print the instrument information in JSON format.")
                    .long("json")
                    .short('j')
                    .action(ArgAction::SetTrue)
            ])
        })
        .subcommand({
            let cmd = Command::new("check-login")
                .about("Check if a login is required for the given instrument.");

            add_connection_subcommands(cmd, [])
        })
        .subcommand({
            let cmd = Command::new("login")
                .about("Log in to the given instrument");

            add_connection_subcommands(cmd, [ ])
        })
        .subcommand({
            let cmd = Command::new("dump")
                .about("Dump the contents of the instrument output and error queue without any initial setup.");

            add_connection_subcommands(cmd, [
                    Arg::new("output")
                        .short('o')
                        .long("output")
                        .help("The file to which the contents of the instrument output queue should be written (defaults to stdout)")
                        .required(false)
                        .value_parser(PathBufValueParser::new()),
            ])
        })
        .subcommand({
            let cmd = Command::new("upgrade")
                .about("Upgrade the firmware of an instrument or module.");

            add_connection_subcommands(cmd, [
                    Arg::new("file")
                        .help("The file path of the firmware image.")
                        .required(true)
                        .value_parser(PathBufValueParser::new()),

                    Arg::new("slot")
                        .short('m')
                        .long("slot")
                        .help("[VersaTest only] Update a module in given slot number instead of the VersaTest mainframe")
                        .required(false)
                        .value_parser(value_parser!(u16).range(1..=3)),
            ])
        })
        .subcommand({
            let cmd = Command::new("script")
                .about("Load the script onto the selected instrument");
            add_connection_subcommands(cmd, [
                    Arg::new("file")
                        .required(true)
                        .help("The file path of the firmware image")
                        .value_parser(PathBufValueParser::new()),

                    Arg::new("run")
                        .short('r')
                        .long("run")
                        .value_parser(value_parser!(bool))
                        .default_value("true")
                        .default_missing_value("true")
                        .action(ArgAction::Set)
                        .help("Run the script immediately after loading. "),

                    Arg::new("save")
                        .short('s')
                        .long("save")
                        .action(ArgAction::SetTrue)
                        .help("Save the script to the non-volatile memory of the instrument"),
            ])
        })
        .subcommand({
            let cmd = Command::new("terminate")
                .about("Terminate all the connections on the given instrument. Only supports LAN");
            TerminateType::augment_subcommands(cmd)
        })
}

fn main() -> anyhow::Result<()> {
    let parent_dir: Option<PathBuf> = std::env::current_exe().map_or(None, |path| {
        path.canonicalize()
            .expect("should have canonicalized path")
            .parent()
            .map(std::convert::Into::into)
    });
    let cmd = cmds();

    let Ok((external_cmd_lut, mut cmd)) = find_subcommands_from_path(&parent_dir, cmd) else {
        return Err(anyhow::Error::msg(
            "Unable to search directory for possible subcommands.",
        ));
    };

    let matches = cmd.clone().get_matches();

    if matches.get_flag("no-color") {
        set_var("NO_COLOR", "1");
    }

    let verbose: bool = matches.get_flag("verbose");
    let log_file: Option<&PathBuf> = matches.get_one("log-file");
    let log_socket: Option<&SocketAddr> = matches.get_one("log-socket");

    #[cfg(debug_assertions)]
    const LOGFILE_LEVEL: LevelFilter = LevelFilter::TRACE;
    #[cfg(not(debug_assertions))]
    const LOGFILE_LEVEL: LevelFilter = LevelFilter::DEBUG;

    const STDERR_LEVEL: LevelFilter = LevelFilter::INFO;

    match (verbose, log_file, log_socket) {
        (true, Some(l), Some(s)) => {
            let err = tracing_subscriber::fmt::layer()
                .with_ansi(true)
                .with_writer(std::io::stderr)
                .with_filter(STDERR_LEVEL);

            let log = OpenOptions::new().append(true).create(true).open(l)?;

            let log = tracing_subscriber::fmt::layer()
                .with_writer(log)
                .fmt_fields(tracing_subscriber::fmt::format::DefaultFields::new())
                .with_ansi(false);

            let sock = TcpStream::connect(s)?;
            let sock = tracing_subscriber::fmt::layer()
                .with_writer(Mutex::new(sock))
                .fmt_fields(tracing_subscriber::fmt::format::DefaultFields::new())
                .json();

            let logger = Registry::default()
                .with(LOGFILE_LEVEL)
                .with(err)
                .with(log)
                .with(sock);

            tracing::subscriber::set_global_default(logger)?;
        }
        (true, Some(l), None) => {
            let err = tracing_subscriber::fmt::layer()
                .with_ansi(true)
                .with_writer(std::io::stderr)
                .with_filter(STDERR_LEVEL);

            let log = OpenOptions::new().append(true).create(true).open(l)?;

            let log = tracing_subscriber::fmt::layer()
                .with_writer(log)
                .fmt_fields(tracing_subscriber::fmt::format::DefaultFields::new())
                .with_ansi(false);

            let logger = Registry::default().with(LOGFILE_LEVEL).with(err).with(log);

            tracing::subscriber::set_global_default(logger)?;
        }
        (false, Some(l), Some(s)) => {
            let log = OpenOptions::new().append(true).create(true).open(l)?;

            let log = tracing_subscriber::fmt::layer()
                .with_writer(log)
                .with_ansi(false);

            let sock = TcpStream::connect(s)?;
            let sock = tracing_subscriber::fmt::layer()
                .with_writer(Mutex::new(sock))
                .fmt_fields(tracing_subscriber::fmt::format::DefaultFields::new())
                .json();

            let logger = Registry::default().with(LOGFILE_LEVEL).with(log).with(sock);

            tracing::subscriber::set_global_default(logger)?;
        }
        (false, Some(l), None) => {
            let log = OpenOptions::new().append(true).create(true).open(l)?;

            let log = tracing_subscriber::fmt::layer()
                .with_writer(log)
                .with_ansi(false);

            let logger = Registry::default().with(LOGFILE_LEVEL).with(log);

            tracing::subscriber::set_global_default(logger)?;
        }
        (true, None, Some(s)) => {
            let err = tracing_subscriber::fmt::layer()
                .with_ansi(true)
                .with_writer(std::io::stderr);

            let sock = TcpStream::connect(s)?;
            let sock = tracing_subscriber::fmt::layer()
                .with_writer(Mutex::new(sock))
                .fmt_fields(tracing_subscriber::fmt::format::DefaultFields::new())
                .json();

            let logger = Registry::default().with(LOGFILE_LEVEL).with(err).with(sock);

            tracing::subscriber::set_global_default(logger)?;
        }
        (true, None, None) => {
            let err = tracing_subscriber::fmt::layer()
                .with_ansi(true)
                .with_writer(std::io::stderr);

            let logger = Registry::default().with(LOGFILE_LEVEL).with(err);

            tracing::subscriber::set_global_default(logger)?;
        }
        (false, None, Some(s)) => {
            let sock = TcpStream::connect(s)?;
            let sock = tracing_subscriber::fmt::layer()
                .with_writer(Mutex::new(sock))
                .fmt_fields(tracing_subscriber::fmt::format::DefaultFields::new())
                .json();

            let logger = Registry::default().with(LOGFILE_LEVEL).with(sock);

            tracing::subscriber::set_global_default(logger)?;
        }
        (false, None, None) => {}
    }

    info!("Application started");
    trace!(
        "Application starting with the following args: {:?}",
        std::env::args()
    );
    match matches.subcommand() {
        Some(("print-description", _)) => {
            println!("{}", clap::crate_description!());
            return Ok(());
        }
        Some(("connect", sub_matches)) => {
            return connect(sub_matches);
        }
        Some(("reset", sub_matches)) => {
            return reset(sub_matches);
        }
        Some(("abort", sub_matches)) => {
            return abort(sub_matches);
        }
        Some(("dump", sub_matches)) => {
            return dump(sub_matches);
        }
        Some(("upgrade", sub_matches)) => {
            return upgrade(sub_matches);
        }
        Some(("terminate", sub_matches)) => {
            return terminate(sub_matches);
        }
        Some(("script", sub_matches)) => {
            return script(sub_matches);
        }
        Some(("check-login", sub_matches)) => {
            return check_login(sub_matches);
        }
        Some(("login", sub_matches)) => {
            return login(sub_matches);
        }
        Some(("info", sub_matches)) => {
            return info(sub_matches);
        }
        Some((ext, sub_matches)) => {
            debug!("Subcommand '{ext}' not defined internally, checking external commands");
            if let Some((path, ..)) = external_cmd_lut.get(ext) {
                debug!("Subcommand exists at '{path:?}'");

                let mut args: Vec<_> = sub_matches
                    .get_many::<String>("options")
                    .into_iter()
                    .flatten()
                    .cloned()
                    .collect();

                if verbose {
                    args.push("--verbose".to_string())
                }

                if let Some(log_file) = log_file {
                    args.push("--log-file".to_string());
                    args.push(log_file.to_str().unwrap().to_string())
                }

                if let Some(log_socket) = log_socket {
                    args.push("--log-socket".to_string());
                    args.push(log_socket.to_string());
                }

                debug!("Replacing this executable with '{path:?}' args: {args:?}");

                if let Err(e) = Process::new(path.clone(), args)
                    .exec_replace()
                    .context(format!("{ext} subcommand should launch in a child process"))
                {
                    error!("{e}");
                    return Err(e);
                }
                //Process::exec_replace() only returns to this function if there was a error.
            } else {
                let err = clap::Error::new(clap::error::ErrorKind::UnknownArgument);
                error!("{err}");
                println!("{err}");
                cmd.print_help()?;
                return Err(err.into());
            }
        }
        _ => unreachable!(),
    }

    info!("Application closing");

    Ok(())
}

/// Check the connection status of the instrument. This will cause a connect and disconnect
/// from the instrument.
fn check_connection_login_status(conn: &ConnectionInfo) -> Result<(), KicError> {
    // We can check instrument login with Authentication::NoAuth because we aren't trying to log
    // in but simply check whether the instrument is password protected.
    let mut instrument: Box<dyn Instrument> =
        match connect_sync_instrument(conn, Authentication::NoAuth) {
            Ok(i) => i,
            Err(e) => {
                error!("Unable to connect to instrument interface: {e}");
                return Err(e);
            }
        };

    //TODO: Add call to not reset the instrument after disconnecting.

    match instrument.check_login()? {
        State::Needed => Err(KicError::InstrumentPasswordProtected),
        State::NotNeeded => Ok(()),
        State::LogoutNeeded => Err(KicError::InstrumentLogoutRequired),
    }
}

#[instrument(skip(args))]
fn check_login(args: &ArgMatches) -> anyhow::Result<()> {
    info!("Checking login");
    let Some(conn) = args.get_one::<ConnectionInfo>("addr") else {
        error!("No IP address or VISA resource string given");
        return Err(KicError::ArgParseError {
            details: "No IP address or VISA resource string given".to_string(),
        }
        .into());
    };
    match check_connection_login_status(conn) {
        Ok(()) => println!("NOT PROTECTED"),
        Err(KicError::InstrumentPasswordProtected) => println!("PROTECTED"),
        Err(KicError::InstrumentLogoutRequired) => println!("PROTECTED, IN USE"),
        Err(e) => return Err(e.into()),
    }
    Ok(())
}

#[instrument(skip(args))]
fn login(args: &ArgMatches) -> anyhow::Result<()> {
    info!("Login to instrument");
    let Some(conn) = args.get_one::<ConnectionInfo>("addr") else {
        error!("No IP address or VISA resource string given");
        return Err(KicError::ArgParseError {
            details: "No IP address or VISA resource string given".to_string(),
        }
        .into());
    };

    let auth = auth_type(conn, args);

    let mut inst = connect_sync_instrument(conn, auth)?;

    inst.login()?;

    let info = inst.info()?;
    println!("{}#{}", info.model, info.serial_number);

    Ok(())
}

#[instrument]
fn connect_async_protocol(t: &ConnectionInfo) -> Result<Protocol, KicError> {
    info!("Asynchronously connecting to interface");
    let interface: Protocol = match t {
        //ConnectionInfo::Lan { addr } => Protocol::new(AsyncStream::try_from(Arc::new(
        //    TcpStream::connect(addr)?,
        //)
        //    as Arc<dyn Interface + Send + Sync>)?),
        ConnectionInfo::Lan { .. }
        | ConnectionInfo::Vxi11 { .. }
        | ConnectionInfo::HiSlip { .. }
        | ConnectionInfo::VisaSocket { .. }
        | ConnectionInfo::Gpib { .. }
        | ConnectionInfo::Usb { .. } => {
            Protocol::connect(t)?
            //Protocol::new(AsyncStream::try_from(Arc::new(
            //    Visa::new(&string)?,
            //)
            //as Arc<dyn Interface + Send + Sync>)?),
        }
    };
    trace!("Asynchronously connected to interface");
    Ok(interface)
}

#[instrument]
fn connect_sync_instrument(
    t: &ConnectionInfo,
    auth: Authentication,
) -> Result<Box<dyn Instrument>, KicError> {
    trace!("Connecting to sync instrument");
    let instrument: Box<dyn Instrument> = connect_to(t, auth)?;
    info!("Successfully connected to sync instrument");
    Ok(instrument)
}

#[instrument]
fn connect_async_instrument(
    t: &ConnectionInfo,
    auth: Authentication,
) -> Result<Box<dyn Instrument>, KicError> {
    let interface: Protocol = connect_async_protocol(t)?;

    trace!("Connecting to async instrument");
    let instrument: Box<dyn Instrument> = connect_protocol(t, interface, auth)?;
    info!("Successfully connected to async instrument");
    Ok(instrument)
}

#[instrument(skip(inst))]
fn get_instrument_access(inst: &mut Box<dyn Instrument>) -> anyhow::Result<()> {
    info!("Configuring instrument for usage.");
    debug!("Checking login");
    match inst.as_mut().check_login()? {
        State::Needed => {
            trace!("Login required");
            inst.as_mut().login()?;
            debug!("Login complete");
        }
        State::LogoutNeeded => {
            return Err(KicError::InstrumentLogoutRequired.into());
        }
        State::NotNeeded => {
            debug!("Login not required");
        }
    };
    debug!("Checking instrument language");
    match inst.as_mut().get_language()? {
        tsp_toolkit_kic_lib::instrument::CmdLanguage::Scpi => {
            warn!("Instrument language set to SCPI, only TSP is supported. Prompting user...");
            eprintln!("Instrument command-set is not set to TSP. Would you like to change the command-set to TSP and reboot? (Y/n)");

            let mut buf = String::new();
            stdin().read_line(&mut buf)?;
            let buf = buf.trim();
            if buf.is_empty() || buf.contains(['Y', 'y']) {
                debug!("User accepted language change on the instrument.");
                info!("Changing instrument language to TSP.");
                inst.as_mut()
                    .change_language(tsp_toolkit_kic_lib::instrument::CmdLanguage::Tsp)?;
                info!("Instrument language changed to TSP.");
                warn!("Instrument rebooting.");
                inst.write_all(b"ki.reboot()\n")?;
                eprintln!("Instrument rebooting, please reconnect after reboot completes.");
                thread::sleep(Duration::from_millis(1500));
                info!("Exiting after instrument reboot");
                exit(0);
            }
        }
        tsp_toolkit_kic_lib::instrument::CmdLanguage::Tsp => {
            debug!("Instrument language already set to TSP, no change necessary.");
        }
    }

    info!("Instrument configured for usage");

    Ok(())
}

fn auth_type(conn: &ConnectionInfo, args: &ArgMatches) -> Authentication {
    if let Some(id) = args.get_one::<String>("keyring") {
        Authentication::Keyring { id: id.to_string() }
    } else if let Some(password) = args.get_one::<String>("password") {
        let username = if let Some(username) = args.get_one::<String>("username") {
            username
        } else {
            &String::new()
        };
        Authentication::Credential {
            username: username.to_string(),
            password: password.to_string(),
        }
    } else if check_connection_login_status(conn).is_ok() {
        Authentication::NoAuth
    } else {
        Authentication::Prompt
    }
}

fn pause_exit_on_error() {
    eprintln!(
        "\n\n{}",
        "An error occured. Press Enter to close this program.".yellow()
    );
    let mut buf = String::new();
    let _ = std::io::stdin().read_line(&mut buf);
}

#[instrument(skip(args))]
fn connect(args: &ArgMatches) -> anyhow::Result<()> {
    info!("Connecting to instrument");
    trace!("args: {args:?}");
    eprintln!(
        "\nTektronix TSP Shell\nType {} for more commands.\n",
        ".help".bold()
    );
    let Some(conn) = args.get_one::<ConnectionInfo>("addr") else {
        error!("No IP address or VISA resource string given");
        eprintln!(
                "{}",
                "\nUnable to parse connection information: no connection information given\n\nUnrecoverable error. Closing.".red()
            );
        pause_exit_on_error();
        return Err(KicError::ArgParseError {
            details: "No IP address or VISA resource string given".to_string(),
        }
        .into());
    };

    if let Some(dump_path) = args.get_one::<PathBuf>("dump-output") {
        if let Ok(mut dump_file) = std::fs::File::open(dump_path) {
            let mut contents = String::new();
            match dump_file.read_to_string(&mut contents) {
                Ok(_) => {
                    if !contents.trim().is_empty() {
                        trace!("Printing dump-output:");
                        eprintln!(
                            "{}",
                            "Data left on output queue of instrument before connecting:".blue()
                        );
                        println!("{}", contents.bright_black());
                    }
                }
                Err(e) => error!("{e}"),
            }
        }
    }

    let auth = auth_type(conn, args);

    let mut instrument: Box<dyn Instrument> = match connect_async_instrument(conn, auth) {
        Ok(i) => i,
        Err(e) => {
            error!("Error connecting to async instrument: {e}");
            eprintln!(
                "{}",
                format!(
                    "\nError connecting to async instrument: {e}\n\nUnrecoverable error. Closing."
                )
                .red()
            );
            pause_exit_on_error();
            return Err(e.into());
        }
    };

    if let Err(e) = get_instrument_access(&mut instrument) {
        error!("Error setting up instrument: {e}");
        eprintln!(
            "{}",
            format!("\nError setting up instrument: {e}\n\nUnrecoverable error. Closing.").red()
        );
        pause_exit_on_error();
        return Err(e);
    }

    let info = match instrument.info() {
        Ok(i) => i,
        Err(e) => {
            error!("Error getting instrument info: {e}");
            eprintln!(
                "{}",
                format!("\nError getting instrument info: {e}\n\nUnrecoverable error. Closing.")
                    .red()
            );
            pause_exit_on_error();
            return Err(e.into());
        }
    };
    info!("IDN: {info}");
    eprintln!("{info}");

    let mut repl = repl::Repl::new(instrument);

    info!("Starting instrument REPL");
    if let Err(e) = repl.start() {
        error!("Error in REPL: {e}");
        eprintln!(
            "{}",
            format!("\n{e}\n\nClosing instrument connection...").red()
        );
        drop(repl);
        pause_exit_on_error();
    }

    Ok(())
}

#[instrument(skip(args))]
fn dump(args: &ArgMatches) -> anyhow::Result<()> {
    info!("Dumping contents of instrument output and error queue");
    trace!("args: {args:?}");

    let Some(conn) = args.get_one::<ConnectionInfo>("addr") else {
        error!("No IP address or VISA resource string given");
        eprintln!(
                "{}",
                "\nUnable to parse connection information: no connection information given\n\nUnrecoverable error. Closing.".red()
            );
        pause_exit_on_error();
        return Err(KicError::ArgParseError {
            details: "No IP address or VISA resource string given".to_string(),
        }
        .into());
    };

    let mut output: Box<dyn Write> = match args.get_one::<PathBuf>("output") {
        Some(o) => Box::new(std::fs::File::create(o)?),
        None => Box::new(std::io::stdout()),
    };

    let auth = auth_type(conn, args);

    let mut instrument = connect_sync_instrument(conn, auth)?;
    //TODO: call option to not do reset on disconnect.

    let timestamp = chrono::Utc::now().to_string();

    trace!("Writing print('{timestamp}') to instrument");
    instrument.write_all(format!("print('{timestamp}')\n").as_bytes())?;
    trace!("Write complete");

    //get output
    loop {
        let mut buf = vec![0u8; 512];
        let bytes = instrument.read(&mut buf)?;

        let buf = &buf[0..bytes];

        if String::from_utf8_lossy(buf).contains(&timestamp) {
            break;
        }

        output.write_all(buf)?;
    }

    Ok(())
}

#[instrument(skip(args))]
fn upgrade(args: &ArgMatches) -> anyhow::Result<()> {
    info!("Upgrading instrument");
    trace!("args: {args:?}");
    eprintln!("\nTektronix TSP Shell\n");

    let Some(conn) = args.get_one::<ConnectionInfo>("addr") else {
        error!("No IP address or VISA resource string given");
        eprintln!(
                "{}",
                "\nUnable to parse connection information: no connection information given\n\nUnrecoverable error. Closing.".red()
            );
        pause_exit_on_error();
        return Err(KicError::ArgParseError {
            details: "No IP address or VISA resource string given".to_string(),
        }
        .into());
    };

    let auth = auth_type(conn, args);

    let mut instrument: Box<dyn Instrument> = match connect_sync_instrument(conn, auth) {
        Ok(i) => i,
        Err(e) => {
            error!("Error connecting to sync instrument: {e}");
            return Err(e.into());
        }
    };

    if let Err(e) = get_instrument_access(&mut instrument) {
        error!("Error setting up instrument: {e}");
        return Err(e);
    }

    let info = match instrument.info() {
        Ok(i) => i,
        Err(e) => {
            error!("Error getting instrument info: {e}");
            return Err(e.into());
        }
    };
    info!("IDN: {info}");
    eprintln!("{info}");

    let slot: Option<u16> = args.get_one::<u16>("slot").copied();
    let Some(file) = args.get_one::<PathBuf>("file").cloned() else {
        let e = KicError::ArgParseError {
            details: "firmware file path was not provided".to_string(),
        };
        error!("{e}");
        return Err(e.into());
    };

    let mut image: Vec<u8> = Vec::new();

    let mut file = match std::fs::File::open(file) {
        Ok(file) => file,
        Err(e) => {
            error!("Error opening firmware file: {e}");
            return Err(e.into());
        }
    };

    if let Err(e) = file.read_to_end(&mut image) {
        error!("Error reading firmware file: {e}");
        return Err(e.into());
    }

    eprintln!("Flashing instrument firmware. Please do NOT power off or disconnect.");
    if let Err(e) = instrument.flash_firmware(&image, slot) {
        error!("Error upgrading instrument: {e}");
        return Err(e.into());
    }
    eprintln!("Flashing instrument firmware completed. Instrument will restart.");
    info!("Instrument upgrade complete");
    Ok(())
}

fn script(args: &ArgMatches) -> anyhow::Result<()> {
    info!("Loading script to instrument");
    trace!("args: {args:?}");

    eprintln!("\nTektronix TSP Shell\n");

    let Some(conn) = args.get_one::<ConnectionInfo>("addr") else {
        error!("No IP address or VISA resource string given");
        eprintln!(
                "{}",
                "\nUnable to parse connection information: no connection information given\n\nUnrecoverable error. Closing.".red()
            );
        pause_exit_on_error();
        return Err(KicError::ArgParseError {
            details: "No IP address or VISA resource string given".to_string(),
        }
        .into());
    };

    let auth = auth_type(conn, args);
    let mut instrument: Box<dyn Instrument> = match connect_sync_instrument(conn, auth) {
        Ok(i) => i,
        Err(e) => {
            error!("Error connecting to sync instrument: {e}");
            return Err(e.into());
        }
    };

    if let Err(e) = get_instrument_access(&mut instrument) {
        error!("Error setting up instrument: {e}");
        return Err(e);
    }

    let info = match instrument.info() {
        Ok(i) => i,
        Err(e) => {
            error!("Error getting instrument info: {e}");
            return Err(e.into());
        }
    };
    info!("IDN: {info}");
    eprintln!("{info}");

    let run: bool = *args.get_one::<bool>("run").unwrap_or(&true);
    let save: bool = *args.get_one::<bool>("save").unwrap_or(&false);

    let Some(path) = args.get_one::<PathBuf>("file").cloned() else {
        let e = KicError::ArgParseError {
            details: "script file path was not provided".to_string(),
        };
        error!("{e}");
        return Err(e.into());
    };

    let Some(stem) = path.file_stem() else {
        let e = KicError::ArgParseError {
            details: "unable to get file stem".to_string(),
        };

        error!("{e}");
        return Err(e.into());
    };

    let stem = stem.to_string_lossy();

    let re = Regex::new(r"[^A-Za-z\d_]");

    match re {
        Ok(re_res) => {
            let result = re_res.replace_all(&stem, "_");

            let script_name = format!("kic_{result}");

            let mut script_content: Vec<u8> = Vec::new();

            let mut file = match std::fs::File::open(path) {
                Ok(f) => f,
                Err(e) => {
                    error!("Error opening script file: {e}");
                    return Err(e.into());
                }
            };
            if let Err(e) = file.read_to_end(&mut script_content) {
                error!("Error reading script file: {e}");
                return Err(e.into());
            }

            eprintln!("Loading script to instrument.");

            match instrument.write_all(b"localnode.prompts=1\n") {
                Ok(()) => {}
                Err(e) => {
                    error!("Error file: {e}");
                    return Err(e.into());
                }
            }
            if let Err(e) = read_until(
                &mut instrument,
                &["TSP>".to_string()],
                20,
                Duration::from_millis(50),
            ) {
                return Err(e.into());
            };
            match instrument.write_script(script_name.as_bytes(), &script_content, save, run) {
                Ok(_) => {}
                Err(e) => return Err(e.into()),
            }

            eprintln!("Script loading completed.");
            info!("Script loading completed.");

            let mut accumulate = String::new();
            let _ = instrument.set_nonblocking(true);
            loop {
                let mut buf: Vec<u8> = vec![0u8; 512];
                match instrument.read(&mut buf) {
                    Ok(_) => {}
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(1));
                        continue;
                    }
                    Err(e) => return Err(e.into()),
                };
                let first_null = buf.iter().position(|&x| x == b'\0').unwrap_or(buf.len());
                let buf = &buf[..first_null];
                let buf = String::from_utf8_lossy(buf);
                if !buf.is_empty() {
                    accumulate = format!("{accumulate}{}", &buf);
                }
                let buf = buf
                    .split("TSP>")
                    .next()
                    .expect("should have had one element in the buffer");

                print!("{buf}");
                if accumulate.contains("TSP>\n") {
                    return Ok(());
                }
            }
        }
        Err(err_msg) => {
            unreachable!("Issue with regex creation: {}", err_msg.to_string());
        }
    }
}

#[instrument(skip(args))]
fn reset(args: &ArgMatches) -> anyhow::Result<()> {
    info!("Resetting instrument");
    let Some(conn) = args.get_one::<ConnectionInfo>("addr") else {
        error!("No IP address or VISA resource string given");
        eprintln!(
                "{}",
                "\nUnable to parse connection information: no connection information given\n\nUnrecoverable error. Closing.".red()
            );
        pause_exit_on_error();
        return Err(KicError::ArgParseError {
            details: "No IP address or VISA resource string given".to_string(),
        }
        .into());
    };

    let auth = auth_type(conn, args);

    let instrument: Box<dyn Instrument> = match connect_sync_instrument(conn, auth) {
        Ok(i) => i,
        Err(e) => {
            error!("Error connecting to sync instrument: {e}");
            return Err(e.into());
        }
    };

    // dropping the instrument will reset it appropriately.
    drop(instrument);

    info!("Instrument reset");

    Ok(())
}

/// This function will not make an actual connection to an instrument and will instead fetch
/// instrument information from the given connection address by getting the LXI Identification
/// page where possible.
#[instrument(skip(args))]
fn abort(args: &ArgMatches) -> anyhow::Result<()> {
    info!("Aborting instrument operations");
    let Some(conn) = args.get_one::<ConnectionInfo>("addr") else {
        error!("No IP address or VISA resource string given");
        eprintln!(
                "{}",
                "\nUnable to parse connection information: no connection information given\n\nUnrecoverable error. Closing.".red()
            );
        pause_exit_on_error();
        return Err(KicError::ArgParseError {
            details: "No IP address or VISA resource string given".to_string(),
        }
        .into());
    };

    let auth = auth_type(conn, args);

    let mut instrument: Box<dyn Instrument> = match connect_sync_instrument(conn, auth) {
        Ok(i) => i,
        Err(e) => {
            error!("Error connecting to sync instrument: {e}");
            return Err(e.into());
        }
    };

    instrument.abort()?;

    info!("Instrument opearation aborted.");

    Ok(())
}

#[instrument(skip(args))]
fn info(args: &ArgMatches) -> anyhow::Result<()> {
    info!("Getting instrument info");
    let Some(conn) = args.get_one::<ConnectionInfo>("addr") else {
        error!("No IP address or VISA resource string given");
        eprintln!(
                "{}",
                "\nUnable to parse connection information: no connection information given\n\nUnrecoverable error. Closing.".red()
            );
        pause_exit_on_error();
        return Err(KicError::ArgParseError {
            details: "No IP address or VISA resource string given".to_string(),
        }
        .into());
    };
    let info = match conn.get_info() {
        Ok(i) => i,
        Err(e) => {
            error!("Error getting instrument info: {e}");
            return Err(e.into());
        }
    };

    let json: bool = *args.get_one::<bool>("json").unwrap_or(&true);

    trace!("print as json?: {json:?}");

    let info: String = if json {
        serde_json::to_string(&info)?
    } else {
        info.to_string()
    };

    info!("Information to print: {info}");
    println!("{info}");

    Ok(())
}

#[instrument(skip(args))]
fn terminate(args: &ArgMatches) -> anyhow::Result<()> {
    info!("Terminating existing operations");
    trace!("args: {args:?}");
    eprintln!("\nTektronix TSP Shell\n");

    let Some(conn) = args.get_one::<ConnectionInfo>("addr") else {
        error!("No IP address or VISA resource string given");
        eprintln!(
                "{}",
                "\nUnable to parse connection information: no connection information given\n\nUnrecoverable error. Closing.".red()
            );
        pause_exit_on_error();
        return Err(KicError::ArgParseError {
            details: "No IP address or VISA resource string given".to_string(),
        }
        .into());
    };
    let mut conn = match conn {
        ConnectionInfo::VisaSocket { addr, .. } | ConnectionInfo::Lan { addr } => {
            let addr = addr.ip();
            let socket = SocketAddr::new(addr, 5030);
            TcpStream::connect(socket)?
        }
        ConnectionInfo::Vxi11 { addr, .. } => {
            let socket = SocketAddr::new(IpAddr::V4(*addr), 5030);
            TcpStream::connect(socket)?
        }
        ConnectionInfo::HiSlip { addr, .. } => {
            let socket = SocketAddr::new(*addr, 5030);
            TcpStream::connect(socket)?
        }
        ConnectionInfo::Gpib { .. } | ConnectionInfo::Usb { .. } => {
            return Err(KicError::UnsupportedAction(
                "terminate is not supported for GPIB or USBTMC devices".to_string(),
            )
            .into())
        }
    };

    if let Err(e) = conn.write_all(b"ABORT\n") {
        error!("Unable to write 'ABORT': {e}");
        return Err(e.into());
    }

    info!("Operations terminated");

    Ok(())
}

type FindSubcommands = (HashMap<String, (PathBuf, Option<String>)>, Command);

fn find_subcommands_from_path(
    path: &Option<PathBuf>,
    mut cmd: Command,
) -> anyhow::Result<FindSubcommands> {
    let mut lut = HashMap::new();
    if let Some(ref dir) = path {
        let contents: Vec<PathBuf> = dir.read_dir()?.map(|de| de.unwrap().path()).collect();

        for path in contents {
            let filename = path
                .file_stem()
                .unwrap_or_default()
                .to_str()
                .unwrap_or_default();
            if path.is_file() && filename.contains("kic-") && !filename.contains("visa") {
                let cmd_name = filename
                    .split("kic-")
                    .last()
                    .expect("should have been able to split filename")
                    .to_string();

                let Ok(result) = std::process::Command::new(path.clone())
                    .args(vec!["print-description"])
                    .output()
                else {
                    //ignore any issues.
                    continue;
                };
                let result = String::from_utf8_lossy(&result.stdout).trim().to_string();
                lut.insert(cmd_name.clone(), (path.clone(), Some(result.clone())));

                cmd = cmd.subcommand(
                        Command::new(cmd_name.clone())
                            .about(result)
                            .allow_external_subcommands(true)
                            .arg(arg!(<options> ...).trailing_var_arg(true))
                            .override_help(format!("For help on this command, run `{0} {1} help` or `{0} {1} --help` instead.", "kic", cmd_name))
                    );
            }
        }
    }

    Ok((lut, cmd))
}
