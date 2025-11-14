// router.rs

use std::collections::HashMap;
use crate::{
    commands::{
        COMMANDS, 
        CommandDef, 
        Handler,
    },
    error::{
        AppResult, 
        AppError
    },
};
use shell_words::split;

pub struct Router {
    table: HashMap<&'static str, Handler>,
}

impl Router {
    pub fn new() -> Self {
        let mut router = Router {
            table: HashMap::new(),
        };
        // 启动时把所有命令注册进去
        router.register_all(COMMANDS);
        router
    }

    fn register_all(&mut self, commands: &[CommandDef]) {
        for cmd in commands {
            // 同名覆盖就覆盖，问题不大，你也可以加检查
            self.table.insert(cmd.name, cmd.handler);
        }
    }

    /// 只解析，不执行：
    /// 返回：(handler 函数指针, 参数 Vec<String>)
    pub fn resolve(&self, line: &str) -> AppResult<(Handler, Vec<String>)> {
        let parts = split(line)  // 支持引号、转义、空格、特殊符号
            .map_err(|e| AppError::InvalidCommand(e.to_string()))?;

        if parts.is_empty() {
            return Err(AppError::InvalidCommand("空命令".into()));
        }

        let cmd_name = &parts[0];
        let args = parts[1..].to_vec();

        let handler = self.table
            .get(cmd_name.as_str())
            .ok_or_else(|| AppError::UnknownCommand(cmd_name.clone()))?;

        Ok((*handler, args))
    }
}
