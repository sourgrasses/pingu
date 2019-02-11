use crate::error::{Error, Result};
use crate::packet::TunnelPacket;

use pnet::packet::ip::IpNextHeaderProtocol;
use pnet::transport::{transport_channel, TransportChannelType, TransportProtocol, icmp_packet_iter};
use pnet_macros_support::packet::Packet;

use std::net::IpAddr;
//use std::str;
use std::sync::{Arc, mpsc, mpsc::Receiver, mpsc::Sender};
use std::thread;

enum TunnelType {
    Client,
    Server,
}

pub(crate) struct Tunnel {
    tunnel_type: TunnelType,
    listen_addr: IpAddr,
    listen_port: u16,
    remote_addr: IpAddr,
}

impl Tunnel {
    pub(crate) fn new(is_server: bool, listen_addr: IpAddr, listen_port: u16, remote_addr: IpAddr) -> Result<Tunnel> {
        let tunnel_type = if is_server {
            TunnelType::Server
        } else {
            TunnelType::Client
        };

        Ok(Tunnel {
            tunnel_type: tunnel_type,
            listen_addr: listen_addr,
            listen_port: listen_port,
            remote_addr: remote_addr,
        })
    }

    // TODO: probably wanna implement timeout api stuff and all that
    pub(crate) fn run(self, tx: Sender<Arc<TunnelPacket>>, rx: Receiver<Arc<TunnelPacket>>) -> Result<()> {
        let (addr_tx, addr_rx) = mpsc::channel::<IpAddr>();

        let chan_type = TransportChannelType::Layer4(TransportProtocol::Ipv4(IpNextHeaderProtocol(1)));
        let (mut sender, mut server) = transport_channel(84, chan_type).map_err(Error::StdIo)?;

        match &self.tunnel_type {
            TunnelType::Client => {
                let out_thread = thread::spawn(move || {
                    loop {
                        let pack = rx.recv().unwrap();
                        //println!("rx loop: {}", str::from_utf8(pack.payload()).unwrap());
                        sender.send_to(Arc::try_unwrap(pack).unwrap(), self.remote_addr).unwrap();
                    }
                });

                let in_thread = thread::spawn(move || {
                    let mut siter = icmp_packet_iter(&mut server);
                    loop {
                        let (pack, _addr) = siter.next().unwrap();
                        let decoded: TunnelPacket = pack.into();
                        tx.send(Arc::new(decoded)).unwrap();
                    }
                });

                //in_thread.join().map_err(Error::Thread)?;
                //out_thread.join().map_err(Error::Thread)?;
            },
            TunnelType::Server => {
                let in_thread = thread::spawn(move || {
                    let mut siter = icmp_packet_iter(&mut server);
                    loop {
                        let (pack, addr) = siter.next().unwrap();
                        addr_tx.send(addr).unwrap();
                        let decoded: TunnelPacket = pack.into();
                        tx.send(Arc::new(decoded)).unwrap();
                    }
                });

                let out_thread = thread::spawn(move || {
                    loop {
                        let addr = addr_rx.recv().unwrap();
                        let pack = rx.recv().unwrap();
                        sender.send_to(Arc::try_unwrap(pack).unwrap(), addr).unwrap();
                    }
                });

                //in_thread.join().map_err(Error::Thread)?;
                //out_thread.join().map_err(Error::Thread)?;
            },
        };

        Ok(())
    }
}
