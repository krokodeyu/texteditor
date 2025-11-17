use crate::{
    application::Application, 
    error::AppResult, 
    outcome::Outcome,
};
use super::CommandDef;

pub fn cmd_list(app: &mut Application, _args: &[String]) -> AppResult<Outcome> {
    let editors = app.workspace.list()?;

    Ok(Outcome::print(editors))
}

pub const LIST_COMMAND: CommandDef = CommandDef {
    name: "editor-list",
    handler: cmd_list
};