use crate::{
    application::Application, 
    outcome::Outcome, 
    error::AppResult,
    editor::Editor,
    commands::doc_command::DocCommand,
};
use super::CommandDef;

struct AppendLineCommand {
    line_index: usize,
    text: String,
}

impl AppendLineCommand {
    fn new(text: String) -> Self {
        Self { line_index: 0, text }
    }
}

impl DocCommand for AppendLineCommand {
    fn execute(&mut self, ed: &mut Editor) -> AppResult<()> {
        self.line_index = ed.count_lines();
        ed.append_line(&self.text);   // Editor 的“原始操作”
        Ok(())
    }

    fn undo(&mut self, ed: &mut Editor) -> AppResult<()> {
        ed.pop_line()
    }
}

pub fn cmd_append(app: &mut Application, args: &[String]) -> AppResult<Outcome> {
    let text = args
        .get(0)
        .ok_or_else(|| crate::error::AppError::InvalidArgs("append <text>".into()))?;

    let cmd = AppendLineCommand::new(text.clone());

    app.workspace.exec_doc(Box::new(cmd))?;

    Ok(Outcome {
        print: None,
        log: Some(format!("append \"{}\"", text)),
        exit: false,
    })
}

pub const APPEND_COMMAND: CommandDef = CommandDef {
    name: "append",
    handler: cmd_append,
};