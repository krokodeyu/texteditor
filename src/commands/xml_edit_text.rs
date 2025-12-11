// edit-text <attr_id> <new_text>
use crate::{
    commands::xml_command::XmlCommand,
    application::Application,
    error::{AppError, AppResult},
    xml_editor::XmlEditor,
    outcome::Outcome,
};
use super::CommandDef;

pub struct EditTextCommand {
    attr_id: String,
    old_text: Option<String>,
    new_text: String, 
}

impl XmlCommand for EditTextCommand {
    fn execute(&mut self, ed: &mut XmlEditor) -> AppResult<()> {
        self.old_text = ed.change_text(&self.attr_id, &self.new_text)?;
        Ok(())
    }

    fn undo(&mut self, ed: &mut XmlEditor) -> AppResult<()> {
        if let Some(t) = &self.old_text {
            let _ = ed.change_text(&self.attr_id, t)?;
        } else {
            let _ = ed.remove_text(&self.attr_id)?;
        }
        Ok(())
    }
}

pub fn command_edit_text(app: &mut Application, args: &[String]) -> AppResult<Outcome> {
    if args.len() != 2 {
        return Err(AppError::InvalidArgs(
            "usage: edit-text <old_text> <new_text>".into(),
        ));
    }
    let attr_id = args[0].clone();
    let new_text = args[1].clone();

    let cmd: EditTextCommand = EditTextCommand{
        attr_id: attr_id.clone(),
        old_text: None,
        new_text: new_text.clone(),
    };

    app.workspace.exec_xml(Box::new(cmd))?;

    Ok(Outcome::log(format!("edit-text {} {}", attr_id, new_text)))
}

pub const EDIT_TEXT_COMMAND: CommandDef = CommandDef {
    name: "edit-text",
    handler: command_edit_text,
};