use serde_json::Value;
use crate::rpc;
use tokio::prelude::*;

// TODO: Incorporate serde errors
pub enum Error {
    Network(io::Error),
    Rpc(rpc::Error),
    BadResponse(&'static str, Value),
    ChannelClosed(&'static str),
}

impl Error {
    pub fn expected(exp: &'static str, val: impl Into<Value>) -> Self {
        Self::BadResponse(exp, val.into())
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self { Self::Network(e) }
}
impl From<rpc::Error> for Error {
    fn from(e: rpc::Error) -> Self { Self::Rpc(e) }
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Network(e) => e.fmt(f),
            Self::Rpc(e) => e.fmt(f),
            Self::BadResponse(s, v) => write!(f, "Expected {}, got {}", s, serde_json::to_string(v).unwrap()),
            Self::ChannelClosed(s) => write!(f, "Unexpected closure of {} channel", s),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
