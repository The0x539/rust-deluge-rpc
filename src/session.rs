use serde_json::{Value, Map};
use serde::Deserialize;
use std::collections::HashMap;

use crate::rencode;
use crate::rpc;

use tokio_rustls::{TlsConnector, webpki, client::TlsStream};
use tokio::net::TcpStream;

use libflate::zlib;
use std::io::{Read, Write};
use std::sync::Arc;
use std::iter::FromIterator;

use tokio::prelude::*;
use tokio::sync::{oneshot, mpsc};
use tokio::io::{ReadHalf, WriteHalf};

type ReadStream = ReadHalf<TlsStream<TcpStream>>;
type WriteStream = WriteHalf<TlsStream<TcpStream>>;
type RequestTuple = (i64, &'static str, Vec<Value>, HashMap<String, Value>);
type RpcSender = oneshot::Sender<rpc::Result<Vec<Value>>>;

fn compress(input: &[u8]) -> Vec<u8> {
    let mut encoder = zlib::Encoder::new(Vec::new()).unwrap();
    encoder.write_all(input).unwrap();
    encoder.finish().into_result().unwrap()
}

fn decompress(input: &[u8]) -> Vec<u8> {
    let mut decoder = zlib::Decoder::new(input).unwrap();
    let mut buf = Vec::new();
    decoder.read_to_end(&mut buf).unwrap();
    buf
}

// TODO: Incorporate serde errors
pub enum Error {
    Network(io::Error),
    Rpc(rpc::Error),
    BadResponse(&'static str, Value),
    ChannelClosed(&'static str),
}

impl Error {
    pub fn expected(exp: &'static str, val: impl Into<Value>) -> Self {
        Self::BadResponse(exp, val.into())
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self { Self::Network(e) }
}
impl From<rpc::Error> for Error {
    fn from(e: rpc::Error) -> Self { Self::Rpc(e) }
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Network(e) => e.fmt(f),
            Self::Rpc(e) => e.fmt(f),
            Self::BadResponse(s, v) => write!(f, "Expected {}, got {}", s, serde_json::to_string(v).unwrap()),
            Self::ChannelClosed(s) => write!(f, "Unexpected closure of {} channel", s),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

pub struct Session {
    stream: WriteStream,
    prev_req_id: i64,
    listeners: mpsc::Sender<(i64, RpcSender)>,
}

struct MessageReceiver {
    stream: ReadStream,
    listeners: mpsc::Receiver<(i64, RpcSender)>,
    channels: HashMap<i64, RpcSender>,
}

impl MessageReceiver {
    fn spawn(stream: ReadStream, listeners: mpsc::Receiver<(i64, RpcSender)>) {
        tokio::spawn(Self::new(stream, listeners).run());
    }

    fn new(stream: ReadStream, listeners: mpsc::Receiver<(i64, RpcSender)>) -> Self {
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
        buf = decompress(&buf);
        let val: Value = rencode::from_bytes(&buf).unwrap();
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

// This is just a little bit ridiculous.
// For my use case, I at least have the cert *on my local filesystem*.
// I'd also be willing to copy it wherever.
struct NoCertificateVerification;

impl rustls::ServerCertVerifier for NoCertificateVerification {
    fn verify_server_cert(&self, _: &rustls::RootCertStore, _: &[rustls::Certificate], _: webpki::DNSNameRef<'_>, _: &[u8]) -> std::result::Result<rustls::ServerCertVerified, rustls::TLSError> {
        Ok(rustls::ServerCertVerified::assertion())
    }
}

macro_rules! request {
    ($self:ident, $($arg:tt),*$(,)?) => {
        $self.request(rpc_request!($($arg),*)).await?
    }
}

macro_rules! expect {
    ($val:expr, $pat:pat, $expected:expr, $result:expr) => {
        match $val {
            $pat => Ok($result),
            x => Err(Error::expected($expected, x)),
        }
    }
}

macro_rules! expect_val {
    ($val:expr, $pat:pat, $expected:expr, $result:expr) => {
        match $val.len() {
            1 => expect!($val.into_iter().next().unwrap(), $pat, $expected, $result),
            _ => Err(Error::expected(std::concat!("a list containing only ", $expected), $val)),
        }
    }
}

macro_rules! expect_seq {
    ($val:expr, $pat:pat, $expected_val:literal, $result:expr) => {
        $val.into_iter()
            .map(|x| match x {
                $pat => Ok($result.into()),
                v => {
                    let expected = std::concat!("a list where every item is ", $expected_val);
                    let actual = format!("a list containing {:?}", v);
                    Err(Error::expected(expected, actual))
                }
            })
            .collect()
    }
}

#[allow(dead_code)]
impl Session {
    fn prepare_request(&mut self, request: rpc::Request) -> RequestTuple {
        self.prev_req_id += 1;
        (self.prev_req_id, request.method, request.args, request.kwargs)
    }

    async fn send(&mut self, req: RequestTuple) -> Result<()> {
        let body = compress(&rencode::to_bytes(&[req]).unwrap());
        let mut msg = Vec::with_capacity(1 + 4 + body.len());
        byteorder::WriteBytesExt::write_u8(&mut msg, 1).unwrap();
        byteorder::WriteBytesExt::write_u32::<byteorder::BE>(&mut msg, body.len() as u32).unwrap();
        std::io::Write::write_all(&mut msg, &body).unwrap();
        self.stream.write_all(&msg).await?;
        Ok(())
    }

    async fn request(&mut self, req: rpc::Request) -> Result<Vec<Value>> {
        let request = self.prepare_request(req);
        let id = request.0;

        let (sender, receiver) = oneshot::channel();
        self.listeners.send((id, sender))
            .await
            .map_err(|_| Error::ChannelClosed("rpc listeners"))?;

        self.send(request).await?;

        let val = receiver.await.map_err(|_| Error::ChannelClosed("rpc response"))??;
        Ok(val)
    }

    pub async fn new(endpoint: impl tokio::net::ToSocketAddrs) -> Result<Self> {
        let mut tls_config = rustls::ClientConfig::new();
        //let server_pem_file = File::open("/home/the0x539/misc_software/dtui/experiment/certs/server.pem").unwrap();
        //tls_config.root_store.add_pem_file(&mut BufReader::new(pem_file)).unwrap();
        //tls_config.root_store.add_server_trust_anchors(&webpki_roots::TLS_SERVER_ROOTS);
        tls_config.dangerous().set_certificate_verifier(Arc::new(NoCertificateVerification));
        let tls_connector = TlsConnector::from(Arc::new(tls_config));

        let tcp_stream = TcpStream::connect(endpoint).await?;
        tcp_stream.set_nodelay(true)?;
        let stupid_dns_ref = webpki::DNSNameRef::try_from_ascii_str("foo").unwrap();
        let stream = tls_connector.connect(stupid_dns_ref, tcp_stream).await?;

        let (reader, writer) = io::split(stream);
        let (request_send, request_recv) = mpsc::channel(100);

        MessageReceiver::spawn(reader, request_recv);

        Ok(Self { stream: writer, prev_req_id: 0, listeners: request_send })
    }

    pub async fn daemon_info(&mut self) -> Result<String> {
        let val = request!(self, "daemon.info");
        expect_val!(val, Value::String(version), "a version number string", version)
    }

    pub async fn login(&mut self, username: &str, password: &str) -> Result<i64> {
        let val = request!(self, "daemon.login", [username, password], {"client_version" => "2.0.4.dev23"});
        expect_val!(
            val, Value::Number(num), "an i64 auth level",
            match num.as_i64() {
                Some(n) => n,
                None => return Err(Error::expected("an i64", Value::Number(num.clone()))),
            }
        )
    }

    // TODO: make private and add register_event_handler function that takes a channel or closure
    // (haven't decided which) and possibly an enum
    pub async fn set_event_interest(&mut self, events: &[&str]) -> Result<()> {
        let val = request!(self, "daemon.set_event_interest", [events]);
        expect_val!(val, Value::Bool(true), "true", ())
    }

    pub async fn shutdown(mut self) -> Result<()> {
        let val = request!(self, "daemon.shutdown");
        expect_val!(val, Value::Null, "null", ())?;
        self.close().await
    }

    pub async fn get_method_list<T: FromIterator<String>>(&mut self) -> Result<T> {
        let val = request!(self, "daemon.get_method_list");
        expect_seq!(val, Value::String(s), "a string", s)
    }

    pub async fn get_session_state<T: FromIterator<String>>(&mut self) -> Result<T> {
        let val = request!(self, "core.get_session_state");
        expect_seq!(val, Value::String(s), "a string", s)
    }

    pub async fn get_torrent_status(&mut self, torrent_id: &str, keys: &[&str]) -> Result<HashMap<String, Value>> {
        let val = request!(self, "core.get_torrent_status", [torrent_id, keys]);
        expect_val!(val, Value::Object(m), "a torrent's status", m.into_iter().collect())
    }

    pub async fn get_torrents_status<T, U>(
        &mut self,
        filter_dict: Option<Map<String, Value>>,
        keys: &[&str],
    ) -> Result<U>
        where T: for<'de> Deserialize<'de>,
              U: FromIterator<(String, T)>
    {
        let val = request!(self, "core.get_torrents_status", [filter_dict, keys]);
        let ret = expect_val!(val, Value::Object(m), "a map of torrents' statuses", m)?
            .into_iter()
            // TODO: serde_json is probably not the best way to transcode
            .map(|(id, status)| (id, serde_json::from_value(status).unwrap()))
            .collect();
        Ok(ret)
    }

    pub async fn close(mut self) -> Result<()> {
        self.stream.shutdown().await?;
        Ok(())
    }
}
