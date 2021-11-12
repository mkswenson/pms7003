extern crate log;
extern crate nom;
extern crate pms7003;
extern crate prometheus_exporter;
extern crate serialport;
extern crate structopt;

use log::{debug, error, info};
use std::error::Error;
use std::io::BufReader;
use std::io::Read;
use std::thread;
use std::time::Duration;

use env_logger::Env;
use structopt::StructOpt;

const BAUD_RATE: u32 = 9600;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "pms7003-cli",
    about = "Command line tool to pull data from pms7003"
)]
struct Cli {
    #[structopt(short, long)]
    verbose: bool,

    #[structopt(long, help = "List available serial ports")]
    list: bool,

    #[structopt(long, help = "Example: 127.0.0.1:9954")]
    prometheus_bind_addr: Option<String>,

    #[structopt(name = "SERIAL_PORT")]
    port: String,

    #[structopt(long, default_value = "30.0")]
    settle_time_seconds: f64,

    #[structopt(short, long)]
    quiet: bool,
}

fn list() -> Result<(), Box<dyn Error>> {
    let ports = serialport::available_ports()?;
    info!("{:#?}", ports);
    println!("Available ports:");
    for port in ports {
        println!("{}", port.port_name);
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let opt = Cli::from_args();
    let loglevel = if opt.verbose { "info" } else { "warn" };
    env_logger::Builder::from_env(Env::default().default_filter_or(loglevel)).init();
    debug!("{:#?}", opt);

    if opt.list {
        list()?;
        return Ok(());
    }

    let exporter = if let Some(bind_addr) = opt.prometheus_bind_addr {
        let binding = bind_addr.parse()?;
        Some(prometheus_exporter::start(binding)?)
    } else {
        None
    };

    let callback = pms7003::default_callback(
        Duration::from_millis((opt.settle_time_seconds * 1000.0) as u64),
        /*echo=*/ !opt.quiet,
    );
    pms7003::read_active(&opt.port, callback)?;

    Ok(())
}
