use crate::error::{Error, Result};
use crate::packet::{encode_packs, TunnelPacket};
use crate::tunnel::Tunnel;

use futures::{poll, executor::ThreadPool, task::Poll};
use futures::prelude::*;
use pnet_macros_support::packet::Packet;
use romio::tcp::TcpStream;

use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, mpsc};

pub(crate) struct PinguinServer {
    addr: SocketAddr,
    tunnel: Tunnel,
}

impl PinguinServer {
    pub(crate) fn new(listen_addr: &str, listen_port: u16, remote_addr: &str) -> Result<PinguinServer> {
        let listen_ip = listen_addr.parse::<IpAddr>().map_err(Error::AddrError)?;
        let remote_ip = remote_addr.parse::<IpAddr>().map_err(Error::AddrError)?;

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
        let (tun_tx, rx) = mpsc::channel::<Arc<TunnelPacket>>();
        let (tx, tun_rx) = mpsc::channel::<Arc<TunnelPacket>>();

        self.tunnel.run(tun_tx, tun_rx)?;

        let mut cnx = await!(TcpStream::connect(&self.addr).map_err(Error::StdIo))?;

        loop {
            let pack = rx.recv().map_err(Error::RxError)?;
            let conn_id = pack.id;

            let pack = Arc::try_unwrap(pack).unwrap();
            await!(cnx.write_all(pack.payload())).map_err(Error::StdIo)?;

            let mut buf = vec![0u8; 1024];
            match poll!(cnx.read(&mut buf)) {
                Poll::Ready(Ok(bytes)) => buf.truncate(bytes),
                Poll::Pending => (),
                _ => return Err(Error::Other("Error reading from TcpStream")),
            };

            let mut packs = encode_packs(conn_id, buf);
            for pack in packs.drain(..) {
                tx.send(Arc::new(pack)).map_err(Error::TxError)?;
            }
        }

        Ok(())
    }
}
