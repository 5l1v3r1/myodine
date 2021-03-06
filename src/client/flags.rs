use std::str::FromStr;
use std::time::Duration;

use clap::{App, Arg};

use myodine::dns_proto::Domain;

#[derive(Clone)]
pub struct Flags {
    pub addr: String,
    pub host: Domain,
    pub concurrency: usize,
    pub query_window: u16,
    pub response_window: u16,
    pub password: String,
    pub remote_host: Domain,
    pub remote_port: u16,
    pub listen_port: u16,
    pub query_min_time: Duration,
    pub query_max_time: Duration,
    pub query_mtu: Option<u16>,
    pub response_mtu: Option<u16>
}

impl Flags {
    pub fn parse() -> Result<Flags, String> {
        let matches = App::new("myodine-client")
            .arg(Arg::with_name("concurrency")
                .short("c")
                .long("concurrency")
                .value_name("NUM")
                .help("Set the maximum number of concurrent requests")
                .takes_value(true))
            .arg(Arg::with_name("query-window")
                .short("q")
                .long("query-window")
                .value_name("NUM")
                .help("Set the window size for outgoing data")
                .takes_value(true))
            .arg(Arg::with_name("response-window")
                .short("w")
                .long("response-window")
                .value_name("NUM")
                .help("Set the window size for incoming data")
                .takes_value(true))
            .arg(Arg::with_name("remote-host")
                .short("r")
                .long("remote-host")
                .value_name("ADDR")
                .help("Set the remote address to proxy to")
                .takes_value(true))
            .arg(Arg::with_name("remote-port")
                .short("n")
                .long("remote-port")
                .value_name("PORT")
                .help("Set the remote port to proxy to")
                .takes_value(true))
            .arg(Arg::with_name("listen-port")
                .short("l")
                .long("listen-port")
                .value_name("PORT")
                .help("Set the local port to listen on")
                .takes_value(true))
            .arg(Arg::with_name("password")
                .short("p")
                .long("password")
                .value_name("VALUE")
                .help("Set the server password")
                .takes_value(true))
            .arg(Arg::with_name("query-max-time")
                .long("query-max-time")
                .value_name("INT")
                .help("Set the query timeout in milliseconds")
                .takes_value(true))
            .arg(Arg::with_name("query-min-time")
                .long("query-min-time")
                .value_name("INT")
                .help("Set the minimum query delay in milliseconds")
                .takes_value(true))
            .arg(Arg::with_name("query-mtu")
                .long("query-mtu")
                .value_name("INT")
                .help("Set the query MTU to an explicit value")
                .takes_value(true))
            .arg(Arg::with_name("response-mtu")
                .long("response-mtu")
                .value_name("INT")
                .help("Set the response MTU to an explicit value")
                .takes_value(true))
            .arg(Arg::with_name("addr")
                .help("Set the address of the proxy")
                .required(true)
                .index(1))
            .arg(Arg::with_name("host")
                .help("Set the root domain name of the proxy")
                .required(true)
                .index(2))
            .get_matches();

        macro_rules! parse_arg {
            ( $name:expr, $default:expr ) => {
                matches.value_of($name).unwrap_or($default).parse()
                    .map_err(|e| format!("bad {} argument: {}", $name, e))
            }
        }

        let min_time: u64 = parse_arg!("query-min-time", "50")?;
        let max_time: u64 = parse_arg!("query-max-time", "5000")?;
        Ok(Flags{
            addr: matches.value_of("addr").unwrap_or("localhost:53").to_owned(),
            host: parse_arg!("host", "")?,
            concurrency: parse_arg!("concurrency", "2")?,
            query_window: parse_arg!("query-window", "4")?,
            response_window: parse_arg!("response-window", "4")?,
            password: matches.value_of("password").unwrap_or("").to_owned(),
            remote_host: parse_arg!("remote-host", "127.0.0.1")?,
            remote_port: parse_arg!("remote-port", "22")?,
            listen_port: parse_arg!("listen-port", "2222")?,
            query_min_time: Duration::from_millis(min_time),
            query_max_time: Duration::from_millis(max_time),
            query_mtu: parse_optional(matches.value_of("query-mtu"))?,
            response_mtu: parse_optional(matches.value_of("response-mtu"))?
        })
    }
}

fn parse_optional<T: FromStr>(x: Option<&str>) -> Result<Option<T>, String> {
    match x {
        Some(s) => s.parse().map_err(|_| format!("bad argument: {}", s)).map(Some),
        None => Ok(None),
    }
}
