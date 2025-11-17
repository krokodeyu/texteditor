use crate::{
    error::AppResult,
    application::Application,
    outcome::Outcome,
};
use super::CommandDef;

pub fn cmd_edit(app: &mut Application, args: &[String]) -> AppResult<Outcome> {
    let path = args.get(0)
        .ok_or_else(|| crate::error::AppError::InvalidArgs("edit <file>".into()))?;

    app.workspace.edit(path)?;

    Ok(Outcome {
        print: Some(format!("Switch to {}", path)),
        log: Some(format!("edit {}", path)),
        exit: false,
    })
}

pub const EDIT_COMMAND: CommandDef = CommandDef {
    name: "edit",
    handler: cmd_edit
};