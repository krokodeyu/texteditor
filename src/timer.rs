//! 事件订阅者：统计每个文件的编辑时长。
//!
//! 计时策略：
//! - 以「两次命令事件之间的间隔」作为时间片
//! - 时间片记在上一条命令时的活跃文件（last_file）上
//! - 程序退出/订阅者析构时会做一次最终 flush，避免最后一段时间丢失

use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use crate::event::{Event, Subscriber};

/// 对外共享的「文件 -> 累计编辑时长」表
pub type SharedEditTimes = Arc<Mutex<HashMap<PathBuf, Duration>>>;

/// 追踪编辑时间的订阅者
pub struct EditTimeTracker {
    times: SharedEditTimes,
    last_file: Option<PathBuf>,
    last_instant: Option<Instant>,
}

impl EditTimeTracker {
    pub fn new(times: SharedEditTimes) -> Self {
        Self { times, last_file: None, last_instant: None }
    }

    /// 把从 prev_t 到 now 的时间片，累计到 prev_file
    fn add_slice(&mut self, prev_file: &PathBuf, prev_t: Instant, now: Instant) {
        let dt = now.saturating_duration_since(prev_t);
        let mut map = self.times.lock().unwrap();
        let entry = map.entry(prev_file.clone()).or_insert(Duration::ZERO);
        *entry += dt;
    }

    /// 在程序结束/退出前，把最后一段时间也结算掉（如果存在活跃文件）
    fn flush(&mut self) {
        let now = Instant::now();
        if let (Some(prev_file), Some(prev_t)) = (self.last_file.take(), self.last_instant) {
            self.add_slice(&prev_file, prev_t, now);
        }
        self.last_instant = Some(now);
    }
}

impl Drop for EditTimeTracker {
    fn drop(&mut self) {
        // 最终结算一次，避免最后一段时间丢失
        self.flush();
    }
}

impl Subscriber for EditTimeTracker {
    fn on_event(&mut self, e: &Event) {
        match e {
            Event::SessionStart => {
                self.last_file = None;
                self.last_instant = Some(Instant::now());
            }
            Event::Command { file, cmd: _ } => {
                let now = Instant::now();

                // 把「上一条命令」到现在这一段时间，记在 last_file 上。
                // 注意：这里不要用 cmd 过滤，否则像 save/exit 这样的命令会导致整段编辑时间丢失。
                if let (Some(prev_file), Some(prev_t)) = (self.last_file.take(), self.last_instant) {
                    self.add_slice(&prev_file, prev_t, now);
                }

                // 更新当前活跃文件与时间戳
                self.last_file = file.clone();
                self.last_instant = Some(now);
            }
            Event::Error { .. } => {
                // 错误事件不影响计时
            }
        }
    }
}
