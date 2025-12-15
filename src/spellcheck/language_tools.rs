use serde::Deserialize;
use super::{SpellChecker, SpellError, SpellIssue};

pub struct LanguageToolChecker {
    pub endpoint: String,   // "https://api.languagetool.org/v2/check"
    pub language: String,   // "en-US"
    pub timeout_ms: u64,
}

#[derive(Deserialize)]
struct LTResponse {
    matches: Vec<LTMatch>,
}

#[derive(Deserialize)]
struct LTMatch {
    offset: usize,
    length: usize,
    #[serde(default)]
    replacements: Vec<LTReplacement>,
}

#[derive(Deserialize)]
struct LTReplacement {
    value: String,
}

impl SpellChecker for LanguageToolChecker {
    fn check(&self, text: &str) -> Result<Vec<SpellIssue>, SpellError> {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_millis(self.timeout_ms))
            .build()
            .map_err(|e| SpellError::Network(e.to_string()))?;

        let resp = client
            .post(&self.endpoint)
            .form(&[
                ("text", text),
                ("language", &self.language),
            ])
            .send()
            .map_err(|e| SpellError::Network(e.to_string()))?;

        let body: LTResponse = resp.json().map_err(|e| SpellError::Parse(e.to_string()))?;

        let mut issues = Vec::new();
        for m in body.matches {
            let wrong = text.get(m.offset..m.offset + m.length).unwrap_or("").to_string();
            let suggestions = m.replacements.into_iter().map(|r| r.value).collect();
            issues.push(SpellIssue { offset: m.offset, _length: m.length, wrong, suggestions });
        }
        Ok(issues)
    }
}