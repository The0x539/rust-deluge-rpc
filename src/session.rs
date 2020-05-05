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

pub struct Session {
    stream: WriteHalf<TlsStream<TcpStream>>,
    prev_req_id: i64,
    listeners: mpsc::Sender<oneshot::Sender<Value>>,
    listener_loop: tokio::task::JoinHandle<io::Result<ReadHalf<TlsStream<TcpStream>>>>,
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

type RequestTuple = (i64, &'static str, Vec<Value>, HashMap<String, Value>);

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

    pub async fn request(&mut self, req: rpc::Request) -> io::Result<Value> {
        let request = self.prepare_request(req);
        let id = request.0;

        let (sender, receiver) = oneshot::channel();
        self.listeners.send(sender).await.expect("idk");

        self.send(request).await?;

        receiver.await.map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "wat"))
    }

    async fn handle_inbound(
        mut stream: ReadHalf<TlsStream<TcpStream>>,
        listeners: mpsc::Receiver<oneshot::Sender<Value>>,
        shutdown_signal: oneshot::Receiver<()>,
    ) -> io::Result<ReadHalf<TlsStream<TcpStream>>> {
        shutdown_signal.await.map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "wat"))?;
        Ok(stream)
    }

    async fn send(&mut self, req: RequestTuple) -> io::Result<()> {
        let body = compress(&rencode::to_bytes(&[req]).unwrap());
        self.stream.write_u8(1).await?;
        self.stream.write_u32(body.len() as u32).await?;
        self.stream.write_all(&body).await?;
        Ok(())
    }

    async fn recv(stream: &mut ReadHalf<TlsStream<TcpStream>>) -> io::Result<rpc::Inbound> {
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
        rpc::Inbound::from(&data).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    pub async fn close(self) -> io::Result<()> {
        self.shutdown_signal.send(()).map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "wat"))?;
        let read_half = self.listener_loop.await??;
        let mut stream = read_half.unsplit(self.stream);
        stream.shutdown().await
    }
}
