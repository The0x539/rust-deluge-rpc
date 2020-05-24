pub mod rpc;

mod event;
pub use event::{Event, EventKind};

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
pub type RpcSender = oneshot::Sender<rpc::Result<List>>;

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

macro_rules! u8_enum {
    (
        $(#[$attr:meta])*
        $vis:vis enum $name:ident $(= $default:ident)?;
        $(
            $(#[$variant_attr:meta])*
            $variant:ident = $discrim:literal
        ),+$(,)?
    ) => {
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
        #[derive(TryFromPrimitive, IntoPrimitive)]
        #[derive(Serialize, Deserialize)]
        #[serde(try_from = "u8", into = "u8")]
        #[repr(u8)]
        $(#[$attr])*
        $vis enum $name {$(
            $(#[$variant_attr])*
            $variant = $discrim
        ),+}
        $(impl Default for $name { fn default() -> Self { Self::$default } })?
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

// TODO: renaming?
macro_rules! string_enum {
    (
        $(#[$attr:meta])*
        $vis:vis enum $name:ident $(= $default:ident)?;
        $( $(#[$variant_attr:meta])* $variant:ident ),+$(,)?
    ) => {
        #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
        #[derive(Serialize, Deserialize)]
        #[serde(try_from = "String", into = "&'static str")]
        $(#[$attr])*
        $vis enum $name {$( $(#[$variant_attr])* $variant ),+}
        impl FromStr for $name {
            type Err = String;
            fn from_str(s: &str) -> std::result::Result<Self, String> {
                match s {
                    $(stringify!($variant) => Ok(Self::$variant),)+
                    s => Err(format!("Invalid {} value: {:?}", stringify!($name), s)),
                }
            }
        }
        impl Into<&'static str> for $name {
            fn into(self) -> &'static str {
                match self {
                    $(Self::$variant => stringify!($variant),)+
                }
            }
        }
        // It's incredibly dumb that there's no blanket impl for this.
        impl TryFrom<String> for $name {
            type Error = String;
            fn try_from(s: String) -> std::result::Result<Self, String> {
                s.as_str().parse()
            }
        }
        $(impl Default for $name { fn default() -> Self { Self::$default } })?
    }
}

string_enum! {
    pub enum TorrentState;

    Checking, Downloading, Seeding, Allocating,
    Error, Moving, Queued, Paused,
}

macro_rules! option_struct {
    (
        $(#[$attr:meta])*
        $vis:vis struct $name:ident;
        $(
            $(#[$field_attr:meta])*
            $field_vis:vis $field:ident: $type:ty
        ),+$(,)?
    ) => {
        #[derive(Serialize, Deserialize)]
        $(#[$attr])*
        $vis struct $name {$(
            #[serde(default, skip_serializing_if = "Option::is_none")]
            $(#[$field_attr])*
            $field_vis $field: Option<$type>,
        )*}
    }
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
