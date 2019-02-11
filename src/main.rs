#![feature(async_await, await_macro, futures_api)]
#![allow(dead_code, unreachable_code)]

mod client;
mod error;
mod packet;
mod server;
mod tunnel;

use clap::{App, Arg};

use crate::client::PinguinClient;
use crate::server::PinguinServer;

fn main() {
    let matches = App::new("pingu")
        .version("0.1")
        .author("Jenn Wheeler <jwheeler@antiochcollege.edu>")
        .about("Passes incoming TCP packets through ICMP packets to a 'proxy' server it's paired with")
        .arg(Arg::with_name("listen_port")
             .help("Local port on which to listen for packets to forward")
             .short("p")
             .long("listen_port")
             .required_unless("server")
             .value_name("LISTENPORT"))
        .arg(Arg::with_name("client")
             .short("c")
             .required_unless("server")
             .help("Flag to start a client session"))
        .arg(Arg::with_name("server")
             .short("s")
             .required_unless("client")
             .help("Flag to start a server session"))
        .arg(Arg::with_name("remote_addr")
             .help("Specifies the address for the proxy for a client instance or the address to forward to in the case of a proxy server instance")
             .required(true)
             .value_name("REMOTEADDRESS"))
        .get_matches();

    let is_server = matches.is_present("server");

    let listen_port = matches.value_of("listen_port").unwrap();
    let remote_addr = matches.value_of("remote_addr").unwrap();

    let localhost = "127.0.0.1";
    if is_server {
        let client = match PinguinServer::new(localhost, listen_port.parse().unwrap(), remote_addr) {
            Ok(tunnel) => tunnel,
            Err(e) => {
                eprintln!("{}", e);
                return ()
            },
        };

        match client.run() {
            Ok(_) => println!("Serving on port {}...", listen_port),
            Err(e) => {
                eprintln!("{}", e);
                return ()
            },
        };
    } else {
        let client = match PinguinClient::new(localhost, listen_port.parse().unwrap(), remote_addr) {
            Ok(tunnel) => tunnel,
            Err(e) => {
                eprintln!("{}", e);
                return ()
            },
        };

        match client.run() {
            Ok(_) => println!("Serving on port {}...", listen_port),
            Err(e) => {
                eprintln!("{}", e);
                return ()
            },
        };
    }
}
