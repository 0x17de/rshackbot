use std::sync::Arc;
use clap::Parser;
use futures_util::lock::Mutex as FutureMutex;

use crate::client::{Client, ClientRef};
use crate::config::Args;

mod client;
mod config;
mod msg_parser;
mod cmd;
mod user;

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let client = ClientRef::new(Client::new(args).await);
    client.run().await;
}
