use crate::error::{Error, Result};
use crate::packet::TunnelPacket;

use futures::{executor::LocalPool, poll};
use futures::channel::{mpsc, oneshot};
use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures::prelude::*;
use futures::task::{Poll, Spawn, SpawnExt};
use pnet::packet::ip::IpNextHeaderProtocol;
use pnet::transport::{transport_channel, TransportChannelType, TransportProtocol, icmp_packet_iter};
use pnet_macros_support::packet::Packet;

use std::net::IpAddr;
use std::sync::Arc;
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
    pub(crate) fn run(self, mut tx: UnboundedSender<Arc<TunnelPacket>>,
                      mut rx: UnboundedReceiver<Arc<TunnelPacket>>,
                      addr_rx: Option<oneshot::Receiver<IpAddr>>) -> Result<()> {
        let chan_type = TransportChannelType::Layer4(TransportProtocol::Ipv4(IpNextHeaderProtocol(1)));
        let (mut sender, mut server) = transport_channel(84, chan_type).map_err(Error::StdIo)?;

        match &self.tunnel_type {
            TunnelType::Client => {
                let out_thread = thread::Builder::new().name("c_out_thread".to_owned());
                let _out_handler = out_thread.spawn(move || {
                    let mut pool = LocalPool::new();
                    let mut spawner = pool.spawner();
                    spawner.spawn(async move {
                        loop {
                            match await!(rx.next()) {
                                Some(pack) => {
                                    //println!("rx loop: {}", str::from_utf8(pack.payload()).unwrap());
                                    sender.send_to(Arc::try_unwrap(pack).unwrap(), self.remote_addr).unwrap();
                                },
                                None => (),
                            };
                        }
                    }).unwrap();

                    pool.run();
                });

                let in_thread = thread::Builder::new().name("c_in_thread".to_owned());
                let _in_handler = in_thread.spawn(move || {
                    let mut pool = LocalPool::new();
                    let mut spawner = pool.spawner();
                    spawner.spawn(async move {
                        let mut siter = icmp_packet_iter(&mut server);
                        loop {
                            let (pack, _addr) = siter.next().unwrap();
                            let decoded: TunnelPacket = pack.into();
                            //println!("{}", std::str::from_utf8(decoded.payload()).unwrap());
                            await!(tx.send(Arc::new(decoded))).unwrap();
                        }
                    }).unwrap();

                    pool.run();
                });

                //in_thread.join().map_err(Error::Thread)?;
                //out_thread.join().map_err(Error::Thread)?;
            },
            TunnelType::Server => {
                let in_thread = thread::Builder::new().name("s_in_thread".to_owned());
                let _in_handler = in_thread.spawn(move || {
                    let mut pool = LocalPool::new();
                    let mut spawner = pool.spawner();
                    spawner.spawn(async move {
                        let mut siter = icmp_packet_iter(&mut server);
                        loop {
                            let (pack, _addr) = siter.next().unwrap();
                            let decoded: TunnelPacket = pack.into();
                            await!(tx.send(Arc::new(decoded))).unwrap();
                        }
                    }).unwrap();

                    pool.run();
                });

                let out_thread = thread::Builder::new().name("s_out_thread".to_owned());
                let _out_handler = out_thread.spawn(move || {
                    let mut pool = LocalPool::new();
                    let mut spawner = pool.spawner();
                    spawner.spawn(async move {
                        let addr_rx = addr_rx.ok_or(Error::Other("Failed to retrieve connection address")).unwrap();
                        let addr = await!(addr_rx.map(|a| a.unwrap()));
                        loop {
                            match poll!(rx.next()) {
                                Poll::Ready(Some(pack)) => {
                                    //println!("{}", std::str::from_utf8(pack.payload()).unwrap());
                                    let _ = sender.send_to(Arc::try_unwrap(pack).unwrap(), addr).unwrap();
                                    ()
                                },
                                _ => (),
                            };
                        }
                    }).unwrap();

                    pool.run();
                });

                //in_thread.join().map_err(Error::Thread)?;
                //out_thread.join().map_err(Error::Thread)?;
            },
        };

        Ok(())
    }
}
