mod event;
mod message;
mod rpc_error;
#[macro_use]
mod macros;

pub use event::{Event, EventKind};
pub use message::Message;
pub use rpc_error::{AddTorrentError, GenericError as GenericRpcError, RpcError};

use fnv::FnvHashMap;
use thiserror::Error;

use std::collections::HashMap;
pub use std::net::{IpAddr, SocketAddr};

// BitTorrent infohashes are just SHA-1 checksums.
// If they're not maliciously crafted, this means they should already be high-entropy.
// Therefore, if we trust the source of the infohashes, then hashing the hashes seems pointless.
// Even if the user somehow connects to a malicious DelugeRPC endpoint,
// there would be more effective ways for the server to launch a DoS on the client,
// and this isn't really a context where DoS resilience matters, anyway.
#[derive(Default, Clone, Copy, Debug)]
#[cfg(feature = "trust-infohashes")]
pub struct PassThroughBuildHasher;

#[cfg(feature = "trust-infohashes")]
impl std::hash::BuildHasher for PassThroughBuildHasher {
    type Hasher = hashers::null::PassThroughHasher;
    fn build_hasher(&self) -> Self::Hasher {
        Self::Hasher::default()
    }
}

#[cfg(feature = "trust-infohashes")]
pub type InfoHashBuildHasher = PassThroughBuildHasher;

#[cfg(not(feature = "trust-infohashes"))]
pub type InfoHashBuildHasher = fnv::FnvBuildHasher;

use serde::{de::DeserializeOwned, Deserialize, Serialize};

pub use serde_value::Value;
pub type List = Vec<Value>;
pub type Dict = HashMap<String, Value>;

pub type Stream = tokio_rustls::client::TlsStream<tokio::net::TcpStream>;
pub type ReadStream = tokio::io::ReadHalf<Stream>;
pub type WriteStream = tokio::io::WriteHalf<Stream>;
pub type RpcSender = tokio::sync::oneshot::Sender<rpc_error::Result<List>>;

#[derive(Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Hash)]
#[serde(transparent)]
pub struct InfoHash(#[serde(with = "hex")] [u8; 20]);
pub type InfoHashMap<V> = HashMap<InfoHash, V, InfoHashBuildHasher>;

impl hex::FromHex for InfoHash {
    type Error = <[u8; 20] as hex::FromHex>::Error;
    fn from_hex<T: AsRef<[u8]>>(hex: T) -> std::result::Result<Self, Self::Error> {
        hex::FromHex::from_hex(hex).map(Self)
    }
}

impl hex::ToHex for InfoHash {
    fn encode_hex<T: std::iter::FromIterator<char>>(&self) -> T {
        self.0.encode_hex()
    }
    fn encode_hex_upper<T: std::iter::FromIterator<char>>(&self) -> T {
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

// Ideally, all eight of these priorities would have formal names.
// Deluge, however, seems to want to hide the fact that there *are* eight priorities.
// We follow suit. Whatever.

u8_enum! {
    #[serde(try_from = "u8")]
    enum TolerantFilePriority;
    Skip = 0, Low1 = 1, Low2 = 2, Low3 = 3, Normal = 4, High5 = 5, High6 = 6, High7 = 7
}

u8_enum! {
    #[serde(from = "TolerantFilePriority")]
    pub enum FilePriority = Normal;

    Skip = 0, Low = 1, Normal = 4, High = 7
}

impl From<TolerantFilePriority> for FilePriority {
    fn from(value: TolerantFilePriority) -> Self {
        match value as u8 {
            0 => Self::Skip,
            1..=3 => Self::Low,
            4 => Self::Normal,
            5..=7 => Self::High,
            _ => unreachable!(),
        }
    }
}

u8_enum! {
    #[serde(try_from = "u8")]
    pub enum AuthLevel = Normal;

    Nobody = 0, ReadOnly = 1, Normal = 5, Admin = 10
}

string_enum! {
    pub enum TorrentState;

    Checking, Downloading, Seeding, Allocating,
    Error, Moving, Queued, Paused,
}

string_enum! {
    pub enum FilterKey;

    State = "state",
    Tracker = "tracker_host",
    Owner = "owner",
    Label = "label",
}

pub type FilterDict = FnvHashMap<FilterKey, String>;

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

pub trait DeserializeStatic: DeserializeOwned + Send + 'static {}
impl<T: DeserializeOwned + Send + 'static> DeserializeStatic for T {}

pub use deluge_rpc_macro::Query;
pub trait Query: DeserializeStatic + Clone {
    type Diff: DeserializeStatic + Default + Clone + PartialEq;
    fn keys() -> &'static [&'static str];
    fn update(&mut self, diff: Self::Diff) -> bool;
}

// TODO: Incorporate serde errors
#[derive(Error)]
pub enum Error {
    #[error("Network error: {0}")]
    Network(#[from] tokio::io::Error),
    #[error("RPC error: {0}")]
    Rpc(#[from] rpc_error::Error),
    #[error("Response deserialization error: {0}")]
    BadResponse(#[from] rencode::Error),
}

impl Error {
    pub fn ok_if_added(self) -> Result<InfoHash> {
        match self {
            Self::Rpc(e) => e.ok_if_added().map_err(Self::Rpc),
            e => Err(e),
        }
    }
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Network(e) => e.fmt(f),
            Self::Rpc(e) => e.fmt(f),
            Self::BadResponse(e) => e.fmt(f),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
