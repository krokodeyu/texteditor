use crate::{
    application::Application, 
    outcome::Outcome, 
    error::{AppResult, AppError}
};
use std::path::PathBuf;
use super::CommandDef;

pub fn command_xml_tree(app: &mut Application, args: &[String]) -> AppResult<Outcome>{
    if args.len() != 1 {
        return Err(AppError::InvalidArgs(
            "usage: xml-tree <file>".into(),
        ));
    }
    let raw_path: &str = &args[0];
    let path: PathBuf= app.workspace.resolve_path(Some(raw_path));
    let content = app.workspace.show_xml_tree(path)?;

    Ok(Outcome::print(content))
}

pub const XML_TREE_COMMAND: CommandDef = CommandDef {
    name: "xml-tree",
    handler: command_xml_tree,
};