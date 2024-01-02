use std::sync::Arc;
use tokio::time::{sleep, Duration};
use futures_util::{StreamExt, pin_mut, SinkExt, lock::Mutex};
use serde_json::json;
use tokio_tungstenite::tungstenite::Message;

#[tokio::main]
async fn main() {
    let chat = tokio::spawn(async move {
        let (ws, _) = tokio_tungstenite::connect_async("wss://hack.chat/chat-ws").await
            .expect("failed to connect");
        let (mut write, read) = ws.split();
        let reader = read.for_each(|res| async {
            match res {
                Err(e) => eprintln!("failed to read message: {}", e),
                Ok(msg) => {
                    if let Ok(text) = msg.into_text() {
                        println!("TEXT {}", text);
                    }
                },
            }
        });

        let write = Arc::new(Mutex::new(write));
        let ping_write = write.clone();
        // Spawn the ping loop task
        tokio::spawn(async move {
            loop {
                sleep(Duration::from_secs(60)).await; // wait for one minute
                let ping_msg = "Ping!"; // Your ping message here
                ping_write.lock().await.send(Message::Ping(ping_msg.into())).await.expect("Failed to send ping");
            }
        });

        let msg = json!({
            "cmd": "join",
            "channel": "botDev",
            "nick": "rshackbot"
        }).to_string();
        write.lock().await.send(Message::Text(msg)).await.expect("failed to send");

        pin_mut!(reader);
        reader.await;
    });
    if let Err(e) = tokio::try_join!(chat) {
        eprintln!("failed to join: {}", e)
    }
}
