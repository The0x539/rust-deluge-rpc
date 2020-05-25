#![feature(str_strip)] // sorry

mod encoding;
mod session;
mod receiver;
mod wtf;
mod types;
mod methods;

pub use types::{
    List, Dict,
    IpAddr, SocketAddr,
    InfoHash, FilePriority, AuthLevel, TorrentOptions, TorrentState,
    FilterKey, FilterDict,
    Query,
    Error, Result,
    Event, EventKind,
};
pub use session::Session;
