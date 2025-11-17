use crate::{application::Application, outcome::Outcome, error::AppResult};
use super::CommandDef;

pub fn cmd_init(app: &mut Application, args: &[String]) -> AppResult<Outcome> {
    let path: &String = args.get(0)
        .ok_or_else(|| crate::error::AppError::InvalidArgs("load <file>".into()))?;
    let logging: bool = match args.get(1).map(|s| s.as_str()) {
        None => false,
        Some("with-log") => true,
        Some(_) => false,
    };
    app.workspace.init(std::path::Path::new(path), logging)?;
    Ok(Outcome {
        print: Some(format!("Initialized {}", path)),
        log: Some(format!("init {}", path)),
        exit: false,
    })
}

pub const INIT_COMMAND: CommandDef = CommandDef {
    name: "init",
    handler: cmd_init,
};