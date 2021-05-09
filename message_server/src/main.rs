use message_io::network::{NetEvent, Transport};
use message_io::node::{self};
use std::collections::HashMap;

use anyhow;
use chatrs::{ClientMessage, ServerMessage};

struct Client {
    nick: String,
}

fn main() -> anyhow::Result<()> {
    let (handler, listener) = node::split::<()>();

    handler
        .network()
        .listen(Transport::FramedTcp, "0.0.0.0:3042")?;
    handler.network().listen(Transport::Udp, "0.0.0.0:3043")?;
    handler.network().listen(Transport::Ws, "0.0.0.0:3044")?;

    let mut clients = HashMap::new();

    listener.for_each(move |event| match event.network() {
        NetEvent::Connected(endpoint, _) => {
            clients.insert(
                endpoint,
                Client {
                    nick: "anonymous".to_owned(),
                },
            );
            println!("Client connected");
        }
        NetEvent::Message(endpoint, data) => {
            if let Ok(client_message) = ClientMessage::deserialize(&data) {
                match client_message {
                    ClientMessage::Message { content } => {
                        let nick = if let Some(client) = clients.get(&endpoint) {
                            client.nick.clone()
                        } else {
                            "unknown".to_owned()
                        };
                        let message = ServerMessage::Message { nick, content };
                        if let Ok(data) = message.serialize() {
                            for client in clients.keys() {
                                handler.network().send(*client, &data);
                            }
                        } else {
                            eprintln!("ERROR: a serialization error occurred");
                        }
                    }

                    ClientMessage::Nick { nick } => {
                        if let Some(client) = clients.get_mut(&endpoint) {
                            client.nick = nick;
                        }
                    }
                };
            } else {
                eprintln!("ERROR: a deserialization error occurred");
            }
        }
        NetEvent::Disconnected(endpoint) => {
            clients.remove(&endpoint);
            println!("Client disconnected");
        }
    });

    Ok(())
}
