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

    /// 在指定 (line, col) 位置插入文本
    ///
    /// 约定：
    /// - `line` 从 1 开始
    /// - `col` 从 1 开始，按字节计数
    /// - 插入位置可以在行尾后一位（等价于追加）
    /// - 为简化，不允许插入带换行符的字符串
    pub fn insert_text(&mut self, line: usize, col: usize, text: &str) -> AppResult<()> {
        if text.contains('\n') {
            return Err(AppError::InvalidArgs(
                "insert_text: text must not contain newline".into(),
            ));
        }

        // 空文件的特殊情况：只允许在 1:1 插入，视为创建第一行
        if self.lines.is_empty() {
            if line != 1 || col != 1 {
                return Err(AppError::InvalidArgs(
                    "empty editor: can only insert at 1:1".into(),
                ));
            }
            self.lines.push(text.to_string());
            self.modified = true;
            return Ok(());
        }

        let line_str = self.line_mut(line)?;
        let len = line_str.len();

        if col == 0 || col > len + 1 {
            return Err(AppError::InvalidArgs(format!(
                "column {} out of range for line length {} (valid 1..={} for insert)",
                col,
                len,
                len + 1
            )));
        }

        let byte_idx = col - 1; // 按字节偏移
        // String::insert_str 的下标必须是合法字节边界，这里按字节计数，默认认为输入是 ASCII / 简单场景
        line_str.insert_str(byte_idx, text);

        self.modified = true;
        Ok(())
    }

    /// 从 (line, col) 开始删除 len 个字节
    ///
    /// 约定：
    /// - `line` 从 1 起
    /// - `col` 从 1 起，按字节计数
    /// - 删除范围不能超出该行末尾
    pub fn delete_text(&mut self, line: usize, col: usize, len: usize) -> AppResult<()> {
        if len == 0 {
            // 删除 0 个字节视为 no-op
            return Ok(());
        }

        let line_str = self.line_mut(line)?;
        let line_len = line_str.len();

        if col == 0 || col > line_len {
            return Err(AppError::InvalidArgs(format!(
                "column {} out of range for delete on line length {} (valid 1..={})",
                col, line_len, line_len
            )));
        }

        let start = col - 1;
        let end = start + len;
        if end > line_len {
            return Err(AppError::InvalidArgs(format!(
                "delete range [{}..{}) out of line length {}",
                start, end, line_len
            )));
        }

        line_str.replace_range(start..end, "");
        self.modified = true;
        Ok(())
    }

    /// 读取 (line, col) 起 len 个字节的内容，但不修改文档
    pub fn peek_text(&self, line: usize, col: usize, len: usize) -> AppResult<String> {
        if len == 0 {
            return Ok(String::new());
        }

        let line_str = self.line_ref(line)?;
        let line_len = line_str.len();

        if col == 0 || col > line_len {
            return Err(AppError::InvalidArgs(format!(
                "column {} out of range for peek on line length {} (valid 1..={})",
                col, line_len, line_len
            )));
        }

        let start = col - 1;
        let end = start + len;
        if end > line_len {
            return Err(AppError::InvalidArgs(format!(
                "peek range [{}..{}) out of line length {}",
                start, end, line_len
            )));
        }

        Ok(line_str[start..end].to_string())
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
    fn line_mut(&mut self, line: usize) -> AppResult<&mut String> {
        self.check_line(line)?;
        Ok(&mut self.lines[line - 1])
    }
    
    fn line_ref(&self, line: usize) -> AppResult<&str> {
        self.check_line(line)?;
        Ok(&self.lines[line - 1])
    }

    /// 检查行是否在行号内
    fn check_line(&self, line: usize) -> AppResult<()> {
        let n = self.count_lines();
        if line == 0 || line > n {
            return Err(AppError::InvalidArgs(format!(
                "line {} out of range (1..={})",
                line, n
            )));
        }
        Ok(())
    }

    fn line_at(&self, idx: usize) -> Option<&str> { self.lines.get(idx).map(|s| s.as_str()) }
}
