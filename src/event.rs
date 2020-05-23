use serde::Deserialize;
use crate::types::{InfoHash, Value, TorrentState};

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "0", content = "1")]
pub enum Event {
    #[serde(rename = "TorrentAddedEvent")]
    TorrentAdded(InfoHash, bool),

    #[serde(rename = "TorrentRemovedEvent")]
    TorrentRemoved(InfoHash),

    #[serde(rename = "PreTorrentRemovedEvent")]
    PreTorrentRemoved(InfoHash),

    #[serde(rename = "TorrentStateChangedEvent")]
    TorrentStateChanged(InfoHash, TorrentState),

    #[serde(rename = "TorrentTrackerStatusEvent")]
    TorrentTrackerStatus(InfoHash, String),

    #[serde(rename = "TorrentQueueChangedEvent")]
    TorrentQueueChanged,

    #[serde(rename = "TorrentFolderRenamedEvent")]
    TorrentFolderRenamed(InfoHash, String, String),

    #[serde(rename = "TorrentFileRenamedEvent")]
    TorrentFileRenamed(InfoHash, usize, String),

    #[serde(rename = "TorrentFinishedEvent")]
    TorrentFinished(InfoHash),

    #[serde(rename = "TorrentResumedEvent")]
    TorrentResumed(InfoHash),

    #[serde(rename = "TorrentFileCompletedEvent")]
    TorrentFileCompleted(InfoHash, usize),

    #[serde(rename = "TorrentStorageMovedEvent")]
    TorrentStorageMoved(InfoHash, String),

    #[serde(rename = "NewVersionAvailableEvent")]
    NewVersionAvailable(String),

    #[serde(rename = "SessionStartedEvent")]
    SessionStarted,

    #[serde(rename = "SessionPausedEvent")]
    SessionPaused,

    #[serde(rename = "SessionResumedEvent")]
    SessionResumed,

    #[serde(rename = "ConfigValueChangedEvent")]
    ConfigValueChanged(String, Value),

    #[serde(rename = "PluginEnabledEvent")]
    PluginEnabled(String),

    #[serde(rename = "PluginDisabledEvent")]
    PluginDisabled(String),

    #[serde(rename = "ClientDisconnectedEvent")]
    ClientDisconnected(isize),
}
