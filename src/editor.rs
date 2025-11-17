//! 文本编辑器：行数组存储内容。

use std::fmt::Write;
use std::fs;
use std::path::Path;

use crate::{
    error::{
        AppResult,
        AppError
    },
    commands::doc_command::DocCommand,
};

#[derive(Default)]
pub struct Editor {
    lines: Vec<String>,
    modified: bool,
    logging: bool,

    // Undo/Redo操作用栈实现。
    undo_stack: Vec<Box<dyn DocCommand>>,
    redo_stack: Vec<Box<dyn DocCommand>>,
}

impl Editor {
    // bool类型的默认值是false
    pub fn new() -> Self { Self::default() }

    pub fn exec_doc(&mut self, mut cmd: Box<dyn DocCommand>) -> AppResult<()> {
        cmd.execute(self)?;
        self.undo_stack.push(cmd);
        self.redo_stack.clear();
        self.modified = true;
        Ok(())
    }

    pub fn undo(&mut self) -> AppResult<()> {
        if let Some(mut cmd) = self.undo_stack.pop() {
            cmd.undo(self)?;
            self.redo_stack.push(cmd);
            self.modified = true;
            Ok(())
        } else {
            Err(AppError::InvalidArgs("nothing to undo".into()))
        }
    }

    pub fn redo(&mut self) -> AppResult<()> {
        if let Some(mut cmd) = self.redo_stack.pop() {
            cmd.execute(self)?;
            self.undo_stack.push(cmd);
            self.modified = true;
            Ok(())
        } else {
            Err(AppError::InvalidArgs("nothing to redo".into()))
        }
    }
    
    pub fn append_line(&mut self, text: &str) {
        self.lines.push(text.to_string());
    }

    pub fn pop_line(&mut self) -> AppResult<()> {
        self.lines.pop()
            .ok_or_else(|| AppError::InternalError("pop line failed".into()))?;
        Ok(())
    }

    pub fn show(&self, start: usize, end: usize) -> String {
        let mut out = String::new();
        for i in start..=end {
            let line = self.line_at(i-1).unwrap_or("");
            let _ = writeln!(&mut out, "{}: {}", i, line);
        }
        out
    }

    pub fn load_from(&mut self, content: &str) {
        self.lines = content.lines().map(|s| s.to_string()).collect();
        self.modified = false;
        self.logging = self
            .lines
            .first()
            .map(|line| line.trim() == "# log")
            .unwrap_or(false);
    }

    // 加问号后，IO异常会被自动转化为AppError，详见error.rs
    pub fn save_to(&mut self, p: impl AsRef<Path>) -> AppResult<()> {
        fs::write(p.as_ref(), self.to_string())?;
        self.modified = false;
        Ok(())
    }

    pub fn to_string(&self) -> String {
        self.lines.join("\n")
    }

    // 外部调用函数。
    pub fn count_lines(&self) -> usize { self.lines.len() }
    pub fn set_modified(&mut self, modified: bool) { self.modified = modified }
    pub fn set_logging(&mut self, logging: bool) { self.logging = logging }
    pub fn is_modified(&self) -> bool { self.modified }
    pub fn logging_enabled(&self) -> bool { self.logging }

    // 辅助函数。
    fn line_at(&self, idx: usize) -> Option<&str> { self.lines.get(idx).map(|s| s.as_str()) }
}
