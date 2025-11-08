//! 定义事件与观察者系统。

use std::path::PathBuf;

#[derive(Clone, Debug)]
pub enum Event {
    SessionStart,
    Command { file: Option<PathBuf>, cmd: String },
    Error { code: u32, message: String },
}

pub trait Subscriber: Send {
    fn on_event(&mut self, e: &Event);
}

pub struct EventBus {
    subs: Vec<Box<dyn Subscriber>>,
}

impl EventBus {
    pub fn new() -> Self { Self { subs: vec![] } }

    pub fn subscribe(&mut self, s: Box<dyn Subscriber>) {
        self.subs.push(s);
    }

    pub fn publish(&mut self, e: Event) {
        for s in self.subs.iter_mut() {
            s.on_event(&e);
        }
    }
}
