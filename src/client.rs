use std::sync::Arc;
use tokio::{time::{sleep, Duration}, net::TcpStream};
use futures_util::{StreamExt, pin_mut, SinkExt, lock::Mutex as FutureMutex, stream::{SplitSink, SplitStream}};
use serde_json::json;
use tokio_tungstenite::{tungstenite::Message as WsMessage, WebSocketStream, MaybeTlsStream};

use crate::msg_parser::{Parseable, Message, Info};
use crate::config::Args;
use crate::cmd::{ParseableCommand, Command};
use crate::user::User;

type ArcFM<T> = Arc<FutureMutex<T>>;
type WSS = WebSocketStream<MaybeTlsStream<TcpStream>>;
type Reader = SplitStream<WSS>;
type Writer = SplitSink<WSS, WsMessage>;

pub struct Session {
    read: ArcFM<Reader>,
    write: Writer,
    userlist: Vec<User>,
}

impl Session {
    fn new(read: ArcFM<Reader>, write: Writer) -> Session{
        Session{
            read,
            write,
            userlist: Vec::new(),
        }
    }
}

pub struct Client {
    server: String,
    channel: String,
    username: String,
    password: String,

    session: Option<ArcFM<Session>>,
}


impl Client {
    pub fn new(args: Args) -> Client {
        Client{
            server: args.server,
            channel: args.channel,
            username: args.username,
            password: args.password,

            session: None,
        }
    }

    pub async fn json(&self, data: String) {
        let _ = self.session.as_ref().unwrap().lock().await
            .write.send(WsMessage::Text(data)).await;
    }

    pub async fn chat(&self, text: String) {
        self.json(json!({"cmd": "chat", "text": text}).to_string()).await;
    }

    pub async fn get_user(&self, username: &str) -> Option<User> {
        self.session().lock().await
            .userlist.iter()
            .find(|x| x.username == username)
            .cloned()
    }

    pub async fn handle_cmd(&self, _msg: &Message, cmd: &Command) {
        let session = self.session.as_ref().unwrap();
        match cmd {
            Command::Users(_) => {
                let mut userlist = session.as_ref().lock().await
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

    pub fn session(&self) -> &Arc<FutureMutex<Session>> {
        self.session.as_ref().unwrap()
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
                let userlist = &mut self.session().lock().await.userlist;
                userlist.clear();
                for user in &online_set.users {
                    userlist.push(user.into());
                }
            }
            Message::OnlineAdd(online_add) => {
                println!("Joined: {:?}", online_add);
                let userlist = &mut self.session().lock().await.userlist;
                userlist.push(online_add.into())
            }
            Message::OnlineRemove(online_remove) => {
                println!("Left: {:?}", online_remove);
                let userlist = &mut self.session().lock().await.userlist;
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

    pub async fn connect(&mut self) {
        let server = self.server.clone();
        let (ws, _) = tokio_tungstenite::connect_async(server).await
            .expect("failed to connect");
        let (write, read) = ws.split();
        self.session = Some(Arc::new(FutureMutex::new(
            Session::new(Arc::new(FutureMutex::new(read)), write))));
    }

    pub async fn run(self: Arc<Self>) {
        let channel = self.channel.clone();
        let username = self.username.clone();
        let password = self.password.clone();

        // read loop
        let reader_self = self.clone();
        let reader = tokio::spawn(async move {
            let read = {
                let session = reader_self.session().lock().await;
                session.read.clone()
            };
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
                pinger_self.session().lock().await
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

        self.session().lock().await
            .write.send(WsMessage::Text(msg)).await
            .expect("failed to send");

        pin_mut!(reader, pinger);
        tokio::select!{
            _ = &mut reader => {}
            _ = &mut pinger => {}
        }

        reader.abort();
        pinger.abort();
    }
}
