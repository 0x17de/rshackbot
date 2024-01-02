use std::sync::Arc;
use tokio::time::{sleep, Duration};
use futures_util::{StreamExt, pin_mut, SinkExt, lock::Mutex as FutureMutex};
use serde_json::json;
use tokio_tungstenite::tungstenite::Message;

use crate::cancel_guard::CancelGuard;

mod cancel_guard;

#[tokio::main]
async fn main() {
    let chat = tokio::spawn(async move {
        let (ws, _) = tokio_tungstenite::connect_async("wss://hack.chat/chat-ws").await
            .expect("failed to connect");
        let (write, read) = ws.split();

        let write = Arc::new(FutureMutex::new(write));
        let ping_write = write.clone();

        let (ping_cancel, mut ping_oncancel) = tokio::sync::oneshot::channel();
        let ping_cancel_guard = CancelGuard::new(ping_cancel);

        // read loop
        let reader = read.for_each(|res| async {
            match res {
                Err(e) => {
                    eprintln!("failed to read message: {}", e);
                },
                Ok(msg) => {
                    if let Ok(text) = msg.into_text() {
                        println!("TEXT {}", text);
                    }
                },
            }
        });

        // ping every minute
        let pinger = tokio::spawn(async move {
            loop {
                let cancel = &mut ping_oncancel;
                tokio::select! {
                    _ = sleep(Duration::from_secs(60)) => {}
                    _ = cancel => { break; }
                }
                
                let ping_msg = "Ping!"; // Your ping message here
                ping_write.lock().await.send(Message::Ping(ping_msg.into())).await.expect("Failed to send ping");
            }
        });

        // join channel
        let msg = json!({
            "cmd": "join",
            "channel": "botDev",
            "nick": "rshackbot"
        }).to_string();
        write.lock().await.send(Message::Text(msg)).await.expect("failed to send");

        pin_mut!(reader, pinger);
        tokio::select!{
            _ = reader => { ping_cancel_guard.cancel(); }
            res = pinger => { res.unwrap(); }
        }
    });
    if let Err(e) = tokio::try_join!(chat) {
        eprintln!("failed to join: {}", e)
    }
}
