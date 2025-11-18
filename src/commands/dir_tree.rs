//! commands/dir_tree.rs
//!
//! dir-tree [path]
//! 以树状结构显示当前或指定目录的文件树。

use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
};

use crate::{application::Application, outcome::Outcome, error::{AppError, AppResult}};
use super::CommandDef;

pub const DIR_TREE_COMMAND: CommandDef = CommandDef {
    name: "dir-tree",
    handler: cmd_dir_tree,
};

pub fn cmd_dir_tree(app: &mut Application, args: &[String]) -> AppResult<Outcome> {
    // 解析路径
    let root_path: PathBuf = {
        let arg_opt = args.get(0).map(|s| s.as_str());
        app.workspace.resolve_path(arg_opt)   
    };

    if !root_path.exists() {
        return Err(AppError::InvalidArgs(format!(
            "path does not exist: {}",
            root_path.display()
        )));
    }

    // 生成树形字符串
    let mut out = String::new();

    // 顶部一行：显示根目录名（如果是 . 或没有文件名，就用 .）
    let root_name = root_path
        .file_name()
        .unwrap_or_else(|| OsStr::new("."))
        .to_string_lossy();
    out.push_str(&format!("{}\n", root_name));

    build_tree(&root_path, "", &mut out).map_err(AppError::Io)?;

    Ok(Outcome {
        print: Some(out),
        log: Some(format!(
            "dir-tree {}",
            args.get(0).cloned().unwrap_or_else(|| ".".into())
        )),
        exit: false,
    })
}

/// 递归构建树形结构
fn build_tree(path: &Path, prefix: &str, out: &mut String) -> std::io::Result<()> {
    // 读取子项
    let mut entries: Vec<_> = fs::read_dir(path)?
        .filter_map(|e| e.ok())
        .collect();

    // 排序：目录在前，文件在后，同类按名字排序
    entries.sort_by(|a, b| {
        let a_meta = a.metadata();
        let b_meta = b.metadata();

        let a_is_dir = a_meta.as_ref().map(|m| m.is_dir()).unwrap_or(false);
        let b_is_dir = b_meta.as_ref().map(|m| m.is_dir()).unwrap_or(false);

        match (a_is_dir, b_is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a
                .file_name()
                .to_string_lossy()
                .cmp(&b.file_name().to_string_lossy()),
        }
    });

    let count = entries.len();

    for (idx, entry) in entries.into_iter().enumerate() {
        let is_last = idx + 1 == count;

        let connector = if is_last { "└── " } else { "├── " };
        let name = entry.file_name().to_string_lossy().into_owned();

        out.push_str(prefix);
        out.push_str(connector);
        out.push_str(&name);
        out.push('\n');

        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            // 子目录前缀：最后一个用 "    "，中间用 "│   "
            let mut new_prefix = String::from(prefix);
            if is_last {
                new_prefix.push_str("    ");
            } else {
                new_prefix.push_str("│   ");
            }
            build_tree(&entry.path(), &new_prefix, out)?;
        }
    }

    Ok(())
}
