use std::sync::Arc;
use tokio::{time::{sleep, Duration}, task::JoinHandle, net::TcpStream};
use futures_util::{StreamExt, pin_mut, SinkExt, lock::Mutex as FutureMutex, stream::SplitSink};
use serde_json::json;
use tokio_tungstenite::{tungstenite::Message as WsMessage, WebSocketStream, MaybeTlsStream};

use crate::msg_parser::{Parseable, Message};
use crate::config::Args;
use crate::cmd::{ParseableCommand, Command};

pub struct Client {
    server: String,
    channel: String,
    username: String,
    password: String,

    write: Arc<FutureMutex<Option<Write>>>,
}

type Write = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, WsMessage>;

impl Client {
    pub fn new(args: Args) -> Client {
        Client{
            server: args.server,
            channel: args.channel,
            username: args.username,
            password: args.password,

            write: Arc::new(FutureMutex::new(None)),
        }
    }

    pub async fn json(&self, data: String) {
        if let Some(write) = self.write.lock().await.as_mut() {
            let _ = write.send(WsMessage::Text(data)).await;
        }
    }

    pub async fn chat(&self, text: String) {
        self.json(json!({"cmd": "chat", "text": text}).to_string()).await;
    }

    pub async fn handle_cmd(&self, _msg: &Message, cmd: &Command) {
        match cmd {
            Command::Kick(kick) => {
                println!("Found kick");
                self.chat(format!("Would kick {}", kick.target)).await;
            }
        }
    }

    pub async fn handle(&self, msg: &Message) {
        match msg {
            Message::Chat(chat) => {
                println!("<{}> {}", chat.sender, chat.message);

                if let Some(rest) = chat.message.strip_prefix("::") {
                    if let Some(cmd) = rest.parse_cmd() {
                        self.handle_cmd(msg, &cmd).await;
                    }
                }
            },
            Message::Unknown => {},
        }
    }

    pub async fn run(self: Arc<Self>) -> JoinHandle<()> {
        let server = self.server.clone();
        let channel = self.channel.clone();
        let username = self.username.clone();
        let password = self.password.clone();
        tokio::spawn(async move {
            let (ws, _) = tokio_tungstenite::connect_async(server).await
                .expect("failed to connect");
            let (write, mut read) = ws.split();
            let _ = self.write.lock().await.insert(write);

            // read loop
            let reader_self = self.clone();
            let reader = tokio::spawn(async move {
                while let Some(frame) = read.next().await {
                    let frame = frame.unwrap();
                    if !frame.is_empty() && frame.is_text() { continue };
                    let Ok(text) = frame.into_text() else { continue };
                    match text.parse() {
                        Err(e) => {
                            eprintln!("failed to parse: {}; {}", e, text);
                        }
                        Ok(Message::Unknown) => {
                            println!("unknown message: {}", text);
                        }
                        Ok(msg) => {
                            reader_self.handle(&msg).await;
                        }
                    }
                }
            });

            // ping every minute
            let pinger_self = self.clone();
            let pinger = tokio::spawn(async move {
                loop {
                    sleep(Duration::from_secs(60)).await;
                    if let Some(write) = pinger_self.write.lock().await.as_mut() {
                        write.send(WsMessage::Text(json!({"cmd": "ping"}).to_string())).await.expect("Failed to send ping");
                    }
                }
            });

            // join channel
            let msg = json!({
                "cmd": "join",
                "channel": channel,
                "nick": username,
                "password": password,
            }).to_string();
            if let Some(write) = self.write.lock().await.as_mut() {
                write.send(WsMessage::Text(msg)).await.expect("failed to send");
            }

            pin_mut!(reader, pinger);
            tokio::select!{
                _ = &mut reader => {}
                _ = &mut pinger => {}
            }

            reader.abort();
            pinger.abort();
        })
    }
}
