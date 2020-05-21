use std::collections::HashMap;
use serde_yaml::{self, Value};
use serde::de;
use std::convert::{From, TryFrom};
use crate::types::InfoHash;
use lazy_static::lazy_static;
use lazy_regex::regex;
use std::fmt;

const RPC_RESPONSE: i64 = 1;
const RPC_ERROR: i64 = 2;
const RPC_EVENT: i64 = 3;

// TODO: even a single specialized error type is a lot of code, so move errors to separate module

#[derive(Debug, PartialEq, Eq)]
pub struct GenericError {
    pub exception: String,
    pub args: Vec<Value>,
    pub kwargs: HashMap<String, Value>,
    pub traceback: String,
}

impl fmt::Display for GenericError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.traceback)
    }
}

impl std::error::Error for GenericError {}

pub type Error = GenericError;

#[derive(Debug, PartialEq, Eq)]
pub enum AddTorrentError {
    AlreadyInSession(InfoHash),
    AlreadyBeingAdded(InfoHash),
    UnableToAddMagnet(String),
    // Shouldn't naturally occur for users of the API, but worth handling
    MustSpecifyValidTorrent,
    DecodingFiledumpFailed(String),
    UnableToAddToSession(String),
    Other(String),
}

impl fmt::Display for AddTorrentError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::AlreadyInSession(hash) => write!(f, "Torrent already in session: {}", hash),
            Self::AlreadyBeingAdded(hash) => write!(f, "Torrent already being added: {}", hash),
            Self::UnableToAddMagnet(link) => write!(f, "Invalid magnet info: {}", link),
            Self::MustSpecifyValidTorrent => f.write_str("Must specify a valid torrent"),
            Self::DecodingFiledumpFailed(decode_ex) => write!(f, "Decoding filedump failed: {}", decode_ex),
            Self::UnableToAddToSession(ex) => write!(f, "Unable to add torrent to session: {}", ex),
            Self::Other(msg) => f.write_str(msg),
        }
    }
}

impl From<&str> for AddTorrentError {
    fn from(msg: &str) -> Self {
        // TODO: order conditionals based on likelihood of occurrence
        if regex!(r"^Torrent already in session \([0-9a-fA-F]{40}\)\.$").is_match(msg) {
            Self::AlreadyInSession(InfoHash::from_hex(&msg[28..68]).unwrap())
        } else if regex!(r"^Torrent already being added \([0-9a-fA-F]{40}\)\.$").is_match(msg) {
            Self::AlreadyBeingAdded(InfoHash::from_hex(&msg[29..69]).unwrap())
        } else if let Some(magnet) = msg.strip_prefix("Unable to add magnet, invalid magnet info: ") {
            Self::UnableToAddMagnet(magnet.to_string())
        } else if msg == "You must specify a valid torrent_info, torrent state or magnet." {
            Self::MustSpecifyValidTorrent
        } else if let Some(decode_ex) = msg.strip_prefix("Unable to add torrent, decoding filedump failed: ") {
            Self::DecodingFiledumpFailed(decode_ex.to_string())
        } else if let Some(ex) = msg.strip_prefix("Unable to add torrent to session: ") {
            Self::UnableToAddToSession(ex.to_string())
        } else {
            Self::Other(msg.to_string())
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum SpecializedError {
    AddTorrent(AddTorrentError),
    Generic(GenericError),
}

impl fmt::Display for SpecializedError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::AddTorrent(e) => write!(f, "AddTorrentError: {}", e),
            Self::Generic(e) => f.write_str(&e.to_string()),
        }
    }
}

impl From<GenericError> for SpecializedError {
    fn from(err: GenericError) -> Self {
        match (err.exception.as_str(), err.args.as_slice()) {
            ("AddTorrentError", [Value::String(msg)]) => Self::AddTorrent(AddTorrentError::from(msg.as_str())),
            _ => Self::Generic(err),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Inbound {
    Response { request_id: i64, result: Result<Vec<Value>> },
    Event { event_name: String, data: Vec<Value> },
}

impl TryFrom<&[Value]> for Inbound {
    type Error = serde_yaml::Error;

    fn try_from(data: &[Value]) -> serde_yaml::Result<Self> {
        use serde_yaml::from_value;
        let msg_type: i64 = from_value(data[0].clone())?;
        let val = match msg_type {
            RPC_RESPONSE | RPC_ERROR => Inbound::Response {
                request_id: from_value(data[1].clone())?,
                result: match msg_type {
                    RPC_RESPONSE => Ok(from_value(data[2].clone()).unwrap_or(vec![data[2].clone()])),
                    RPC_ERROR => {
                        Err(Error {
                            exception: from_value(data[2].clone())?,
                            args: from_value(data[3].clone())?,
                            kwargs: from_value(data[4].clone())?,
                            traceback: from_value(data[5].clone())?,
                        })
                    },
                    _ => unreachable!(),
                },
            },
            RPC_EVENT => Inbound::Event {
                event_name: from_value(data[1].clone())?,
                data: from_value(data[2].clone())?,
            },
            _ => {
                let unexp = de::Unexpected::Signed(msg_type);
                let exp = &"a known message type (1, 2, or 3)";
                return Err(de::Error::invalid_value(unexp, exp));
            },
        };
        Ok(val)
    }
}
