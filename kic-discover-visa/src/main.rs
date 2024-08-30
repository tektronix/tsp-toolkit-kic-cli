use anyhow::Context;
use async_std::{path::PathBuf, task::sleep};
use jsonrpsee::{
    server::{Server, ServerHandle},
    RpcModule,
};
use kic_discover_visa::instrument_discovery::InstrumentDiscovery;
use tracing::{error, info, instrument, level_filters::LevelFilter, trace, warn};
use tracing_subscriber::{layer::SubscriberExt, Layer, Registry};
use tsp_toolkit_kic_lib::instrument::info::InstrumentInfo;

use std::fs::OpenOptions;
use std::str;
use std::time::Duration;
use std::{
    collections::HashSet,
    net::{SocketAddr, TcpStream},
    sync::Mutex,
};

use clap::{command, Args, Command, FromArgMatches, Parser, Subcommand};

use kic_discover_visa::DISC_INSTRUMENTS;

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Enable logging to stderr. Can be used in conjunction with `--log-file` and `--verbose`.
    #[arg(global = true, short = 'v', long = "verbose")]
    verbose: bool,

    /// Log to the given log file path. Can be used in conjunction with `--log-socket` and `--verbose`.
    #[arg(name = "log-file", global = true, short = 'l', long = "log-file")]
    log_file: Option<PathBuf>,

    /// Log to the given socket (in IPv4 or IPv6 format with port number). Can be used in conjunction with `--log-file` and `--verbose`.
    #[arg(name = "log-socket", global = true, short = 's', long = "log-socket")]
    log_socket: Option<SocketAddr>,

    #[command(subcommand)]
    conn: SubCli,
}

#[derive(Debug, Subcommand)]
enum SubCli {
    /// Look for all devices connected on LAN
    Lan(DiscoverCmd),
    /// Look for all devices connected on USB
    Usb(DiscoverCmd),
    /// Look for all devices on all interface types.
    All(DiscoverCmd),
}

#[derive(Debug, Args, Clone, PartialEq)]
pub(crate) struct DiscoverCmd {
    /// Enable logging to stderr. Can be used in conjunction with `--log-file` and `--verbose`.
    #[arg(from_global)]
    verbose: bool,

    /// Log to the given log file path. Can be used in conjunction with `--log-socket` and `--verbose`.
    #[clap(name = "log-file", from_global)]
    log_file: Option<PathBuf>,

    /// Log to the given socket (in IPv4 or IPv6 format with port number). Can be used in conjunction with `--log-file` and `--verbose`.
    #[clap(name = "log-socket", from_global)]
    log_socket: Option<SocketAddr>,

    /// Print JSON-encoded instrument information.
    #[clap(long)]
    json: bool,

    /// The number of seconds to wait for instrument to be discovered.
    /// If not specified, run continuously until the application is signalled.
    #[clap(name = "seconds", long = "timeout", short = 't')]
    timeout_secs: Option<usize>,

    /// This parameter specifies whether we need to wait for a few seconds before closing the json rpc connection.
    /// If not specified, last few instruments discovered may not make it to the discovery pane UI.
    #[clap(name = "exit", long, action)]
    exit: bool,
}

fn start_logger(
    verbose: &bool,
    log_file: &Option<PathBuf>,
    log_socket: &Option<SocketAddr>,
) -> anyhow::Result<()> {
    match (verbose, log_file, log_socket) {
        (true, Some(l), Some(s)) => {
            let err = tracing_subscriber::fmt::layer()
                .with_ansi(true)
                .with_writer(std::io::stderr)
                .with_filter(LevelFilter::INFO);

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
                .with(LevelFilter::TRACE)
                .with(err)
                .with(log)
                .with(sock);

            tracing::subscriber::set_global_default(logger)?;
        }
        (true, Some(l), None) => {
            let err = tracing_subscriber::fmt::layer()
                .with_ansi(true)
                .with_writer(std::io::stderr)
                .with_filter(LevelFilter::INFO);

            let log = OpenOptions::new().append(true).create(true).open(l)?;

            let log = tracing_subscriber::fmt::layer()
                .with_writer(log)
                .fmt_fields(tracing_subscriber::fmt::format::DefaultFields::new())
                .with_ansi(false);

            let logger = Registry::default()
                .with(LevelFilter::TRACE)
                .with(err)
                .with(log);

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

            let logger = Registry::default()
                .with(LevelFilter::TRACE)
                .with(log)
                .with(sock);

            tracing::subscriber::set_global_default(logger)?;
        }
        (false, Some(l), None) => {
            let log = OpenOptions::new().append(true).create(true).open(l)?;

            let log = tracing_subscriber::fmt::layer()
                .with_writer(log)
                .with_ansi(false);

            let logger = Registry::default().with(LevelFilter::TRACE).with(log);

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

            let logger = Registry::default()
                .with(LevelFilter::TRACE)
                .with(err)
                .with(sock);

            tracing::subscriber::set_global_default(logger)?;
        }
        (true, None, None) => {
            let err = tracing_subscriber::fmt::layer()
                .with_ansi(true)
                .with_writer(std::io::stderr);

            let logger = Registry::default().with(LevelFilter::TRACE).with(err);

            tracing::subscriber::set_global_default(logger)?;
        }
        (false, None, Some(s)) => {
            let sock = TcpStream::connect(s)?;
            let sock = tracing_subscriber::fmt::layer()
                .with_writer(Mutex::new(sock))
                .fmt_fields(tracing_subscriber::fmt::format::DefaultFields::new())
                .json();

            let logger = Registry::default().with(LevelFilter::TRACE).with(sock);

            tracing::subscriber::set_global_default(logger)?;
        }
        (false, None, None) => {}
    }

    info!("Application started");
    trace!(
        "Application starting with the following args: {:?}",
        std::env::args()
    );
    Ok(())
}

#[tokio::main]
#[instrument]
async fn main() -> anyhow::Result<()> {
    let cmd = command!()
        .propagate_version(true)
        .subcommand_required(true)
        .allow_external_subcommands(true);

    let cmd = Cli::augment_args(cmd);
    let cmd = cmd.subcommand(Command::new("print-description").hide(true));

    let matches = cmd.clone().get_matches();

    if let Some(("print-description", _)) = matches.subcommand() {
        println!("{}", cmd.get_about().unwrap_or_default());
        return Ok(());
    }

    let sub = SubCli::from_arg_matches(&matches)
        .map_err(|err| err.exit())
        .unwrap();

    eprintln!("Keithley Instruments Discovery");
    let close_handle = init_rpc()
        .await
        .context("Unable to start JSON RPC server")?;

    let is_exit_timer = require_exit_timer(&sub);

    match sub {
        SubCli::Lan(args) => {
            start_logger(&args.verbose, &args.log_file, &args.log_socket)?;
            info!("Discovering LAN instruments");
            #[allow(clippy::mutable_key_type)]
            let lan_instruments = match discover_lan(args).await {
                Ok(i) => i,
                Err(e) => {
                    error!("Error in LAN discovery: {e}");
                    return Err(e);
                }
            };
            info!("LAN Discovery complete");
            trace!("Discovered {} LAN instruments", lan_instruments.len());
            println!("Discovered {} LAN instruments", lan_instruments.len());
            trace!("Discovered instruments: {lan_instruments:?}");
            for instrument in lan_instruments {
                println!("{instrument}");
            }
        }
        SubCli::Usb(args) => {
            start_logger(&args.verbose, &args.log_file, &args.log_socket)?;
            info!("Discovering USB instruments");
            #[allow(clippy::mutable_key_type)]
            let usb_instruments = match discover_usb().await {
                Ok(i) => i,
                Err(e) => {
                    error!("Error in USB discovery: {e}");
                    return Err(e);
                }
            };
            info!("USB Discovery complete");
            trace!("Discovered {} USB instruments", usb_instruments.len());
            trace!("Discovered instruments: {usb_instruments:?}");
            for instrument in usb_instruments {
                println!("{instrument}");
            }
        }
        SubCli::All(args) => {
            start_logger(&args.verbose, &args.log_file, &args.log_socket)?;
            info!("Discovering USB instruments");
            #[allow(clippy::mutable_key_type)]
            let usb_instruments = match discover_usb().await {
                Ok(i) => i,
                Err(e) => {
                    error!("Error in USB discovery: {e}");
                    return Err(e);
                }
            };
            info!("USB Discovery complete");
            trace!("Discovered {} USB instruments", usb_instruments.len());
            println!("Discovered {} USB instruments", usb_instruments.len());
            trace!("Discovered USB instruments: {usb_instruments:?}");
            for instrument in usb_instruments {
                println!("{instrument}");
            }

            info!("Discovering LAN instruments");
            #[allow(clippy::mutable_key_type)]
            let lan_instruments = match discover_lan(args).await {
                Ok(i) => i,
                Err(e) => {
                    error!("Error in LAN discovery: {e}");
                    return Err(e);
                }
            };
            info!("LAN Discovery complete");
            trace!("Discovered {} LAN instruments", lan_instruments.len());
            println!("Discovered {} LAN instruments", lan_instruments.len());
            trace!("Discovered LAN instruments: {lan_instruments:?}");
            for instrument in lan_instruments {
                println!("{instrument}");
            }
        }
    }

    if is_exit_timer {
        sleep(Duration::from_secs(5)).await;
    }
    close_handle.stop()?;

    info!("Discovery complete");

    Ok(())
}

const fn require_exit_timer(sub: &SubCli) -> bool {
    if let SubCli::All(args) = sub {
        if args.exit {
            return true;
        }
    }
    false
}

async fn init_rpc() -> anyhow::Result<ServerHandle> {
    let server = Server::builder().build("127.0.0.1:3030").await?;

    let mut module = RpcModule::new(());
    module.register_method("get_instr_list", |_, ()| {
        let mut new_out_str = String::new();

        if let Ok(db) = DISC_INSTRUMENTS.lock() {
            db.iter()
                .for_each(|item| new_out_str = format!("{new_out_str}{item}\n"));
        };

        #[cfg(debug_assertions)]
        eprintln!("newoutstr = {new_out_str}");

        serde_json::Value::String(new_out_str)
    })?;

    let handle = server.start(module);

    tokio::spawn(handle.clone().stopped());

    Ok(handle)
}

async fn discover_lan(args: DiscoverCmd) -> anyhow::Result<HashSet<InstrumentInfo>> {
    let dur = Duration::from_secs(args.timeout_secs.unwrap_or(20) as u64);
    let discover_instance = InstrumentDiscovery::new(dur);
    let instruments = discover_instance.lan_discover().await?;

    Ok(instruments)
}

async fn discover_usb() -> anyhow::Result<HashSet<InstrumentInfo>> {
    let dur = Duration::from_secs(5); //Not used in USB
    let discover_instance = InstrumentDiscovery::new(dur);
    let instruments = discover_instance.usb_discover().await?;

    Ok(instruments)
}
