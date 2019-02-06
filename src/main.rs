#![allow(dead_code)]

mod packet;
mod tunnel;

use clap::{App, Arg};

use crate::tunnel::Tunnel;

fn main() {
    let matches = App::new("icmptunnel")
        .version("0.1")
        .author("Jenn Wheeler <jwheeler@antiochcollege.edu>")
        .about("Passes incoming TCP packets through ICMP packets to a 'proxy' server it's paired with")
        .arg(Arg::with_name("listen_port")
             .help("Local port on which to listen for packets to forward")
             .short("p")
             .long("listen_port")
             .required_unless("server")
             .value_name("LISTENPORT"))
        .arg(Arg::with_name("remote_port")
             .help("Specifies the port for the proxy for a client instance or the port to forward to in the case of a proxy server instance")
             .short("r")
             .long("remote_port")
             .required_unless("client")
             .value_name("REMOTEPORT"))
        .arg(Arg::with_name("remote_addr")
             .help("Specifies the address for the proxy for a client instance or the address to forward to in the case of a proxy server instance")
             .short("a")
             .long("remote_address")
             .required(true)
             .value_name("REMOTEADDRESS"))
        .arg(Arg::with_name("client")
             .short("c")
             .required_unless("server")
             .help("Flag to start a client session"))
        .arg(Arg::with_name("server")
             .short("s")
             .required_unless("client")
             .help("Flag to start a server session"))
        .get_matches();

    let is_server = matches.is_present("server");

    let listen_port = matches.value_of("listen_port").unwrap();
    let remote_addr = matches.value_of("remote_addr").unwrap();
    let remote_port = matches.value_of("remote_port").unwrap();

    let localhost = "127.0.0.1";
    let tunnel = match Tunnel::new(is_server, localhost, listen_port, remote_addr, remote_port) {
        Ok(tunnel) => tunnel,
        Err(e) => {
            eprintln!("{}", e);
            return ()
        },
    };

    match tunnel.serve() {
        Ok(_) => println!("Serving on port {}...", listen_port),
        Err(e) => {
            eprintln!("{}", e);
            return ()
        },
    };
}
