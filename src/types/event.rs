use serde::{Serialize, Serializer, Deserialize};
use crate::types::{InfoHash, Value, List, TorrentState, IpAddr};
use deluge_rpc_macro::rpc_events;
use enum_kinds::EnumKind;

#[rpc_events]
#[derive(Debug, Clone, Deserialize, EnumKind)]
#[enum_kind(EventKind, derive(Hash))]
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
    CreateTorrentProgress(usize, usize),
    NewVersionAvailable(String),
    SessionStarted,
    SessionPaused,
    SessionResumed,
    ConfigValueChanged(String, Value),
    PluginEnabled(String),
    PluginDisabled(String),
    ClientDisconnected(isize),
    #[serde(rename = "ExternalIPEvent")] // Ip (RFC #430) vs IP (PEP 8)
    ExternalIp(IpAddr),

    #[serde(skip)]
    Unrecognized(String, List),
}

impl Serialize for EventKind {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        if *self == EventKind::ExternalIp {
            return ser.serialize_str("ExternalIPEvent");
        }
        ser.serialize_str(&format!("{:?}Event", self))
    }
}

#[macro_export]
macro_rules! events {
    ($($kind:ident),+$(,)?) => {
        {
            const CAPACITY: usize = [$($crate::EventKind::$kind),+].len();
            let mut set = ::std::collections::HashSet::with_capacity(CAPACITY);
            $(set.insert($crate::EventKind::$kind);)+
            set
        }
    }
}
