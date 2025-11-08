//! 命令分发路由
//! 把字符串映射到具体的处理逻辑。

use std::collections::HashMap;
use crate::{
    application::Application, 
    outcome::Outcome, 
    error::{AppResult, AppError},
    command::*,
};

pub type Handler = fn(&mut Application, &[String]) -> AppResult<Outcome>;

pub struct Router {
    map: HashMap<String, Handler>,
}

impl Router {
    // 初始化：注册函数表。
    pub fn new() -> Self {
        let mut r = Self { map: HashMap::new() };
        r.register("load",    cmd_load);
        r.register("append",  cmd_append);
        r.register("show",    cmd_show);
        r.register("exit",    cmd_exit);
        r.register("save",    cmd_save);
        r
    }

    pub fn register(&mut self, name: &str, handler: Handler) -> &mut Self {
        let key = name.trim().to_ascii_lowercase();
        self.map.insert(key, handler);
        self
    }

    /// 解析：“要调用哪个处理器”和“参数”
    pub fn resolve(&self, line: &str) -> AppResult<(Handler, Vec<String>)> {
        let (cmd, args) = parse_command(line)?;
        let h = self.map.get(&cmd).ok_or(AppError::UnknownCommand(cmd))?;
        Ok((*h, args))
    }
}


fn parse_command(line: &str) -> AppResult<(String, Vec<String>)> {
    let parts = shell_words::split(line)
        .map_err(|_| AppError::InvalidArgs("parse failed".into()))?;

    if parts.is_empty() {
        return Ok((String::new(), Vec::new()));
    }
    let cmd  = parts[0].clone();
    let args = parts.into_iter().skip(1).collect();
    Ok((cmd, args))
}
