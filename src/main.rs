use tokio::net::TcpStream;
use tokio::prelude::*;
use std::sync::Arc;
use tokio_rustls::{TlsConnector, webpki};
use rustls;

mod rencode;
mod rpc;
mod session;

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

#[tokio::main()]
async fn main() {
    // TODO: move all this crap into Session::new

    let mut tls_config = rustls::ClientConfig::new();

    //let server_pem_file = File::open("/home/the0x539/misc_software/dtui/experiment/certs/server.pem").unwrap();
    //tls_config.root_store.add_pem_file(&mut BufReader::new(pem_file)).unwrap();
    //tls_config.root_store.add_server_trust_anchors(&webpki_roots::TLS_SERVER_ROOTS);
    tls_config.dangerous().set_certificate_verifier(Arc::new(NoCertificateVerification));

    let tls_connector = TlsConnector::from(Arc::new(tls_config));

    let tcp_stream = TcpStream::connect(read_file("./experiment/endpoint")).await.unwrap();
    tcp_stream.set_nodelay(true).unwrap();
    let stupid_dns_ref = webpki::DNSNameRef::try_from_ascii_str("foo").unwrap();
    let stream = tls_connector.connect(stupid_dns_ref, tcp_stream).await.unwrap();

    let mut session = session::Session::new(stream);

    let user = read_file("./experiment/username");
    let pass = read_file("./experiment/password");
    let req = rpc_request!(37, "daemon.login", [user, pass]);
    session.send(req).await.unwrap();

    let val = session.recv().await.unwrap();

    println!("{:?}", val);

    session.close().await.unwrap();
}
