use crate::{
    error::AppResult,
    xml_editor::XmlEditor,
};

pub trait XmlCommand {
    fn execute(&mut self, ed: &mut XmlEditor) -> AppResult<()>;
    fn undo(&mut self, ed: &mut XmlEditor) -> AppResult<()>;
}