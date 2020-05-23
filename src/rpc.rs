use serde::Deserialize;
use std::convert::{From, TryFrom};
use crate::types::{InfoHash, Value, List, Dict};
use crate::event::Event;
use lazy_static::lazy_static;
use lazy_regex::regex;
use std::fmt;
use deluge_rpc_macro::value_enum;

// TODO: even a single specialized error type is a lot of code, so move errors to separate module

#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(from = "(String, List, Dict, String)")]
pub struct GenericError {
    pub exception: String,
    pub args: List,
    pub kwargs: Dict,
    pub traceback: String,
}

impl fmt::Display for GenericError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.traceback)
    }
}

impl From<(String, List, Dict, String)> for GenericError {
    fn from((exception, args, kwargs, traceback): (String, List, Dict, String)) -> Self {
        Self { exception, args, kwargs, traceback }
    }
}

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

#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(from="GenericError")]
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

pub type Error = SpecializedError;
pub type Result<T> = std::result::Result<T, SpecializedError>;

#[derive(Debug, Deserialize)]
#[serde(try_from="List")]
pub enum Inbound {
    Response { request_id: i64, result: Result<List> },
    Event(Event),
}

#[value_enum(u8)]
enum MessageType { Response = 1, Error = 2, Event = 3 }

impl TryFrom<List> for Inbound {
    type Error = ron::Error;

    fn try_from(data: List) -> ron::Result<Self> {
        let mut data = data.into_iter();
        let msg_type = data.next().unwrap().into_rust()?;
        let val = match msg_type {
            MessageType::Response => Inbound::Response {
                request_id: data.next().unwrap().into_rust()?,
                result: Ok(match data.next().unwrap() {
                    Value::Seq(x) => x,
                    x => vec![x],
                }),
            },
            MessageType::Error => Inbound::Response {
                request_id: data.next().unwrap().into_rust()?,
                result: Err(Value::Seq(data.collect()).into_rust()?),
            },
            MessageType::Event => Inbound::Event(Value::Seq(data.collect()).into_rust()?),
        };
        Ok(val)
    }
}
