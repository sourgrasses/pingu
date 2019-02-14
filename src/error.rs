use crate::packet::TunnelPacket;

use futures::channel::mpsc::{SendError, TryRecvError, TrySendError};

use std::fmt;
use std::net::AddrParseError;
use std::sync::Arc;

pub(crate) type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub(crate) enum Error {
    Addr(AddrParseError),
    Other(&'static str),
    StdIo(std::io::Error),
    Rx(TryRecvError),
    Tx(SendError),
    TxTry(TrySendError<Arc<TunnelPacket>>),
    Thread(std::boxed::Box<dyn std::any::Any + std::marker::Send>),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Addr(e) => write!(f, "{}", e),
            Error::Other(e) => write!(f, "Other error: {}", e),
            Error::StdIo(e) => write!(f, "std::io error: {}", e),
            Error::Rx(e) => write!(f, "mpsc receiver error: {}", e),
            Error::Tx(e) => write!(f, "mpsc sender error: {}", e),
            Error::TxTry(e) => write!(f, "mpsc sender error: {}", e),
            Error::Thread(e) => write!(f, "{:?}", e),
        }
    }
}
