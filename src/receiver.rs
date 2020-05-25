use std::collections::HashMap;
use std::sync::Arc;

use crate::encoding;
use crate::types::{ReadStream, RpcSender, Message, Event, Error, Result};

use tokio::io::AsyncReadExt;
use tokio::sync::{mpsc, broadcast, Notify};

pub struct MessageReceiver {
    stream: ReadStream,
    listeners: mpsc::Receiver<(i64, RpcSender)>,
    events: broadcast::Sender<Event>,
    channels: HashMap<i64, RpcSender>,
}

impl MessageReceiver {
    pub fn new(
        stream: ReadStream,
        listeners: mpsc::Receiver<(i64, RpcSender)>,
        events: broadcast::Sender<Event>,
    ) -> Self {
        Self { stream, listeners, events, channels: HashMap::new() }
    }

    async fn recv(&mut self) -> Result<Message> {
        // Get protocol version
        let ver = self.stream.read_u8().await?;
        // In theory, this could kill the session rather than the program, but eh
        assert_eq!(ver, 1, "Unknown DelugeRPC protocol version: {}", ver);

        // Get message length
        let len = self.stream.read_u32().await?;

        // Get message body
        let mut buf = vec![0; len as usize];
        self.stream.read_exact(&mut buf).await?;

        // Decode (decompress+deserialize) message body
        let message = encoding::decode(&buf)?;

        Ok(message)
    }

    async fn update_listeners(&mut self) -> Result<()> {
        use mpsc::error::TryRecvError;
        loop {
            match self.listeners.try_recv() {
                Ok((id, listener)) => {
                    // This is unrealistic if request IDs are chosen sanely.
                    assert!(!self.channels.contains_key(&id), "Request ID conflict for ID {}", id);
                    self.channels.insert(id, listener);
                },
                Err(TryRecvError::Empty) => return Ok(()),
                Err(TryRecvError::Closed) => return Err(Error::ChannelClosed("rpc listeners")),
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
                        self.update_listeners().await?;
                        self.channels
                            .remove(&request_id)
                            .expect(&format!("Received result for nonexistent request #{}", request_id))
                            .send(result)
                            .expect(&format!("Failed to send result for request #{}", request_id));
                    }
                    Message::Event(event) => {
                        self.events
                            .send(event)
                            .expect("Failed to send event");
                    }
                },
                _ = shutdown.notified() => return Ok(self.stream),
            }
        }
    }
}
