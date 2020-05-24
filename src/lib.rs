#![feature(str_strip)] // sorry

mod encoding;
mod session;
mod receiver;
mod wtf;
mod types;

pub use deluge_rpc_macro::Query;

pub use types::{
    List, Dict,
    IpAddr, SocketAddr,
    InfoHash, FilePriority, AuthLevel, TorrentOptions, TorrentState,
    Query,
    Error, Result,
    Event, EventKind,
};
pub use session::Session;
