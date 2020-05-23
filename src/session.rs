use serde::{Serialize, de::DeserializeOwned};

use std::sync::Arc;
use std::collections::{HashMap, HashSet};
use std::net::{IpAddr, SocketAddr};

use tokio::prelude::*;
use tokio::sync::{oneshot, mpsc, broadcast};
use tokio::net::TcpStream;
use tokio_rustls::{TlsConnector, webpki};

use crate::encoding;
use crate::receiver::MessageReceiver;
use crate::wtf;
use crate::types::*;
use deluge_rpc_macro::rpc_method;

pub struct Session {
    stream: WriteStream,
    cur_req_id: i64,
    listeners: mpsc::Sender<(i64, RpcSender)>,
    events: broadcast::Sender<Event>, // Only used for .subscribe()
    auth_level: AuthLevel,
}

impl Session {
    pub async fn new(endpoint: impl tokio::net::ToSocketAddrs) -> Result<Self> {
        let mut tls_config = rustls::ClientConfig::new();
        tls_config.dangerous().set_certificate_verifier(Arc::new(wtf::NoCertificateVerification));
        let tls_connector = TlsConnector::from(Arc::new(tls_config));

        let tcp_stream = TcpStream::connect(endpoint).await?;
        let stupid_dns_ref = webpki::DNSNameRef::try_from_ascii_str("foo").unwrap();
        let stream = tls_connector.connect(stupid_dns_ref, tcp_stream).await?;

        let (reader, writer) = io::split(stream);
        let (request_send, request_recv) = mpsc::channel(100);
        let (event_send, _) = broadcast::channel(100);

        MessageReceiver::spawn(reader, request_recv, event_send.clone());

        Ok(Self {
            stream: writer,
            cur_req_id: 0,
            listeners: request_send,
            events: event_send,
            auth_level: AuthLevel::default(),
        })
    }

    pub async fn close(mut self) -> Result<()> {
        self.stream.shutdown().await?;
        Ok(())
    }

    async fn send(&mut self, req: impl Serialize) -> Result<()> {
        let body = encoding::encode(&[req]).unwrap();
        self.stream.write_u8(1).await?;
        self.stream.write_u32(body.len() as u32).await?;
        self.stream.write_all(&body).await?;
        self.stream.flush().await?;
        Ok(())
    }

    async fn request<T: DeserializeOwned, U: Serialize, V: Serialize>(
        &mut self,
        method: &'static str,
        args: U,
        kwargs: V,
    ) -> Result<T> {
        let id = self.cur_req_id;
        self.cur_req_id += 1;
        let request = (id, method, args, kwargs);

        let (sender, receiver) = oneshot::channel();
        self.listeners.send((id, sender))
            .await
            .map_err(|_| Error::ChannelClosed("rpc listeners"))?;

        self.send(request).await?;

        // This is an RPC result inside a oneshot result.
        let msg: List = match receiver.await {
            Ok(Ok(msg)) => Ok(msg), // Success
            Ok(Err(e)) => Err(Error::Rpc(e)), // RPC error
            Err(_) => Err(Error::ChannelClosed("rpc response")), // Channel error
        }?;

        // Kinda annoying that serde::de::Error is a trait rather than a struct.
        // Otherwise, I might go for bincode here.
        // Would also rather serialize on the receiver thread before sending.
        // If I can figure that out, I might be able to reduce usage of serde_yaml.
        // Not that serde_yaml's bad, but it's arbitrarily introducing a format.
        let val = rencode::from_bytes(rencode::to_bytes(&msg)?.as_slice())?;

        Ok(val)
    }

    // This gives a receiver for all events.
    // Subscribing to specific kinds of events would be... complicated in every facet.
    // Such functionality doesn't strike me as incredibly necessary.
    // For the time being, this isn't a particularly terrible burden to impose on this crate's users.
    pub fn subscribe_events(&self) -> broadcast::Receiver<Event> {
        self.events.subscribe()
    }

    #[rpc_method(class="daemon", method="info", auth_level="Nobody")]
    pub async fn daemon_info(&mut self) -> String;

    #[rpc_method(class="daemon", auth_level="Nobody", client_version="2.0.4.dev23")]
    pub async fn login(&mut self, username: &str, password: &str) -> AuthLevel {
        self.auth_level = val;
        Ok(self.auth_level)
    }

    #[rpc_method(class="daemon")]
    async fn _set_event_interest(&mut self, events: &[String]) -> bool;

    pub async fn set_event_interest(&mut self, events: &HashSet<EventKind>) -> Result<bool> {
        // TODO: Error variant for incorrect crate usage, like here.
        assert!(self.events.receiver_count() > 0, "Cannot set event interest without an active receiver handle (try calling .subscribe_events() first)");
        let keys = events
            .iter()
            .map(EventKind::key)
            .collect::<Vec<_>>();
        self._set_event_interest(&keys).await
    }

    #[rpc_method(class="daemon")]
    pub async fn shutdown(mut self) -> () {
        self.close().await
    }

    #[rpc_method(class="daemon")]
    pub async fn get_method_list(&mut self) -> Vec<String>;

    #[rpc_method]
    pub async fn add_torrent_file(&mut self, filename: &str, filedump: &str, options: &TorrentOptions) -> Option<InfoHash>;

    #[rpc_method]
    pub async fn add_torrent_files(&mut self, torrent_files: &[(&str, &str, &TorrentOptions)]);

    // TODO: accept a guaranteed-valid struct for the magnet URI
    #[rpc_method]
    pub async fn add_torrent_magnet(&mut self, uri: &str, options: &TorrentOptions) -> InfoHash;

    // TODO: accept guaranteed-valid structs for the URL and HTTP headers
    #[rpc_method]
    pub async fn add_torrent_url(&mut self, url: &str, options: &TorrentOptions, headers: Option<HashMap<String, String>>) -> Option<InfoHash>;

    #[rpc_method]
    async fn _connect_peer(&mut self, torrent_id: InfoHash, peer_ip: IpAddr, port: u16);

    pub async fn connect_peer(&mut self, torrent_id: InfoHash, peer_addr: SocketAddr) -> Result<()> {
        self._connect_peer(torrent_id, peer_addr.ip(), peer_addr.port()).await
    }

    #[rpc_method(auth_level="Admin")]
    pub async fn create_account(&mut self, username: &str, password: &str, auth_level: AuthLevel);

    // FORGIVE ME: I have no idea whether these types are correct.
    #[rpc_method]
    pub async fn create_torrent(
        &mut self,
        path: &str,
        comment: &str,
        target: &str,
        webseeds: &[&str],
        private: bool,
        created_by: &str,
        add_to_session: bool,
    );

    #[rpc_method]
    pub async fn disable_plugin(&mut self, plugin: &str);

    #[rpc_method]
    pub async fn enable_plugin(&mut self, plugin: &str);

    #[rpc_method]
    pub async fn force_reannounce(&mut self, torrent_ids: &[InfoHash]);

    #[rpc_method]
    pub async fn force_recheck(&mut self, torrent_ids: &[InfoHash]);

    // I hardcoded these same mappings into the AuthLevel enum, so this isn't particularly useful, but hey
    #[rpc_method]
    pub async fn get_auth_levels_mappings(&mut self) -> (HashMap<String, AuthLevel>, HashMap<AuthLevel, String>);

    #[rpc_method]
    pub async fn get_config<T: DeserializeOwned>(&mut self) -> HashMap<String, T>;

    #[rpc_method]
    pub async fn get_config_value<T: DeserializeOwned>(&mut self, key: &str) -> T;

    // TODO: ConfigQuery trait and/or ConfigKey enum
    #[rpc_method]
    pub async fn get_config_values<T: DeserializeOwned>(&mut self, keys: &[&str]) -> HashMap<String, T>;

    #[rpc_method]
    pub async fn get_enabled_plugins(&mut self) -> Vec<String>;

    #[rpc_method]
    pub async fn get_external_ip(&mut self) -> IpAddr;

    #[rpc_method]
    pub async fn get_filter_tree(&mut self, show_zero_hits: bool, hide_cat: &[&str]) -> HashMap<String, Vec<(String, u64)>>;

    #[rpc_method]
    pub async fn get_free_space(&mut self, path: Option<&str>) -> u64;

    // TODO: Account struct
    #[rpc_method(auth_level="Admin")]
    pub async fn get_known_accounts<T: DeserializeOwned>(&mut self) -> Vec<HashMap<String, T>>;

    #[rpc_method]
    pub async fn get_libtorrent_version(&mut self) -> String;

    #[rpc_method]
    pub async fn get_listen_port(&mut self) -> u16;

    // Uses a return value of -1 rather than a proper error to indicate filesystem errors.
    // Why? Is this a thin wrapper for some system call?
    #[rpc_method]
    pub async fn get_path_size(&mut self, path: &str) -> i64;

    // TODO: Proxy struct (low importance)
    #[rpc_method]
    pub async fn get_proxy<T: DeserializeOwned>(&mut self) -> T;

    #[rpc_method]
    pub async fn get_session_state(&mut self) -> Vec<InfoHash>;

    // TODO: SessionQuery trait and/or SessionKey enum
    #[rpc_method]
    pub async fn get_session_status<T: DeserializeOwned>(&mut self, keys: &[&str]) -> HashMap<String, T>;

    #[rpc_method(method="get_torrent_status")]
    pub async fn get_torrent_status_dyn<T: DeserializeOwned>(&mut self, torrent_id: InfoHash, keys: &[&str], diff: bool) -> T;

    pub async fn get_torrent_status<T: Query>(&mut self, torrent_id: InfoHash) -> Result<T> {
        self.get_torrent_status_dyn(torrent_id, T::keys(), false).await
    }

    pub async fn get_torrent_status_diff<T: Query>(&mut self, torrent_id: InfoHash) -> Result<T::Diff> {
        self.get_torrent_status_dyn(torrent_id, T::keys(), true).await
    }

    #[rpc_method(method="get_torrents_status")]
    pub async fn get_torrents_status_dyn<T: DeserializeOwned, U: Serialize>(&mut self, filter_dict: Option<HashMap<String, U>>, keys: &[&str], diff: bool) -> HashMap<InfoHash, T>;

    pub async fn get_torrents_status<T: Query, U: Serialize>(&mut self, filter_dict: Option<HashMap<String, U>>) -> Result<HashMap<InfoHash, T>> {
        self.get_torrents_status_dyn(filter_dict, T::keys(), false).await
    }

    pub async fn get_torrents_status_diff<T: Query, U: Serialize>(&mut self, filter_dict: Option<HashMap<String, U>>) -> Result<HashMap<InfoHash, T::Diff>> {
        self.get_torrents_status_dyn(filter_dict, T::keys(), true).await
    }

    #[rpc_method]
    pub async fn glob(&mut self, path: &str) -> Vec<String>;

    #[rpc_method(method="is_session_paused")]
    pub async fn is_libtorrent_session_paused(&mut self) -> bool;

    #[rpc_method]
    pub async fn move_storage(&mut self, torrent_ids: &[InfoHash], dest: &str);

    #[rpc_method(method="pause_session")]
    pub async fn pause_libtorrent_session(&mut self);

    #[rpc_method]
    pub async fn pause_torrent(&mut self, torrent_id: InfoHash);

    #[rpc_method]
    pub async fn pause_torrents(&mut self, torrent_ids: &[InfoHash]);

    // TODO: MagnetMetadata struct
    #[rpc_method]
    pub async fn prefetch_magnet_metadata<T: DeserializeOwned>(&mut self, magnet: &str, timeout: u64) -> (InfoHash, HashMap<String, T>);

    #[rpc_method]
    pub async fn queue_bottom(&mut self, torrent_ids: &[InfoHash]);

    #[rpc_method]
    pub async fn queue_down(&mut self, torrent_ids: &[InfoHash]);

    #[rpc_method]
    pub async fn queue_top(&mut self, torrent_ids: &[InfoHash]);

    #[rpc_method]
    pub async fn queue_up(&mut self, torrent_ids: &[InfoHash]);

    #[rpc_method(auth_level="Admin")]
    pub async fn remove_account(&mut self, username: &str);

    #[rpc_method]
    pub async fn remove_torrent(&mut self, torrent_id: InfoHash, remove_data: bool);

    #[rpc_method]
    pub async fn remove_torrents(&mut self, torrent_ids: &[InfoHash], remove_data: bool);

    #[rpc_method]
    pub async fn rename_files(&mut self, torrent_id: InfoHash, filenames: &[(u64, &str)]);

    #[rpc_method]
    pub async fn rename_folder(&mut self, torrent_id: InfoHash, folder: &str, new_folder: &str);

    #[rpc_method]
    pub async fn rescan_plugins(&mut self);

    #[rpc_method(method="resume_session")]
    pub async fn resume_libtorrent_session(&mut self);

    #[rpc_method]
    pub async fn resume_torrent(&mut self, torrent_id: InfoHash);

    #[rpc_method]
    pub async fn resume_torrents(&mut self, torrent_ids: &[InfoHash]);

    #[rpc_method]
    pub async fn set_config(&mut self, config: HashMap<String, impl Serialize>);

    #[rpc_method]
    pub async fn set_torrent_options(&mut self, torrent_ids: &[InfoHash], options: &TorrentOptions);

    #[rpc_method]
    pub async fn test_listen_port(&mut self) -> bool;

    #[rpc_method(auth_level="Admin")]
    pub async fn update_account(&mut self, username: &str, password: &str, auth_level: AuthLevel);

    #[rpc_method]
    pub async fn upload_plugin(&mut self, filename: &str, filedump: &[u8]);

    #[rpc_method(class="daemon")]
    pub async fn get_version(&mut self) -> String;

    #[rpc_method(class="daemon", auth_level="ReadOnly")]
    pub async fn authorized_call(&mut self, rpc: &str) -> bool;

    #[rpc_method(class="label")]
    pub async fn get_labels(&mut self) -> Vec<String>;

    #[rpc_method(class="label", method="add")]
    pub async fn add_label(&mut self, label_id: &str);

    #[rpc_method(class="label", method="remove")]
    pub async fn remove_label(&mut self, label_id: &str);

    #[rpc_method(class="label", method="get_options")]
    pub async fn get_label_options<T: DeserializeOwned>(&mut self, label_id: &str) -> HashMap<String, T>;

    #[rpc_method(class="label", method="set_options")]
    pub async fn set_label_options(&mut self, label_id: &str, options: HashMap<String, impl Serialize>);

    #[rpc_method(class="label", method="set_torrent")]
    pub async fn set_torrent_label(&mut self, torrent_id: InfoHash, label_id: &str);

    #[rpc_method(class="label", method="get_config")]
    pub async fn get_label_config<T: DeserializeOwned>(&mut self) -> HashMap<String, T>;

    #[rpc_method(class="label", method="set_config")]
    pub async fn set_label_config(&mut self, config: HashMap<String, impl Serialize>);
}
