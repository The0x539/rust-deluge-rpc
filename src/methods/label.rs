use serde::Serialize;

use std::collections::HashMap;

use deluge_rpc_macro::rpc_class;

use crate::session::Session;
use crate::types::{AuthLevel, DeserializeStatic, InfoHash, Result};

/// Basically just "this is AsRef<str> but serializable".
/// Should generally be a string of some sort.
pub trait LabelId: AsRef<str> + Serialize {}
impl<T: AsRef<str> + Serialize> LabelId for T {}

rpc_class! {
    impl Session::label;

    pub rpc fn get_labels(&self) -> Vec<String>;

    #[rpc(method = "add")]
    pub rpc fn add_label(&self, label_id: impl LabelId);

    #[rpc(method = "remove")]
    pub rpc fn remove_label(&self, label_id: impl LabelId);

    #[rpc(method = "get_options")]
    pub rpc fn get_label_options<T: DeserializeStatic>(&self, label_id: impl LabelId) -> HashMap<String, T>;

    #[rpc(method = "set_options")]
    pub rpc fn set_label_options(&self, label_id: impl LabelId, options: &HashMap<String, impl Serialize>);

    #[rpc(method = "set_torrent")]
    pub rpc fn set_torrent_label(&self, torrent_id: InfoHash, label_id: impl LabelId);

    #[rpc(method = "get_config")]
    pub rpc fn get_label_config<T: DeserializeStatic>(&self) -> HashMap<String, T>;

    #[rpc(method = "set_config")]
    pub rpc fn set_label_config(&self, config: &HashMap<String, impl Serialize>);
}
