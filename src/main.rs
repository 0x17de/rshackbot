use clap::Parser;
use futures_util::pin_mut;
use tokio::signal;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

use crate::client::Client;
use crate::config::Args;

mod client;
mod config;
mod msg_parser;
mod cmd;
mod user;

#[tokio::main]
async fn main() {
    let token = CancellationToken::new();
    let tracker = TaskTracker::new();
    
    let args = Args::parse();
    let client = Client::new(&args.into()).await;
    let runner_token = token.clone();
    tracker.spawn(async move { client.run(runner_token).await });

    tracker.close();

    tokio::select! {
        _ = tracker.wait() => {},
        _ = signal::ctrl_c() => {
            eprintln!("starting graceful termination");
            token.cancel();
        },
    }

    tracker.wait().await;
}
