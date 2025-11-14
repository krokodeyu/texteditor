use crate::{
    error::AppResult,
    editor::Editor,
};

pub trait DocCommand {
    fn execute(&mut self, ed: &mut Editor) -> AppResult<()>;
    fn undo(&mut self, ed: &mut Editor) -> AppResult<()>;
}
