use serde::Deserialize;
use std::convert::TryFrom;
use crate::types::{Value, List, Event};
use num_enum::TryFromPrimitive;

use super::rpc_error::Result;

#[derive(Debug, Deserialize)]
#[serde(try_from="List")]
pub enum Inbound {
    Response { request_id: i64, result: Result<List> },
    Event(Event),
}

#[derive(Deserialize, TryFromPrimitive)]
#[repr(u8)]
#[serde(try_from = "u8")]
enum MessageType { Response = 1, Error = 2, Event = 3 }

impl TryFrom<List> for Inbound {
    type Error = ron::Error;

    fn try_from(data: List) -> ron::Result<Self> {
        let mut data = data.into_iter();
        let msg_type = data.next().unwrap().into_rust()?;
        let val = match msg_type {
            MessageType::Response => Inbound::Response {
                request_id: data.next().unwrap().into_rust()?,
                result: Ok(match data.next().unwrap() {
                    Value::Seq(x) => x,
                    x => vec![x],
                }),
            },
            MessageType::Error => Inbound::Response {
                request_id: data.next().unwrap().into_rust()?,
                result: Err(Value::Seq(data.collect()).into_rust()?),
            },
            MessageType::Event => {
                let data: List = data.collect();
                let event = Value::Seq(data.clone())
                    .into_rust()
                    .unwrap_or(Event::Unrecognized(data[0].clone().into_rust()?, data[1].clone().into_rust()?));
                Inbound::Event(event)
            },
        };
        Ok(val)
    }
}
