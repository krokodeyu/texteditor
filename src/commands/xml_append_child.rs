use crate::{
    application::Application, 
    outcome::Outcome, 
    error::{AppResult, AppError},
    commands::xml_command::XmlCommand,
    xml_editor::XmlEditor,
};
use super::CommandDef;


struct AppendChildCommand {
    tag_name: String,
    child_id: String,
    parent_id: String, 
    text: Option<String>,
}

impl XmlCommand for AppendChildCommand {
    fn execute(&mut self, ed: &mut XmlEditor) -> AppResult<()>{
        ed.append_child(&self.tag_name, &self.child_id, &self.parent_id, self.text.as_ref())
    }

    fn undo(&mut self, ed: &mut XmlEditor) -> AppResult<()>{
        let _ = ed.delete_node(&self.child_id)?;
        Ok(())
    }
}

pub fn command_append_child(app: &mut Application, args: &[String]) -> AppResult<Outcome> {
    if args.len() < 3 || args.len() > 4 {
        return Err(AppError::InvalidArgs(
            "usage: append-child <tag-name> <child-id> <parent-id> [text]".into(),
        ));
    }
    let tag_name: String = args[0].clone();
    let child_id: String = args[1].clone();
    let parent_id: String = args[2].clone();
    let text: Option<String> = args.get(3).cloned();

    let cmd = AppendChildCommand {
        tag_name:tag_name.clone(),
        child_id:child_id.clone(),
        parent_id:parent_id.clone(),
        text:text.clone(),
    };

    app.workspace.exec_xml(Box::new(cmd))?;

    let log_text = text.unwrap_or("".to_string());
    Ok(Outcome::log(format!("append_child {} {} {} {}", tag_name, child_id, parent_id, log_text)))
}

pub const APPEND_CHILD_COMMAND: CommandDef = CommandDef {
    name: "append-child",
    handler: command_append_child,
};