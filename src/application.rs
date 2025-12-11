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
                        if self.workspace.check_modified() {
                            // 询问用户是否保存
                            if !self.confirm_exit_and_maybe_save()? {
                                // 用户选择取消退出（比如按了 c），继续下一轮循环
                                continue;
                            }
                        }
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

    /// 在退出前检查是否有未保存修改，询问用户是否保存。
    /// 返回 Ok(true) 表示可以退出，Ok(false) 表示取消退出。
    fn confirm_exit_and_maybe_save(&mut self) -> AppResult<bool> {
        // 没有未保存修改，直接退出
        if !self.workspace.check_modified() {
            return Ok(true);
        }

        println!("[warn] unsave change detected.");
        println!("choose one:");
        println!("  s) save and exit.");
        println!("  d) discard and exit.");
        println!("  c) cancel.");

        loop {
            print!("请输入选择 (s/d/c) > ");
            io::stdout().flush().ok();

            let mut line = String::new();
            if io::stdin().read_line(&mut line).is_err() {
                // 读输入失败时，谨慎起见当作取消退出
                return Ok(false);
            }

            match line.trim().to_lowercase().as_str() {
                "s" | "save" => {
                    if let Err(e) = self.workspace.save_all() {
                        eprintln!("[error] 保存文件时出错: {}", e);
                        // 出错时不给你强行退出，返回 false 让你自己决定
                        return Ok(false);
                    }
                    return Ok(true);
                }
                "d" | "discard" => {
                    // 不保存，直接退出
                    return Ok(true);
                }
                "c" | "cancel" | "" => {
                    // 取消退出
                    return Ok(false);
                }
                _ => {
                    println!("无效输入，请输入 s(保存)/d(不保存)/c(取消)。");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use tempfile::tempdir;

    use crate::event::Subscriber;
    use crate::error::AppResult;

    /// 记录所有事件的订阅者，用于测试 EventBus 行为。
    /// 内部用 Arc<Mutex<...>>，这样整体是 Send，满足 Subscriber 的约束。
    struct RecordingSubscriber {
        events: Arc<Mutex<Vec<Event>>>,
    }

    impl RecordingSubscriber {
        fn new(shared: Arc<Mutex<Vec<Event>>>) -> Self {
            Self { events: shared }
        }
    }

    impl Subscriber for RecordingSubscriber {
        fn on_event(&mut self, event: &Event) {
            // 测试环境里简单粗暴地 unwrap 就行
            let mut vec = self.events.lock().unwrap();
            vec.push(event.clone());
        }
    }

    /// 构造一个在临时目录下运行的 Application：
    /// - Workspace.base_dir = <tmp>/work_dir
    /// - EventBus 只挂一个 RecordingSubscriber（不挂 Logger）
    fn new_test_app(
    ) -> AppResult<(Application, Arc<Mutex<Vec<Event>>>, tempfile::TempDir)> {
        let tmp = tempdir()?; // 每个测试一个独立目录

        let workspace = Workspace::default();

        // EventBus：注册 RecordingSubscriber
        let mut bus = EventBus::new();
        let shared_events: Arc<Mutex<Vec<Event>>> = Arc::new(Mutex::new(Vec::new()));
        bus.subscribe(Box::new(RecordingSubscriber::new(shared_events.clone())));

        // Router：正常初始化
        let router = Router::new();

        let app = Application {
            router,
            workspace,
            bus,
        };

        Ok((app, shared_events, tmp))
    }

    #[test]
    fn save_workspace_memento_writes_file_in_workspace_base_dir() -> AppResult<()> {
        let (app, _events, _tmp) = new_test_app()?;
        let base = app.workspace.get_base_dir().to_path_buf();

        app.save_workspace_memento()?;

        let ws_path = base.join(".editor_workspace");
        assert!(
            ws_path.exists(),
            "expected workspace memento at {:?}",
            ws_path
        );

        let meta = std::fs::metadata(&ws_path)?;
        assert!(meta.len() > 0, ".editor_workspace should not be empty");

        Ok(())
    }

    #[test]
    fn successful_command_publishes_command_event() -> AppResult<()> {
        let (mut app, events, _tmp) = new_test_app()?;

        // 模拟 run() 里的核心逻辑：解析一行命令，然后执行 handler，再根据 Outcome.log 发送 Event::Command
        let line = "load a.txt";

        // 1. 解析
        let (handler, args) = app.router.resolve(line)?;

        // 2. 执行命令
        let outcome = handler(&mut app, &args)?;

        // 3. 按 Application::run 的逻辑发送命令事件
        if let Some(cmd) = outcome.log {
            app.bus.publish(Event::Command {
                file: app.workspace.active_file_path(),
                cmd,
            });
        }

        let evs = events.lock().unwrap();
        assert!(
            evs.iter().any(|e| matches!(e, Event::Command { .. })),
            "expected at least one Event::Command, got: {:?}",
            *evs
        );

        let (file_opt, cmd_str) = evs
            .iter()
            .find_map(|e| {
                if let Event::Command { file, cmd } = e {
                    Some((file.clone(), cmd.clone()))
                } else {
                    None
                }
            })
            .expect("no Event::Command found");

        assert!(
            cmd_str.contains("load") && cmd_str.contains("a.txt"),
            "unexpected cmd log: {:?}",
            cmd_str
        );
        assert!(
            file_opt.is_some(),
            "expected Some(path) for Event::Command.file, got None"
        );

        Ok(())
    }

    #[test]
    fn publish_error_sends_error_event_to_bus() -> AppResult<()> {
        let (mut app, events, _tmp) = new_test_app()?;

        // 构造一个简单的错误
        let err = AppError::InvalidArgs("bad args".into());

        // 调用 Application 的错误发布接口
        app.publish_error(err);

        let evs = events.lock().unwrap();
        assert!(
            evs.iter().any(|e| matches!(e, Event::Error { .. })),
            "expected at least one Event::Error, got: {:?}",
            *evs
        );

        // 检查 message
        let (code, msg) = evs
            .iter()
            .find_map(|e| {
                if let Event::Error { code, message } = e {
                    Some((*code, message.clone()))
                } else {
                    None
                }
            })
            .expect("no Event::Error found");

        // code 的值由 AppError::code() 决定，这里主要盯 message
        assert!(
            msg.contains("bad args"),
            "unexpected error message: code={code}, msg={:?}",
            msg
        );

        Ok(())
    }
}
