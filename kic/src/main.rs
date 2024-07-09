#![feature(lint_reasons, rustdoc_missing_doc_code_examples)]
#![doc(html_logo_url = "../../../ki-comms_doc_icon.png")]

//! The `kic` executable is a command-line tool that will allow a user to interact with
//! an instrument over all the media provided by the [`tsp-instrument`] crate.
//! This is done via an easy to understand command-line interface and, when
//! interactively connected to an instrument, with a REPL

mod error;
mod process;
use crate::error::KicError;
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
    io::{stdin, Read, Write},
    net::{IpAddr, SocketAddr, TcpStream},
    path::PathBuf,
    process::exit,
    sync::Arc,
    thread,
    time::Duration,
};

use tsp_toolkit_kic_lib::{
    instrument::{self, Instrument},
    interface::async_stream::AsyncStream,
    usbtmc::{self, UsbtmcAddr},
    Interface,
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
    let command: Command = command.into();

    let mut lan = Command::new("lan")
        .about("Perform the given action over a LAN connection")
        .arg(
            Arg::new("port")
                .help("The port on which to connect to the instrument")
                .short('p')
                .long("port")
                .value_parser(value_parser!(u16))
                .default_value("5025"),
        )
        .arg(
            Arg::new("ip_addr")
                .help("The IP address of the instrument to connect to")
                .required(true)
                .value_parser(value_parser!(IpAddr)),
        );

    let mut usb = Command::new("usb")
        .about("Perform the given action over a USBTMC connection")
        .arg(
            Arg::new("addr")
                .help("The instrument address in the form of, for example, `05e6:2461:012345`, where the first part is the vendor id, the second part is the product id, and the third part is the serial number.")
                .required(true)
                .value_parser(value_parser!(UsbtmcAddr)),
        );

    for arg in additional_args {
        lan = lan.arg(arg.clone());
        usb = usb.arg(arg.clone());
    }

    command.subcommand(lan).subcommand(usb)
}

#[must_use]
fn cmds() -> Command {
    command!()
        .propagate_version(true)
        .subcommand_required(true)
        .allow_external_subcommands(true)
        .arg(
            Arg::new("no-color")
                .short('c')
                .long("no-color")
                .help("Turn off ANSI color and formatting codes")
                .action(ArgAction::SetTrue),
        )
        // This is mostly for subcommands, but is left here as an example.
        // We want to find all `kic-*` applications and run it with this option in order to add the sub command here.
        .subcommand(Command::new("print-description").hide(true))
        .subcommand({
            let cmd = Command::new("connect")
                .about("Connect to an instrument over one of the provided interfaces");
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
            let cmd = Command::new("upgrade")
                .about("Upgrade the firmware of an instrument or module.");

            add_connection_subcommands(cmd, [
                    Arg::new("file")
                        .help("The file path of the firmware image.")
                        .required(true)
                        .value_parser(PathBufValueParser::new()),

                    Arg::new("slot")
                        .short('s')
                        .long("slot")
                        .help("[VersaTest only] Update a module in given slot number instead of the VersaTest mainframe")
                        .required(false)
                        .value_parser(value_parser!(u16).range(1..3)),
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

    match matches.subcommand() {
        Some(("print-description", _)) => {
            println!("{}", clap::crate_description!());
            return Ok(());
        }
        Some(("connect", sub_matches)) => {
            return connect(sub_matches);
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
        Some(("info", sub_matches)) => {
            return info(sub_matches);
        }
        Some((ext, sub_matches)) => {
            if let Some((path, ..)) = external_cmd_lut.get(ext) {
                let args: Vec<_> = sub_matches
                    .get_many::<String>("options")
                    .into_iter()
                    .flatten()
                    .cloned()
                    .collect();

                Process::new(path.clone(), args)
                    .exec_replace()
                    .context(format!("{ext} subcommand should launch in a child process"))?;
                //Process::exec_replace() only returns to this function if there was a error.
            } else {
                let err = clap::Error::new(clap::error::ErrorKind::UnknownArgument);
                println!("{err}");
                cmd.print_help()?;
                return Err(err.into());
            }
        }
        _ => unreachable!(),
    }

    Ok(())
}

enum ConnectionType {
    Lan(SocketAddr),
    Usb(UsbtmcAddr),
}

impl ConnectionType {
    fn try_from_arg_matches(args: &ArgMatches) -> anyhow::Result<Self> {
        match args.subcommand() {
            Some(("lan", sub_matches)) => {
                let ip_addr: IpAddr =
                    *sub_matches
                        .get_one::<IpAddr>("ip_addr")
                        .ok_or(KicError::ArgParseError {
                            details: "no IP address provided".to_string(),
                        })?;

                let port: u16 = *sub_matches.get_one::<u16>("port").unwrap_or(&5025);

                let socket_addr = SocketAddr::new(ip_addr, port);
                Ok(Self::Lan(socket_addr))
            }
            Some(("usb", sub_matches)) => {
                let addr: String = sub_matches
                    .get_one::<String>("addr")
                    .ok_or(KicError::ArgParseError {
                        details: "no USB address provided".to_string(),
                    })?
                    .clone();
                let usb_addr: UsbtmcAddr = addr.parse()?;

                Ok(Self::Usb(usb_addr))
            }
            Some((ct, _sub_matches)) => {
                println!();
                Err(KicError::ArgParseError {
                    details: format!("unknown connection type: \"{ct}\""),
                }
                .into())
            }
            None => unreachable!("connection type not specified"),
        }
    }
}

fn connect_sync_instrument(t: ConnectionType) -> anyhow::Result<Box<dyn Instrument>> {
    let interface: Box<dyn Interface> = match t {
        ConnectionType::Lan(addr) => Box::new(TcpStream::connect(addr)?),
        ConnectionType::Usb(addr) => Box::new(usbtmc::Stream::try_from(addr)?),
    };

    let instrument: Box<dyn Instrument> = interface.try_into()?;
    Ok(instrument)
}

fn connect_async_instrument(t: ConnectionType) -> anyhow::Result<Box<dyn Instrument>> {
    let interface: Box<dyn Interface> = match t {
        ConnectionType::Lan(addr) => Box::new(AsyncStream::try_from(Arc::new(TcpStream::connect(
            addr,
        )?)
            as Arc<dyn Interface + Send + Sync>)?),
        ConnectionType::Usb(addr) => Box::new(AsyncStream::try_from(Arc::new(
            usbtmc::Stream::try_from(addr)?,
        )
            as Arc<dyn Interface + Send + Sync>)?),
    };

    let instrument: Box<dyn Instrument> = interface.try_into()?;
    Ok(instrument)
}

fn get_instrument_access(inst: &mut Box<dyn Instrument>) -> anyhow::Result<()> {
    match inst.as_mut().check_login()? {
        instrument::State::Needed => inst.as_mut().login()?,
        instrument::State::LogoutNeeded => return Err(KicError::InstrumentLogoutRequired.into()),
        instrument::State::NotNeeded => {}
    };
    match inst.as_mut().get_language()? {
        instrument::CmdLanguage::Scpi => {
            eprintln!("Instrument command-set is not set to TSP. Would you like to change the command-set to TSP and reboot? (Y/n)");

            let mut buf = String::new();
            stdin().read_line(&mut buf)?;
            let buf = buf.trim();
            if buf.is_empty() || buf.contains(['Y', 'y']) {
                inst.as_mut()
                    .change_language(instrument::CmdLanguage::Tsp)?;
                inst.write_all(b"ki.reboot()\n")?;
                eprintln!("Instrument rebooting, please reconnect after reboot completes.");
                thread::sleep(Duration::from_millis(1500));
                exit(0);
            }
        }
        instrument::CmdLanguage::Tsp => {}
    }

    Ok(())
}
fn connect(args: &ArgMatches) -> anyhow::Result<()> {
    eprintln!(
        "\nKeithley TSP Shell\nType {} for more commands.\n",
        ".help".bold()
    );
    let mut instrument: Box<dyn Instrument> =
        connect_async_instrument(ConnectionType::try_from_arg_matches(args)?)?;

    get_instrument_access(&mut instrument)?;

    eprintln!("{}", instrument.info()?);
    let mut repl = repl::Repl::new(instrument);
    Ok(repl.start()?)
}

fn upgrade(args: &ArgMatches) -> anyhow::Result<()> {
    eprintln!("\nKeithley TSP Shell\n");
    let lan = ConnectionType::try_from_arg_matches(args)?;
    let Some((_, args)) = args.subcommand() else {
        unreachable!("arguments didn't exist")
    };
    let mut instrument: Box<dyn Instrument> = connect_sync_instrument(lan)?;
    get_instrument_access(&mut instrument)?;
    eprintln!("{}", instrument.info()?);
    let slot: Option<u16> = args.get_one::<u16>("slot").copied();
    let Some(file) = args.get_one::<PathBuf>("file").cloned() else {
        return Err(KicError::ArgParseError {
            details: "firmware file path was not provided".to_string(),
        }
        .into());
    };

    let mut image: Vec<u8> = Vec::new();

    let mut file = std::fs::File::open(file)?;
    file.read_to_end(&mut image)?;

    eprintln!("Flashing instrument firmware. Please do NOT power off or disconnect.");
    instrument.flash_firmware(&image, slot)?;
    eprintln!("Flashing instrument firmware completed. Instrument will restart.");
    Ok(())
}

fn script(args: &ArgMatches) -> anyhow::Result<()> {
    eprintln!("\nKeithley TSP Shell\n");
    let mut instrument: Box<dyn Instrument> =
        connect_async_instrument(ConnectionType::try_from_arg_matches(args)?)?;
    get_instrument_access(&mut instrument)?;
    eprintln!("{}", instrument.info()?);
    let Some((_, args)) = args.subcommand() else {
        unreachable!("arguments didn't exist")
    };

    let run: bool = *args.get_one::<bool>("run").unwrap_or(&true);
    let save: bool = *args.get_one::<bool>("save").unwrap_or(&false);

    let Some(path) = args.get_one::<PathBuf>("file") as Option<&PathBuf> else {
        return Err(KicError::ArgParseError {
            details: "firmware file path was not provided".to_string(),
        }
        .into());
    };

    let stem = path
        .file_stem()
        .context(KicError::ArgParseError {
            details: "unable to get file stem".to_string(),
        })?
        .to_string_lossy();

    let re = Regex::new(r"[^A-Za-z\d_]");

    match re {
        Ok(re_res) => {
            let result = re_res.replace_all(&stem, "_");

            let script_name = format!("kic_{result}");

            let mut script_content: Vec<u8> = Vec::new();

            let mut file = std::fs::File::open(path)?;
            file.read_to_end(&mut script_content)?;

            eprintln!("Loading script to instrument.");
            instrument.write_script(script_name.as_bytes(), &script_content, save, run)?;
            eprintln!("Script loading completed.");
        }
        Err(err_msg) => {
            unreachable!("Issue with regex creation: {}", err_msg.to_string());
        }
    }

    Ok(())
}

fn info(args: &ArgMatches) -> anyhow::Result<()> {
    let mut instrument: Box<dyn Instrument> =
        connect_async_instrument(ConnectionType::try_from_arg_matches(args)?)?;

    let Some((_, args)) = args.subcommand() else {
        unreachable!("arguments didn't exist")
    };

    let json: bool = *args.get_one::<bool>("json").unwrap_or(&true);

    let info = instrument.info()?;
    let info: String = if json {
        serde_json::to_string(&info)?
    } else {
        info.to_string()
    };

    println!("{info}");

    Ok(())
}

fn terminate(args: &ArgMatches) -> anyhow::Result<()> {
    eprintln!("\nKeithley TSP Shell\n");
    let connection = ConnectionType::try_from_arg_matches(args)?;
    match connection {
        ConnectionType::Lan(socket) => {
            let mut connection = TcpStream::connect(socket)?;
            connection.write_all(b"ABORT\n")?;
        }
        ConnectionType::Usb(_) => {}
    }

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
            if path.is_file() && filename.contains("kic-") {
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
