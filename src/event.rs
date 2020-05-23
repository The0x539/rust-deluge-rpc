use serde::Deserialize;
use crate::types::{InfoHash, Value, List, TorrentState};
use deluge_rpc_macro::rename_event_enum;
use enum_kinds::EnumKind;

#[rename_event_enum]
#[enum_kind(EventKind, derive(Hash))]
#[derive(Debug, Clone, Deserialize, EnumKind)]
#[serde(tag = "0", content = "1")]
pub enum Event {
    TorrentAdded(InfoHash, bool),
    TorrentRemoved(InfoHash),
    PreTorrentRemoved(InfoHash),
    TorrentStateChanged(InfoHash, TorrentState),
    TorrentTrackerStatus(InfoHash, String),
    TorrentQueueChanged,
    TorrentFolderRenamed(InfoHash, String, String),
    TorrentFileRenamed(InfoHash, usize, String),
    TorrentFinished(InfoHash),
    TorrentResumed(InfoHash),
    TorrentFileCompleted(InfoHash, usize),
    TorrentStorageMoved(InfoHash, String),
    NewVersionAvailable(String),
    SessionStarted,
    SessionPaused,
    SessionResumed,
    ConfigValueChanged(String, Value),
    PluginEnabled(String),
    PluginDisabled(String),
    ClientDisconnected(isize),

    #[serde(skip)]
    Unrecognized(String, List),
}

impl EventKind {
    // in theory, this could be made to return &'static str,
    // either using more boilerplate or a more specialized macro.
    // This code is unlikely to be used other than at startup, though, so whatever.
    pub fn key(&self) -> String {
        format!("{:?}Event", self)
    }
}
