use serde_json::Value;
use std::collections::HashMap;

use crate::encoding;
use crate::rpc;
use crate::error::{Error, Result};

use tokio_rustls::client::TlsStream;
use tokio::net::TcpStream;

use tokio::prelude::*;
use tokio::sync::{oneshot, mpsc};
use tokio::io::ReadHalf;

type ReadStream = ReadHalf<TlsStream<TcpStream>>;
type RpcSender = oneshot::Sender<rpc::Result<Vec<Value>>>;

pub struct MessageReceiver {
    stream: ReadStream,
    listeners: mpsc::Receiver<(i64, RpcSender)>,
    channels: HashMap<i64, RpcSender>,
}

impl MessageReceiver {
    pub fn spawn(stream: ReadStream, listeners: mpsc::Receiver<(i64, RpcSender)>) {
        tokio::spawn(Self::new(stream, listeners).run());
    }

    pub fn new(stream: ReadStream, listeners: mpsc::Receiver<(i64, RpcSender)>) -> Self {
        Self {
            stream,
            listeners,
            channels: HashMap::new(),
        }
    }

    async fn recv(&mut self) -> Result<rpc::Inbound> {
        let ver = self.stream.read_u8().await?;
        // In theory, this could kill the session rather than the program, but eh
        assert_eq!(ver, 1, "Unknown DelugeRPC protocol version: {}", ver);
        let len = self.stream.read_u32().await?;
        let mut buf = vec![0; len as usize];
        self.stream.read_exact(&mut buf).await?;
        let val: Value = encoding::decode(&buf).unwrap();
        let data = val.as_array().ok_or(Error::expected("a list", val.clone()))?;
        rpc::Inbound::from(data).map_err(|_| Error::expected("a valid RPC message", data.as_slice()))
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

    async fn run(mut self) -> Result<()> {
        loop {
            match self.recv().await? {
                rpc::Inbound::Response { request_id, result } => {
                    // request() always sends the listener oneshot before invoking RPC
                    // therefore, if we're handling a valid response, it's guaranteed that the
                    // request's oneshot either is already in our hashmap or is in the mpsc.
                    // doing this here turns that guarantee of (A or B) into a guarantee of A.
                    self.update_listeners().await?;
                    self.channels
                        .remove(&request_id)
                        .expect(&format!("Received result for nonexistent request #{}", request_id))
                        .send(result)
                        .expect(&format!("Failed to send result for request with #{}", request_id));
                }
                rpc::Inbound::Event { event_name, data } => {
                    // TODO: Event handler registration or something
                    println!("Received event {}: {:?}", event_name, data);
                }
            }
        }
    }
}
