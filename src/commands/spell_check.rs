use std::path::PathBuf;

use crate::{
    application::Application,
    error::{AppError, AppResult},
    outcome::Outcome,
    spellcheck::{SpellError, SpellIssue},
    workspace::WorkspaceFileKind,
};

use super::CommandDef;

pub fn cmd_spell_check(app: &mut Application, args: &[String]) -> AppResult<Outcome> {
    // 解析参数：支持 --all / -a / [file]
    let parsed = SpellCheckArgs::parse(args)?;

    let mut out = String::new();
    out.push_str("拼写检查结果:\n");

    match parsed.mode {
        SpellCheckMode::All => {
            let files = app.workspace.list_open_files()?; // Workspace 需要提供
            if files.is_empty() {
                out.push_str("（workspace 中没有可检查的已加载文件）\n");
                return Ok(Outcome::print(out));
            }

            for file in files {
                append_file_result(app, &file, &mut out);
            }
        }
        SpellCheckMode::One(target) => {
            let file = match target {
                SpellCheckTarget::_Active => app.workspace
                    .active_file_path()
                    .ok_or_else(|| AppError::InternalError("no active file".into()))?,
                SpellCheckTarget::Path(p) => p,
            };

            // 单文件模式：如果目标文件出错，直接返回 Err，让全局 error 机制处理。
            out.push_str(&format!("\n[{}]\n", file.display()));
            let lines = spell_check_file(app, &file)?;
            if lines.is_empty() {
                out.push_str("未发现拼写错误\n");
            } else {
                for l in lines {
                    out.push_str(&l);
                    out.push('\n');
                }
            }
        }
    }

    Ok(Outcome::print(out))
}

// ======================= 参数解析 =======================

#[derive(Debug)]
struct SpellCheckArgs {
    mode: SpellCheckMode,
}

#[derive(Debug)]
enum SpellCheckMode {
    All,
    One(SpellCheckTarget),
}

#[derive(Debug)]
enum SpellCheckTarget {
    _Active,
    Path(PathBuf),
}

impl SpellCheckArgs {
    fn parse(args: &[String]) -> AppResult<Self> {
        if args.is_empty() {
            return Ok(Self { mode: SpellCheckMode::All });
        }

        // 只支持以下几种形式：
        // spell-check --all
        // spell-check -a
        // spell-check <file>
        if args.len() == 1 {
            let a0 = args[0].as_str();
            if a0 == "--all" || a0 == "-a" {
                return Ok(Self { mode: SpellCheckMode::All });
            }
            return Ok(Self {
                mode: SpellCheckMode::One(SpellCheckTarget::Path(PathBuf::from(a0))),
            });
        }

        Err(AppError::InvalidArgs(
            "usage: spell-check [file] | spell-check --all".into(),
        ))
    }
}

// ======================= 文件级处理 =======================

fn append_file_result(app: &Application, file: &PathBuf, out: &mut String) {
    out.push_str(&format!("\n[{}]\n", file.display()));

    match spell_check_file(app, file) {
        Ok(lines) => {
            if lines.is_empty() {
                out.push_str("no spelling errors.\n");
            } else {
                for l in lines {
                    out.push_str(&l);
                    out.push('\n');
                }
            }
        }
        Err(e) => {
            // 失败提示，但不影响其它文件/功能。
            // 同时复用全局错误编码规则，便于定位与一致性。
            out.push_str(&format!("[warn {}] {}\n", e.code(), e));
        }
    }
}

fn spell_check_file(app: &Application, file: &PathBuf) -> AppResult<Vec<String>> {
    let kind = app.workspace.file_kind(file)?;

    match kind {
        WorkspaceFileKind::Text => {
            let text = app.workspace.read_text_all(file)?;
            let issues = app
                .spell_checker
                .check(&text)
                .map_err(map_spell_error)?;

            Ok(format_text_issues(&text, &issues))
        }
        WorkspaceFileKind::Xml => {
            // XML：只检查元素文本内容
            let parts = app.workspace.extract_xml_element_texts(file)?;

            let mut lines = Vec::new();
            for (label, txt) in parts {
                let issues = app
                    .spell_checker
                    .check(&txt)
                    .map_err(map_spell_error)?;
                lines.extend(format_xml_issues(&label, &txt, &issues));
            }
            Ok(lines)
        }
        WorkspaceFileKind::_Other(ext) => Err(AppError::InvalidArgs(format!(
            "unsupported file type: {ext}"
        ))),
    }
}

fn map_spell_error(e: SpellError) -> AppError {
    // spell-check 属于“外部依赖”的能力：失败时用 InternalError 编码。
    // 这样既能复用全局 error 机制（code/report/event），也不会误导为参数问题。
    AppError::InternalError(format!("spell-check unavailable: {e}"))
}

// ======================= 输出格式化 =======================

fn format_text_issues(full_text: &str, issues: &[SpellIssue]) -> Vec<String> {
    let mut lines = Vec::new();
    for it in issues {
        let (line, col) = offset_to_line_col(full_text, it.offset);
        let sug = it.suggestions.get(0).cloned().unwrap_or_else(|| "(no suggestion)".into());
        lines.push(format!(
            "line {}, col {}: \"{}\" -> suggest: {}",
            line, col, it.wrong, sug
        ));
    }
    lines
}

fn format_xml_issues(label: &str, _txt: &str, issues: &[SpellIssue]) -> Vec<String> {
    let mut lines = Vec::new();
    for it in issues {
        let sug = it.suggestions.get(0).cloned().unwrap_or_else(|| "(no suggestion)".into());
        lines.push(format!("element {}: \"{}\" -> suggest: {}", label, it.wrong, sug));
    }
    lines
}

fn offset_to_line_col(text: &str, offset: usize) -> (usize, usize) {
    let mut line = 1usize;
    let mut col = 1usize;
    for (i, ch) in text.char_indices() {
        if i >= offset { break; }
        if ch == '\n' { line += 1; col = 1; } else { col += 1; }
    }
    (line, col)
}

pub const SPELL_CHECK_COMMAND: CommandDef = CommandDef {
    name: "spell-check",
    handler: cmd_spell_check,
};

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_issue(offset: usize, wrong: &str, sug: &str) -> SpellIssue {
        SpellIssue {
            offset,
            wrong: wrong.to_string(),
            _length: 1,
            suggestions: vec![sug.to_string()],
        }
    }

    #[test]
    fn parse_args_empty() {
        let a = SpellCheckArgs::parse(&[]).unwrap();
        assert!(matches!(a.mode, SpellCheckMode::All));
    }

    #[test]
    fn parse_args_all() {
        let a = SpellCheckArgs::parse(&["--all".into()]).unwrap();
        assert!(matches!(a.mode, SpellCheckMode::All));
    }

    #[test]
    fn parse_args_file() {
        let a = SpellCheckArgs::parse(&["work_dir/a.txt".into()]).unwrap();
        match a.mode {
            SpellCheckMode::One(SpellCheckTarget::Path(p)) => {
                assert!(p.to_string_lossy().contains("a.txt"));
            }
            _ => panic!("expected Path"),
        }
    }

    #[test]
    fn offset_to_line_col_basic() {
        let s = "ab\ncde\nXYZ";
        let (line, col) = offset_to_line_col(s, 7);
        assert_eq!((line, col), (3, 1));
    }

    #[test]
    fn format_text_issue_has_location_and_suggestion() {
        let text = "I recieve it\nand it occured\n";
        let issues = vec![mk_issue(2, "recieve", "receive")];
        let out = format_text_issues(text, &issues);

        assert!(!out.is_empty());
        assert!(out[0].contains("\"recieve\""));
        assert!(out[0].contains("->"));
        assert!(out[0].contains("receive"));

        let lower = out[0].to_lowercase();
        let has_cn = out[0].contains("行") && out[0].contains("列");
        let has_en = lower.contains("line") && (lower.contains("col") || lower.contains("column"));
        assert!(has_cn || has_en, "unexpected format: {}", out[0]);
    }

    #[test]
    fn format_xml_issue_has_element_label_and_suggestion() {
        let issues = vec![mk_issue(0, "Itallian", "Italian")];
        let out = format_xml_issues("title1", "Itallian", &issues);

        assert!(!out.is_empty());
        assert!(out[0].contains("title1"));
        assert!(out[0].contains("\"Itallian\""));
        assert!(out[0].contains("->"));
        assert!(out[0].contains("Italian"));
    }
}
