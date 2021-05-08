use serde::{Serialize, Deserialize};
use bincode;

pub mod client;

#[derive(Serialize, Deserialize)]
pub enum ServerMessage {
    Message { nick: String, content: String }
}

#[derive(Serialize, Deserialize)]
pub enum ClientMessage {
    Message { content: String },
    Nick { nick: String }
}

impl ServerMessage {
    pub fn serialize(&self) -> bincode::Result<Vec<u8>> {
        bincode::serialize(self)
    }
    pub fn deserialize(bytes: &[u8]) -> bincode::Result<Self> {
        bincode::deserialize(bytes)
    }
}

impl ClientMessage {
    pub fn serialize(&self) -> bincode::Result<Vec<u8>> {
        bincode::serialize(self)
    }
    pub fn deserialize(bytes: &[u8]) -> bincode::Result<Self> {
        bincode::deserialize(bytes)
    }
}
