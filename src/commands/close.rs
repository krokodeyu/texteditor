use crate::{
    application::Application, 
    error::{
        AppError, AppResult
    }, 
    outcome::Outcome,
};
use std::io::{self, Write};
use super::CommandDef;

pub fn cmd_close(app: &mut Application, _args: &[String]) -> AppResult<Outcome> {
    // 判断是否存在活跃文件。
    if !app.workspace.has_active() {
        return Err(AppError::InvalidArgs("no active file".into()));
    }
    
    let path = app.workspace.active_file_path();
    let path_str = path.expect("has active file, yet can't get its path");

    // 判断活跃文件是否修改
    let modified: bool = app.workspace.active_modified().expect("has active file, yet can't access modified flag"); 

    if modified {
        print!("File modified! Do you want to save it?(y/n):");
        std::io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input).expect("Failed to read input");

        let input = input.trim().to_lowercase();
        if input == "y" || input == "yes" {
            app.workspace.save_file(&path_str)?;
            println!("Saving...");
        } else if input == "n" || input == "no" {
            println!("Not saving...");
        } else {
            println!("Please enter 'y' or 'n'.");
        }
    } 

    app.workspace.close()?;

    Ok(Outcome {
        print: Some("file closed".into()),
        log: Some(format!("close {}", path_str.display())),
        exit: false,
    })
}

pub const CLOSE_COMMAND: CommandDef = CommandDef {
    name: "close",
    handler: cmd_close,
};