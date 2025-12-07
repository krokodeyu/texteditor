use crate::{
    error::{AppResult,AppError},
    text_editor::TextEditor,
    xml_editor::XmlEditor
};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EditorKind {
    Text,
    Xml,
}

impl Default for EditorKind {
    fn default() -> Self {
        EditorKind::Text   // 你可以认为 Text 是默认类型
    }
}

pub enum EditorInstance {
    Text(TextEditor),
    Xml(XmlEditor),
}

impl EditorInstance {
    pub fn undo(&mut self) -> AppResult<()> {
        match self {
            EditorInstance::Text(ed) => ed.undo(),
            EditorInstance::Xml(ed)  => ed.undo(),
        }
    }

    pub fn redo(&mut self) -> AppResult<()> {
        match self {
            EditorInstance::Text(ed) => ed.redo(),
            EditorInstance::Xml(ed)  => ed.redo(),
        }
    }

    pub fn is_modified(&self) -> bool {
        match self {
            EditorInstance::Text(ed) => ed.is_modified(),
            EditorInstance::Xml(ed) => ed.is_modified(),
        }
    }

    pub fn logging_enabled(&self) -> bool {
        match self {
            EditorInstance::Text(ed) => ed.logging_enabled(),
            EditorInstance::Xml(ed) => ed.logging_enabled(),
        }
    }

    pub fn set_modified(&mut self, modified: bool) {
        match self {
            EditorInstance::Text(ed) => ed.set_modified(modified),
            EditorInstance::Xml(ed) => ed.set_modified(modified),
        }
    }

    pub fn set_logging(&mut self, logging: bool) {
        match self {
            EditorInstance::Text(ed) => ed.set_logging(logging),
            EditorInstance::Xml(ed) => ed.set_logging(logging),
        }
    }

    pub fn save_to(&mut self, p: impl AsRef<std::path::Path>) -> AppResult<()> {
        match self {
            EditorInstance::Text(ed) => ed.save_to(&p),
            EditorInstance::Xml(ed) => ed.save_to(&p),
        }
    }

    // 将不同的变体从enum中取出来
    pub fn as_text_mut(&mut self) -> AppResult<&mut TextEditor> {
        match self {
            EditorInstance::Text(ed) => Ok(ed),
            _ => Err(AppError::InvalidArgs("current editor is not a text file".into())),
        }
    }

    pub fn as_xml_mut(&mut self) -> AppResult<&mut XmlEditor> {
        match self {
            EditorInstance::Xml(ed) => Ok(ed),
            _ => Err(AppError::InvalidArgs("current editor is not an XML file".into())),
        }
    }

    pub fn as_text(&self) -> AppResult<&TextEditor> {
        match self {
            EditorInstance::Text(ed) => Ok(ed),
            _ => Err(AppError::InvalidArgs("not a text editor".into())),
        }
    }

    pub fn as_xml(& self) -> AppResult<&XmlEditor> {
        match self {
            EditorInstance::Xml(ed) => Ok(ed),
            _ => Err(AppError::InvalidArgs("not a xml editor".into())),
        }
    }

    pub fn kind(&self) -> EditorKind {
        match self {
            EditorInstance::Text(_) => EditorKind::Text,
            EditorInstance::Xml(_) => EditorKind::Xml,
        }
    }
}
