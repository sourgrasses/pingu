use crate::error::{Error, Result};
use crate::packet::{encode_packs, TunnelPacket};
use crate::tunnel::Tunnel;

use futures::{poll, executor::ThreadPool, task::Poll};
use futures::channel::{mpsc, oneshot};
use futures::prelude::*;
use pnet_macros_support::packet::Packet;
use romio::tcp::TcpStream;

use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

pub(crate) struct PinguinServer {
    addr: SocketAddr,
    tunnel: Tunnel,
}

impl PinguinServer {
    pub(crate) fn new(listen_addr: &str, listen_port: u16, remote_addr: &str) -> Result<PinguinServer> {
        let listen_ip = listen_addr.parse::<IpAddr>().map_err(Error::Addr)?;
        let remote_ip = remote_addr.parse::<IpAddr>().map_err(Error::Addr)?;

        let tunnel = Tunnel::new(true, listen_ip, listen_port, remote_ip)?;

        Ok(PinguinServer {
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

        let (addr_tx, addr_rx) = oneshot::channel::<IpAddr>();

        self.tunnel.run(tun_tx, tun_rx, Some(addr_rx))?;

        let mut cnx = await!(TcpStream::connect(&self.addr).map_err(Error::StdIo))?;
        addr_tx.send("127.0.0.1".parse().unwrap()).unwrap();

        loop {
            let mut conn_id = None;
            match poll!(rx.next()) {
                Poll::Ready(Some(pack)) => {
                    conn_id = Some(pack.id);

                    let pack = Arc::try_unwrap(pack).unwrap();
                    await!(cnx.write_all(pack.payload())).map_err(Error::StdIo)?;
                },
                _ => (),
            };

            let mut buf = vec![0u8; 1024];
            match poll!(cnx.read(&mut buf)) {
                Poll::Ready(Ok(bytes)) => {
                    buf.truncate(bytes);
                    let mut packs = encode_packs(conn_id.unwrap_or(0), buf);
                    for pack in packs.drain(..) {
                        //println!("{}", std::str::from_utf8(pack.payload()).unwrap());
                        await!(tx.send(Arc::new(pack))).unwrap();
                    }
                },
                Poll::Pending => (),
                _ => return Err(Error::Other("Error reading from TcpStream")),
            };
        }
    }
}
