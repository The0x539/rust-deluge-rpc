use tokio::net::TcpStream;
use tokio::prelude::*;
use std::sync::Arc;
use tokio_rustls::{TlsConnector, webpki};
use rustls;
use libflate::zlib;
use std::io::{Read, Write};

mod rencode;
mod rpc;

// This is just a little bit ridiculous.
// For my use case, I at least have the cert *on my local filesystem*.
// I'd also be willing to copy it wherever.
struct NoCertificateVerification;

impl rustls::ServerCertVerifier for NoCertificateVerification {
    fn verify_server_cert(&self,
                          _roots: &rustls::RootCertStore,
                          _presented_certs: &[rustls::Certificate],
                          _dns_name: webpki::DNSNameRef<'_>,
                          _ocsp: &[u8]) -> Result<rustls::ServerCertVerified, rustls::TLSError> {
        Ok(rustls::ServerCertVerified::assertion())
    }
}

fn read_file(path: &'static str) -> String {
    std::fs::read_to_string(path).unwrap()
}

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

async fn send<W: AsyncWrite + Unpin>(stream: &mut W, req: rpc::RpcRequest) -> io::Result<()> {
    let body = compress(&rencode::to_bytes(&[req]).unwrap());
    stream.write_u8(1).await?;
    stream.write_u32(body.len() as u32).await?;
    stream.write_all(&body).await?;
    Ok(())
}

async fn recv<R: AsyncRead + Unpin>(stream: &mut R) -> io::Result<Vec<u8>> {
    let ver = stream.read_u8().await?;
    if ver != 1 {
        panic!("unsupported DelugeRPC protocol version");
    }
    let len = stream.read_u32().await?;
    let mut buf = vec![0; len as usize];
    stream.read_exact(&mut buf).await?;
    Ok(decompress(&buf))
}

#[tokio::main()]
async fn main() {
    let mut tls_config = rustls::ClientConfig::new();

    //let server_pem_file = File::open("/home/the0x539/misc_software/dtui/experiment/certs/server.pem").unwrap();
    //tls_config.root_store.add_pem_file(&mut BufReader::new(pem_file)).unwrap();
    //tls_config.root_store.add_server_trust_anchors(&webpki_roots::TLS_SERVER_ROOTS);
    tls_config.dangerous().set_certificate_verifier(Arc::new(NoCertificateVerification));

    let tls_connector = TlsConnector::from(Arc::new(tls_config));

    let tcp_stream = TcpStream::connect(read_file("./experiment/endpoint")).await.unwrap();
    tcp_stream.set_nodelay(true).unwrap();
    let stupid_dns_ref = webpki::DNSNameRef::try_from_ascii_str("foo").unwrap();
    let mut stream = tls_connector.connect(stupid_dns_ref, tcp_stream).await.unwrap();

    let user = read_file("./experiment/username");
    let pass = read_file("./experiment/password");
    let req = rpc_request!(37, "daemon.login", [user, pass]);
    send(&mut stream, req).await.unwrap();

    let buf = recv(&mut stream).await.unwrap();
    let val: serde_json::Value = rencode::from_bytes(&buf).unwrap();

    println!("{}", serde_json::to_string_pretty(&val).unwrap());

    stream.shutdown().await.unwrap();
}
