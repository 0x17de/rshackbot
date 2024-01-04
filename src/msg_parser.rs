use std::sync::Arc;
use std::io::Result;

use serde::Deserialize;
use crate::user::User;

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

#[derive(Deserialize, Debug)]
#[serde(tag = "cmd")]
pub enum Message {
    #[serde(rename = "chat")]
    Chat(MsgChat),
    #[serde(rename = "onlineSet")]
    OnlineSet(MsgOnlineSet),
    #[serde(other)]
    Unknown,
}

#[derive(Deserialize, Debug)]
pub struct MsgChat {
    #[serde(rename = "nick")]
    pub sender: String,
    #[serde(rename = "text")]
    pub message: String,
}

#[derive(Deserialize, Debug)]
pub struct MsgOnlineSet {
    #[serde(rename = "users")]
    pub users: Vec<MsgOnlineSetUser>,
}

#[derive(Deserialize, Debug)]
pub struct MsgOnlineSetUser {
    #[serde(rename = "nick")]
    pub username: String,
    #[serde(rename = "isme")]
    pub is_me: bool,
    pub trip: String,
    pub level: i32,
}

impl From<&MsgOnlineSetUser> for Arc<User> {
    fn from(value: &MsgOnlineSetUser) -> Self {
        Arc::new(User{
            username: value.username.clone(),
        })
    }
}
