use std::convert::TryFrom;
use std::collections::HashMap;
pub use std::net::{IpAddr, SocketAddr};

use serde_yaml::Value;
use serde::{Serialize, Deserialize};

use tokio::prelude::*;
use tokio::sync::oneshot;
use tokio::net::TcpStream;
use tokio_rustls::client::TlsStream;

use crate::rpc;
use deluge_rpc_macro::*;

pub type List = Vec<Value>;
pub type Dict = HashMap<String, Value>;

pub type WriteStream = io::WriteHalf<TlsStream<TcpStream>>;
pub type RequestTuple = (i64, &'static str, List, Dict);
pub type RpcSender = oneshot::Sender<rpc::Result<List>>;

#[derive(Copy, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(try_from = "String", into = "String")]
pub struct InfoHash([u8; 20]);

impl InfoHash {
    pub fn from_hex(hex_str: &str) -> Option<Self> {
        if hex_str.len() != 40 {
            println!("len = {}", hex_str.len());
            return None;
        }

        let bytes_vec = match hex::decode(hex_str) {
            Ok(x) => x,
            Err(e) => {
                println!("{:?}", e);
                return None;
            },
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

pub trait Query: for<'de> Deserialize<'de> {
    fn keys() -> &'static [&'static str];
}
