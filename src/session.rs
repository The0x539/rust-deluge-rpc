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
    stream: TlsStream<TcpStream>,
    request_id: i64,
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
        self.request_id += 1;
        (self.request_id, request.method, request.args, request.kwargs)
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

        Ok(Self {
            stream,
            request_id: 0,
        })
    }

    pub async fn send(&mut self, req: rpc::Request) -> io::Result<()> {
        let body = compress(&rencode::to_bytes(&[self.prepare_request(req)]).unwrap());
        self.stream.write_u8(1).await?;
        self.stream.write_u32(body.len() as u32).await?;
        self.stream.write_all(&body).await?;
        Ok(())
    }

    pub async fn recv(&mut self) -> io::Result<rpc::Inbound> {
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
        rpc::Inbound::from(&data).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    pub async fn close(mut self) -> io::Result<()> {
        self.stream.shutdown().await
    }
}
