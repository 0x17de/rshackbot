use clap::Parser;

use crate::client::Client;
use crate::config::Args;

mod client;
mod config;

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let mut client = Client::new(args);
    let runner = client.run().await;
    if let Err(e) = tokio::try_join!(runner) {
        eprintln!("failed to join: {}", e)
    }
}
