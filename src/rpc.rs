use std::collections::HashMap;
use serde::{
    Serialize, Serializer, ser::SerializeTuple,
    //de::{Deserialize, Deserializer, Visitor, Error as _, Unexpected, EnumAccess, VariantAccess},
};
use serde_json::{self, Value};

// NOTE: the server should receive a (serialized) list of these, not one
pub struct RpcRequest {
    pub request_id: i64,
    pub method: &'static str,
    pub args: Vec<Value>,
    pub kwargs: HashMap<String, Value>,
}

impl Serialize for RpcRequest {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        let mut s = ser.serialize_tuple(4)?;
        s.serialize_element(&self.request_id)?;
        s.serialize_element(&self.method)?;
        s.serialize_element(&self.args)?;
        s.serialize_element(&self.kwargs)?;
        s.end()
    }
}

#[macro_export]
macro_rules! rpc_request {
    (
        $request_id:expr,
        $method:expr,
        [$($arg:expr),*],
        {$($kw:expr => $kwarg:expr),*}
        $(,)?
    ) => {
        {
            use maplit::hashmap;
            $crate::rpc::RpcRequest {
                request_id: $request_id as i64,
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
    ($request_id:expr, $method:expr, [$($arg:expr),*] $(,)?) => {
        rpc_request!($request_id, $method, [$($arg),*], {})
    };
    ($request_id:expr, $method:expr, {$($kw:expr => $kwarg:expr),+} $(,)?) => {
        rpc_request!($request_id, $method, [], {$($kw => $kwarg),*})
    };
    ($request_id:expr, $method:expr $(,)?) => {
        rpc_request!($request_id, $method, [], {})
    };
}

#[allow(dead_code)]
pub enum RpcInbound {
    Response {
        request_id: i64,
        return_value: Vec<Value>,
    },
    Error {
        request_id: i64,
        exception_type: String,
        exception_msg: String,
        traceback: String,
    },
    Event {
        event_name: String,
        data: Vec<Value>,
    }
}
