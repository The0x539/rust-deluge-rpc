use libflate::zlib;
use std::io::{Read, Write};

// TODO: expose encode and decode, rather than compress and decompress

pub(crate) mod rencode;

pub fn compress(input: &[u8]) -> Vec<u8> {
    let mut encoder = zlib::Encoder::new(Vec::new()).unwrap();
    encoder.write_all(input).unwrap();
    encoder.finish().into_result().unwrap()
}

pub fn decompress(input: &[u8]) -> Vec<u8> {
    let mut decoder = zlib::Decoder::new(input).unwrap();
    let mut buf = Vec::new();
    decoder.read_to_end(&mut buf).unwrap();
    buf
}
