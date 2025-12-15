use crate::{
    application::Application,
    outcome::Outcome,
    error::{AppResult, AppError},
};
use super::CommandDef;

pub fn cmd_show(app: &mut Application, args: &[String]) -> AppResult<Outcome> {
    let (start, end) = match args.get(0).map(|s| s.as_str()) {
        None => (None, None),
        Some(spec) => parse_range_spec(spec)?,
    };

    // ✅ FileTarget 已移除：show 直接展示当前 active file
    let content = app.workspace.show(start, end)?;
    Ok(Outcome::print(content))
}

pub const SHOW_COMMAND: CommandDef = CommandDef {
    name: "show",
    handler: cmd_show,
};

fn parse_range_spec(spec: &str) -> AppResult<(Option<usize>, Option<usize>)> {
    let s = spec.trim();

    if !s.contains(':') {
        let n = parse_pos_usize(s)?;
        return Ok((Some(n), None));
    }

    let (a, b) = s.split_once(':').unwrap();
    let start = if a.trim().is_empty() { None } else { Some(parse_pos_usize(a.trim())?) };
    let end   = if b.trim().is_empty() { None } else { Some(parse_pos_usize(b.trim())?) };
    Ok((start, end))
}

fn parse_pos_usize(s: &str) -> AppResult<usize> {
    let v: usize = s
        .parse()
        .map_err(|_| AppError::InvalidArgs(format!("invalid number: {s}")))?;

    if v == 0 {
        return Err(AppError::InvalidArgs("line numbers are 1-based (>=1)".into()));
    }
    Ok(v)
}
