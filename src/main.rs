use std::sync::Arc;
use clap::Parser;

use crate::client::Client;
use crate::config::Args;

mod client;
mod config;
mod msg_parser;
mod cmd;
mod user;

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let mut client = Client::new(args);
    client.connect().await;
    let client = Arc::new(client);
    client.run().await;
}
