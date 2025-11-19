use crate::{
    application::Application, 
    outcome::Outcome, 
    error::{AppResult, AppError},
};
use std::path::PathBuf;
use super::CommandDef;

fn resolve_log_target(app: &Application, args: &[String]) -> AppResult<(PathBuf, String)> {
    // 有参数：按 base_dir 解析
    if let Some(raw) = args.get(0) {
        let path = app.workspace.resolve_path(Some(raw.as_str()));
        return Ok((path, raw.clone()));
    }

    // 无参数：使用当前活跃文件
    let path = app
        .workspace
        .active_file_path()
        .ok_or_else(|| AppError::InvalidArgs(
            "no file specified and no active file".into()
        ))?;

    // 为了日志好看一点，尽量把前缀 base_dir 去掉
    let rel_str = 
    if let Ok(rel) = path.strip_prefix(app.workspace.get_base_dir()) {
        rel.to_string_lossy().into_owned()
    } else {
        path.to_string_lossy().into_owned()
    };

    Ok((path, rel_str))
}

pub fn cmd_log_on(app: &mut Application, args: &[String]) -> AppResult<Outcome> {
    let (path, label) = resolve_log_target(app, args)?;
    app.workspace.log_on(path)?;   // 假设这个接口已经是按规范路径工作的

    Ok(Outcome::log(format!("log-on {}", label)))
}

pub const LOG_ON_COMMAND: CommandDef = CommandDef {
    name: "log-on",
    handler: cmd_log_on,
};

pub fn cmd_log_off(app: &mut Application, args: &[String]) -> AppResult<Outcome> {
    let (path, label) = resolve_log_target(app, args)?;
    app.workspace.log_off(path)?;

    Ok(Outcome::log(format!("log-off {}", label)))
}

pub const LOG_OFF_COMMAND: CommandDef = CommandDef {
    name: "log-off",
    handler: cmd_log_off,
};

pub fn cmd_log_show(app: &mut Application, args: &[String]) -> AppResult<Outcome> {
    let (path, _label) = resolve_log_target(app, args)?;

    let content: String = app.workspace.log_show(path)?;
    Ok(Outcome::print(content))
}

pub const LOG_SHOW_COMMAND: CommandDef = CommandDef {
    name: "log-show",
    handler: cmd_log_show,
};
