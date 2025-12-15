pub mod language_tools;

use std::fmt;

#[derive(Debug, Clone)]
pub struct SpellIssue {
    /// 在输入文本中的字节 offset。
    pub offset: usize,
    /// 错词长度（字节）。
    pub _length: usize,
    /// 错词原文。
    pub wrong: String,
    /// 建议修正（按优先级排序，命令层通常取第一个）。
    pub suggestions: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum SpellError {
    /// 外部服务/网络错误。
    Network(String),
    /// 解析第三方返回值失败。
    Parse(String),
    /// 不支持的语言/格式。
    _Unsupported(String),
}

impl fmt::Display for SpellError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SpellError::Network(s) => write!(f, "network: {s}"),
            SpellError::Parse(s) => write!(f, "parse: {s}"),
            SpellError::_Unsupported(s) => write!(f, "unsupported: {s}"),
        }
    }
}

/// 拼写检查器接口。
///
/// - 命令层只依赖该接口，便于替换第三方实现与 Mock。
/// - 实现方负责将第三方库输出适配为 `SpellIssue`。
pub trait SpellChecker: Send + Sync {
    fn check(&self, text: &str) -> Result<Vec<SpellIssue>, SpellError>;
}