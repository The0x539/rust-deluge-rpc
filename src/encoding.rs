use libflate::zlib;
use serde::{Serialize, de::DeserializeOwned};
use rencode;

pub fn decode<T: DeserializeOwned>(input: &[u8]) -> rencode::Result<T> {
    rencode::from_reader(zlib::Decoder::new(input).unwrap())
}

pub fn encode<T: Serialize>(input: T) -> rencode::Result<Vec<u8>> {
    let mut encoder = zlib::Encoder::new(Vec::new()).unwrap();
    rencode::to_writer(&mut encoder, &input)?;
    let buf = encoder.finish().into_result().unwrap();
    Ok(buf)
}
