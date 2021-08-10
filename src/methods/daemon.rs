use std::collections::HashSet;

use deluge_rpc_macro::rpc_class;

use crate::session::Session;
use crate::types::{AuthLevel, EventKind, Result};

rpc_class! {
    impl Session::daemon;

    #[rpc(method = "info", auth_level = "Nobody")]
    pub rpc fn daemon_info(&self) -> String;

    #[rpc(auth_level = "Nobody", client_version = "2.0.4.dev23")]
    rpc fn _login(&self, username: &str, password: &str) -> AuthLevel;

    pub async fn login(&mut self, username: &str, password: &str) -> Result<AuthLevel> {
        self.auth_level = self._login(username, password).await?;
        Ok(self.auth_level)
    }

    rpc fn _set_event_interest(&self, events: &[EventKind]) -> bool;

    pub async fn set_event_interest(&self, events: &HashSet<EventKind>) -> Result<bool> {
        // TODO: Error variant for incorrect crate usage, like here.
        assert!(self.event_receiver_count() > 0, "Cannot set event interest without an active receiver handle (try calling .subscribe_events() first)");
        let keys: Vec<EventKind> = events.iter().copied().collect();
        self._set_event_interest(&keys).await
    }

    pub rpc fn shutdown(&self) -> ();

    pub rpc fn get_method_list(&self) -> Vec<String>;

    pub rpc fn get_version(&self) -> String;

    #[rpc(auth_level = "ReadOnly")]
    pub rpc fn authorized_call(&self, rpc: &str) -> bool;
}
