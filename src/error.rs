//! 定义统一错误类型与结果封装。

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Unknown command: {0}")]
    UnknownCommand(String),

    #[error("Invalid arguments: {0}")]
    InvalidArgs(String),

    #[error("Invalid command: {0}")]
    InvalidCommand(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Internal error: {0}")]
    InternalError(String),
}

pub type AppResult<T> = Result<T, AppError>;

impl AppError {
    pub fn code(&self) -> u32 {
        match self {
            AppError::UnknownCommand(_) => 1001,
            AppError::InvalidArgs(_)    => 1002,
            AppError::InvalidCommand(_) => 1003,
            AppError::Io(_)             => 2001,
            AppError::Json(_)           => 2002,
            AppError::InternalError(_)  => 3001,
        }
    }

    pub fn report(&self) {
        eprintln!("[error] {}", self);
    }
}