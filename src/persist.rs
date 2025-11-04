use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug)]
pub struct WorkspaceMemento {
    pub open_files: HashMap<String, FileFlags>,
    pub active: Option<String>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FileFlags {
    pub modified: bool,
    pub logging: bool
}

pub fn save_workspace(m: &WorkspaceMemento) -> std::io::Result<()> {
    let s = serde_json::to_string_pretty(m).unwrap();
    std::fs::write(".editor_workspace", s)
}

pub fn load_workspace() -> Option<WorkspaceMemento> {
    let s = std::fs::read_to_string(".editor_workspace").ok()?;
    serde_json::from_str(&s).ok()
}