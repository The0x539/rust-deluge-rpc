use serde::Serialize;

use fnv::FnvHashMap;

use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};

use deluge_rpc_macro::rpc_class;

use crate::types::{Result, TorrentOptions, AuthLevel, InfoHash, Query, FilterKey, FilterDict, DeserializeStatic};
use crate::session::Session;

rpc_class! {
    impl Session::core;

    pub rpc fn add_torrent_file(&self, filename: &str, filedump: &str, options: &TorrentOptions) -> Option<InfoHash>;

    pub rpc fn add_torrent_files(&self, torrent_files: &[(&str, &str, &TorrentOptions)]);

    // TODO: accept a guaranteed-valid struct for the magnet URI
    pub rpc fn add_torrent_magnet(&self, uri: &str, options: &TorrentOptions) -> InfoHash;

    // TODO: accept guaranteed-valid structs for the URL and HTTP headers
    pub rpc fn add_torrent_url(&self, url: &str, options: &TorrentOptions, headers: Option<&HashMap<String, String>>) -> Option<InfoHash>;

    rpc fn _connect_peer(&self, torrent_id: InfoHash, peer_ip: IpAddr, port: u16);

    pub async fn connect_peer(&self, torrent_id: InfoHash, peer_addr: SocketAddr) -> Result<()> {
        self._connect_peer(torrent_id, peer_addr.ip(), peer_addr.port()).await
    }

    #[rpc(auth_level = "Admin")]
    pub rpc fn create_account(&self, username: &str, password: &str, auth_level: AuthLevel);

    // FORGIVE ME: I have no idea whether these types are correct.
    pub rpc fn create_torrent(
        &self,
        path: &str,
        comment: &str,
        target: &str,
        webseeds: &[&str],
        private: bool,
        created_by: &str,
        add_to_session: bool,
    );

    pub rpc fn disable_plugin(&self, plugin: &str);

    pub rpc fn enable_plugin(&self, plugin: &str);

    pub rpc fn force_reannounce(&self, torrent_ids: &[InfoHash]);

    pub rpc fn force_recheck(&self, torrent_ids: &[InfoHash]);

    // I hardcoded these same mappings into the AuthLevel enum, so this isn't particularly useful, but hey
    pub rpc fn get_auth_levels_mappings(&self) -> (HashMap<String, AuthLevel>, HashMap<AuthLevel, String>);

    pub rpc fn get_config<T: DeserializeStatic>(&self) -> HashMap<String, T>;

    pub rpc fn get_config_value<T: DeserializeStatic>(&self, key: &str) -> T;

    #[rpc(method = "get_config_values")]
    pub rpc fn get_config_values_dyn<T: DeserializeStatic>(&self, keys: &[&str]) -> T;

    pub async fn get_config_values<T: Query>(&self) -> Result<T> {
        self.get_config_values_dyn(T::keys()).await
    }

    pub rpc fn get_enabled_plugins(&self) -> Vec<String>;

    pub rpc fn get_external_ip(&self) -> IpAddr;

    pub rpc fn get_filter_tree(&self, show_zero_hits: bool, hide_cat: &[&str]) -> FnvHashMap<FilterKey, Vec<(String, u64)>>;

    pub rpc fn get_free_space(&self, path: Option<&str>) -> u64;

    // TODO: Account struct
    #[rpc(auth_level = "Admin")]
    pub rpc fn get_known_accounts<T: DeserializeStatic>(&self) -> Vec<HashMap<String, T>>;

    pub rpc fn get_libtorrent_version(&self) -> String;

    pub rpc fn get_listen_port(&self) -> u16;

    // Uses a return value of -1 rather than a proper error to indicate filesystem errors.
    // Why? Is this a thin wrapper for some system call?
    pub rpc fn get_path_size(&self, path: &str) -> i64;

    // TODO: Proxy struct (low importance)
    pub rpc fn get_proxy<T: DeserializeStatic>(&self) -> T;

    pub rpc fn get_session_state(&self) -> Vec<InfoHash>;

    #[rpc(method = "get_session_status")]
    pub rpc fn get_session_status_dyn<T: DeserializeStatic>(&self, keys: &[&str]) -> T;

    pub async fn get_session_status<T: Query>(&self) -> Result<T> {
        self.get_session_status_dyn(T::keys()).await
    }

    #[rpc(method = "get_torrent_status")]
    pub rpc fn get_torrent_status_dyn<T: DeserializeStatic>(&self, torrent_id: InfoHash, keys: &[&str], diff: bool) -> T;

    pub async fn get_torrent_status<T: Query>(&self, torrent_id: InfoHash) -> Result<T> {
        self.get_torrent_status_dyn(torrent_id, T::keys(), false).await
    }

    pub async fn get_torrent_status_diff<T: Query>(&self, torrent_id: InfoHash) -> Result<T::Diff> {
        self.get_torrent_status_dyn(torrent_id, T::keys(), true).await
    }

    #[rpc(method = "get_torrents_status")]
    pub rpc fn get_torrents_status_dyn<T: DeserializeStatic>(&self, filter_dict: Option<&FilterDict>, keys: &[&str], diff: bool) -> FnvHashMap<InfoHash, T>;

    pub async fn get_torrents_status<T: Query>(&self, filter_dict: Option<&FilterDict>) -> Result<FnvHashMap<InfoHash, T>> {
        self.get_torrents_status_dyn(filter_dict, T::keys(), false).await
    }

    pub async fn get_torrents_status_diff<T: Query>(&self, filter_dict: Option<&FilterDict>) -> Result<FnvHashMap<InfoHash, T::Diff>> {
        self.get_torrents_status_dyn(filter_dict, T::keys(), true).await
    }

    pub rpc fn glob(&self, path: &str) -> Vec<String>;

    #[rpc(method = "is_session_paused")]
    pub rpc fn is_libtorrent_session_paused(&self) -> bool;

    pub rpc fn move_storage(&self, torrent_ids: &[InfoHash], dest: &str);

    #[rpc(method = "pause_session")]
    pub rpc fn pause_libtorrent_session(&self);

    pub rpc fn pause_torrent(&self, torrent_id: InfoHash);

    pub rpc fn pause_torrents(&self, torrent_ids: &[InfoHash]);

    // TODO: MagnetMetadata struct
    pub rpc fn prefetch_magnet_metadata<T: DeserializeStatic>(&self, magnet: &str, timeout: u64) -> (InfoHash, HashMap<String, T>);

    pub rpc fn queue_bottom(&self, torrent_ids: &[InfoHash]);

    pub rpc fn queue_down(&self, torrent_ids: &[InfoHash]);

    pub rpc fn queue_top(&self, torrent_ids: &[InfoHash]);

    pub rpc fn queue_up(&self, torrent_ids: &[InfoHash]);

    #[rpc(auth_level = "Admin")]
    pub rpc fn remove_account(&self, username: &str);

    pub rpc fn remove_torrent(&self, torrent_id: InfoHash, remove_data: bool) -> bool;

    // TODO: HashMap<InfoHash, Error>?
    pub rpc fn remove_torrents(&self, torrent_ids: &[InfoHash], remove_data: bool) -> Vec<(InfoHash, String)>;

    pub rpc fn rename_files(&self, torrent_id: InfoHash, filenames: &[(u64, &str)]);

    pub rpc fn rename_folder(&self, torrent_id: InfoHash, folder: &str, new_folder: &str);

    pub rpc fn rescan_plugins(&self);

    #[rpc(method = "resume_session")]
    pub rpc fn resume_libtorrent_session(&self);

    pub rpc fn resume_torrent(&self, torrent_id: InfoHash);

    pub rpc fn resume_torrents(&self, torrent_ids: &[InfoHash]);

    pub rpc fn set_config(&self, config: &HashMap<String, impl Serialize>);

    pub rpc fn set_torrent_options(&self, torrent_ids: &[InfoHash], options: &TorrentOptions);

    pub rpc fn test_listen_port(&self) -> bool;

    #[rpc(auth_level = "Admin")]
    pub rpc fn update_account(&self, username: &str, password: &str, auth_level: AuthLevel);

    pub rpc fn upload_plugin(&self, filename: &str, filedump: &[u8]);
}
