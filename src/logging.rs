//! 事件订阅者：写入日志文件。

use std::{
    collections::HashSet,
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
};
use chrono::Local;
use crate::event::{Event, Subscriber};

pub struct Logger {
    written: HashSet<PathBuf>,
}

impl Logger {
    pub fn new() -> Self { Self { written: HashSet::new() } }

    fn logfile_for(path: &Option<PathBuf>) -> PathBuf {
        if let Some(p) = path {
            let dir = p.parent().unwrap_or_else(|| Path::new("."));
            let base = p.file_name().unwrap_or_default().to_string_lossy();
            dir.join(format!(".{}.log", base))
        } else {
            PathBuf::from(".app.log")
        }
    }
}

impl Subscriber for Logger {
    fn on_event(&mut self, e: &Event) {
        match e {
            Event::SessionStart => {}
            Event::Command { file, cmd } => {
                let path = Self::logfile_for(file);
                let write_header = self.written.insert(path.clone());
                if let Ok(mut f) = OpenOptions::new().append(true).create(true).open(&path) {
                    if write_header {
                        let _ = writeln!(f, "session start at {}", Local::now());
                    }
                    let _ = writeln!(f, "{} {}", Local::now().format("%Y-%m-%d %H:%M:%S"), cmd);
                }
            }
            Event::Error { code, message } => {
                let path = PathBuf::from(".app.log");
                if let Ok(mut f) = OpenOptions::new().append(true).create(true).open(path) {
                    let _ = writeln!(f, "[error:{}] {} {}", code, Local::now(), message);
                }
            }
        }
    }
}
