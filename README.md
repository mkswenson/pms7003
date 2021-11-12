# PMS7003 - Particle Sensor CLI

Prometheus exporter for a PMS7003 particle sensor.

## Example

```
$ ./pms7003 --prometheus-bind-addr 127.0.0.1:9954 /dev/ttyUSB0
Waiting 30s until data is trusted...
------------------------------------------------
Concentration units (standard)
pm1.0: 3	pm2.5: 4	pm10.0: 6

Concentration units (environmental)
pm1.0: 3	pm2.5: 4	pm10.0: 6

Particle counts
pm0.3: 723	pm0.5: 207	pm1.0: 20
pm2.5: 8	pm5.0: 0	pm10.0: 0
------------------------------------------------
[...]
```

## Usage

```
$ pms7003 --help
pms7003-cli 0.1.0
Command line tool to pull data from pms7003

USAGE:
    pms7003 [FLAGS] [OPTIONS] <SERIAL_PORT>

FLAGS:
    -h, --help       Prints help information
        --list       List available serial ports
    -q, --quiet
    -V, --version    Prints version information
    -v, --verbose

OPTIONS:
        --prometheus-bind-addr <prometheus-bind-addr>    Example: 127.0.0.1:9954
        --settle-time-seconds <settle-time-seconds>       [default: 30.0]

ARGS:
    <SERIAL_PORT>
```
