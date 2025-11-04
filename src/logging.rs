use std::{collections::HashSet, fs::OpenOptions, io::Write, path::{Path, PathBuf}};
use chrono::Local;
use crate::event::{Event, Subscriber};

pub struct Logger { session_written: HashSet<String> }
impl Logger {
    pub fn new() -> Self { Self { session_written: HashSet::new() } }
    fn logfile_for(src: &Path) -> PathBuf {
        let dir = src.parent().unwrap_or_else(|| Path::new("."));
        let base = src.file_name().unwrap_or_default().to_string_lossy();
        dir.join(format!(".{}.log", base))
    }
}
impl Subscriber for Logger {
    fn on_event(&mut self, e: &Event) {
        match e {
            Event::SessionStart => { /* 可在此写 banner，本文按“每文件第一次写入再写” */ }
            Event::Command { file, cmdline, logging_enabled } => {
                if !*logging_enabled { return; }
                if let Some(f) = file {
                    let path = Self::logfile_for(f);
                    let key = path.display().to_string();
                    let write = |line: &str| -> std::io::Result<()> {
                        let mut fh = OpenOptions::new().create(true).append(true).open(&path)?;
                        writeln!(fh, "{}", line)?;
                        Ok(())
                    };
                    if !self.session_written.contains(&key) {
                        let head = format!("session start at {}", Local::now().format("%Y%m%d %H:%M:%S"));
                        if write(&head).is_err() { eprintln!("[warn] 写日志失败（session）"); return; }
                        self.session_written.insert(key);
                    }
                    let line = format!("{} {}", Local::now().format("%Y%m%d %H:%M:%S"), cmdline);
                    if write(&line).is_err() { eprintln!("[warn] 写日志失败"); }
                }
            }
        }
    }
}
