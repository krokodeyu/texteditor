use crate::{
    error::AppResult,
    application::Application,
    outcome::Outcome,
};
use std::path::PathBuf;
use super::CommandDef;

pub fn cmd_edit(app: &mut Application, args: &[String]) -> AppResult<Outcome> {
    let raw_arg: String = args.get(0).cloned().unwrap_or_else(|| ".".into());
    let path: PathBuf = app.workspace.resolve_path(Some(raw_arg.as_str()));

    app.workspace.edit(&path)?;

    Ok(Outcome {
        print: Some(format!("Switch to {}", path.to_string_lossy())),
        log: Some(format!("edit {}", raw_arg)),
        exit: false,
    })
}

pub const EDIT_COMMAND: CommandDef = CommandDef {
    name: "edit",
    handler: cmd_edit
};