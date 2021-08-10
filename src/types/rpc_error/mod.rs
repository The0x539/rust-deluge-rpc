mod add_torrent;
use add_torrent::Error as AddTorrentError;

use crate::types::{Dict, List, Value};
use serde::Deserialize;
use std::convert::From;
use thiserror::Error;

#[derive(Debug, PartialEq, Eq, Deserialize, Error)]
#[serde(from = "(String, List, Dict, String)")]
#[error("{traceback}")]
pub struct GenericError {
    pub exception: String,
    pub args: List,
    pub kwargs: Dict,
    pub traceback: String,
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

#[derive(Debug, PartialEq, Eq, Deserialize, Error)]
#[serde(from = "GenericError")]
pub enum SpecializedError {
    #[error("AddTorrentError: {0}")]
    AddTorrent(AddTorrentError),
    #[error("{0}")]
    Generic(GenericError),
}

impl From<GenericError> for SpecializedError {
    fn from(err: GenericError) -> Self {
        match (err.exception.as_str(), err.args.as_slice()) {
            ("AddTorrentError", [Value::String(msg)]) => Self::AddTorrent(msg.parse().unwrap()),
            _ => Self::Generic(err),
        }
    }
}

pub type Error = SpecializedError;
pub type Result<T> = std::result::Result<T, SpecializedError>;
