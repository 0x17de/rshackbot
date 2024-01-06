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

    session: Option<Session>,
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

}

pub struct ClientRef {
    client: Arc<FutureMutex<Client>>
}

impl ClientRef {
    pub fn new(c: Client) -> Arc<ClientRef> {
        Arc::new(ClientRef{
            client: Arc::new(FutureMutex::new(c)),
        })
    }
    
    pub async fn json(&self, data: String) {
        let _ = self.client.lock().await.session.as_mut().unwrap()
            .write.send(WsMessage::Text(data)).await;
    }

    pub async fn chat(&self, text: String) {
        self.json(json!({"cmd": "chat", "text": text}).to_string()).await;
    }

    pub async fn get_user(&self, username: &str) -> Option<User> {
        self.client.lock().await.session.as_mut().unwrap()
            .userlist.iter()
            .find(|x| x.username == username)
            .cloned()
    }

    pub async fn handle_cmd(&self, _msg: &Message, cmd: &Command) {
        match cmd {
            Command::Users(_) => {
                let mut userlist = self.client.lock().await.session.as_ref().unwrap()
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
                let mut lock = self.client.lock().await;
                let userlist = &mut lock.session.as_mut().unwrap().userlist;
                userlist.clear();
                for user in &online_set.users {
                    userlist.push(user.into());
                }
            }
            Message::OnlineAdd(online_add) => {
                println!("Joined: {:?}", online_add);
                let mut lock = self.client.lock().await;
                let userlist = &mut lock.session.as_mut().unwrap().userlist;
                userlist.push(online_add.into())
            }
            Message::OnlineRemove(online_remove) => {
                println!("Left: {:?}", online_remove);
                let mut lock = self.client.lock().await;
                let userlist = &mut lock.session.as_mut().unwrap().userlist;
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

    pub async fn run(self: Arc<Self>) {
        let mut this = self.client.lock().await;

        let server = this.server.clone();
        let (ws, _) = tokio_tungstenite::connect_async(server).await
            .expect("failed to connect");
        let (write, read) = ws.split();
        this.session = Some(Session::new(Arc::new(FutureMutex::new(read)), write));

        let channel = this.channel.clone();
        let username = this.username.clone();
        let password = this.password.clone();
        drop(this);

        // read loop
        let reader_self = self.clone();
        let reader = tokio::spawn(async move {
            let read = {
                let mut lock = reader_self.client.lock().await;
                lock.session.as_mut().unwrap().read.clone()
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
                pinger_self.client.lock().await.session.as_mut().unwrap()
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

        self.client.lock().await.session.as_mut().unwrap()
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
