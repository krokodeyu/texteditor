// edit-id <oldId> <newId>
use crate::{
    commands::xml_command::XmlCommand,
    application::Application,
    error::{AppError, AppResult},
    xml_editor::XmlEditor,
    outcome::Outcome,
};
use super::CommandDef;

pub struct EditIdCommand {
    old_id: String,
    new_id: String, 
}

impl XmlCommand for EditIdCommand {
    fn execute(&mut self, ed: &mut XmlEditor) -> AppResult<()> {
        ed.change_attr_id(&self.old_id, &self.new_id)
    }

    fn undo(&mut self, ed: &mut XmlEditor) -> AppResult<()> {
        ed.change_attr_id(&self.new_id, &self.old_id)
    }
}

pub fn command_edit_id(app: &mut Application, args: &[String]) -> AppResult<Outcome> {
    if args.len() != 2 {
        return Err(AppError::InvalidArgs(
            "usage: edit-id <oldId> <newId>".into(),
        ));
    }
    let old_id = args[0].clone();
    let new_id = args[1].clone();

    if old_id == "root" {
        return Err(AppError::InvalidArgs(
            "can't change root id!".into(),
        ));
    }
    
    let cmd: EditIdCommand = EditIdCommand{
        old_id: old_id.clone(), 
        new_id: new_id.clone(),
    };

    app.workspace.exec_xml(Box::new(cmd))?;

    Ok(Outcome::log(format!("edit-id {} {}", old_id, new_id)))
}

pub const EDIT_ID_COMMAND: CommandDef = CommandDef {
    name: "edit-id",
    handler: command_edit_id,
};