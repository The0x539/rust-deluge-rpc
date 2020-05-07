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
type RpcSender = (i64, oneshot::Sender<rpc::Result>);

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
    listeners: mpsc::Sender<RpcSender>,
    listener_loop: tokio::task::JoinHandle<io::Result<ReadStream>>,
    shutdown_signal: oneshot::Sender<()>,
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
        let (shutdown_send, shutdown_recv) = oneshot::channel();

        let listener_loop = tokio::spawn(Self::handle_inbound(reader, request_recv, shutdown_recv));

        let obj = Self {
            stream: writer,
            prev_req_id: 0,
            listeners: request_send,
            listener_loop,
            shutdown_signal: shutdown_send,
        };

        Ok(obj)
    }

    pub async fn request(&mut self, req: rpc::Request) -> io::Result<rpc::Result> {
        let request = self.prepare_request(req);
        let id = request.0;

        let (sender, receiver) = oneshot::channel();
        self.listeners.send((id, sender)).await.map_err(|_| broken_pipe_err("rpc listeners"))?;

        self.send(request).await?;

        receiver.await.map_err(|_| broken_pipe_err("rpc response"))
    }

    async fn handle_inbound(
        mut stream: ReadStream,
        mut listeners: mpsc::Receiver<RpcSender>,
        mut shutdown_signal: oneshot::Receiver<()>,
    ) -> io::Result<ReadStream> {
        let mut channels = HashMap::new();
        loop {
            match shutdown_signal.try_recv() {
                Ok(()) => return Ok(stream),
                Err(oneshot::error::TryRecvError::Empty) => (),
                Err(oneshot::error::TryRecvError::Closed) => {
                    return Err(broken_pipe_err("shutdown signal"));
                }
            }
            match listeners.try_recv() {
                Ok((id, listener)) => {
                    if let Some(_) = channels.insert(id, listener) {
                        panic!("Request ID conflict for ID {}", id);
                    }
                },
                Err(mpsc::error::TryRecvError::Empty) => (),
                Err(mpsc::error::TryRecvError::Closed) => {
                    return Err(broken_pipe_err("rpc listeners"));
                }
            }
            // TODO: this needs to poll non-blockingly like the other two
            match Self::recv(&mut stream).await? {
                // TODO: fix race condition. I make sure to send the listener to this thread first,
                // but I might still get a response before storing it in the channels object.
                // Perhaps I could poll for new listeners whenever we get a response.
                rpc::Inbound::Response { request_id, result } => {
                    channels
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

    async fn send(&mut self, req: RequestTuple) -> io::Result<()> {
        let body = compress(&rencode::to_bytes(&[req]).unwrap());
        self.stream.write_u8(1).await?;
        self.stream.write_u32(body.len() as u32).await?;
        self.stream.write_all(&body).await
    }

    async fn recv(stream: &mut ReadStream) -> io::Result<rpc::Inbound> {
        let ver = stream.read_u8().await?;
        if ver != 1 {
            panic!("unsupported DelugeRPC protocol version");
        }
        let len = stream.read_u32().await?;
        let mut buf = vec![0; len as usize];
        stream.read_exact(&mut buf).await?;
        buf = decompress(&buf);
        let val: serde_json::Value = rencode::from_bytes(&buf).unwrap();
        let data: Vec<serde_json::Value> = val.as_array().unwrap().clone();
        rpc::Inbound::from(&data).map_err(|e| invalid_data_err(e))
    }

    pub async fn close(self) -> io::Result<()> {
        self.shutdown_signal.send(()).map_err(|_| broken_pipe_err("shutdown signal"))?;
        let read_half = self.listener_loop.await??;
        let mut stream = read_half.unsplit(self.stream);
        stream.shutdown().await
    }
}
