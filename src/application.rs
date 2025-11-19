//! 入口层：负责交互循环、命令分发与事件发布。
use std::{
    io::{self, Write},
    path::{PathBuf, Path}
};
use crate::{
    error::{AppResult, AppError}, 
    event::{Event, EventBus}, 
    logging::Logger, 
    persist::WorkspaceMemento, 
    router::Router, 
    workspace::Workspace
};

pub struct Application {
    pub router: Router,
    pub workspace: Workspace,
    pub bus: EventBus,
}

impl Application {
    pub fn new() -> AppResult<Self> {
        let mut workspace = Workspace::default();
        let mut bus = EventBus::new();
        bus.subscribe(Box::new(Logger::new(workspace.get_base_dir())));

        let path = Path::new(".editor_workspace");
        if path.exists() {
            if let Ok(m) = WorkspaceMemento::load(path) {
                workspace.from_memento(m)?;
                println!("[info] restored workspace from .editor_workspace");
            }
        }

        Ok(Self { router: Router::new(), workspace, bus })
    }

    pub fn run(&mut self) -> AppResult<()> {
        self.bus.publish(Event::SessionStart);
        loop {
            print!("> ");
            io::stdout().flush()?;
            let mut line_buf = String::new();
            if io::stdin().read_line(&mut line_buf)? == 0 { break; }

            let line = line_buf.trim();
            if line.is_empty() { continue; }

            // —— 第一步：只用 &self.router 解析，拿到 handler 和 args —— //
            let (handler, args)
                = match self.router.resolve(line) {
                Ok(x) => x,
                Err(e) => {
                    self.publish_error(e);
                    continue;
                }
            };

            // —— 第二步：前一个不可变借用已结束；现在再可变借用 self 执行 —— //
            match handler(self, &args) {
                Ok(outcome) => {
                    if let Some(p) = outcome.print { println!("{p}"); }
                    if let Some(cmd) = outcome.log {
                        self.bus.publish(Event::Command {
                            file: self.workspace.active_file_path(),
                            cmd
                        });
                    }
                    if outcome.exit { 
                        if let Err(e) = self.save_workspace_memento() {
                            eprintln!("[warn] failed to save workspace: {}", e);
                        }
                        break; 
                    }
                }
                Err(e) => { self.publish_error(e); }
            }
        }
        Ok(())
    }

    pub fn save_workspace_memento(&self) -> AppResult<()> {
        let memento = self.workspace.to_memento();
        let base: PathBuf = self.workspace.get_base_dir();
        let path: PathBuf = base.join(".editor_workspace");
        memento.save(&path)?;
        println!("[info] workspace saved to {:?}", path);
        Ok(())
    }

    fn publish_error(&mut self, e: AppError) {
        e.report();
        self.bus.publish(
            Event::Error { 
                code: e.code(), 
                message: e.to_string() 
            });
    }
}
