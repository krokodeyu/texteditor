//! 事件订阅者：写入日志文件。

use std::{
    collections::HashSet,
    fs::OpenOptions,
    io::Write,
    path::{PathBuf},
};
use chrono::Local;
use crate::event::{Event, Subscriber};

pub struct Logger {
    written: HashSet<PathBuf>,
    base_dir: PathBuf,
}

impl Logger {
    pub fn new(base_dir: PathBuf) -> Self { Self { written: HashSet::new(), base_dir: base_dir } }

    fn logfile_for(&self, path: &Option<PathBuf>) -> PathBuf {
        match path {
            Some(p) => {
                // 文件日志：放在 base_dir 下，名字基于文件名
                let file_name = p.file_name().unwrap_or_default().to_string_lossy();
                self.base_dir.join(format!(".{}.log", file_name))
            }
            None => {
                // app 级别日志：work_dir/.app.log
                self.base_dir.join(".app.log")
            }
        }
    }
}

impl Subscriber for Logger {
    fn on_event(&mut self, e: &Event) {
        match e {
            Event::SessionStart => {}
            Event::Command { file, cmd } => {
                let path = self.logfile_for(file);
                let write_header = self.written.insert(path.clone());
                if let Ok(mut f) = OpenOptions::new().append(true).create(true).open(&path) {
                    if write_header {
                        let _ = writeln!(f, "session start at {}", Local::now());
                    }
                    let _ = writeln!(f, "{} {}", Local::now().format("%Y-%m-%d %H:%M:%S"), cmd);
                }
            }
            Event::Error { code, message } => {
                let path = self.base_dir.join(".app.log");
                if let Ok(mut f) = OpenOptions::new().append(true).create(true).open(path) {
                    let _ = writeln!(f, "[error:{}] {} {}", code, Local::now(), message);
                }
            }
        }
    }
}
