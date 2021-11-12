extern crate log;
extern crate nom;
extern crate pms7003;
extern crate prometheus_exporter;
extern crate serialport;
extern crate structopt;

use log::{error, info};
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

    #[structopt(long, help = "Example: 127.0.0.1:9954")]
    prometheus_bind_addr: Option<String>,

    #[structopt(subcommand)]
    cmd: Command,
}

#[derive(Debug, StructOpt)]
enum Command {
    /// List available serial ports.
    List,
    /// Read a measurement from the pms7003.
    Read {
        #[structopt(name = "PORT")]
        port: String,

        #[structopt(long, default_value="30.0")]
        settle_time_seconds: f64,

        #[structopt(short, long)]
        quiet: bool,
    },
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
    info!("{:#?}", opt);

    let exporter = if let Some(bind_addr) = opt.prometheus_bind_addr {
        let binding = bind_addr.parse()?;
        Some(prometheus_exporter::start(binding)?)
    } else {
        None
    };

    // thread::sleep(Duration::from_secs(5));
    // pms7003::PARTICLE_CONCENTRATION
    //     .with_label_values(&["2.5"])
    //     .set(10.0);
    // thread::sleep(Duration::from_secs(1000));

    match opt.cmd {
        Command::List => list()?,
        Command::Read { port, settle_time_seconds, quiet } => {
            let callback = pms7003::default_callback(
                Duration::from_millis((settle_time_seconds * 1000.0) as u64), /*echo=*/!quiet);
            pms7003::read_active(&port, callback)?
        },
    }

    Ok(())
}
