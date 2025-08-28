use chrono::Utc;
use clap::{command, value_parser, Arg, ArgMatches, Command};
use colored::Colorize;
use kic_debug_visa::debugger::Debugger;
use kic_lib::{
    instrument::{authenticate::Authentication, CmdLanguage, Instrument, State},
    model::connect_to,
    ConnectionInfo,
};
use std::io::{stdin, ErrorKind};
use std::process::exit;
use std::thread;
use std::time::Duration;
use thiserror::Error;
use tracing::{debug, error, info, trace, warn};

#[derive(Error, Debug)]
#[allow(clippy::module_name_repetitions)]
pub enum KicError {
    /// The user didn't provide required information or the information provided was
    /// invalid
    #[error("Error parsing arguments: {details}")]
    ArgParseError {
        /// The reason why the arguments failed to parse.
        details: String,
    },

    /// Another user must relinquish the instrument before it can be logged into.
    #[error("there is another session connected to the instrument that must logout")]
    InstrumentLogoutRequired,

    /// The instrument is protected over the given interface. This should ONLY be used
    /// for checking the login status of an instrument.
    #[error("the instrument is password protected")]
    InstrumentPasswordProtected,

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// The requested action was not supported.
    #[error("the requested action is not supported: {0}")]
    UnsupportedAction(String),

    #[error("instrument error: {0}")]
    InstrumentError(#[from] kic_lib::InstrumentError),
}

fn main() -> anyhow::Result<()> {
    let cmd = command!()
        .propagate_version(true)
        .subcommand_required(true)
        .allow_external_subcommands(true)
        .subcommand(Command::new("print-description").hide(true))
        .subcommand({
            let connect_command = Command::new("connect")
                .about("Connect to an instrument over one of the provided interfaces");
            add_connection_subcommands(connect_command)
        });
    let matches = cmd.clone().get_matches();

    if let Some(("print-description", _)) = matches.subcommand() {
        println!("{}", cmd.get_about().unwrap_or_default());
        return Ok(());
    }

    let mut debugger: Debugger = match matches.subcommand() {
        Some(("connect", sub_matches)) => {
            let mut instrument = connect(sub_matches).map_err(|e| {
                eprintln!("Failed to connect to instrument: {e}");
                e
            })?;
            clear_output_queue(&mut instrument, 5000, Duration::from_millis(1))?;
            Debugger::new(instrument)
        }
        _ => unreachable!(),
    };

    Ok(debugger.start()?)
}

fn connect(args: &ArgMatches) -> anyhow::Result<Box<dyn Instrument>> {
    let Some(conn) = args.get_one::<ConnectionInfo>("addr") else {
        error!("No IP address or VISA resource string given");
        eprintln!(
                "{}",
                "\nUnable to parse connection information: no connection information given\n\nUnrecoverable error. Closing.".red()
            );

        return Err(KicError::ArgParseError {
            details: "No IP address or VISA resource string given".to_string(),
        }
        .into());
    };
    let auth = auth_type(conn, args);

    let mut instrument: Box<dyn Instrument> = match get_instrument(conn, auth) {
        Ok(i) => i,
        Err(e) => {
            error!("Error connecting to async instrument: {e}");
            return Err(e.into());
        }
    };

    if let Err(e) = get_instrument_access(&mut instrument) {
        error!("Error setting up instrument: {e}");
        return Err(e);
    }

    Ok(instrument)
}

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
        CmdLanguage::Scpi => {
            warn!("Instrument language set to SCPI, only TSP is supported. Prompting user...");
            eprintln!("Instrument command-set is not set to TSP. Would you like to change the command-set to TSP and reboot? (Y/n)");

            let mut buf = String::new();
            stdin().read_line(&mut buf)?;
            let buf = buf.trim();
            if buf.is_empty() || buf.contains(['Y', 'y']) {
                debug!("User accepted language change on the instrument.");
                info!("Changing instrument language to TSP.");
                inst.as_mut()
                    .change_language(kic_lib::instrument::CmdLanguage::Tsp)?;
                info!("Instrument language changed to TSP.");
                warn!("Instrument rebooting.");
                inst.write_all(b"ki.reboot()\n")?;
                eprintln!("Instrument rebooting, please reconnect after reboot completes.");
                thread::sleep(Duration::from_millis(1500));
                info!("Exiting after instrument reboot");
                exit(0);
            }
        }
        kic_lib::instrument::CmdLanguage::Tsp => {
            debug!("Instrument language already set to TSP, no change necessary.");
        }
    }

    info!("Instrument configured for usage");

    Ok(())
}

fn add_connection_subcommands(command: impl Into<Command>) -> Command {
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

    command
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

/// Check the connection status of the instrument. This will cause a connect and disconnect
/// from the instrument.
fn check_connection_login_status(conn: &ConnectionInfo) -> Result<(), KicError> {
    // We can check instrument login with Authentication::NoAuth because we aren't trying to log
    // in but simply check whether the instrument is password protected.
    let mut instrument: Box<dyn Instrument> = match get_instrument(conn, Authentication::NoAuth) {
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
fn get_instrument(
    t: &ConnectionInfo,
    auth: Authentication,
) -> Result<Box<dyn Instrument>, KicError> {
    trace!("Connecting to async instrument");
    let instrument: Box<dyn Instrument> = connect_to(t, auth)?;
    info!("Successfully connected to async instrument");
    Ok(instrument)
}

fn clear_output_queue(
    inst: &mut Box<dyn Instrument>,
    max_attempts: usize,
    delay_between_attempts: Duration,
) -> Result<(), KicError> {
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
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
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
    Err(KicError::UnsupportedAction(
        "unable to clear instrument output queue".to_string(),
    ))
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
