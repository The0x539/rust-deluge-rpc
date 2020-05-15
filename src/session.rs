use serde_json::Value;
use serde::Deserialize;

use crate::encoding;
use crate::rpc;
use crate::receiver::MessageReceiver;
use crate::error::{Error, Result};
use crate::wtf;

use tokio_rustls::{TlsConnector, webpki, client::TlsStream};
use tokio::net::TcpStream;

use std::sync::Arc;
use std::iter::FromIterator;

use tokio::prelude::*;
use tokio::sync::{oneshot, mpsc};

type List = Vec<Value>;
// I don't expect to come across any non-string keys.
type Dict = std::collections::HashMap<String, Value>;

type WriteStream = io::WriteHalf<TlsStream<TcpStream>>;
type RequestTuple = (i64, &'static str, List, Dict);
type RpcSender = oneshot::Sender<rpc::Result<List>>;

pub struct Session {
    stream: WriteStream,
    prev_req_id: i64,
    listeners: mpsc::Sender<(i64, RpcSender)>,
}

#[macro_export]
macro_rules! dict {
    ($($key:expr => $val:expr),*$(,)?) => {
        {
            use maplit::hashmap;
            maplit::convert_args!(
                keys=String::from,
                values=serde_json::Value::from,
                hashmap!($($key => $val),*)
            )
        }
    }
}

macro_rules! build_request {
    (
        $method:expr
        $(, [$($arg:expr),*])?
        $(, {$($kw:expr => $kwarg:expr),*})?
        $(,)?
    ) => {
        $crate::rpc::Request {
            method: $method,
            args: vec![$($(serde_json::json!($arg)),*)?],
            kwargs: dict!{$($($kw => $kwarg),*)?}
        }
    };
}

macro_rules! make_request {
    ($self:ident, $($arg:tt),*$(,)?) => {
        $self.request(build_request!($($arg),*)).await?
    }
}

macro_rules! expect {
    ($val:expr, ?$pat:pat, $expected:expr, $result:expr) => {
        match $val {
            $pat => Ok(Some($result)),
            Value::Null => Ok(None),
            x => Err(Error::expected($expected, x)),
        }
    };

    ($val:expr, $pat:pat, $expected:expr, $result:expr) => {
        match $val {
            $pat => Ok($result),
            x => Err(Error::expected($expected, x)),
        }
    }
}

macro_rules! expect_val {
    ($val:expr, ?$pat:pat, $expected:expr, $result:expr) => {
        match $val.len() {
            1 => expect!($val.into_iter().next().unwrap(), ?$pat, $expected, $result),
            _ => Err(Error::expected(std::concat!("a list containing only ", $expected), $val)),
        }
    };

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

// TODO: derive macro
pub trait Query: for<'de> Deserialize<'de> {
    fn keys() -> &'static [&'static str];
}

#[allow(dead_code)]
impl Session {
    fn prepare_request(&mut self, request: rpc::Request) -> RequestTuple {
        self.prev_req_id += 1;
        (self.prev_req_id, request.method, request.args, request.kwargs)
    }

    async fn send(&mut self, req: RequestTuple) -> Result<()> {
        let body = encoding::encode(&[req]).unwrap();
        let mut msg = Vec::with_capacity(1 + 4 + body.len());
        byteorder::WriteBytesExt::write_u8(&mut msg, 1).unwrap();
        byteorder::WriteBytesExt::write_u32::<byteorder::BE>(&mut msg, body.len() as u32).unwrap();
        std::io::Write::write_all(&mut msg, &body).unwrap();
        self.stream.write_all(&msg).await?;
        Ok(())
    }

    async fn request(&mut self, req: rpc::Request) -> Result<List> {
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
        tls_config.dangerous().set_certificate_verifier(Arc::new(wtf::NoCertificateVerification));
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
        let val = make_request!(self, "daemon.info");
        expect_val!(val, Value::String(version), "a version number string", version)
    }

    pub async fn login(&mut self, username: &str, password: &str) -> Result<i64> {
        let val = make_request!(self, "daemon.login", [username, password], {"client_version" => "2.0.4.dev23"});
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
        let val = make_request!(self, "daemon.set_event_interest", [events]);
        expect_val!(val, Value::Bool(true), "true", ())
    }

    pub async fn shutdown(mut self) -> Result<()> {
        let val = make_request!(self, "daemon.shutdown");
        expect_val!(val, Value::Null, "null", ())?;
        self.close().await
    }

    pub async fn get_method_list<T: FromIterator<String>>(&mut self) -> Result<T> {
        let val = make_request!(self, "daemon.get_method_list");
        expect_seq!(val, Value::String(s), "a string", s)
    }

    pub async fn get_session_state<T: FromIterator<String>>(&mut self) -> Result<T> {
        let val = make_request!(self, "core.get_session_state");
        expect_seq!(val, Value::String(s), "a string", s)
    }

    pub async fn get_torrent_status<T: Query>(&mut self, torrent_id: &str) -> Result<T> {
        let val = make_request!(self, "core.get_torrent_status", [torrent_id, T::keys()]);
        expect_val!(val, m @ Value::Object(_), "a torrent's status", serde_json::from_value(m).unwrap())
    }

    pub async fn get_torrents_status<T, U>(
        &mut self,
        filter_dict: Option<Dict>,
    ) -> Result<U>
        where T: Query,
              U: FromIterator<(String, T)>
    {
        let val = make_request!(self, "core.get_torrents_status", [filter_dict, T::keys()]);
        let ret = expect_val!(val, Value::Object(m), "a map of torrents' statuses", m)?
            .into_iter()
            .map(|(id, status)| (id, serde_json::from_value(status).unwrap()))
            .collect();
        Ok(ret)
    }

    pub async fn close(mut self) -> Result<()> {
        self.stream.shutdown().await?;
        Ok(())
    }
}
