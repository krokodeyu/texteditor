use crate::{
    application::Application, 
    error::AppResult, 
    outcome::Outcome,
};
use super::CommandDef;

pub fn cmd_list(app: &mut Application, _args: &[String]) -> AppResult<Outcome> {
    let times_guard = app.edit_times.lock().unwrap();
    let s = app.workspace.editor_list(Some(&*times_guard))?;
    Ok(Outcome {
        print: Some(s),
        log: Some("editor-list".into()),
        exit: false,
    })
}

pub const LIST_COMMAND: CommandDef = CommandDef {
    name: "editor-list",
    handler: cmd_list
};