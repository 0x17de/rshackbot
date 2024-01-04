use clap::{Parser};

pub enum Command {
    Kick(CmdKick),
}

#[derive(Parser)]
pub struct CmdKick {
    pub target: String
}

pub trait ParseableCommand {
    fn parse_cmd(&self) -> Option<Command>;
}

impl ParseableCommand for &str {
    fn parse_cmd(&self) -> Option<Command> {
        if let Some(args) = shlex::split(self) {
            let name = &args[0];

            return match name.to_lowercase().as_ref() {
                "kick" => {
                    match CmdKick::try_parse_from(args.iter()) {
                        Ok(res) => { println!("parsed kick"); Some(Command::Kick(res)) },
                        Err(e) => { println!("failed to parse cmd: {}", e); None },
                    }
                },
                _ => None,
            }
        }
        None
    }
}
