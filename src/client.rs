use std::sync::Arc;
use tokio::{time::{sleep, Duration}, task::JoinHandle};
use futures_util::{StreamExt, pin_mut, SinkExt, lock::Mutex as FutureMutex};
use serde_json::json;
use tokio_tungstenite::tungstenite::Message;

use crate::config::Args;

pub struct Client {
    server: String,
    channel: String,
    username: String,
    password: String,
}

impl Client {
    pub fn new(args: Args) -> Client {
        Client{
            server: args.server,
            channel: args.channel,
            username: args.username,
            password: args.password,
        }
    }

    pub async fn run(&mut self) -> JoinHandle<()> {
        let server = self.server.clone();
        let channel = self.channel.clone();
        let username = self.username.clone();
        let password = self.password.clone();
        tokio::spawn(async move {
            let (ws, _) = tokio_tungstenite::connect_async(server).await
                .expect("failed to connect");
            let (write, mut read) = ws.split();

            let write = Arc::new(FutureMutex::new(write));
            let ping_write = write.clone();

            // read loop
            let reader = tokio::spawn(async move {
                while let Some(msg) = read.next().await {
                    let msg = msg.unwrap();
                    if let Ok(text) = msg.into_text() {
                        println!("TEXT {}", text);
                    }
                }
            });

            // ping every minute
            let pinger = tokio::spawn(async move {
                loop {
                    sleep(Duration::from_secs(60)).await;
                    let ping_msg = "Ping!"; // Your ping message here
                    ping_write.lock().await.send(Message::Ping(ping_msg.into())).await.expect("Failed to send ping");
                }
            });

            // join channel
            let msg = json!({
                "cmd": "join",
                "channel": channel,
                "nick": username,
                "password": password,
            }).to_string();
            write.lock().await.send(Message::Text(msg)).await.expect("failed to send");

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
