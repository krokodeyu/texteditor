//! commands/undo.rs

use crate::{application::Application, outcome::Outcome, error::AppResult};
use super::{CommandDef};

fn cmd_undo(app: &mut Application, _args: &[String]) -> AppResult<Outcome> {
    app.workspace.undo()?;

    Ok(Outcome {
        print: None,              // 撤销类命令一般不打印内容
        log: Some("undo".into()), // 但可以记录日志
        exit: false,
    })
}

pub const UNDO_COMMAND: CommandDef = CommandDef {
    name: "undo",
    handler: cmd_undo,
};
