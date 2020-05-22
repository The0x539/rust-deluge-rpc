use std::convert::TryFrom;
use std::collections::HashMap;
pub use std::net::{IpAddr, SocketAddr};

use serde::{Serialize, Deserialize, de::DeserializeOwned};

use tokio::prelude::*;
use tokio::sync::oneshot;
use tokio::net::TcpStream;
use tokio_rustls::client::TlsStream;

use crate::rpc;
use deluge_rpc_macro::*;

pub use ron::Value;
pub type List = Vec<Value>;
pub type Dict = HashMap<String, Value>;

pub type ReadStream = io::ReadHalf<TlsStream<TcpStream>>;
pub type WriteStream = io::WriteHalf<TlsStream<TcpStream>>;
pub type RpcSender = oneshot::Sender<rpc::Result<List>>;

#[derive(Copy, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(try_from = "String", into = "String")]
pub struct InfoHash([u8; 20]);

impl InfoHash {
    pub fn from_hex(hex_str: &str) -> Option<Self> {
        if hex_str.len() != 40 {
            return None;
        }

        let bytes_vec = match hex::decode(hex_str) {
            Ok(x) => x,
            Err(_) => return None,
        };

        let mut final_array = [0; 20];
        final_array.copy_from_slice(&bytes_vec);

        Some(Self(final_array))
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }
}

impl Into<String> for InfoHash {
    fn into(self) -> String { self.to_string() }
}

impl TryFrom<String> for InfoHash {
    type Error = &'static str;
    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        Self::from_hex(&value).ok_or("invalid infohash")
    }
}

impl std::fmt::Display for InfoHash {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(&self.to_hex())
    }
}

impl std::fmt::Debug for InfoHash {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

#[value_enum(u8)]
pub enum FilePriority { Skip = 0, Low = 1, Normal = 4, High = 7 }
impl Default for FilePriority { fn default() -> Self { Self::Normal } }

#[value_enum(u8)]
pub enum AuthLevel { Nobody = 0, ReadOnly = 1, Normal = 5, Admin = 10 }
impl Default for AuthLevel { fn default() -> Self { Self::Normal } }

#[value_enum(&str)]
pub enum TorrentState {
    Checking, Downloading, Seeding, Allocating,
    Error, Moving, Queued, Paused,
}

#[option_struct]
#[derive(Clone, Default)]
pub struct TorrentOptions {
    pub add_paused: bool,
    pub auto_managed: bool,
    pub download_location: String,
    pub file_priorities: Vec<FilePriority>,
    pub mapped_files: HashMap<String, String>,
    pub max_connections: i64,
    pub max_download_speed: f64,
    pub max_upload_slots: i64,
    pub max_upload_speed: f64,
    pub move_completed: bool,
    pub move_completed_path: String,
    pub name: String,
    pub owner: String,
    pub pre_allocate_storage: bool,
    pub prioritize_first_last_pieces: bool,
    pub remove_at_ratio: bool,
    pub seed_mode: bool, // Only used when adding a torrent
    pub sequential_download: bool,
    pub shared: bool,
    pub stop_at_ratio: bool,
    pub stop_ratio: f64,
    pub super_seeding: bool,
}

pub trait Query: DeserializeOwned {
    type Diff: DeserializeOwned + Default + PartialEq;
    fn keys() -> &'static [&'static str];
    fn update(&mut self, diff: Self::Diff) -> bool;
}

// TODO: Incorporate serde errors
pub enum Error {
    Network(io::Error),
    Rpc(rpc::Error),
    BadResponse(rencode::Error),
    ChannelClosed(&'static str),
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self { Self::Network(e) }
}
impl From<rpc::Error> for Error {
    fn from(e: rpc::Error) -> Self { Self::Rpc(e) }
}
impl From<rencode::Error> for Error {
    fn from(e: rencode::Error) -> Self { Self::BadResponse(e) }
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Network(e) => e.fmt(f),
            Self::Rpc(e) => e.fmt(f),
            Self::BadResponse(e) => e.fmt(f),
            Self::ChannelClosed(s) => write!(f, "Unexpected closure of {} channel", s),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
