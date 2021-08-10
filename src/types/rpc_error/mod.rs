mod add_torrent;
use add_torrent::Error as AddTorrentError;

use crate::types::{Dict, List, Value};
use serde::Deserialize;
use std::convert::From;
use std::fmt;

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
        Self {
            exception,
            args,
            kwargs,
            traceback,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(from = "GenericError")]
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
            ("AddTorrentError", [Value::String(msg)]) => {
                Self::AddTorrent(AddTorrentError::from(msg.as_str()))
            }
            _ => Self::Generic(err),
        }
    }
}

pub type Error = SpecializedError;
pub type Result<T> = std::result::Result<T, SpecializedError>;
