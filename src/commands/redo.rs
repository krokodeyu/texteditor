//! commands/redo.rs

use crate::{application::Application, outcome::Outcome, error::AppResult};
use super::{CommandDef};


fn cmd_redo(app: &mut Application, _args: &[String]) -> AppResult<Outcome> {
    app.workspace.redo()?;

    Ok(Outcome {
        print: None,
        log: Some("redo".into()),
        exit: false,
    })
}

pub const REDO_COMMAND: CommandDef = CommandDef {
    name: "redo",
    handler: cmd_redo,
};
