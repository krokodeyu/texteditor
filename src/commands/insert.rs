use crate::{
    application::Application, commands::{doc_command::DocCommand, util}, editor::Editor, error::{AppError, AppResult}, outcome::Outcome
};
use super::CommandDef;

// ==== DocCommand 实现 ====

struct InsertTextCommand {
    line: usize,
    col: usize,
    text: String,
}

impl InsertTextCommand {
    fn new(line: usize, col: usize, text: String) -> Self {
        Self { line, col, text }
    }
}

impl DocCommand for InsertTextCommand {
    fn execute(&mut self, ed: &mut Editor) -> AppResult<()> {
        ed.insert_text(self.line, self.col, &self.text)
    }

    fn undo(&mut self, ed: &mut Editor) -> AppResult<()> {
        // 假设 text 不含换行，长度用 text.len() 即可
        ed.delete_text(self.line, self.col, self.text.len())
    }
}

// ==== CLI 命令 ====

pub fn cmd_insert(app: &mut Application, args: &[String]) -> AppResult<Outcome> {
    if args.len() < 2 {
        return Err(AppError::InvalidArgs(
            "insert <line:col> \"text\"".into(),
        ));
    }

    let (line, col) = util::parse_pos(&args[0])?;
    let text = args[1].clone(); // shell_words 已经帮你把引号去掉了

    let cmd = InsertTextCommand::new(line, col, text.clone());
    app.workspace.exec_doc(Box::new(cmd))?;

    Ok(Outcome {
        print: None,
        log: Some(format!("insert {} \"{}\"", args[0], text)),
        exit: false,
    })
}

pub const INSERT_COMMAND: CommandDef = CommandDef {
    name: "insert",
    handler: cmd_insert,
};
