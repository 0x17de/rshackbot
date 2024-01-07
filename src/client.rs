use std::sync::Arc;
use tokio::{time::{sleep, Duration}, net::TcpStream};
use futures_util::{StreamExt, pin_mut, SinkExt, lock::Mutex as FutureMutex, stream::{SplitSink, SplitStream}};
use serde_json::json;
use tokio_tungstenite::{tungstenite::Message as WsMessage, WebSocketStream, MaybeTlsStream};
use tokio_util::sync::CancellationToken;

use crate::msg_parser::{Parseable, Message, Info};
use crate::config::Args;
use crate::cmd::{ParseableCommand, Command};
use crate::user::User;

type ArcFM<T> = Arc<FutureMutex<T>>;
type Wss = WebSocketStream<MaybeTlsStream<TcpStream>>;
type Reader = SplitStream<Wss>;
type Writer = SplitSink<Wss, WsMessage>;

struct State {
    #[allow(unused)]
    server: String,
    channel: String,
    username: String,
    password: String,

    read: ArcFM<Reader>,
    write: Writer,
    userlist: Vec<User>,
}

pub struct Client {
    state: FutureMutex<State>
}

pub struct ClientConfig {
    server: String,
    channel: String,
    username: String,
    password: String,
}

impl From<Args> for ClientConfig {
    fn from(value: Args) -> Self {
        ClientConfig {
            server: value.server,
            channel: value.channel,
            username: value.username,
            password: value.password,
        }
    }
}

impl Client {
    pub async fn new(config: &ClientConfig) -> Arc<Client> {
        let (ws, _) = tokio_tungstenite::connect_async(config.server.clone()).await
            .expect("failed to connect");
        let (write, read) = ws.split();

        Arc::new(Client{
            state: FutureMutex::new(State{
                server: config.server.clone(),
                channel: config.channel.clone(),
                username: config.username.clone(),
                password: config.password.clone(),

                read: Arc::new(FutureMutex::new(read)),
                write,
                userlist: Vec::new(),
            }),
        })
    }
    
    pub async fn json(&self, data: String) {
        let _ = self.state.lock().await.write.send(WsMessage::Text(data)).await;
    }

    pub async fn chat(&self, text: String) {
        self.json(json!({"cmd": "chat", "text": text}).to_string()).await;
    }

    pub async fn get_user(&self, username: &str) -> Option<User> {
        let username_lower = username.to_lowercase();
        self.state.lock().await
            .userlist.iter()
            .find(|x| x.username.to_lowercase() == username_lower)
            .cloned()
    }

    pub async fn handle_cmd(&self, _msg: &Message, cmd: &Command) {
        match cmd {
            Command::Users(_) => {
                let mut userlist = self.state.lock().await
                    .userlist.clone();
                userlist.sort_by(|a, b| b.username.cmp(&a.username));
                let Some(users) = userlist.iter()
                    .map(|x| x.username.clone())
                    .reduce(|a, b| a + ", " + &b) else {
                        return;
                    };
                self.chat(format!("Users: {users}")).await;
            }
            Command::Kick(kick) => {
                println!("Found kick");
                if let Some(user) = self.get_user(&kick.target).await {
                    self.chat(format!("Would kick {}", &user.username)).await;
                } else {
                    self.chat(format!("User not in userlist: {}", kick.target)).await;
                }
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
            Message::OnlineSet(online_set) => {
                println!("Users: {:?}", online_set);
                let userlist = &mut self.state.lock().await.userlist;
                userlist.clear();
                for user in &online_set.users {
                    userlist.push(user.into());
                }
            }
            Message::OnlineAdd(online_add) => {
                println!("Joined: {:?}", online_add);
                let userlist = &mut self.state.lock().await.userlist;
                userlist.push(online_add.into());
            }
            Message::OnlineRemove(online_remove) => {
                println!("Left: {:?}", online_remove);
                let userlist = &mut self.state.lock().await.userlist;
                if let Some(index) = userlist.iter().position(|x| x.username == online_remove.username) {
                    userlist.remove(index);
                }
            },
            Message::Info(Info::Whisper(whisper)) => {
                println!("<{}|whisper> {}", whisper.sender, whisper.message);

                if let Some(rest) = whisper.message.strip_prefix("::") {
                    if let Some(cmd) = rest.parse_cmd() {
                        self.handle_cmd(msg, &cmd).await;
                    }
                }
            },
            Message::Info(Info::Unknown) => {},
            Message::Unknown => {},
        }
    }

    pub async fn run(self: Arc<Self>, token: CancellationToken) {
        let this = self.state.lock().await;
        let channel = this.channel.clone();
        let username = this.username.clone();
        let password = this.password.clone();
        drop(this);

        // read loop
        let reader_self = self.clone();
        let reader = tokio::spawn(async move {
            let read = reader_self.state.lock().await.read.clone();
            let mut read = read.lock().await;
            while let Some(frame) = read.next().await {
                let frame = frame.unwrap();
                if frame.is_empty() || !frame.is_text() { continue };
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
                let msg = WsMessage::Text(json!({"cmd": "ping"}).to_string());
                pinger_self.state.lock().await
                    .write.send(msg).await
                    .expect("Failed to send ping");
            }
        });

        // join channel
        let msg = json!({
            "cmd": "join",
            "channel": channel,
            "nick": username,
            "password": password,
        }).to_string();

        self.state.lock().await
            .write.send(WsMessage::Text(msg)).await
            .expect("failed to send");

        pin_mut!(reader, pinger);

        tokio::select!{
            _ = &mut reader => {}
            _ = &mut pinger => {}
            _ = token.cancelled() => {}
        }

        reader.abort();
        pinger.abort();
    }
}
