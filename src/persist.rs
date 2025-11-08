//! Memento持久化模块。
//! 供Workspace调用。

use std::{collections::HashMap, fs, path::Path};
use serde::{Serialize, Deserialize};
use crate::error::AppResult;

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct FileFlags { pub modified: bool, pub logging: bool }

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct WorkspaceMemento {
    pub open_files: HashMap<String, FileFlags>,
    pub active: Option<String>,
}

impl WorkspaceMemento {
    pub fn save(&self, path: &Path) -> AppResult<()> {
        let data = serde_json::to_string_pretty(self)?;
        fs::write(path, data)?;
        Ok(())
    }

    pub fn load(path: &Path) -> AppResult<Self> {
        let s = fs::read_to_string(path)?;
        Ok(serde_json::from_str(&s)?)
    }
}