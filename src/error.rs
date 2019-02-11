use crate::packet::TunnelPacket;

use std::convert::From;
use std::fmt;
use std::net::AddrParseError;
use std::sync::Arc;

pub(crate) type Result<T> = std::result::Result<T, Error>;

pub(crate) enum Error {
    AddrError(AddrParseError),
    Other(&'static str),
    StdIo(std::io::Error),
    TxError(std::sync::mpsc::SendError<Arc<TunnelPacket>>),
    RxError(std::sync::mpsc::RecvError),
    Thread(std::boxed::Box<dyn std::any::Any + std::marker::Send>),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::AddrError(e) => write!(f, "{}", e),
            Error::Other(e) => write!(f, "Other error: {}", e),
            Error::StdIo(e) => write!(f, "std::io error: {}", e),
            Error::TxError(e) => write!(f, "mpsc sender error: {}", e),
            Error::RxError(e) => write!(f, "mpsc receiver error: {}", e),
            Error::Thread(e) => write!(f, "{:?}", e),
        }
    }
}
