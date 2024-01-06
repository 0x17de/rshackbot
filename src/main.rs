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
    let client = Client::new(&args.into()).await;
    client.run().await;
}
