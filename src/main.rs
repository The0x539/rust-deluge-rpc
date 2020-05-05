//use serde::{Serialize, Deserialize};
//use serde_json;

use tokio::net::TcpStream;
use tokio::prelude::*;
use std::sync::Arc;
use std::fs;
use tokio_rustls::{TlsConnector, webpki};
use rustls;

mod rencode;
mod rpc;

/*
fn main() {
    let y = rpc_request!(5, "foo", [1, 2, 3], {"foo" => "bar"});
    let encoded = rencode::to_bytes(&y).unwrap();
    let decoded: serde_json::Value = rencode::from_bytes(&encoded).unwrap();
    println!("{}", serde_json::to_string_pretty(&decoded).unwrap());
}
*/

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

#[tokio::main()]
async fn main() {
    let mut tls_config = rustls::ClientConfig::new();

    //let server_pem_file = File::open("/home/the0x539/misc_software/dtui/experiment/certs/server.pem").unwrap();
    //tls_config.root_store.add_pem_file(&mut BufReader::new(pem_file)).unwrap();
    //tls_config.root_store.add_server_trust_anchors(&webpki_roots::TLS_SERVER_ROOTS);
    tls_config.dangerous().set_certificate_verifier(Arc::new(NoCertificateVerification));

    let tls_connector = TlsConnector::from(Arc::new(tls_config));

    let endpoint = fs::read_to_string("./experiment/endpoint").unwrap();
    let tcp_stream = TcpStream::connect(endpoint).await.unwrap();
    tcp_stream.set_nodelay(true).unwrap();
    let stupid_dns_ref = webpki::DNSNameRef::try_from_ascii_str("foo").unwrap();
    let mut stream = tls_connector.connect(stupid_dns_ref, tcp_stream).await.unwrap();

    stream.shutdown().await.unwrap();
}
