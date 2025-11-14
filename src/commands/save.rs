use crate::{application::Application, outcome::Outcome, error::AppResult};
use super::CommandDef;

pub fn cmd_save(app: &mut Application, _args: &[String]) -> AppResult<Outcome> {
    match _args.get(0).map(|s| s.as_str()) {
        None => { app.workspace.save_all()?; }
        Some(path) => { app.workspace.save_file(path)?; }
    };
    
    Ok(Outcome::log("save"))
}

pub const SAVE_COMMAND: CommandDef = CommandDef {
    name: "save",
    handler: cmd_save,
};