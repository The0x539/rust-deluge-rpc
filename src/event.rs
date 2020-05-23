use serde::{Serialize, Serializer, de, Deserialize};
use crate::types::{InfoHash, Value, List, TorrentState, IpAddr};
use deluge_rpc_macro::rename_event_enum;
use enum_kinds::EnumKind;

struct UntupleVisitor<'de, T: Deserialize<'de>>(std::marker::PhantomData<(T, &'de ())>);
impl<'de, T: Deserialize<'de>> de::Visitor<'de> for UntupleVisitor<'de, T> {
    type Value = T;
    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("a sequence containing a single value")
    }
    fn visit_seq<A: de::SeqAccess<'de>>(self, mut seq: A) -> Result<T, A::Error> {
        seq.next_element()?.ok_or(de::Error::invalid_length(0, &"1"))
    }
}

fn untuple<'de, D: de::Deserializer<'de>, T: Deserialize<'de>>(de: D) -> Result<T, D::Error> {
    de.deserialize_tuple(1, UntupleVisitor(Default::default()))
}

struct UnitVisitor;
impl<'de> de::Visitor<'de> for UnitVisitor {
    type Value = ();
    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("an empty list")
    }
    fn visit_seq<A: de::SeqAccess<'de>>(self, mut seq: A) -> Result<(), A::Error> {
        match seq.size_hint() {
            Some(0) => Ok(()),
            Some(n) => Err(de::Error::invalid_length(n, &self)),
            None => match seq.next_element::<Value>()? {
                None => Ok(()),
                Some(_) => Err(de::Error::invalid_value(de::Unexpected::Seq, &self)),
            }
        }
    }
}

fn untuple0<'de, D: de::Deserializer<'de>>(de: D) -> Result<(), D::Error> {
    de.deserialize_tuple(0, UnitVisitor)
}

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
    #[serde(deserialize_with = "untuple0", rename = "TorrentQueueChangedEvent")]
    TorrentQueueChanged,
    TorrentFolderRenamed(InfoHash, String, String),
    TorrentFileRenamed(InfoHash, usize, String),
    TorrentFinished(InfoHash),
    TorrentResumed(#[serde(deserialize_with = "untuple")] InfoHash),
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
