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

const RPC_RESPONSE: i64 = 1;
const RPC_ERROR: i64 = 2;
const RPC_EVENT: i64 = 3;

#[derive(Debug)]
pub struct Error {
    exception: String,
    args: Vec<Value>,
    kwargs: HashMap<String, Value>,
    traceback: String,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        /*
        use serde_json::to_string_pretty;
        let args = self.args
            .iter()
            .map(|v|to_string_pretty(v).unwrap());
        let kwargs = self.kwargs
            .iter()
            .map(|(k, v)| format!("{}={}", to_string_pretty(k).unwrap(), to_string_pretty(v).unwrap()));
        write!(
            f,
            "{}({})\n{}",
            self.exception,
            args.chain(kwargs).collect::<Vec<String>>().join(", "),
            self.traceback,
        )
        */
        f.write_str(&self.traceback)
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
                    RPC_ERROR => {
                        Err(Error {
                            exception: from_value(data[2].clone())?,
                            args: from_value(data[3].clone())?,
                            kwargs: from_value(data[4].clone())?,
                            traceback: from_value(data[5].clone())?,
                        })
                    },
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
