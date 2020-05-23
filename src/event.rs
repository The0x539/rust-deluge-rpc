use serde::{Serialize, Serializer, Deserialize};
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

impl Serialize for EventKind {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_str(&format!("{:?}Event", self))
    }
}
