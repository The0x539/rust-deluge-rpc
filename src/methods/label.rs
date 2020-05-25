use serde::{Serialize, de::DeserializeOwned};

use std::collections::HashMap;

use deluge_rpc_macro::rpc_class;

use crate::types::{Result, AuthLevel, InfoHash};
use crate::session::Session;

rpc_class! {
    impl Session::label;

    pub rpc fn get_labels(&mut self) -> Vec<String>;

    #[rpc(method = "add")]
    pub rpc fn add_label(&mut self, label_id: &str);

    #[rpc(method = "remove")]
    pub rpc fn remove_label(&mut self, label_id: &str);

    #[rpc(method = "get_options")]
    pub rpc fn get_label_options<T: DeserializeOwned>(&mut self, label_id: &str) -> HashMap<String, T>;

    #[rpc(method = "set_options")]
    pub rpc fn set_label_options(&mut self, label_id: &str, options: &HashMap<String, impl Serialize>);

    #[rpc(method = "set_torrent")]
    pub rpc fn set_torrent_label(&mut self, torrent_id: InfoHash, label_id: &str);

    #[rpc(method = "get_config")]
    pub rpc fn get_label_config<T: DeserializeOwned>(&mut self) -> HashMap<String, T>;

    #[rpc(method = "set_config")]
    pub rpc fn set_label_config(&mut self, config: &HashMap<String, impl Serialize>);
}
