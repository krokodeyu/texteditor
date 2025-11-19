use crate::{application::Application, outcome::Outcome, error::AppResult};
use std::path::PathBuf;
use super::CommandDef;

pub fn cmd_init(app: &mut Application, args: &[String]) -> AppResult<Outcome> {
    let raw_arg: String = args.get(0).cloned().unwrap_or_else(|| ".".into());
    let path: PathBuf = app.workspace.resolve_path(Some(raw_arg.as_str()));
    let logging: bool = match args.get(1).map(|s| s.as_str()) {
        None => false,
        Some("with-log") => true,
        Some(_) => false,
    };
    app.workspace.init(&path, logging)?;
    app.workspace.edit(&path)?;
    Ok(Outcome {
        print: Some(format!("Initialized {}", raw_arg)),
        log: Some(format!("init {}", raw_arg)),
        exit: false,
    })
}

pub const INIT_COMMAND: CommandDef = CommandDef {
    name: "init",
    handler: cmd_init,
};