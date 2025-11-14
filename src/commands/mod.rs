//! commands/mod.rs
//!
//! 命令统一定义为：名字 + handler 函数
//! handler 的签名与原来相同

use crate::{application::Application, outcome::Outcome, error::AppResult};

pub type Handler = fn(&mut Application, &[String]) -> AppResult<Outcome>;

pub struct CommandDef {
    pub name: &'static str,
    pub handler: Handler,
}

// 各命令模块
mod append;
mod load;
mod show;
mod exit;
mod save;
mod undo;
mod redo;
pub mod doc_command;

// 导出子模块内部的 CommandDef 列表
use append::APPEND_COMMAND;
use load::LOAD_COMMAND;
use show::SHOW_COMMAND;
use exit::EXIT_COMMAND;
use save::SAVE_COMMAND;
use undo::UNDO_COMMAND;
use redo::REDO_COMMAND;

/// 全局静态命令表
pub static COMMANDS: &[CommandDef] = &[
    APPEND_COMMAND,
    LOAD_COMMAND,
    SHOW_COMMAND,
    EXIT_COMMAND,
    SAVE_COMMAND,
    UNDO_COMMAND,
    REDO_COMMAND,
];
