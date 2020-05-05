use std::collections::HashMap;
use serde_json::{self, Value};

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

const RPC_RESPONSE: u64 = 1; 
const RPC_ERROR: u64 = 2;
const RPC_EVENT: u64 = 3;

#[derive(Debug)]
pub enum Inbound {
    Response {
        request_id: i64,
        return_value: Vec<Value>,
    },
    Error {
        request_id: i64,
        exception_type: String,
        exception_msg: Vec<Value>,
        traceback: String,
    },
    Event {
        event_name: String,
        data: Vec<Value>,
    }
}

impl Inbound {
    pub fn from(data: &[Value]) -> Result<Self, &str> {
        let msg_type = data[0].as_u64().ok_or("data[0] isn't int")?;
        let val = match msg_type {
            RPC_RESPONSE => Inbound::Response {
                request_id: data[1].as_i64().ok_or("data[1] isn't int")?,
                return_value: data[2].as_array().cloned().ok_or("data[2] isn't list")?,
            },
            RPC_ERROR => Inbound::Error {
                request_id: data[1].as_i64().ok_or("data[1] isn't int")?,
                exception_type: data[2].as_str().map(String::from).ok_or("data[2] isn't str")?,
                exception_msg: data[3].as_array().cloned().ok_or("data[3] isn't str")?,
                traceback: data[5].as_str().map(String::from).ok_or("data[5] isn't str")?,
            },
            RPC_EVENT => Inbound::Event {
                event_name: data[1].as_str().map(String::from).ok_or("data[1] isn't str")?,
                data: data[2].as_array().cloned().ok_or("data[2] isn't list")?,
            },
            _ => {
                println!("unrecognized event type: {}", msg_type);
                return Err("unrecognized event type");
            }
        };
        Ok(val)
    }
}
