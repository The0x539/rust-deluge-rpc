use cloudflare_zlib as zlib;
use serde::{de::DeserializeOwned, Serialize};

// TODO: represent zlib errors in Error type

pub fn decode<T: DeserializeOwned>(input: &[u8]) -> rencode::Result<T> {
    let inflated = zlib::inflate(input).expect("Decompression failed");
    let decoded = rencode::from_bytes(&inflated)?;
    Ok(decoded)
}

pub fn encode<T: Serialize>(input: T) -> rencode::Result<Vec<u8>> {
    let encoded = rencode::to_bytes(&input)?;
    let deflated = zlib::deflate(&encoded).expect("Compression failed");
    Ok(deflated)
}
