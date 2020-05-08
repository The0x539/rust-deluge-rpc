use std::collections::HashMap;
use serde_json::{self, Value};
use serde::de;

// NOTE: the server should receive a (serialized) list of these, not one
#[derive(Clone)]
pub struct Request {
    pub method: &'static str,
    pub args: Vec<Value>,
    pub kwargs: HashMap<String, Value>,
}

#[macro_export]
macro_rules! rpc_request {
    (
        $method:expr,
        [$($arg:expr),*],
        {$($kw:expr => $kwarg:expr),*}
        $(,)?
    ) => {
        {
            use maplit::hashmap;
            $crate::rpc::Request {
                method: $method,
                args: vec![$(serde_json::json!($arg)),*],
                kwargs: maplit::convert_args!(
                    keys=String::from,
                    values=serde_json::Value::from,
                    hashmap!($($kw => $kwarg),*)
                ),
            }
        }
    };
    ($method:expr, [$($arg:expr),*] $(,)?) => {
        rpc_request!($method, [$($arg),*], {})
    };
    ($method:expr, {$($kw:expr => $kwarg:expr),+} $(,)?) => {
        rpc_request!($method, [], {$($kw => $kwarg),*})
    };
    ($method:expr $(,)?) => {
        rpc_request!($method, [], {})
    };
}

const RPC_RESPONSE: i64 = 1;
const RPC_ERROR: i64 = 2;
const RPC_EVENT: i64 = 3;

// TODO: Determine what we can expect from the server
// Make this data structure less free-form accordingly
#[derive(Debug)]
pub struct Error(Vec<Value>);

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        // Won't panic because we know we can serialize a Vec<Value>
        let s = serde_json::to_string_pretty(&self.0).unwrap();
        f.write_str(&s)
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Inbound {
    Response { request_id: i64, result: Result<Vec<Value>> },
    Event { event_name: String, data: Vec<Value> },
}

impl Inbound {
    pub fn from(data: &[Value]) -> serde_json::Result<Self> {
        use serde_json::from_value;
        let msg_type: i64 = from_value(data[0].clone())?;
        let val = match msg_type {
            RPC_RESPONSE | RPC_ERROR => Inbound::Response {
                request_id: from_value(data[1].clone())?,
                result: match msg_type {
                    RPC_RESPONSE => Ok(from_value(data[2].clone()).unwrap_or(vec![data[2].clone()])),
                    RPC_ERROR => Err(Error(data[2..].to_vec())),
                    _ => unreachable!(),
                },
            },
            RPC_EVENT => Inbound::Event {
                event_name: from_value(data[1].clone())?,
                data: from_value(data[2].clone())?,
            },
            _ => {
                let unexp = de::Unexpected::Signed(msg_type);
                let exp = &"a known message type (1, 2, or 3)";
                return Err(de::Error::invalid_value(unexp, exp));
            },
        };
        Ok(val)
    }
}
