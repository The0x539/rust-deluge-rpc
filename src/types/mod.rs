mod inbound;
mod rpc_error;
mod event;
#[macro_use] mod macros;

pub use event::{Event, EventKind};
pub use inbound::Inbound;

use std::collections::HashMap;
use std::iter::FromIterator;
use std::str::FromStr;
use std::convert::TryFrom;
pub use std::net::{IpAddr, SocketAddr};

use serde::{Serialize, Deserialize, de::DeserializeOwned};

use num_enum::{IntoPrimitive, TryFromPrimitive};

use tokio::prelude::*;
use tokio::sync::oneshot;
use tokio::net::TcpStream;
use tokio_rustls::client::TlsStream;

use hex::{FromHex, ToHex};

pub use ron::Value;
pub type List = Vec<Value>;
pub type Dict = HashMap<String, Value>;

pub type Stream = TlsStream<TcpStream>;
pub type ReadStream = io::ReadHalf<Stream>;
pub type WriteStream = io::WriteHalf<Stream>;
pub type RpcSender = oneshot::Sender<rpc_error::Result<List>>;

#[derive(Copy, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(transparent)]
pub struct InfoHash(#[serde(with = "hex")] [u8; 20]);

impl FromHex for InfoHash {
    type Error = <[u8; 20] as FromHex>::Error;
    fn from_hex<T: AsRef<[u8]>>(hex: T) -> std::result::Result<Self, Self::Error> {
        FromHex::from_hex(hex).map(Self)
    }
}

impl ToHex for InfoHash {
    fn encode_hex<T: FromIterator<char>>(&self) -> T {
        self.0.encode_hex()
    }
    fn encode_hex_upper<T: FromIterator<char>>(&self) -> T {
        self.0.encode_hex_upper()
    }
}

impl std::fmt::Display for InfoHash {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut bytes = [0u8; 40];

        // Okay to unwrap because the buffer is exactly the right size.
        hex::encode_to_slice(self.0, &mut bytes).unwrap();

        // hex::encode_to_slice can be trusted to emit valid UTF-8.
        let hex_str: &str = unsafe {
            debug_assert!(std::str::from_utf8(&bytes).is_ok());
            std::str::from_utf8_unchecked(&bytes)
        };

        f.write_str(hex_str)
    }
}

impl std::fmt::Debug for InfoHash {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

u8_enum! {
    pub enum FilePriority = Normal;

    Skip = 0, Low = 1, Normal = 4, High = 7
}

u8_enum! {
    pub enum AuthLevel = Normal;

    Nobody = 0, ReadOnly = 1, Normal = 5, Admin = 10
}

string_enum! {
    pub enum TorrentState;

    Checking, Downloading, Seeding, Allocating,
    Error, Moving, Queued, Paused,
}

option_struct! {
    #[derive(Clone, Default)]
    pub struct TorrentOptions;

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
    Rpc(rpc_error::Error),
    BadResponse(rencode::Error),
    ChannelClosed(&'static str),
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self { Self::Network(e) }
}
impl From<rpc_error::Error> for Error {
    fn from(e: rpc_error::Error) -> Self { Self::Rpc(e) }
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
