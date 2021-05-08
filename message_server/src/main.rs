use message_io::network::{NetEvent, Transport};
use message_io::node::{self};
use std::collections::HashMap;

use chatrs::{ClientMessage, ServerMessage};

struct Client {
    nick: String,
}

fn main() {
    // Create a node, the main message-io entity. It is divided in 2 parts:
    // The 'handler', used to make actions (connect, send messages, signals, stop the node...)
    // The 'listener', used to read events from the network or signals.
    let (handler, listener) = node::split::<()>();

    // Listen for TCP, UDP and WebSocket messages at the same time.
    handler
        .network()
        .listen(Transport::FramedTcp, "0.0.0.0:3042")
        .unwrap();
    handler
        .network()
        .listen(Transport::Udp, "0.0.0.0:3043")
        .unwrap();
    handler
        .network()
        .listen(Transport::Ws, "0.0.0.0:3044")
        .unwrap();

    let mut clients = HashMap::new();

    // Read incoming network events.
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
            let client_message = ClientMessage::deserialize(&data).unwrap();
            match client_message {
                ClientMessage::Message { content } => {
                    let nick = if let Some(client) = clients.get(&endpoint) {
                        client.nick.clone()
                    } else {
                        "unknown".to_owned()
                    };
                    let message = ServerMessage::Message { nick, content }
                        .serialize()
                        .unwrap();
                    for client in clients.keys() {
                        handler.network().send(*client, &message);
                    }
                }
                ClientMessage::Nick { nick } => {
                    if let Some(client) = clients.get_mut(&endpoint) {
                        client.nick = nick;
                    }
                }
            };
        }
        NetEvent::Disconnected(endpoint) => {
            clients.remove(&endpoint);
            println!("Client disconnected");
        }
    });
}
