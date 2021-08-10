use crate::types::InfoHash;
use hex::FromHex;
use lazy_regex::regex;
use lazy_static::lazy_static;
use std::convert::From;
use std::fmt;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    AlreadyInSession(InfoHash),
    AlreadyBeingAdded(InfoHash),
    UnableToAddMagnet(String),
    // Shouldn't naturally occur for users of the API, but worth handling
    MustSpecifyValidTorrent,
    DecodingFiledumpFailed(String),
    UnableToAddToSession(String),
    Other(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::AlreadyInSession(hash) => write!(f, "Torrent already in session: {}", hash),
            Self::AlreadyBeingAdded(hash) => write!(f, "Torrent already being added: {}", hash),
            Self::UnableToAddMagnet(link) => write!(f, "Invalid magnet info: {}", link),
            Self::MustSpecifyValidTorrent => f.write_str("Must specify a valid torrent"),
            Self::DecodingFiledumpFailed(decode_ex) => {
                write!(f, "Decoding filedump failed: {}", decode_ex)
            }
            Self::UnableToAddToSession(ex) => write!(f, "Unable to add torrent to session: {}", ex),
            Self::Other(msg) => f.write_str(msg),
        }
    }
}

impl From<&str> for Error {
    fn from(msg: &str) -> Self {
        // TODO: order conditionals based on likelihood of occurrence
        if regex!(r"^Torrent already in session \([0-9a-fA-F]{40}\)\.$").is_match(msg) {
            Self::AlreadyInSession(InfoHash::from_hex(&msg[28..68]).unwrap())
        } else if regex!(r"^Torrent already being added \([0-9a-fA-F]{40}\)\.$").is_match(msg) {
            Self::AlreadyBeingAdded(InfoHash::from_hex(&msg[29..69]).unwrap())
        } else if let Some(magnet) = msg.strip_prefix("Unable to add magnet, invalid magnet info: ")
        {
            Self::UnableToAddMagnet(magnet.to_string())
        } else if msg == "You must specify a valid torrent_info, torrent state or magnet." {
            Self::MustSpecifyValidTorrent
        } else if let Some(decode_ex) =
            msg.strip_prefix("Unable to add torrent, decoding filedump failed: ")
        {
            Self::DecodingFiledumpFailed(decode_ex.to_string())
        } else if let Some(ex) = msg.strip_prefix("Unable to add torrent to session: ") {
            Self::UnableToAddToSession(ex.to_string())
        } else {
            Self::Other(msg.to_string())
        }
    }
}
