// insert-before <tagName> <newId> <targetId> ["text"]
use crate::{
    application::Application,
    commands::xml_command::XmlCommand,
    xml_editor::XmlEditor,
    error::{AppError, AppResult},
    outcome::Outcome,
};
pub struct InsertBeforeCommand {
    tag_name: String,
    new_attr_id: String,
    target_attr_id: String,
    text: Option<String>,
}
use super::CommandDef;

impl XmlCommand for InsertBeforeCommand {
    fn execute(&mut self, ed: &mut XmlEditor) -> AppResult<()> {
        ed.insert_before(&self.tag_name, &self.new_attr_id, &self.target_attr_id, self.text.as_ref())
    }

    fn undo(&mut self, ed: &mut XmlEditor) -> AppResult<()> {
        let _ = ed.delete_node(&self.new_attr_id);
        Ok(())
    }
}

pub fn command_insert_before(app: &mut Application, args: &[String]) -> AppResult<Outcome>{
    if args.len() < 3 || args.len() > 4 {
        return Err(AppError::InvalidArgs(
            "insert-before <tagName> <newId> <targetId> [\"text\"]".into(),
        ));
    }

    let tag_name = args[0].clone();
    let new_attr_id = args[1].clone();
    let target_attr_id = args[2].clone();
    let text = args.get(3).cloned();

    let cmd: InsertBeforeCommand = InsertBeforeCommand {
        tag_name: tag_name.clone(),
        new_attr_id: new_attr_id.clone(),
        target_attr_id: target_attr_id.clone(),
        text: text.clone(),
    };

    app.workspace.exec_xml(Box::new(cmd))?;

    let log_text = text.unwrap_or("".to_string());
    Ok(Outcome::log(format!("insert_before {} {} {} {}", tag_name, new_attr_id, target_attr_id, log_text)))
}

pub const INSERT_BEFORE_COMMAND: CommandDef = CommandDef {
    name: "insert-before",
    handler: command_insert_before,
};