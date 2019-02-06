use crate::packet::{encode_packs, decode_packs, TunnelPacket};

use pnet::packet::icmp::IcmpPacket;
use pnet::packet::ip::IpNextHeaderProtocol;
use pnet::transport::{transport_channel, TransportChannelType, TransportProtocol, icmp_packet_iter};
use pnet_macros_support::packet::Packet;

use std::net::{AddrParseError, IpAddr, SocketAddr};
use std::str;
use std::sync::{Arc, mpsc};
use std::thread;

enum TunnelType {
    Client,
    Server,
}

pub struct Tunnel {
    tunnel_type: TunnelType,
    listen_addr: IpAddr,
    listen_port: String,
    remote: SocketAddr,
    remote_addr: IpAddr,
}

impl Tunnel {
    pub fn new(is_server: bool,
               listen_addr: &str,
               listen_port: &str,
               remote_addr: &str,
               remote_port: &str) -> Result<Tunnel, AddrParseError>
    {
        let remote = match format!("{}:{}", remote_addr, remote_port).parse::<SocketAddr>() {
            Ok(remote) => remote,
            Err(e) => return Err(e)
        };
        let listen = match format!("{}:{}", remote_addr, remote_port).parse::<SocketAddr>() {
            Ok(remote) => remote,
            Err(e) => return Err(e),
        };

        let tunnel_type = if is_server {
            TunnelType::Server
        } else {
            TunnelType::Client
        };

        Ok(Tunnel {
            tunnel_type: tunnel_type,
            listen_addr: listen_addr.parse::<IpAddr>()?,
            listen_port: listen_port.to_owned(),
            remote: remote,
            remote_addr: remote.ip(),
        })
    }

    // TODO: probably wanna implement timeout api stuff and all that
    pub fn serve(self) -> Result<(), &'static str> {
        let (tx, rx) = mpsc::channel::<Arc<TunnelPacket>>();

        match &self.tunnel_type {
            TunnelType::Client => {
                let chan_type = TransportChannelType::Layer4(TransportProtocol::Ipv4(IpNextHeaderProtocol(1)));
                let (mut sender, _server) = match transport_channel(64, chan_type) {
                    Ok(res) => res,
                    Err(e) => unimplemented!(),
                };
                let payload = "here is a big glob of text that we're going to turn into a vec of bytes and then send to the other endpoint to make sure that the server doesn't panic if it gets packets of the correct size".as_bytes();

                let packs = Arc::try_unwrap(encode_packs(1, payload.to_vec())).unwrap();
                for pack in &packs {
                    println!("{:?}", pack);
                    sender.send_to(pack.clone(), "127.0.0.1".parse::<IpAddr>().unwrap()).unwrap();
                }

                println!("{:?}", rx.recv().unwrap());
            },
            TunnelType::Server => {
                let chan_type = TransportChannelType::Layer4(TransportProtocol::Ipv4(IpNextHeaderProtocol(1)));
                let (_sender, mut server) = match transport_channel(84, chan_type) {
                    Ok(res) => res,
                    Err(e) => unimplemented!(),
                };
                let _sthread = thread::spawn(move || {
                    let mut siter = icmp_packet_iter(&mut server);
                    loop {
                        match siter.next() {
                            Ok((pack, _addr)) => {
                                let decoded: TunnelPacket = pack.into();
                                tx.send(Arc::new(decoded)).unwrap()
                            },
                            Err(e) => unimplemented!(),
                        };
                    }
                });

                loop {
                    let pack = rx.recv().unwrap().raw_pack;
                    let pack_str = str::from_utf8(&pack[8..]).unwrap();
                    println!("{}", pack_str);
                }
            },
        }

        Ok(())
    }
}
