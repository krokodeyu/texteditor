use std::{collections::HashMap, io, path::{Path, PathBuf}};
use crate::editor::Editor;
use crate::persist::{WorkspaceMemento, FileFlags};

pub struct Workspace {
    editors: HashMap<PathBuf, Editor>,
    active: Option<PathBuf>
}

impl Workspace {
    pub fn new() -> Self {
        Self {
            editors: HashMap::new(),
            active: None
        }
    }

    pub fn init_file(&mut self, p: impl AsRef<Path>, with_log: bool) -> io::Result<&mut Editor> {
        let p = p.as_ref().to_path_buf();
        let mut ed = Editor::new_empty();
        ed.init_file(p.clone(), with_log)?;
        self.editors.insert(p.clone(), ed);
        self.active = Some(p.clone());
        Ok(self.editors.get_mut(&p).unwrap())
    }

    pub fn load_file(&mut self, p: impl AsRef<Path>) -> io::Result<&mut Editor> {
        let p = p.as_ref().to_path_buf();
        if !self.editors.contains_key(&p) {
            let mut ed = Editor::new_empty();
            ed.load_file(p.clone())?;
            self.editors.insert(p.clone(), ed);
        }
        self.active = Some(p.clone());
        Ok(self.editors.get_mut(&p).unwrap())
    }

    /* 
    need to use std::path::fs
    pub fn save(&mut self, p: Option<&Path>) -> io::Result<()> {
        let targets: Vec<PathBuf> = if let Some(one) = p {
            // 指定路径，只保存目标路径文件
            vec![one.to_path_buf()] 
        } else {
            // 不指定路径，将所有editor路径文件保存。
            self.editors.keys().cloned().collect()
        };
        for f in targets {
            if let Some(ed) = self.editors.get_mut(&f) {
                ed.save(None)?;
            }
        }
        Ok(())
    }
    */

    pub fn active_editor_mut(&mut self) -> anyhow::Result<&mut Editor> {
        let p = self.active.clone().ok_or_else(|| anyhow::anyhow!("无活动文件"))?;
        self.editors.get_mut(&p).ok_or_else(|| anyhow::anyhow!("内部错误：活动文件不存在"))
    }

    pub fn list_editors(&self) -> Vec<String> {
        let mut out = vec![];
        for (p, ed) in &self.editors {
            let starred = if Some(p) == self.active.as_ref() { "*" } else { " " };
            let modified = if ed.modified() { " [modified]" } else { "" };
            out.push(format!("{starred} {}{modified}", p.display()));
        }
        out
    }

    pub fn to_memento(&self) -> WorkspaceMemento {
        let mut map = HashMap::new();
        for (p, ed) in &self.editors {
            map.insert(
                p.display().to_string(),
                FileFlags{
                    modified: ed.modified(),
                    logging: ed.logging_enabled()
                }
            );
        }
        WorkspaceMemento { 
            open_files: map, 
            active: self.active.as_ref().map(|p| p.display().to_string())
        }
    }

    pub fn from_memento(&mut self, m: &WorkspaceMemento) {
        self.editors.clear();
        self.active = None;
        for (path, flags) in &m.open_files {
            let p = PathBuf::from(path);
            let mut ed = Editor::new_empty();
            // let _ 代表只执行函数，忽略返回值。
            let _ = ed.load_file(p.clone());
            ed.set_modified(flags.modified);
            ed.set_logging(flags.logging);
            self.editors.insert(p.clone(), ed);
        }
        if let Some(a) = &m.active {
            let p = PathBuf::from(a);
            if self.editors.contains_key(&p) {
                self.active = Some(p);
            }
        }
    }
}