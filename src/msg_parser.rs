use std::io::Result as IOResult;

use serde::{Deserialize, Deserializer};
use crate::user::User;

pub trait Parseable {
    fn parse(&self) -> IOResult<Message>;
}

impl Parseable for String {
    fn parse(&self) -> IOResult<Message> {
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
    #[serde(rename = "info")]
    Info(Info),
    #[serde(rename = "onlineSet")]
    OnlineSet(MsgOnlineSet),
    #[serde(rename = "onlineAdd")]
    OnlineAdd(MsgOnlineAdd),
    #[serde(rename = "onlineRemove")]
    OnlineRemove(MsgOnlineRemove),
    #[serde(other)]
    Unknown,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
pub enum Info {
    #[serde(rename = "whisper")]
    Whisper(MsgWhisper),
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

// Step 1: Define your struct
#[derive(Debug)]
struct WhisperString(String);

fn deserialize_whisper<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>
{
    let s = String::deserialize(deserializer)?;
    let res: Vec<&str> = s.split_whitespace().collect();
    Ok(res[2..].join(" "))
}

#[derive(Deserialize, Debug)]
pub struct MsgWhisper {
    #[serde(rename = "from")]
    pub sender: String,
    #[serde(rename = "text", deserialize_with = "deserialize_whisper")]
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

impl From<&MsgOnlineSetUser> for User {
    fn from(value: &MsgOnlineSetUser) -> Self {
        User{
            username: value.username.clone(),
            level: value.level,
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct MsgOnlineAdd {
    #[serde(rename = "nick")]
    pub username: String,
    pub trip: String,
    pub level: i32,
}

impl From<&MsgOnlineAdd> for User {
    fn from(value: &MsgOnlineAdd) -> Self {
        User{
            username: value.username.clone(),
            level: value.level,
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct MsgOnlineRemove {
    #[serde(rename = "nick")]
    pub username: String,
}
