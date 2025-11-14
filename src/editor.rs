//! 文本编辑器：行数组存储内容。

use std::fmt::Write;
use std::fs;
use std::path::Path;

use crate::error::AppResult;

#[derive(Default)]
pub struct Editor {
    lines: Vec<String>,
    modified: bool,
    logging: bool,
}

impl Editor {
    pub fn new() -> Self { Self::default() }

    pub fn append(&mut self, text: &str) {
        self.lines.push(text.to_string());
        self.modified = true;
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
    }

    pub fn save_to(&mut self, p: impl AsRef<Path>) -> AppResult<()> {
        fs::write(p.as_ref(), self.to_string())?;
        self.modified = false;
        Ok(())
    }

    pub fn to_string(&self) -> String {
        self.lines.join("\n")
    }

    pub fn count_lines(&self) -> usize { self.lines.len() }
    pub fn line_at(&self, idx: usize) -> Option<&str> { self.lines.get(idx).map(|s| s.as_str()) }
    pub fn set_modified(&mut self, modified: bool) { self.modified = modified }
    pub fn set_logging(&mut self, logging: bool) { self.logging = logging }
    pub fn is_modified(&self) -> bool { self.modified }
    pub fn logging_enabled(&self) -> bool { self.logging }
}
