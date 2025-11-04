use std::path::PathBuf;

pub enum Event {
    SessionStart,
    Command { file: Option<PathBuf>, cmdline: String, logging_enabled: bool },
}

pub trait Subscriber { fn on_event(&mut self, e: &Event); }

pub struct Bus { subs: Vec<Box<dyn Subscriber>> }
impl Bus {
    pub fn new() -> Self { Self { subs: vec![] } }
    pub fn subscribe<S: Subscriber + 'static>(&mut self, s: S) { self.subs.push(Box::new(s)); }
    pub fn publish(&mut self, e: &Event) { for s in self.subs.iter_mut() { s.on_event(e); } }
}
