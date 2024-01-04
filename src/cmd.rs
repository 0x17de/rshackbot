use clap::Parser;
use derive_more::From;

#[derive(From)]
pub enum Command {
    Kick(CmdKick),
    Users(CmdUsers),
}

#[derive(Parser)]
pub struct CmdKick {
    pub target: String
}

pub struct CmdUsers {
}

pub trait ParseableCommand {
    fn parse_cmd(&self) -> Option<Command>;
}

impl ParseableCommand for &str {
    fn parse_cmd(&self) -> Option<Command> {
        let args = shlex::split(self)?;
        let name = &args[0];

        Some(match name.to_lowercase().as_ref() {
            "kick" => CmdKick::try_parse_from(args.iter()).ok()?.into(),
            "users" => CmdUsers{}.into(),
            &_ => todo!(),
        })
    }
}
