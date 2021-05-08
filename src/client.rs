use crate::{ServerMessage, ClientMessage};
use std::str::FromStr;
use thiserror::Error;

pub trait ChatUserInterface {
    fn receive_message(&mut self, nick: String, content: String);
    fn change_nick(&mut self, nick: String);
    fn quit(&mut self);
}

pub trait ChatClient {
    fn connect(&mut self, address: String) -> ChatResult<()>;
    fn disconnect(&mut self);
    fn is_connected(&self) -> bool;
    fn send_binary(&mut self, data: Vec<u8>) -> ChatResult<()>;
}

pub trait ChatClientCommon {
    fn send(&mut self, message: ClientMessage) -> ChatResult<()>;
    fn send_message(&mut self, content: String) -> ChatResult<()>;
    fn recv(&mut self, message: ServerMessage) -> ChatResult<()>;
    fn recv_binary(&mut self, data: &[u8]) -> ChatResult<()>;
    fn handle_command(&mut self, name: String, params: Vec<String>) -> ChatResult<()>;
    fn handle_input(&mut self, input: String) -> ChatResult<()>;
}

#[derive(Debug, Error)]
pub enum ChatError {
    #[error("Serialization error")]
    SerializationError,
    #[error("Could not send message")]
    SendError,
    #[error("Unknown command: {name}")]
    UnknownCommand { name: String },
    #[error("Invalid parameters")]
    InvalidParameters,
    #[error("Invalid address: {address}")]
    InvalidAddress { address: String },
    #[error("Error connecting to server")]
    ConnectionError,
    #[error("Already connected to a server")]
    AlreadyConnected,
    #[error("An unexpected error occurred")]
    Unexpected,
}

pub type ChatResult<T> = std::result::Result<T, ChatError>;

pub enum ParsedInput {
    Message { content: String },
    Command { name: String, params: Vec<String> },
    Empty
}

impl FromStr for ParsedInput {
    type Err = ();
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.chars().next() {
            Some('/') => {
                let mut parts = s.split_whitespace();
                let name = parts.next().unwrap().to_owned(); // Always succeeds
                let params = parts.map(str::to_owned).collect();
                Ok(Self::Command { name, params })
            },
            Some(_) => Ok(Self::Message { content: s.to_owned() }),
            None => Ok(Self::Empty)
        }
    }
}

impl<T> ChatClientCommon for T where T: ChatClient + ChatUserInterface {
    fn send(&mut self, message: ClientMessage) -> ChatResult<()> {
        let data = message.serialize().map_err(|_| ChatError::SerializationError)?;
        self.send_binary(data)
    }
    fn recv(&mut self, message: ServerMessage) -> ChatResult<()> {
        match message {
            ServerMessage::Message { nick, content } => self.receive_message(nick, content),
        };
        Ok(())
    }
    fn recv_binary(&mut self, data: &[u8]) -> ChatResult<()> {
        let message = ServerMessage::deserialize(data).map_err(|_| ChatError::SerializationError)?;
        self.recv(message)
    }
    fn handle_command(&mut self, name: String, params: Vec<String>) -> ChatResult<()> {
        match name.as_str() {
            "/nick" => match params.as_slice() {
                [nick] => {
                    self.change_nick(nick.clone());
                    self.send(ClientMessage::Nick { nick: nick.clone() })
                }
                _ => return Err(ChatError::InvalidParameters)
            },
            "/connect" => match params.as_slice() {
                [address] => self.connect(address.clone()),
                _ => Err(ChatError::InvalidParameters),
            },
            "/disconnect" => match params.as_slice() {
                [] => Ok(self.disconnect()),
                _ => Err(ChatError::InvalidParameters),
            },
            "/quit" => match params.as_slice() {
                [] => Ok(self.quit()),
                _ => Err(ChatError::InvalidParameters),
            },
            _ => Err(ChatError::UnknownCommand { name })
        }
    }
    fn handle_input(&mut self, input: String) -> ChatResult<()> {
        match input.parse().expect("Parsing input should never fail") {
            ParsedInput::Command { name, params } => self.handle_command(name, params),
            ParsedInput::Message { content } => self.send_message(content),
            ParsedInput::Empty => Ok(())
        }
    }
    fn send_message(&mut self, content: String) -> ChatResult<()> {
        if self.is_connected() {
            self.send(ClientMessage::Message { content })
        } else {
            Err(ChatError::SendError)
        }
    }
}
