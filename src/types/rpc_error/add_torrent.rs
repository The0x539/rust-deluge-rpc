use crate::types::InfoHash;
use hex::FromHex;
use lazy_static::lazy_static;
use regex::Regex;
use std::str::FromStr;
use thiserror::Error;

#[derive(Debug, PartialEq, Eq, Error)]
pub enum Error {
    #[error("Torrent already in session: {0}")]
    AlreadyInSession(InfoHash),
    #[error("Torrent already being added: {0}")]
    AlreadyBeingAdded(InfoHash),
    #[error("Invalid magnet info: {0}")]
    UnableToAddMagnet(String),
    // Shouldn't naturally occur for users of the API, but worth handling
    #[error("Must specify a valid torrent")]
    MustSpecifyValidTorrent,
    #[error("Decoding filedump failed: {0}")]
    DecodingFiledumpFailed(String),
    #[error("Unable to add torrent to session: {0}")]
    UnableToAddToSession(String),
    #[error("{0}")]
    Other(String),
}

lazy_static! {
    static ref ALREADY_IN_SESSION: Regex =
        Regex::new(r"^Torrent already in session \([0-9a-fA-F]{40}\)\.$").unwrap();
    static ref ALREADY_BEING_ADDED: Regex =
        Regex::new(r"^Torrent already being added \([0-9a-fA-F]{40}\)\.$").unwrap();
}

static UNABLE_TO_ADD_MAGNET: &str = "Unable to add magnet, invalid magnet info: ";
static MUST_SPECIFY_VALID_TORRENT: &str =
    "You must specify a valid torrent_info, torrent state or magnet.";
static UNABLE_TO_ADD_TORRENT: &str = "Unable to add torrent, decoding filedump failed: ";
static UNABLE_TO_ADD_TO_SESSION: &str = "Unable to add torrent to session: ";

impl FromStr for Error {
    type Err = std::convert::Infallible;
    fn from_str(msg: &str) -> Result<Self, Self::Err> {
        // TODO: order conditionals based on likelihood of occurrence
        Ok(if ALREADY_IN_SESSION.is_match(msg) {
            let hash = InfoHash::from_hex(&msg[28..][..20]).unwrap();
            Self::AlreadyInSession(hash)
        } else if ALREADY_BEING_ADDED.is_match(msg) {
            let hash = InfoHash::from_hex(&msg[29..][..20]).unwrap();
            Self::AlreadyBeingAdded(hash)
        } else if let Some(magnet) = msg.strip_prefix(UNABLE_TO_ADD_MAGNET) {
            Self::UnableToAddMagnet(magnet.to_owned())
        } else if msg == MUST_SPECIFY_VALID_TORRENT {
            Self::MustSpecifyValidTorrent
        } else if let Some(decode_ex) = msg.strip_prefix(UNABLE_TO_ADD_TORRENT) {
            Self::DecodingFiledumpFailed(decode_ex.to_owned())
        } else if let Some(ex) = msg.strip_prefix(UNABLE_TO_ADD_TO_SESSION) {
            Self::UnableToAddToSession(ex.to_owned())
        } else {
            Self::Other(msg.to_owned())
        })
    }
}
