use anyhow::Context;
use async_std::task::sleep;
use jsonrpc_http_server::CloseHandle;
use kic_discover::instrument_discovery::InstrumentDiscovery;
use tsp_toolkit_kic_lib::instrument::info::InstrumentInfo;

use std::collections::HashSet;
use std::str;
use std::sync::mpsc;
use std::thread::JoinHandle;
use std::time::Duration;

use clap::{command, Args, Command, FromArgMatches, Parser, Subcommand};

use kic_discover::DISC_INSTRUMENTS;

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
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
    /// Print JSON-encoded instrument information.
    #[clap(long)]
    json: bool,

    /// The number of seconds to wait for instrument to be discovered.
    /// If not specified, run continuously until the application is signalled.
    #[clap(name = "seconds", long = "timeout", short)]
    timeout_secs: Option<usize>,

    /// This parameter specifies whether we need to wait for a few seconds before closing the json rpc connection.
    /// If not specified, last few instruments discovered may not make it to the discovery pane UI.
    #[clap(name = "exit", long, action)]
    exit: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cmd = command!()
        .propagate_version(true)
        .subcommand_required(true)
        .allow_external_subcommands(true);

    let cmd = SubCli::augment_subcommands(cmd);
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
    let (exit_tx, exit_rx) = mpsc::channel();
    let handler = init_json_rpc(exit_tx).context("Unable to start JSON RPC server")?;
    let is_exit_timer = require_exit_timer(&sub);

    match sub {
        SubCli::Lan(args) => {
            #[allow(clippy::mutable_key_type)]
            let lan_instruments = discover_lan(args).await?;
            println!("Discovered {} Lan instruments", lan_instruments.len());
            for instrument in lan_instruments {
                println!("{}", instrument);
            }
        }
        SubCli::Usb(_) => {
            #[allow(clippy::mutable_key_type)]
            let usb_instruments = discover_usb().await?;
            for instrument in usb_instruments {
                println!("{}", instrument);
            }
        }
        SubCli::All(_args) => {
            #[allow(clippy::mutable_key_type)]
            let usb_instruments = discover_usb().await?;
            for instrument in usb_instruments {
                println!("{}", instrument);
            }

            #[allow(clippy::mutable_key_type)]
            let lan_instruments = discover_lan(_args).await?;
            println!("Discovered {} Lan instruments", lan_instruments.len());
            for instrument in lan_instruments {
                println!("{}", instrument);
            }
        }
    }

    if let Ok(close_handle) = exit_rx.recv() {
        if is_exit_timer {
            sleep(Duration::from_secs(5)).await;
        }
        close_handle.close();
    }

    handler.join().expect("JSON RPC server closed with errors");
    //handler.join().context("JSON RPC server closed with errors")?;

    Ok(())
}

fn require_exit_timer(sub: &SubCli) -> bool {
    if let SubCli::All(_args) = sub {
        if _args.exit {
            return true;
        }
    }
    false
}

fn init_json_rpc(rpc_close: mpsc::Sender<CloseHandle>) -> anyhow::Result<JoinHandle<()>> {
    let handler = std::thread::spawn(move || {
        let mut io = jsonrpc_http_server::jsonrpc_core::IoHandler::default();
        io.add_method("get_instr_list", |_| {
            let mut new_out_str = "".to_owned();

            if let Ok(db) = DISC_INSTRUMENTS.lock() {
                db.iter()
                    .enumerate()
                    .for_each(|(_i, item)| new_out_str = format!("{new_out_str}{item}\n"));
            };

            #[cfg(debug_assertions)]
            eprintln!("newoutstr = {new_out_str}");
            Ok(jsonrpc_http_server::jsonrpc_core::serde_json::Value::String(new_out_str))
        });

        let server = jsonrpc_http_server::ServerBuilder::new(io)
            .cors(jsonrpc_http_server::DomainsValidation::AllowOnly(vec![
                jsonrpc_http_server::AccessControlAllowOrigin::Null,
            ]))
            .start_http(&"127.0.0.1:3030".parse().unwrap())
            .expect("Unable to start RPC server");

        if rpc_close.send(server.close_handle()).is_ok() {
            server.wait();
        }
    });

    Ok(handler)
}

async fn discover_lan(args: DiscoverCmd) -> anyhow::Result<HashSet<InstrumentInfo>> {
    let mut instr_str = "".to_owned();
    let dur = Duration::from_secs(args.timeout_secs.unwrap_or(20) as u64);
    let discover_instance = InstrumentDiscovery::new(dur);
    let instruments = discover_instance.lan_discover().await;

    match &instruments {
        Ok(instrs_set) => {
            for instr in instrs_set {
                instr_str = format!("{}{}\n", instr_str, instr);
            }
        }

        Err(e) => {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string(),
            ))
            .into());
        }
    };

    Ok(instruments.unwrap())
}

async fn discover_usb() -> anyhow::Result<HashSet<InstrumentInfo>> {
    let mut instr_str = "".to_owned();

    let dur = Duration::from_secs(5); //Not used in USB
    let discover_instance = InstrumentDiscovery::new(dur);
    let instruments = discover_instance.usb_discover().await;

    match &instruments {
        Ok(instrs_set) => {
            for instr in instrs_set {
                instr_str = format!("{}{}\n", instr_str, instr);
            }
        }

        Err(e) => {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string(),
            ))
            .into());
        }
    };

    Ok(instruments.unwrap())
}