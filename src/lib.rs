extern crate lazy_static;
extern crate nom;
extern crate prometheus_exporter;
extern crate serialport;
extern crate aqi;

use lazy_static::lazy_static;
use log::{debug, error, info};
use nom::branch::alt;
use nom::bytes::streaming::tag;
use nom::bytes::streaming::take;
use nom::bytes::streaming::take_until;
use nom::combinator::map;
use nom::number::streaming::be_u16;
use nom::sequence::tuple;
use nom::IResult;
use nom::Needed;
use prometheus_exporter::prometheus::{register_gauge_vec, GaugeVec, Gauge};
use std::error::Error;
use std::num::NonZeroUsize;
use std::thread;
use std::time::{Duration, SystemTime};

lazy_static! {
    pub static ref PARTICLE_CONCENTRATION_STANDARD: GaugeVec = register_gauge_vec!(
        "particle_concentration_standard",
        "concentration (CF=1 standard particle) µg/m³",
        &["particle_size"]
    )
    .unwrap();
    pub static ref PARTICLE_CONCENTRATION_ENVIRONMENT: GaugeVec = register_gauge_vec!(
        "particle_concentration_environment",
        "concentration (under atmospheric environment) µg/m³",
        &["particle_size"]
    )
    .unwrap();
    pub static ref PARTICLE_COUNT: GaugeVec = register_gauge_vec!(
        "particle_count",
        "number of particles with diameter beyond particle_size",
        &["particle_size"]
    )
    .unwrap();
    pub static ref AIR_QUALITY_INDEX: GaugeVec = register_gauge_vec!(
        "air_quality_index",
        "air quality index (aqi) defined by united states environmental protection agency (us epa)",
        &["particle_size"]
    )
    .unwrap();
}

const START_MARKER: &str = "\x42\x4d";
const BAUD_RATE: u32 = 9600;

#[derive(Debug, PartialEq, Eq)]
pub struct PmsData {
    frame_length: u16,
    pm1_cf1: u16,
    pm2_5_cf1: u16,
    pm10_cf1: u16,
    pm1_atmo: u16,
    pm2_5_atmo: u16,
    pm10_atmo: u16,
    pm0_3_count: u16,
    pm0_5_count: u16,
    pm1_0_count: u16,
    pm2_5_count: u16,
    pm5_0_count: u16,
    pm10_0_count: u16,
    reserved: u16,
    checksum: u16,
}

fn parse_data(input: &[u8]) -> IResult<&[u8], PmsData> {
    map(
        tuple((
            tag(START_MARKER),
            be_u16, // frame length
            be_u16, // data 1
            be_u16, // data 2
            be_u16, // ...
            be_u16,
            be_u16,
            be_u16,
            be_u16,
            be_u16,
            be_u16,
            be_u16,
            be_u16,
            be_u16,
            be_u16, // data 13
            be_u16, // checksum
        )),
        |(
            _start_marker,
            frame_length,
            data1,
            data2,
            data3,
            data4,
            data5,
            data6,
            data7,
            data8,
            data9,
            data10,
            data11,
            data12,
            data13,
            checksum,
        )| PmsData {
            frame_length: frame_length,
            pm1_cf1: data1,
            pm2_5_cf1: data2,
            pm10_cf1: data3,
            pm1_atmo: data4,
            pm2_5_atmo: data5,
            pm10_atmo: data6,
            pm0_3_count: data7,
            pm0_5_count: data8,
            pm1_0_count: data9,
            pm2_5_count: data10,
            pm5_0_count: data11,
            pm10_0_count: data12,
            reserved: data13,
            checksum: checksum,
        },
    )(input)
}

pub fn parse(input: &[u8]) -> IResult<&[u8], Option<PmsData>> {
    alt((map(parse_data, Some), map(take(1usize), |_| None)))(input)
}

pub fn default_callback(settle_time: Duration, echo: bool) -> Box<FnMut(PmsData)> {
    let mut start_time = None;
    Box::new(move |data| {
        if start_time == None {
            start_time = Some(SystemTime::now());
            if echo && settle_time > Duration::from_secs(0) {
                println!("Waiting {:?} until data is trusted...", settle_time);
            }
        }
        if let Ok(duration) = start_time.unwrap().elapsed() {
            if duration < settle_time {
                info!(
                    "{:?} until data is trusted, ignoring: {:?}",
                    settle_time - duration,
                    data
                );
                return;
            }
        }
        update_metrics(&data);
        if echo {
            println!("------------------------------------------------");
            println!("Concentration units (standard)");
            println!(
                "pm1.0: {}\tpm2.5: {}\tpm10.0: {}",
                data.pm1_cf1, data.pm2_5_cf1, data.pm10_cf1
            );
            println!();
            println!("Concentration units (environmental)");
            println!(
                "pm1.0: {}\tpm2.5: {}\tpm10.0: {}",
                data.pm1_atmo, data.pm2_5_atmo, data.pm10_atmo
            );
            println!();
            println!("Particle counts");
            println!(
                "pm0.3: {}\tpm0.5: {}\tpm1.0: {}",
                data.pm0_3_count, data.pm0_5_count, data.pm1_0_count
            );
            println!(
                "pm2.5: {}\tpm5.0: {}\tpm10.0: {}",
                data.pm2_5_count, data.pm5_0_count, data.pm10_0_count
            );
            println!("------------------------------------------------");
        }
    })
}

pub fn update_aqi(value: aqi::Result<aqi::AirQuality>, metric: &Gauge) {
    let air_quality = match value {
	Err(e) => {
	    error!("Could not compute AQI: {}", e);
	    return;
	},
	Ok(v) => metric.set(v.aqi().into()),
    };
}

pub fn update_metrics(data: &PmsData) {
    PARTICLE_CONCENTRATION_STANDARD
        .with_label_values(&["1.0"])
        .set(data.pm1_cf1 as f64);
    PARTICLE_CONCENTRATION_STANDARD
        .with_label_values(&["2.5"])
        .set(data.pm2_5_cf1 as f64);
    PARTICLE_CONCENTRATION_STANDARD
        .with_label_values(&["10.0"])
        .set(data.pm10_cf1 as f64);

    PARTICLE_CONCENTRATION_ENVIRONMENT
        .with_label_values(&["1.0"])
        .set(data.pm1_atmo as f64);
    PARTICLE_CONCENTRATION_ENVIRONMENT
        .with_label_values(&["2.5"])
        .set(data.pm2_5_atmo as f64);
    PARTICLE_CONCENTRATION_ENVIRONMENT
        .with_label_values(&["10.0"])
        .set(data.pm10_atmo as f64);

    PARTICLE_COUNT
        .with_label_values(&["0.3"])
        .set(data.pm0_3_count as f64);
    PARTICLE_COUNT
        .with_label_values(&["0.5"])
        .set(data.pm0_5_count as f64);
    PARTICLE_COUNT
        .with_label_values(&["1.0"])
        .set(data.pm1_0_count as f64);
    PARTICLE_COUNT
        .with_label_values(&["2.5"])
        .set(data.pm2_5_count as f64);
    PARTICLE_COUNT
        .with_label_values(&["5.0"])
        .set(data.pm5_0_count as f64);
    PARTICLE_COUNT
        .with_label_values(&["10.0"])
        .set(data.pm10_0_count as f64);

    update_aqi(aqi::pm2_5(data.pm2_5_cf1.into()), &AIR_QUALITY_INDEX.with_label_values(&["2.5"]));
    update_aqi(aqi::pm10(data.pm10_cf1.into()), &AIR_QUALITY_INDEX.with_label_values(&["10.0"]));
}

pub fn read_active<F>(port: &str, mut callback: F) -> Result<(), Box<dyn Error>>
where
    F: FnMut(PmsData),
{
    info!("Reading from {:?}", port);
    let mut port = serialport::new(port, BAUD_RATE).open()?;
    info!("Starting read");

    let mut buf = vec![0u8; 64];
    loop {
        match port.read(&mut buf[..]) {
            Ok(p) => {
                info!("read {} bytes", p);
                let mut input = &buf[..p];
                loop {
                    match parse(input) {
                        Ok((remainder, None)) => {
                            debug!("wait for start marker");
                            input = remainder;
                        }
                        Ok((remainder, Some(data))) => {
                            debug!("got data: {:#?}", data);
                            callback(data);
                            input = remainder;
                        }
                        Err(nom::Err::Incomplete(nom::Needed::Size(s))) => {
                            debug!("need {} more bytes!", s);
                            break;
                        }
                        Err(e) => {
                            error!("{}", e);
                            break;
                        }
                    };
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {
                info!("timed out, sleeping...");
                thread::sleep(Duration::from_millis(1000));
            }
            Err(e) => Err(e)?,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    lazy_static! {
	// To prevent concurrent access to the metrics.
	static ref TEST_MUTEX: Mutex<()> = Mutex::new(());
    }

    const GOLDEN_PACKET: &[u8] = &[
        0x42, 0x4d, 0x00, 0x1c, 0x00, 0x03, 0x00, 0x04, 0x00, 0x07, 0x00, 0x03, 0x00, 0x04, 0x00,
        0x07, 0x02, 0xd0, 0x00, 0xb8, 0x00, 0x19, 0x00, 0x08, 0x00, 0x04, 0x00, 0x02, 0x97, 0x00,
        0x03, 0x0f,
    ];

    use super::*;
    #[test]
    fn test_parse_data() {
        let expected = PmsData {
            frame_length: 28,
            pm1_cf1: 3,
            pm2_5_cf1: 4,
            pm10_cf1: 7,
            pm1_atmo: 3,
            pm2_5_atmo: 4,
            pm10_atmo: 7,
            pm0_3_count: 720,
            pm0_5_count: 184,
            pm1_0_count: 25,
            pm2_5_count: 8,
            pm5_0_count: 4,
            pm10_0_count: 2,
            reserved: 38656,
            checksum: 783,
        };
        assert_eq!(parse(GOLDEN_PACKET), Ok(("".as_bytes(), Some(expected))));
    }

    #[test]
    fn test_partial() {
        assert_eq!(
            parse(START_MARKER.as_bytes()),
            Err(nom::Err::Incomplete(Needed::Size(
                NonZeroUsize::new(2).unwrap()
            )))
        );
    }

    fn testdata() -> PmsData {
    PmsData {
            frame_length: 28,
            pm1_cf1: 3,
            pm2_5_cf1: 4,
            pm10_cf1: 7,
            pm1_atmo: 3,
            pm2_5_atmo: 4,
            pm10_atmo: 7,
            pm0_3_count: 720,
            pm0_5_count: 184,
            pm1_0_count: 25,
            pm2_5_count: 8,
            pm5_0_count: 4,
            pm10_0_count: 2,
            reserved: 38656,
            checksum: 783,
    }
    }

    #[test]
    fn test_metrics() {
	let _ = env_logger::builder().is_test(true).try_init();
	let _guard = TEST_MUTEX.lock().unwrap();
        update_metrics(&testdata());
	assert_eq!(PARTICLE_CONCENTRATION_STANDARD.with_label_values(&["1.0"]).get(),
		   3.0);
	assert_eq!(PARTICLE_CONCENTRATION_STANDARD.with_label_values(&["2.5"]).get(),
		   4.0);
	assert_eq!(PARTICLE_CONCENTRATION_STANDARD.with_label_values(&["10.0"]).get(),
		   7.0);
	assert_eq!(AIR_QUALITY_INDEX.with_label_values(&["2.5"]).get(), 17.0);
	assert_eq!(AIR_QUALITY_INDEX.with_label_values(&["10.0"]).get(), 6.0);
    }

    #[test]
    fn test_parse_invalid() {
        const INVALID: &str = "abc";
        assert_eq!(parse(INVALID.as_bytes()), Ok(("bc".as_bytes(), None)));
    }

    #[test]
    fn test_aqi_valid() {
	let _ = env_logger::builder().is_test(true).try_init();
	let _guard = TEST_MUTEX.lock().unwrap();
	let mut data = testdata();
	data.pm2_5_cf1 = 37;
	update_metrics(&data);
	assert_eq!(AIR_QUALITY_INDEX.with_label_values(&["2.5"]).get(), 105.0);
    }

    #[test]
    fn test_aqi_out_of_range() {
	let _ = env_logger::builder().is_test(true).try_init();
	let _guard = TEST_MUTEX.lock().unwrap();
	update_metrics(&testdata());
	let before = AIR_QUALITY_INDEX.with_label_values(&["10.0"]).get();
	let mut data = testdata();
	data.pm10_cf1 = u16::MAX;
	update_metrics(&data);
	assert_eq!(AIR_QUALITY_INDEX.with_label_values(&["10.0"]).get(), before);
    }
}
