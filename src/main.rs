//use serde::{Serialize, Deserialize};
//use serde_json;

use tokio::net::TcpStream;
use tokio::prelude::*;

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

#[tokio::main()]
async fn main() {
    let mut stream = TcpStream::connect("127.0.0.1:6142").await.unwrap();
    println!("created stream");

    let result = stream.write(b"hello world\n").await;
    println!("wrote to stream; success={:?}", result.is_ok());
}
