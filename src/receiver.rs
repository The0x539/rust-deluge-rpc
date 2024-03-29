use fnv::FnvHashMap;
use std::sync::Arc;

use crate::encoding;
use crate::types::{Event, Message, ReadStream, Result, RpcSender};

use tokio::io::AsyncReadExt;
use tokio::sync::{broadcast, mpsc, Notify};
use tokio::task;

use futures::future::FutureExt;

pub struct MessageReceiver {
    stream: ReadStream,
    listeners: mpsc::Receiver<(i64, RpcSender)>,
    events: broadcast::Sender<Event>,
    channels: FnvHashMap<i64, RpcSender>,
}

impl MessageReceiver {
    pub fn new(
        stream: ReadStream,
        listeners: mpsc::Receiver<(i64, RpcSender)>,
        events: broadcast::Sender<Event>,
    ) -> Self {
        Self {
            stream,
            listeners,
            events,
            channels: FnvHashMap::default(),
        }
    }

    async fn recv(&mut self) -> Result<Message> {
        // Get protocol version
        let ver = self.stream.read_u8().await?;
        // In theory, this could kill the session rather than the program, but eh
        assert_eq!(ver, 1, "Unknown DelugeRPC protocol version: {}", ver);

        // Get message length
        let len = self.stream.read_u32().await? as usize;

        // Get message body
        let mut buf = vec![0u8; len];
        self.stream.read_exact(&mut buf).await?;

        // Decode (decompress+deserialize) message body
        let message = task::spawn_blocking(move || encoding::decode(&buf))
            .await
            .unwrap()?;

        Ok(message)
    }

    fn update_listeners(&mut self) {
        while let Some(res) = self.listeners.recv().now_or_never() {
            let (id, listener) = res.expect("rpc listeners channel closed");

            if let Some(_old_listener) = self.channels.insert(id, listener) {
                // This is unrealistic if request IDs are chosen sanely.
                panic!("Request ID conflict for ID {}", id);
            }
        }
    }

    pub async fn run(mut self, shutdown: Arc<Notify>) -> Result<ReadStream> {
        loop {
            tokio::select! {
                message = self.recv() => match message? {
                    Message::Response { request_id, result } => {
                        // request() always sends the listener oneshot before invoking RPC
                        // therefore, if we're handling a valid response, it's guaranteed that the
                        // request's oneshot either is already in our hashmap or is in the mpsc.
                        // doing this here turns that guarantee of (A or B) into a guarantee of A.
                        self.update_listeners();
                        self.channels
                            .remove(&request_id)
                            .unwrap_or_else(|| panic!("Received result for nonexistent request #{}", request_id))
                            .send(result)
                            // The application is free to drop the receiver for any reason.
                            // If it does, it's not our problem; just discard the result accordingly.
                            .unwrap_or(());
                    }
                    Message::Event(event) => {
                        // See above
                        self.events.send(event).unwrap_or(0);
                    }
                },
                _ = shutdown.notified() => return Ok(self.stream),
            }
        }
    }
}
