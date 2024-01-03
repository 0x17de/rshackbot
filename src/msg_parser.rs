use std::io::Result;

use serde::{Serialize, Deserialize};

pub trait Parseable {
    fn parse(&self) -> Result<Message>;
}

impl Parseable for String {
    fn parse(&self) -> Result<Message> {
        match serde_json::from_str(self) {
            Ok(res) => Ok(res),
            Err(e) => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                e.to_string())),
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "cmd")]
pub enum Message {
    #[serde(rename = "chat")]
    Chat(MsgChat),
    #[serde(other)]
    Unknown,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MsgChat {
    #[serde(rename = "nick")]
    pub sender: String,
    #[serde(rename = "text")]
    pub message: String,
}
