use crate::{
    application::Application, commands::{doc_command::DocCommand, util}, editor::Editor, error::{AppError, AppResult}, outcome::Outcome
};
use super::CommandDef;

// ==== DocCommand ====

struct DeleteTextCommand {
    line: usize,
    col: usize,
    len: usize,
    deleted_text: String,
}

impl DeleteTextCommand {
    fn new(line: usize, col: usize, len: usize) -> Self {
        Self {
            line,
            col,
            len,
            deleted_text: String::new(),
        }
    }
}

impl DocCommand for DeleteTextCommand {
    fn execute(&mut self, ed: &mut Editor) -> AppResult<()> {
        // 先把要删的内容记下来，方便 undo
        self.deleted_text = ed.peek_text(self.line, self.col, self.len)?;
        ed.delete_text(self.line, self.col, self.len)
    }

    fn undo(&mut self, ed: &mut Editor) -> AppResult<()> {
        ed.insert_text(self.line, self.col, &self.deleted_text)
    }
}

// ==== CLI ====

pub fn cmd_delete(app: &mut Application, args: &[String]) -> AppResult<Outcome> {
    if args.len() < 2 {
        return Err(AppError::InvalidArgs(
            "delete <line:col> <len>".into(),
        ));
    }

    let (line, col) = util::parse_pos(&args[0])?;
    let len = args[1]
        .parse::<usize>()
        .map_err(|_| AppError::InvalidArgs("len must be a number".into()))?;

    let cmd = DeleteTextCommand::new(line, col, len);
    app.workspace.exec_doc(Box::new(cmd))?;

    Ok(Outcome {
        print: None,
        log: Some(format!("delete {} {}", args[0], len)),
        exit: false,
    })
}

pub const DELETE_COMMAND: CommandDef = CommandDef {
    name: "delete",
    handler: cmd_delete,
};
