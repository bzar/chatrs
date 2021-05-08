#![recursion_limit = "1024"]

use chatrs::client::{ChatClient, ChatClientCommon, ChatError, ChatResult, ChatUserInterface};
use wasm_bindgen::prelude::*;
use yew::prelude::*;
use yew::services::websocket::{WebSocketService, WebSocketStatus, WebSocketTask};

enum Message {
    Chat { nick: String, content: String },
    ChangeNick { nick: String },
    Status { content: String },
    Error { content: String },
}

struct Model {
    link: ComponentLink<Self>,
    nick: String,
    input: Option<String>,
    messages: Vec<Message>,
    ws: Option<WebSocketTask>,
}

enum Msg {
    Connect(String),
    Disconnect,
    Connected(String),
    Disconnected,
    MessageInput(String),
    RecvMessage(Vec<u8>),
    Enter,
    Nope,
}

impl Component for Model {
    type Message = Msg;
    type Properties = ();
    fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
        Self {
            link,
            nick: "anonymous".to_owned(),
            input: None,
            messages: Vec::new(),
            ws: None,
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::Connect(address) => self.connect(address),
            Msg::Disconnect => Ok(self.disconnect()),
            Msg::Connected(address) => Ok(self.connected(address)),
            Msg::Disconnected => Ok(self.disconnected()),
            Msg::MessageInput(input) => Ok(self.input = Some(input)),
            Msg::RecvMessage(data) => self.recv_binary(&data),
            Msg::Enter => self
                .input
                .take()
                .map(|input| self.handle_input(input))
                .unwrap_or(Ok(())),
            Msg::Nope => Ok(()),
        }
        .unwrap_or_else(|e| self.handle_error(e));
        true
    }

    fn change(&mut self, _props: Self::Properties) -> ShouldRender {
        false
    }

    fn view(&self) -> Html {
        html! {
            <div>
                <div class="toolbar">
                    <button onclick=self.link.callback(|_| Msg::Connect("ws://127.0.0.1:3044/".to_owned()))
                            disabled=self.is_connected()>{ "Connect" }</button>
                    <button onclick=self.link.callback(|_| Msg::Disconnect)
                            disabled=!self.is_connected()>{ "Disconnect" }</button>
                </div>
                <ul class="buffer">
                    {for self.messages.iter().map(|m| view_message(m)) }
                </ul>
                <div class="inputbar">
                    <label for="input">{ &self.nick }</label>
                    <input value={ if let Some(ref m) = self.input { m.as_str() } else { "" } }
                           name="input"
                           onkeypress=self.link.callback(|e: KeyboardEvent| { if e.key() == "Enter" { Msg::Enter } else { Msg::Nope } })
                           oninput=self.link.callback(|e: InputData| Msg::MessageInput(e.value))
                    />
                </div>
            </div>
        }
    }
}

fn view_message(m: &Message) -> Html {
    match m {
        Message::Chat { nick, content } => html! {
            <li><span class="nick">{ nick }{ ": " }</span> { content }</li>
        },
        Message::ChangeNick { nick } => html! {
            <li class="status">{ "Changed nick to " }<span class="nick">{ nick }</span></li>
        },
        Message::Status { content } => html! {
            <li class="status">{ content }</li>
        },
        Message::Error { content } => html! {
            <li class="error">{ content }</li>
        },
    }
}

impl Model {
    fn connected(&mut self, address: String) {
        self.handle_status(format!("Connected to {}", address));
    }
    fn disconnected(&mut self) {
        self.handle_status("Disconnected");
    }
    fn handle_status(&mut self, content: impl ToString) {
        self.messages.push(Message::Status {
            content: content.to_string(),
        });
    }
    fn handle_error(&mut self, content: impl ToString) {
        self.messages.push(Message::Error {
            content: content.to_string(),
        });
    }
}

impl ChatUserInterface for Model {
    fn receive_message(&mut self, nick: String, content: String) {
        self.messages.push(Message::Chat { nick, content });
    }
    fn change_nick(&mut self, nick: String) {
        self.nick = nick.clone();
        self.messages.push(Message::ChangeNick { nick });
    }
    fn quit(&mut self) {
        self.disconnect();
    }
}

impl ChatClient for Model {
    fn connect(&mut self, address: String) -> ChatResult<()> {
        if self.is_connected() {
            return Err(ChatError::AlreadyConnected.into());
        }
        let cb_recv = self
            .link
            .callback(|r: Result<Vec<u8>, _>| r.map(Msg::RecvMessage).unwrap_or(Msg::Nope));
        let connecting_to_address = address.clone();
        let cb_notify = self.link.callback(move |input| match input {
            WebSocketStatus::Closed | WebSocketStatus::Error => Msg::Disconnected,
            WebSocketStatus::Opened => Msg::Connected(connecting_to_address.clone()),
        });
        self.ws = Some(
            WebSocketService::connect_binary(&address, cb_recv, cb_notify.into())
                .map_err(|_| ChatError::ConnectionError)?,
        );
        Ok(())
    }

    fn disconnect(&mut self) {
        self.ws = None;
        self.disconnected();
    }
    fn is_connected(&self) -> bool {
        self.ws.is_some()
    }
    fn send_binary(&mut self, data: Vec<u8>) -> ChatResult<()> {
        if let Some(ref mut ws) = self.ws {
            Ok(ws.send_binary(Ok(data)))
        } else {
            Err(ChatError::SendError.into())
        }
    }
}

#[wasm_bindgen(start)]
pub fn run_app() {
    App::<Model>::new().mount_to_body();
}
