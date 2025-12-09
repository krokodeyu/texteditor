use crate::{
    xml_editor::{DeletedNodeToken, XmlEditor},
    commands::xml_command::XmlCommand,
    error::{AppError, AppResult},
    application::Application,
    outcome::Outcome,
};
use super::CommandDef;

pub struct DeleteElementCommand {
    attr_id: String,
    token: Option<DeletedNodeToken>,
}

impl DeleteElementCommand {
    pub fn new(attr_id: String) -> Self {
        Self { attr_id, token: None }
    }
}

impl XmlCommand for DeleteElementCommand {
    fn execute(&mut self, ed: &mut XmlEditor) -> AppResult<()> {
        let token = ed.delete_node(&self.attr_id)?;
        self.token = Some(token);
        Ok(())
    }

    fn undo(&mut self, ed: &mut XmlEditor) -> AppResult<()> {
        let token = self
            .token
            .take()
            .ok_or_else(|| AppError::InternalError("delete undo missing token".into()))?;
        ed.restore_node(token)
    }
}

pub fn command_delete_element(app: &mut Application, args: &[String]) -> AppResult<Outcome>{
    if args.len() != 1 {
        return Err(AppError::InvalidArgs(
            "usage: delete-element <element-id>".into(),
        ));
    }
    let attr_id: String = args[0].clone();

    let cmd = DeleteElementCommand::new(attr_id.clone());
    app.workspace.exec_xml(Box::new(cmd))?;

    Ok(Outcome::log(format!("delete-element {}", attr_id)))
}

pub const DELETE_ELEMENT_COMMAND: CommandDef = CommandDef {
    name: "delete-element",
    handler: command_delete_element,
};