use clap::{arg, command, Args, Command, FromArgMatches, Parser, Subcommand};
use kic_debug::debugger::Debugger;
use std::ffi::OsString;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, TcpStream};
use std::sync::Arc;
use tsp_instrument::instrument::Instrument;
use tsp_instrument::interface::async_stream::AsyncStream;
use tsp_instrument::Interface;
#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    conn: SubCli,
}

#[derive(Debug, Subcommand)]
enum SubCli {
    /// Perform the given action over a LAN connection.
    Lan(LanConnectArgs),

    /// Perform the given action over a USBTMC connection.
    Usb(UsbConnectArgs),
}

#[derive(Debug, Args)]
struct LanConnectArgs {
    ///The port on which to connect to the instrument.
    #[arg(long, short = 'p', name = "lan_port")]
    port: Option<u16>,

    /// The IP address of the instrument to connect to.
    ip_addr: OsString,
}

#[derive(Debug, Args)]
struct UsbConnectArgs {
    /// The instrument address in the form of, for example, `05e6:2461:3`, where the
    /// first part is the vendor id, the second part is the product id, and the third
    /// part is the USB address on the bus.
    addr: OsString,
}

fn main() -> anyhow::Result<()> {
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

    eprintln!("Keithley Instruments Script Debugger");

    let mut debugger = match sub {
        SubCli::Lan(args) => {
            let addr: Ipv4Addr = args.ip_addr.to_str().unwrap().parse().unwrap();
            let port = args.port.unwrap_or(5025);
            let socket_addr = SocketAddr::V4(SocketAddrV4::new(addr, port));
            let lan: Arc<dyn Interface + Send + Sync> = Arc::new(TcpStream::connect(socket_addr)?);
            let lan: Box<dyn Interface> = Box::new(AsyncStream::try_from(lan)?);
            let instrument: Box<dyn Instrument> = lan.try_into()?;
            Debugger::new(instrument)
        }
        SubCli::Usb(_args) => todo!(),
    };

    Ok(debugger.start()?)
}
