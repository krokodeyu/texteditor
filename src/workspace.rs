//! 工作区
//! 管理多文件上下文、状态持久化。

use std::{
    collections::HashMap, 
    fs, 
    io,
    path::{Path, PathBuf}
};
use crate::{
    commands::doc_command::DocCommand, editor::Editor, error::{AppError, AppResult}, persist::{FileFlags, WorkspaceMemento}
};


#[derive(Default)]
pub struct Workspace {
    editors: HashMap<PathBuf, Editor>,
    active: Option<PathBuf>
}

impl Workspace {
    pub fn exec_doc(&mut self, cmd: Box<dyn DocCommand>) -> AppResult<()> {
        let ed = self.get_active_editor_mut()?;
        ed.exec_doc(cmd)
    }

    pub fn undo(&mut self) -> AppResult<()> {
        let ed = self.get_active_editor_mut()?;
        ed.undo()
    }

    pub fn redo(&mut self) -> AppResult<()> {
        let ed = self.get_active_editor_mut()?;
        ed.redo()
    }

    // 用AsRef<Path>，调用方可传入多种类型。
    pub fn open(&mut self, i_path: impl AsRef<Path>) -> AppResult<()> {
        let path = i_path.as_ref();
        let key: PathBuf = path.to_path_buf();

        let content = match fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) if e.kind() == io::ErrorKind::NotFound => String::new(),
            Err(e) => return Err(AppError::Io(e)),
        };

        // 如果已存在，直接读取；否则新建一个editor
        let ed = self
            .editors
            .entry(key.clone())
            // or_insert_with():需要一个显式闭包或者函数作传入值。
            .or_insert_with(Editor::new);

        ed.load_from(&content);
        self.active = Some(key);
        Ok(())
    }

    pub fn show(&self, start: Option<usize>, end: Option<usize>) -> AppResult<String> {
        let active = self
            .active
            .clone()
            .ok_or_else(|| AppError::InternalError("no active file".into()))?;

        let ed = self
            .editors
            .get(&active)
            .ok_or_else(|| AppError::InternalError("couldn't open active file".into()))?;

        let n = ed.count_lines();
        if n == 0 {
            return Ok("<empty>".to_string());
        }
        let s = start.unwrap_or(1).clamp(1, n);
        let e = end.unwrap_or(n).clamp(1,n);
        if e < s {
            return Err(AppError::InvalidArgs(format!("invalid range: {}..{}", s, e)));
        }
        Ok(ed.show(s, e))
    }

    pub fn save_file(&mut self, path: impl AsRef<Path>) -> AppResult<()> {
        let p = path.as_ref();
        let key: PathBuf = p.to_path_buf();

        let ed = self
            .editors
            .get_mut(&key)
            .ok_or_else(|| AppError::InvalidArgs("no such path".into()))?;

        ed.save_to(p)?;
        Ok(())
    }

    pub fn save_all(&mut self) -> AppResult<()> {
        for (p, ed) in self.editors.iter_mut() {
            ed.save_to(p)?;
        }
        Ok(())
    }

    pub fn active_file_path(&self) -> Option<PathBuf> {
        self.active.clone()
    }

    pub fn from_memento(&mut self, m: WorkspaceMemento) -> AppResult<()> {
        self.editors.clear();
        self.active = None;

        for (path_str, flags) in m.open_files {
            let path = PathBuf::from(&path_str);
            let content = match fs::read_to_string(&path) {
                Ok(s) => s,
                Err(e) if e.kind() == io::ErrorKind::NotFound => String::new(),
                Err(e) => return Err(AppError::Io(e)),
            };

            let mut editor = Editor::new();
            editor.load_from(&content);
            editor.set_modified(flags.modified);
            editor.set_logging(flags.logging);

            self.editors.insert(path, editor);
        }

        if let Some(active_str) = m.active {
            let active_path = PathBuf::from(&active_str);
            if self.editors.contains_key(&active_path) {
                self.active = Some(active_path);
            }
        }

        Ok(())
    }

    pub fn to_memento(&self) -> WorkspaceMemento {
        let mut open_files = HashMap::new();
        for (p, e) in &self.editors {
            open_files.insert(
                // to_string_lossy(): 以有损方式生成UTF-8文本。
                p.to_string_lossy().into_owned(),
                FileFlags {
                    modified: e.is_modified(),
                    logging: e.logging_enabled(),
                },
            );
        }
        WorkspaceMemento {
            open_files,
            active: self
                .active
                .as_ref()
                .map(|p| p.to_string_lossy().into_owned()),
        }
    }

    // 辅助函数
    fn get_active_editor_mut(&mut self) -> AppResult<&mut Editor> {
        let path = self
            .active
            .clone()
            .ok_or_else(|| AppError::InternalError("no active file.".into()))?;
        
        self.editors
            .get_mut(&path)
            .ok_or_else(|| AppError::InvalidArgs("active editor not found".into()))
    }
}