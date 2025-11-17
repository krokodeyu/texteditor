use crate::{application::Application, outcome::Outcome, error::AppResult};
use super::CommandDef;

pub fn cmd_load(app: &mut Application, args: &[String]) -> AppResult<Outcome> {
    let path = args.get(0)
        .ok_or_else(|| crate::error::AppError::InvalidArgs("load <file>".into()))?;

    app.workspace.load(std::path::Path::new(path))?;

    Ok(Outcome {
        print: Some(format!("Loaded {}", path)),
        log: Some(format!("load {}", path)),
        exit: false,
    })
}

pub const LOAD_COMMAND: CommandDef = CommandDef {
    name: "load",
    handler: cmd_load,
};