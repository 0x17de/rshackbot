use clap::Parser;

#[derive(Parser, Debug)]
#[command(version)]
pub struct Args {
    #[arg(long, env = "HACK_SERVER")]
    pub server: String,
    #[arg(long, env = "HACK_CHANNEL")]
    pub channel: String,
    #[arg(long, env = "HACK_USERNAME")]
    pub username: String,
    #[arg(long, env = "HACK_PASSWORD", default_value = "")]
    pub password: String,
}
