use std::{fs, io, path::PathBuf};

#[derive(Debug, Clone)]
pub struct Editor {
    path: Option<PathBuf>,
    buf: Vec<String>,
    modified: bool,
    logging_enabled: bool,
}

impl Editor {
    pub fn new_empty() -> Self {
        Self { path: None, buf: vec![], modified: false, logging_enabled: false }
    }

    pub fn init_file(&mut self, path: PathBuf, with_log: bool) -> io::Result<()> {
        let head = if with_log { "# log\n" } else { "" };
        fs::write(&path, head)?;
        self.path = Some(path);
        self.buf = if with_log { vec!["# log".into()] } else { vec![] };
        self.logging_enabled = with_log;
        self.modified = false;
        Ok(())
    }

    pub fn load_file(&mut self, path: PathBuf) -> io::Result<()> {
        let content = fs::read_to_string(&path).unwrap_or_default();
        self.path = Some(path);
        self.buf = if content.is_empty() {
            vec![]
        } else {
            content.split('\n').map(|s| s.to_string()).collect()
        };
        if let Some(last) = self.buf.last() { if last.is_empty() { /* 去掉末尾空行 */ } }
        self.logging_enabled = self.buf.first().map(|s| s == "# log").unwrap_or(false);
        self.modified = false;
        Ok(())
    }

    pub fn append(&mut self, text: &str) { self.buf.push(text.to_string()); self.modified = true; }

    pub fn show(&self, start: Option<usize>, end: Option<usize>) -> Vec<String> {
        let n = self.buf.len(); if n == 0 { return vec![]; }
        let s = start.unwrap_or(1).clamp(1, n);
        let e = end.unwrap_or(n).clamp(1, n);
        if e < s { return vec![]; }
        (s..=e).map(|i| format!("{}: {}", i, self.buf[i-1])).collect()
    }

    pub fn save(&mut self, file: Option<PathBuf>) -> io::Result<PathBuf> {
        if let Some(p) = file { self.path = Some(p); }
        let p = self.path.clone().ok_or_else(|| io::Error::new(io::ErrorKind::Other, "没有目标文件"))?;
        let content = if self.buf.is_empty() { String::new() } else { self.buf.join("\n") };
        fs::write(&p, content)?;
        self.modified = false;
        Ok(p)
    }

    // 访问器
    pub fn path(&self) -> Option<&PathBuf> { self.path.as_ref() }
    pub fn modified(&self) -> bool { self.modified }
    pub fn set_modified(&mut self, v: bool) { self.modified = v; }
    pub fn logging_enabled(&self) -> bool { self.logging_enabled }
    pub fn set_logging(&mut self, on: bool) { self.logging_enabled = on; }
}
