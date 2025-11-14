use crate::{application::Application, outcome::Outcome, error::AppResult};
use super::CommandDef;

pub fn cmd_exit(_app: &mut Application, _args: &[String]) -> AppResult<Outcome> {
    Ok(Outcome::exit())
}

pub const EXIT_COMMAND: CommandDef = CommandDef {
    name: "exit",
    handler: cmd_exit,
};