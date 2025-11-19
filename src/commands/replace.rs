use crate::{
    application::Application,
    outcome::Outcome,
    error::{AppError, AppResult},
    editor::Editor,
    commands::{
        doc_command::DocCommand,
        util,
    },
};
use super::CommandDef;

// ==== DocCommand ====

struct ReplaceTextCommand {
    line: usize,
    col: usize,
    len: usize,
    old_text: String,
    new_text: String,
}

impl ReplaceTextCommand {
    fn new(line: usize, col: usize, len: usize, new_text: String) -> Self {
        Self {
            line,
            col,
            len,
            old_text: String::new(),
            new_text,
        }
    }
}

impl DocCommand for ReplaceTextCommand {
    fn execute(&mut self, ed: &mut Editor) -> AppResult<()> {
        self.old_text = ed.peek_text(self.line, self.col, self.len)?;
        ed.delete_text(self.line, self.col, self.len)?;
        ed.insert_text(self.line, self.col, &self.new_text)?;
        Ok(())
    }

    fn undo(&mut self, ed: &mut Editor) -> AppResult<()> {
        let new_len = self.new_text.len();
        ed.delete_text(self.line, self.col, new_len)?;
        ed.insert_text(self.line, self.col, &self.old_text)?;
        Ok(())
    }
}

pub fn cmd_replace(app: &mut Application, args: &[String]) -> AppResult<Outcome> {
    if args.len() < 3 {
        return Err(AppError::InvalidArgs(
            "replace <line:col> <len> \"text\"".into(),
        ));
    }

    let (line, col) = util::parse_pos(&args[0])?;
    let len = args[1]
        .parse::<usize>()
        .map_err(|_| AppError::InvalidArgs("len must be a number".into()))?;
    let text = args[2].clone();

    let cmd = ReplaceTextCommand::new(line, col, len, text.clone());
    app.workspace.exec_doc(Box::new(cmd))?;

    Ok(Outcome {
        print: None,
        log: Some(format!("replace {} {} \"{}\"", args[0], len, text)),
        exit: false,
    })
}

pub const REPLACE_COMMAND: CommandDef = CommandDef {
    name: "replace",
    handler: cmd_replace,
};
