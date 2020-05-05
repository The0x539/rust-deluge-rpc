use crate::rencode;
use crate::rpc;

use tokio_rustls::client::TlsStream;
use tokio::net::TcpStream;

use libflate::zlib;
use std::io::{Read, Write};

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

impl Session {
    pub fn new(stream: TlsStream<TcpStream>) -> Self {
        Self {
            stream,
            request_id: 0,
        }
    }

    pub async fn send(&mut self, req: rpc::Request) -> io::Result<()> {
        let body = compress(&rencode::to_bytes(&[req]).unwrap());
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
        println!("{:?}", data);
        rpc::Inbound::from(&data).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    pub async fn close(mut self) -> io::Result<()> {
        self.stream.shutdown().await
    }
}
