use crate::error::{Error, Result};
use crate::packet::{encode_packs, TunnelPacket};
use crate::tunnel::Tunnel;

use futures::{poll, channel::mpsc, executor::ThreadPool, task::Poll};
use futures::prelude::*;
use pnet_macros_support::packet::Packet;
use rand::Rng;
use romio::tcp::{TcpListener, TcpStream};

use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

pub(crate) struct PinguinClient {
    addr: SocketAddr,
    tunnel: Tunnel,
}

impl PinguinClient {
    pub(crate) fn new(listen_addr: &str, listen_port: u16, remote_addr: &str) -> Result<PinguinClient> {
        let listen_ip = listen_addr.parse::<IpAddr>().map_err(Error::Addr)?;
        let remote_ip = remote_addr.parse::<IpAddr>().map_err(Error::Addr)?;

        let tunnel = Tunnel::new(false, listen_ip, listen_port, remote_ip)?;

        Ok(PinguinClient {
            addr: SocketAddr::new("127.0.0.1".parse().unwrap(), listen_port),
            tunnel: tunnel,
        })
    }

    pub fn run(self) -> Result<()> {
        ThreadPool::new().expect("Error creating threadpool").run(self.connect())?;

        Ok(())
    }

    async fn connect(self) -> Result<()> {
        let (tun_tx, mut rx) = mpsc::unbounded::<Arc<TunnelPacket>>();
        let (mut tx, tun_rx) = mpsc::unbounded::<Arc<TunnelPacket>>();

        self.tunnel.run(tun_tx, tun_rx, None)?;

        let server = TcpListener::bind(&self.addr).map_err(Error::StdIo)?;
        let mut cnx = server.incoming();

        while let Some(stream) = await!(cnx.next()) {
            let mut rng = rand::thread_rng();
            let conn_id: u16 = rng.gen();

            let mut stream = stream.map_err(Error::StdIo)?;
            loop {
                let mut buf = vec![0u8; 1024];
                let i = await!(stream.read(&mut buf)).map_err(Error::StdIo)?;
                // most of the time we won't need the entire kilobyte in the buffer
                buf.truncate(i);

                let mut packs = encode_packs(conn_id, buf);
                for pack in packs.drain(..) {
                    match poll!(tx.send(Arc::new(pack))) {
                        Poll::Ready(Ok(_)) => {
                            match await!(rx.next()) {
                                Some(packs) => {
                                    //println!("{}", std::str::from_utf8(packs.payload()).unwrap());
                                    await!(stream.write_all(packs.payload())).map_err(Error::StdIo)?
                                },
                                None => (),
                            };
                        },
                        _ => unimplemented!(),
                    };
                }
            }
        }

        Ok(())
    }
}
