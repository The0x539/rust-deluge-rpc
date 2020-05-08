use serde_json::Value;
use std::collections::HashMap;

use crate::rencode;
use crate::rpc;

use tokio_rustls::{TlsConnector, webpki, client::TlsStream};
use tokio::net::TcpStream;

use libflate::zlib;
use std::io::{Read, Write};
use std::sync::Arc;

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

fn broken_pipe_err(channel_name: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::BrokenPipe, format!("unexpected closure of {} channel", channel_name))
}

fn invalid_data_err(e: impl Into<Box<dyn std::error::Error + 'static + Send + Sync>>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, e)
}

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

    async fn recv(&mut self) -> io::Result<rpc::Inbound> {
        let ver = self.stream.read_u8().await?;
        if ver != 1 {
            panic!("unsupported DelugeRPC protocol version");
        }
        let len = self.stream.read_u32().await?;
        let mut buf = vec![0; len as usize];
        self.stream.read_exact(&mut buf).await?;
        buf = decompress(&buf);
        let val: serde_json::Value = rencode::from_bytes(&buf).unwrap();
        let data: Vec<serde_json::Value> = val.as_array().unwrap().clone();
        rpc::Inbound::from(&data).map_err(|e| invalid_data_err(e))
    }

    async fn update_listeners(&mut self) -> io::Result<()> {
        loop {
            match self.listeners.try_recv() {
                Ok((id, listener)) => {
                    // This is unrealistic if request IDs are chosen sanely.
                    assert!(!self.channels.contains_key(&id), "Request ID conflict for ID {}", id);
                    self.channels.insert(id, listener);
                },
                Err(mpsc::error::TryRecvError::Empty) => return Ok(()),
                Err(mpsc::error::TryRecvError::Closed) => return Err(broken_pipe_err("rpc listeners")),
            }
        }
    }

    async fn run(mut self) -> io::Result<()> {
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
                        .expect(&format!("Received result for nonexistent request ID {}", request_id))
                        .send(result)
                        .unwrap();
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
    fn verify_server_cert(&self, _: &rustls::RootCertStore, _: &[rustls::Certificate], _: webpki::DNSNameRef<'_>, _: &[u8]) -> Result<rustls::ServerCertVerified, rustls::TLSError> {
        Ok(rustls::ServerCertVerified::assertion())
    }
}

impl Session {
    fn prepare_request(&mut self, request: rpc::Request) -> RequestTuple {
        self.prev_req_id += 1;
        (self.prev_req_id, request.method, request.args, request.kwargs)
    }

    async fn send(&mut self, req: RequestTuple) -> io::Result<()> {
        let body = compress(&rencode::to_bytes(&[req]).unwrap());
        let mut msg = Vec::with_capacity(1 + 4 + body.len());
        byteorder::WriteBytesExt::write_u8(&mut msg, 1).unwrap();
        byteorder::WriteBytesExt::write_u32::<byteorder::BE>(&mut msg, body.len() as u32).unwrap();
        std::io::Write::write_all(&mut msg, &body).unwrap();
        self.stream.write_all(&msg).await
    }

    pub async fn request(&mut self, req: rpc::Request) -> io::Result<rpc::Result<Vec<Value>>> {
        let request = self.prepare_request(req);
        let id = request.0;

        let (sender, receiver) = oneshot::channel();
        self.listeners.send((id, sender)).await.map_err(|_| broken_pipe_err("rpc listeners"))?;

        self.send(request).await?;

        receiver.await.map_err(|_| broken_pipe_err("rpc response"))
    }

    pub async fn new(endpoint: impl tokio::net::ToSocketAddrs) -> io::Result<Self> {
        let mut tls_config = rustls::ClientConfig::new();
        //let server_pem_file = File::open("/home/the0x539/misc_software/dtui/experiment/certs/server.pem").unwrap();
        //tls_config.root_store.add_pem_file(&mut BufReader::new(pem_file)).unwrap();
        //tls_config.root_store.add_server_trust_anchors(&webpki_roots::TLS_SERVER_ROOTS);
        tls_config.dangerous().set_certificate_verifier(Arc::new(NoCertificateVerification));
        let tls_connector = TlsConnector::from(Arc::new(tls_config));

        let tcp_stream = TcpStream::connect(endpoint).await.unwrap();
        tcp_stream.set_nodelay(true).unwrap();
        let stupid_dns_ref = webpki::DNSNameRef::try_from_ascii_str("foo").unwrap();
        let stream = tls_connector.connect(stupid_dns_ref, tcp_stream).await.unwrap();

        let (reader, writer) = io::split(stream);
        let (request_send, request_recv) = mpsc::channel(100);

        MessageReceiver::spawn(reader, request_recv);

        Ok(Self { stream: writer, prev_req_id: 0, listeners: request_send })
    }

    pub async fn close(mut self) -> io::Result<()> {
        self.stream.shutdown().await
    }
}
