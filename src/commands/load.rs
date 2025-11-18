use crate::{application::Application, outcome::Outcome, error::AppResult};
use std::path::PathBuf;
use super::CommandDef;

pub fn cmd_load(app: &mut Application, args: &[String]) -> AppResult<Outcome> {
    let raw_arg: String = args.get(0).cloned().unwrap_or_else(|| ".".into());
    let path: PathBuf = app.workspace.resolve_path(Some(raw_arg.as_str()));

    app.workspace.load(path)?;

    Ok(Outcome {
        print: Some(format!("Loaded {}", raw_arg)),
        log: Some(format!("load {}", raw_arg)),
        exit: false,
    })
}

pub const LOAD_COMMAND: CommandDef = CommandDef {
    name: "load",
    handler: cmd_load,
};