use serde::Serialize;

use std::sync::Arc;
use std::sync::atomic::AtomicI64;
use std::sync::atomic::Ordering;
use std::convert::TryFrom;

use tokio::io::{self, AsyncWriteExt};
use tokio::sync::{oneshot, mpsc, broadcast, Notify, Mutex};
use tokio::net::TcpStream;
use tokio::task::{self, JoinHandle};
use tokio_rustls::{TlsConnector, webpki};

use crate::types::{ReadStream, WriteStream, Result, Event, Stream, AuthLevel, RpcSender, DeserializeStatic};
use crate::encoding;
use crate::receiver::MessageReceiver;
use crate::wtf::NoCertificateVerification;

#[derive(Debug)]
pub struct Session {
    stream: Mutex<WriteStream>,
    cur_req_id: AtomicI64,
    listeners: Mutex<mpsc::Sender<(i64, RpcSender)>>,
    events: broadcast::Sender<Event>, // Only used for .subscribe()
    receiver_thread: JoinHandle<Result<ReadStream>>,
    shutdown_notify: Arc<Notify>,
    pub(crate) auth_level: AuthLevel,
}

impl Session {
    pub async fn connect(endpoint: impl tokio::net::ToSocketAddrs) -> io::Result<Self> {
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
        let receiver_thread = task::spawn(receiver.run(shutdown_notify.clone()));

        Ok(Self {
            stream: Mutex::new(writer),
            cur_req_id: AtomicI64::new(0),
            listeners: Mutex::new(request_send),
            events: event_send,
            receiver_thread,
            shutdown_notify,
            auth_level: AuthLevel::Nobody,
        })
    }

    pub async fn disconnect(self) -> std::result::Result<(), (Stream, io::Error)> {
        self.shutdown_notify.notify();
        let read_stream = self.receiver_thread.await.expect("receiver thread panicked").expect("receiver thread errored");
        let write_stream = self.stream.into_inner();
        let mut stream = read_stream.unsplit(write_stream);
        stream.shutdown().await.map_err(|e| (stream, e))
    }

    async fn send(&self, req: impl Serialize) -> io::Result<()> {
        let body = task::block_in_place(|| encoding::encode(&[req]).unwrap());

        let len = u32::try_from(body.len()).expect("request body too large");

        let mut stream = self.stream.lock().await;
        stream.write_u8(1).await?;
        stream.write_u32(len).await?;
        stream.write_all(&body).await?;
        stream.flush().await?;

        Ok(())
    }

    pub(crate) async fn request<T: DeserializeStatic, U: Serialize, V: Serialize>(
        &self,
        method: &'static str,
        args: U,
        kwargs: V,
    ) -> Result<T> {
        let id = self.cur_req_id.fetch_add(1, Ordering::Relaxed);
        let request = (id, method, args, kwargs);

        let (sender, receiver) = oneshot::channel();
        self.listeners
            .lock()
            .await
            .send((id, sender))
            .await
            .expect("rpc listeners channel closed");

        self.send(request).await?;

        let msg = receiver.await.expect("rpc response channel closed")?;

        // TODO: "response was compliant with calling convention, but not API" error
        let val: T = ron::Value::Seq(msg).into_rust().unwrap_or_else(|e| {
            panic!(
                "Error while converting value of type {}: {}",
                std::any::type_name::<T>(),
                e
            )
        });

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
