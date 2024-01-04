use std::sync::Arc;
use clap::Parser;

use crate::client::Client;
use crate::config::Args;

mod client;
mod config;
mod msg_parser;
mod cmd;

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let client = Arc::new(Client::new(args));
    let runner = client.run().await;
    if let Err(e) = tokio::try_join!(runner) {
        eprintln!("failed to join: {}", e)
    }
}
