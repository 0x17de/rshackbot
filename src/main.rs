use clap::Parser;
use futures_util::pin_mut;
use tokio::signal;
use tokio::sync::mpsc::unbounded_channel;

use crate::client::Client;
use crate::config::Args;

mod client;
mod config;
mod msg_parser;
mod cmd;
mod user;

#[tokio::main]
async fn main() {
    let (shutdown_send, shutdown_recv) = unbounded_channel();
    
    let args = Args::parse();
    let client = Client::new(&args.into()).await;
    let runner = tokio::spawn(async move { client.run(shutdown_recv).await });

    pin_mut!(runner);
    tokio::select! {
        _ = &mut runner => {},
        _ = signal::ctrl_c() => {
            eprintln!("starting graceful termination");
            let _ = shutdown_send.send(());
        },
    }

    if !runner.is_finished() {
        let _ = tokio::join!(runner);
    }
}
