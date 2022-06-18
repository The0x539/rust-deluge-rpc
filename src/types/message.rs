use crate::types::{Event, List, Value};
use num_enum::TryFromPrimitive;
use serde::Deserialize;
use std::convert::TryFrom;

use super::rpc_error::Result;

#[derive(Deserialize, TryFromPrimitive)]
#[repr(u8)]
#[serde(try_from = "u8")]
enum MessageType {
    Response = 1,
    Error = 2,
    Event = 3,
}

#[derive(Debug, Deserialize)]
#[serde(try_from = "List")]
pub enum Message {
    Response {
        request_id: i64,
        result: Result<List>,
    },
    Event(Event),
}

impl TryFrom<List> for Message {
    type Error = serde_value::DeserializerError;

    fn try_from(data: List) -> Result<Self, Self::Error> {
        let mut data = data.into_iter();
        let msg_type = data.next().unwrap().deserialize_into()?;
        let val = match msg_type {
            MessageType::Response => Self::Response {
                request_id: data.next().unwrap().deserialize_into()?,
                result: Ok(match data.next().unwrap() {
                    Value::Seq(x) => x,
                    x => vec![x],
                }),
            },
            MessageType::Error => Self::Response {
                request_id: data.next().unwrap().deserialize_into()?,
                result: Err(Value::Seq(data.collect()).deserialize_into()?),
            },
            MessageType::Event => {
                let data: List = data.collect();
                let event =
                    Value::Seq(data.clone())
                        .deserialize_into()
                        .unwrap_or(Event::Unrecognized(
                            data[0].clone().deserialize_into()?,
                            data[1].clone().deserialize_into()?,
                        ));
                Self::Event(event)
            }
        };
        Ok(val)
    }
}
