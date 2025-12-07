use crate::{application::Application, outcome::Outcome, error::{AppResult, AppError}};
use crate::editor_instance::EditorKind;
use std::path::PathBuf;
use super::CommandDef;

pub fn cmd_init(app: &mut Application, args: &[String]) -> AppResult<Outcome> {
    // 命令格式：init <text|xml> <file> [with-log]
    if args.len() < 2 {
        return Err(AppError::InvalidArgs(
            "usage: init <text|xml> <file> [with-log]".into(),
        ));
    }

    // 1. 解析编辑器类型
    let kind_str = args[0].as_str();
    let kind = match kind_str {
        "text" => EditorKind::Text,
        "xml"  => EditorKind::Xml,
        other  => {
            return Err(AppError::InvalidArgs(format!(
                "unknown editor kind '{}', expected 'text' or 'xml'",
                other
            )));
        }
    };

    let raw_path: &str = &args[1];
    let path: PathBuf = app.workspace.resolve_path(Some(raw_path));
    let logging: bool = match args.get(2).map(|s| s.as_str()) {
        None => false,
        Some("with-log") => true,
        Some(_) => false,
    };

    app.workspace.init(kind, &path, logging)?;
    app.workspace.edit(&path)?;

    let mut msg = format!("Initialized {} editor for {}", kind_str, raw_path);
    if logging {
        msg.push_str(" with log");
    }

    Ok(Outcome {
        print: Some(msg),
        log: Some(format!(
            "init {} {}{}",
            kind_str,
            raw_path,
            if logging { " with-log" } else { "" }
        )),
        exit: false,
    })
}

pub const INIT_COMMAND: CommandDef = CommandDef {
    name: "init",
    handler: cmd_init,
};