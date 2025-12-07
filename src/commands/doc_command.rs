use crate::{
    error::AppResult,
    text_editor::TextEditor,
};

pub trait DocCommand {
    fn execute(&mut self, ed: &mut TextEditor) -> AppResult<()>;
    fn undo(&mut self, ed: &mut TextEditor) -> AppResult<()>;
}
