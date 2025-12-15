use crate::{
    application::Application, 
    outcome::Outcome, 
    error::{AppResult, AppError}
};
use std::path::PathBuf;
use super::CommandDef;

pub fn command_xml_tree(app: &mut Application, args: &[String]) -> AppResult<Outcome> {
    let content = match args.len() {
        0 => {
            // 不传参数：展示当前活动文件
            let active = app
                .workspace
                .active_file_path()
                .ok_or_else(|| AppError::InternalError("no active file".into()))?;

            app.workspace.show_xml_tree(active)?
        }
        1 => {
            // 传入参数：展示指定文件
            let raw_path: &str = &args[0];
            let path: PathBuf = app.workspace.resolve_path(Some(raw_path));
            app.workspace.show_xml_tree(path)?
        }
        _ => {
            return Err(AppError::InvalidArgs(
                "usage: xml-tree [file]".into(),
            ));
        }
    };

    Ok(Outcome::print(content))
}

pub const XML_TREE_COMMAND: CommandDef = CommandDef {
    name: "xml-tree",
    handler: command_xml_tree,
};