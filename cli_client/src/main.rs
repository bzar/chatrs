use anyhow;
use message_io::network::{NetEvent, ToRemoteAddr, Transport};
use message_io::node::{self, NodeEvent, NodeHandler};
use std::thread;

use chatrs::client::{ChatClient, ChatClientCommon, ChatError, ChatResult, ChatUserInterface};

use std::io;
use std::sync::mpsc;
use std::time::Duration;

use termion::{
    event::Key, input::MouseTerminal, input::TermRead, raw::IntoRawMode, screen::AlternateScreen,
};
use tui::{
    backend::{Backend, TermionBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Terminal,
};

use unicode_width::UnicodeWidthStr;

pub enum Event<I> {
    Connect(String),
    Disconnect,
    Connected(String),
    Disconnected,
    RecvMessage(Vec<u8>),
    Enter,
    Nope,
    Input(I),
    Tick,
}

pub struct Events {
    tx: mpsc::Sender<Event<Key>>,
    rx: mpsc::Receiver<Event<Key>>,
    _input_handle: thread::JoinHandle<()>,
    _tick_handle: thread::JoinHandle<()>,
}

enum Message {
    Chat { nick: String, content: String },
    ChangeNick { nick: String },
    Status { content: String },
    Error { content: String },
}

enum ChatSignal {
    Message { data: Vec<u8> },
}

struct App {
    running: bool,
    nick: Option<String>,
    input: String,
    history: Vec<String>,
    history_index: Option<usize>,
    messages: Vec<Message>,
    handler: Option<NodeHandler<ChatSignal>>,
    events: Events,
}

impl Default for App {
    fn default() -> Self {
        Self {
            running: true,
            nick: None,
            input: "/connect 127.0.0.1:3042".to_owned(),
            history: Vec::new(),
            history_index: None,
            messages: Vec::new(),
            handler: None,
            events: Events::new(),
        }
    }
}

fn main() -> anyhow::Result<()> {
    let stdout = io::stdout().into_raw_mode().expect("Error opening stdout");
    let stdout = MouseTerminal::from(stdout);
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend).expect("Error initializing terminal");

    let mut app = App::default();

    while app.running {
        app.render_ui(&mut terminal)?;
        app.handle_events()?;
    }

    Ok(())
}

impl App {
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
    fn handle_events(&mut self) -> anyhow::Result<()> {
        match self.events.next()? {
            Event::Connect(address) => self.connect(address),
            Event::Disconnect => Ok(self.disconnect()),
            Event::Connected(address) => Ok(self.connected(address)),
            Event::Disconnected => Ok(self.disconnected()),
            Event::Input(input) => {
                match input {
                    Key::Char('\n') => self.events.tx.send(Event::Enter)?,
                    Key::Char(c) => self.input.push(c),
                    Key::Backspace => {
                        self.input.pop();
                    }
                    Key::Up => self.history_prev(),
                    Key::Down => self.history_next(),
                    _ => {}
                }
                Ok(())
            }
            Event::RecvMessage(data) => self.recv_binary(&data),
            Event::Enter => {
                let input = self.input.clone();
                self.history.push(input.clone());
                self.history_index = None;
                self.input.clear();
                self.handle_input(input)
            }
            Event::Nope | Event::Tick => Ok(()),
        }
        .unwrap_or_else(|e| self.handle_error(e));
        Ok(())
    }

    fn history_prev(&mut self) {
        if !self.history.is_empty() && self.history_index != Some(0) {
            self.history_index = self
                .history_index
                .map(|x| (x - 1))
                .or(Some(self.history.len() - 1));
        }
        if let Some(index) = self.history_index {
            self.input = self.history[index].clone();
        }
    }

    fn history_next(&mut self) {
        self.history_index =
            if self.history.is_empty() || self.history_index == Some(self.history.len() - 1) {
                None
            } else {
                self.history_index.map(|x| (x + 1)).or(None)
            };

        if let Some(index) = self.history_index {
            self.input = self.history[index].clone();
        }
    }

    fn render_ui<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> anyhow::Result<()> {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(2)
                .constraints(
                    [
                        Constraint::Length(1),
                        Constraint::Length(3),
                        Constraint::Min(1),
                    ]
                    .as_ref(),
                )
                .split(f.size());

            let bold_style = Style::default().add_modifier(Modifier::BOLD);
            let help_message = vec![
                Span::raw("Type "),
                Span::styled("/connect 127.0.0.1:3042", bold_style),
                Span::raw(" to connect to a server, "),
                Span::styled("/nick MyNick", bold_style),
                Span::raw(" to change your name, "),
                Span::styled("/quit", bold_style),
                Span::raw(" to quit."),
            ];
            let help_message_widget = Paragraph::new(Text::from(Spans::from(help_message)));
            f.render_widget(help_message_widget, chunks[0]);

            let nick = self
                .nick
                .as_ref()
                .map(|n| n.as_str())
                .unwrap_or("anonymous");

            let input_paragraph = Paragraph::new(self.input.as_ref())
                .block(Block::default().borders(Borders::ALL).title(nick));
            f.render_widget(input_paragraph, chunks[1]);
            f.set_cursor(
                // Put cursor past the end of the input text
                chunks[1].x + self.input.width() as u16 + 1,
                // Move one line down, from the border to the input line
                chunks[1].y + 1,
            );

            let nick_style = Style::default().fg(Color::Magenta);
            let status_style = Style::default().fg(Color::Gray);
            let error_style = Style::default().fg(Color::Red);

            let messages: Vec<ListItem> = self
                .messages
                .iter()
                .map(|m| {
                    let content: Text = match m {
                        Message::Chat { nick, content } => Spans::from(vec![
                            Span::styled(nick, nick_style),
                            Span::from(format!(": {}", content)),
                        ])
                        .into(),
                        Message::ChangeNick { nick } => Spans::from(vec![
                            Span::from("Changed nick to "),
                            Span::styled(nick, nick_style),
                        ])
                        .into(),
                        Message::Status { content } => Span::styled(content, status_style).into(),
                        Message::Error { content } => Span::styled(content, error_style).into(),
                    };
                    ListItem::new(content)
                })
                .collect();
            let messages =
                List::new(messages).block(Block::default().borders(Borders::ALL).title("Messages"));
            f.render_widget(messages, chunks[2]);
        })?;
        Ok(())
    }
}

impl Events {
    pub fn new() -> Events {
        Events::with_tick_rate(Duration::from_millis(250))
    }

    pub fn with_tick_rate(tick_rate: Duration) -> Events {
        let (tx, rx) = mpsc::channel();
        let input_handle = {
            let tx = tx.clone();
            thread::spawn(move || {
                let stdin = io::stdin();
                for evt in stdin.keys() {
                    if let Ok(key) = evt {
                        if let Err(err) = tx.send(Event::Input(key)) {
                            eprintln!("{}", err);
                            return;
                        }
                    }
                }
            })
        };
        let tick_handle = {
            let tx = tx.clone();
            thread::spawn(move || loop {
                if tx.send(Event::Tick).is_err() {
                    break;
                }
                thread::sleep(tick_rate);
            })
        };
        Events {
            tx,
            rx,
            _input_handle: input_handle,
            _tick_handle: tick_handle,
        }
    }

    pub fn next(&self) -> Result<Event<Key>, mpsc::RecvError> {
        self.rx.recv()
    }
}

impl ChatUserInterface for App {
    fn receive_message(&mut self, nick: String, content: String) {
        self.messages.push(Message::Chat { nick, content });
    }
    fn change_nick(&mut self, nick: String) {
        self.nick = Some(nick.clone());
        self.messages.push(Message::ChangeNick { nick });
    }
    fn quit(&mut self) {
        self.disconnect();
        self.running = false;
    }
}

impl ChatClient for App {
    fn connect(&mut self, address: String) -> ChatResult<()> {
        if self.is_connected() {
            return Err(ChatError::AlreadyConnected.into());
        }

        let remote_addr = address
            .to_remote_addr()
            .map_err(|_| ChatError::InvalidAddress {
                address: address.clone(),
            })?;
        if !remote_addr.is_socket_addr() {
            return Err(ChatError::InvalidAddress { address }.into());
        }

        let (handler, listener) = node::split();

        let (server, _) = handler
            .network()
            .connect(Transport::FramedTcp, remote_addr)
            .map_err(|_| ChatError::ConnectionError)?;
        self.events
            .tx
            .send(Event::Connected(address.clone()))
            .map_err(|_| ChatError::Unexpected)?;

        let sender = self.events.tx.clone();

        let listener_handler = handler.clone();
        thread::spawn(move || {
            let listener_sender = sender.clone();
            listener.for_each(move |event| match event {
                NodeEvent::Signal(signal) => match signal {
                    ChatSignal::Message { data } => {
                        listener_handler.network().send(server, &data);
                    }
                },
                NodeEvent::Network(net_event) => match net_event {
                    NetEvent::Message(_endpoint, data) => {
                        listener_sender.send(Event::RecvMessage(data.to_vec())).ok();
                    }
                    _ => unreachable!(),
                },
            });
            sender.send(Event::Disconnected).ok();
        });

        self.handler = Some(handler);
        Ok(())
    }
    fn disconnect(&mut self) {
        if let Some(handler) = self.handler.take() {
            handler.stop();
        }
    }
    fn is_connected(&self) -> bool {
        self.handler.is_some()
    }
    fn send_binary(&mut self, data: Vec<u8>) -> ChatResult<()> {
        if let Some(ref handler) = self.handler {
            handler.signals().send(ChatSignal::Message { data });
            Ok(())
        } else {
            Err(ChatError::SendError.into())
        }
    }
}
