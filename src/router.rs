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

// 所有命令在这里注册。
static REGISTRY: &[(&str, Handler)] = &[
    ("load",    cmd_load),
    ("append",  cmd_append),
    ("show",    cmd_show),
    ("exit",    cmd_exit),
    ("save",    cmd_save),
];

pub struct Router {
    map: HashMap<String, Handler>,
}

impl Router {
    // 初始化：注册函数表。
    pub fn new() -> Self {
        let mut map = HashMap::with_capacity(REGISTRY.len());
        for &(name, handler) in REGISTRY {
            map.insert(name.to_string(), handler);
        }
        Self { map }
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
