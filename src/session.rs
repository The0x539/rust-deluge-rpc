use serde::{Serialize, de::DeserializeOwned};

use std::sync::Arc;
use std::convert::TryFrom;

use tokio::io::{self, AsyncWriteExt};
use tokio::sync::{oneshot, mpsc, broadcast, Notify};
use tokio::net::TcpStream;
use tokio::task::JoinHandle;
use tokio_rustls::{TlsConnector, webpki};

use crate::types::{ReadStream, WriteStream, List, Result, Event, Stream, AuthLevel, Error, RpcSender};
use crate::encoding;
use crate::receiver::MessageReceiver;
use crate::wtf::NoCertificateVerification;

#[derive(Debug)]
pub struct Session {
    stream: WriteStream,
    cur_req_id: i64,
    listeners: mpsc::Sender<(i64, RpcSender)>,
    events: broadcast::Sender<Event>, // Only used for .subscribe()
    receiver_thread: JoinHandle<Result<ReadStream>>,
    shutdown_notify: Arc<Notify>,
    pub(crate) auth_level: AuthLevel,
}

impl Session {
    pub async fn connect(endpoint: impl tokio::net::ToSocketAddrs) -> Result<Self> {
        let mut tls_config = rustls::ClientConfig::new();
        tls_config.dangerous().set_certificate_verifier(Arc::new(NoCertificateVerification));
        let tls_connector = TlsConnector::from(Arc::new(tls_config));

        let tcp_stream = TcpStream::connect(endpoint).await?;
        let stupid_dns_ref = webpki::DNSNameRef::try_from_ascii_str("foo").unwrap();
        let stream = tls_connector.connect(stupid_dns_ref, tcp_stream).await?;

        let (reader, writer) = io::split(stream);
        let (request_send, request_recv) = mpsc::channel(100);
        let (event_send, _) = broadcast::channel(100);
        let shutdown_notify = Arc::new(Notify::new());

        let receiver = MessageReceiver::new(reader, request_recv, event_send.clone());
        let receiver_thread = tokio::spawn(receiver.run(shutdown_notify.clone()));

        Ok(Self {
            stream: writer,
            cur_req_id: 0,
            listeners: request_send,
            events: event_send,
            receiver_thread,
            shutdown_notify,
            auth_level: AuthLevel::Nobody,
        })
    }

    pub async fn disconnect(self) -> std::result::Result<(), (Stream, io::Error)> {
        self.shutdown_notify.notify();
        let read_stream = self.receiver_thread.await.expect("receiver thread panicked").expect("receiver thread errored");
        let mut stream = read_stream.unsplit(self.stream);
        stream.shutdown().await.map_err(|e| (stream, e))
    }

    async fn send(&mut self, req: impl Serialize) -> Result<()> {
        let body = encoding::encode(&[req]).unwrap();
        let len = u32::try_from(body.len()).expect("request body too large");
        self.stream.write_u8(1).await?;
        self.stream.write_u32(len).await?;
        self.stream.write_all(&body).await?;
        self.stream.flush().await?;
        Ok(())
    }

    pub(crate) async fn request<T: DeserializeOwned, U: Serialize, V: Serialize>(
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

    pub(crate) fn event_receiver_count(&self) -> usize {
        self.events.receiver_count()
    }
}
