use crate::error::{AppError, AppResult};

pub fn parse_pos(s: &str) -> AppResult<(usize, usize)> {
    let mut parts = s.split(':');
    let line = parts
        .next()
        .ok_or_else(|| AppError::InvalidArgs("missing line in <line:col>".into()))?
        .parse::<usize>()
        .map_err(|_| AppError::InvalidArgs("invalid line number".into()))?;

    let col = parts
        .next()
        .ok_or_else(|| AppError::InvalidArgs("missing col in <line:col>".into()))?
        .parse::<usize>()
        .map_err(|_| AppError::InvalidArgs("invalid column number".into()))?;

    if parts.next().is_some() {
        return Err(AppError::InvalidArgs("too many ':' in <line:col>".into()));
    }

    Ok((line, col))
}
