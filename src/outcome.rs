//! 命令执行结果结构。

#[derive(Default)]
pub struct Outcome {
    pub print: Option<String>,
    pub log: Option<String>,
    pub exit: bool,
}

impl Outcome {
    pub fn print<S: Into<String>>(s: S) -> Self {
        Self { 
            print: Some(s.into()), 
            log: None, 
            exit: false 
        }
    }

    pub fn exit() -> Self {
        Self { print: Some("Goodbye!".into()),
        log: Some("exit".into()),
        // 向Application发出退出信号，让Application执行持久化并退出程序。
        exit: true, }
    }

    pub fn log<S: Into<String>>(s: S) -> Self {
        Self { 
            print: None, 
            log: Some(s.into()), 
            exit: false 
        }
    }
}
