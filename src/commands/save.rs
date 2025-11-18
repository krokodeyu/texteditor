use crate::{application::Application, outcome::Outcome, error::AppResult};
use super::CommandDef;

pub fn cmd_save(app: &mut Application, _args: &[String]) -> AppResult<Outcome> {
    match _args.get(0).map(|s| s.as_str()) {
        // 没有参数：保存所有已打开文件
        None => {
            app.workspace.save_all()?;
            Ok(Outcome::log("save all"))
        }
        // 有参数：按 base_dir 解析路径，再只保存这个文件
        Some(raw) => {
            let path = app.workspace.resolve_path(Some(raw));
            app.workspace.save_file(&path)?;
            Ok(Outcome::log(&format!("save {}", raw)))
        }
    }
}

pub const SAVE_COMMAND: CommandDef = CommandDef {
    name: "save",
    handler: cmd_save,
};