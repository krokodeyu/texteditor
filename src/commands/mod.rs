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
mod close;
mod delete;
mod dir_tree;
mod load;
mod log;
mod show;
mod edit;
mod editor_list;
mod exit;
mod init;
mod insert;
mod save;
mod undo;
mod redo;
mod replace;
mod util;
pub mod doc_command;

// 导出子模块内部的 CommandDef 列表
use append::APPEND_COMMAND;
use close::CLOSE_COMMAND;
use delete::DELETE_COMMAND;
use dir_tree::DIR_TREE_COMMAND;
use load::LOAD_COMMAND;
use log::LOG_ON_COMMAND;
use log::LOG_OFF_COMMAND;
use log::LOG_SHOW_COMMAND;
use show::SHOW_COMMAND;
use edit::EDIT_COMMAND;
use editor_list::LIST_COMMAND;
use exit::EXIT_COMMAND;
use init::INIT_COMMAND;
use insert::INSERT_COMMAND;
use save::SAVE_COMMAND;
use undo::UNDO_COMMAND;
use redo::REDO_COMMAND;
use replace::REPLACE_COMMAND;

/// 全局静态命令表
pub static COMMANDS: &[CommandDef] = &[
    APPEND_COMMAND,
    CLOSE_COMMAND,
    DELETE_COMMAND,
    DIR_TREE_COMMAND,
    LOAD_COMMAND,
    LOG_ON_COMMAND,
    LOG_OFF_COMMAND,
    LOG_SHOW_COMMAND,
    SHOW_COMMAND,
    EDIT_COMMAND,
    LIST_COMMAND,
    EXIT_COMMAND,
    INIT_COMMAND,
    INSERT_COMMAND,
    SAVE_COMMAND,
    UNDO_COMMAND,
    REDO_COMMAND,
    REPLACE_COMMAND,
];
