//! Bring-your-own-key LLM layer. Every call asks for JSON that matches a schema, so the
//! app never parses free-form prose, and every call reports token usage for the cost panel.

pub mod cover;
pub mod followup;
mod anthropic;
mod openai;
pub mod prompts;

use std::time::Duration;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Anthropic,
    OpenAI,
    OpenRouter,
    /// Local models through Ollama's OpenAI-compatible endpoint. Free and private; no key.
    Ollama,
}

impl Provider {
    pub fn needs_key(self) -> bool {
        !matches!(self, Provider::Ollama)
    }

    pub fn default_model(self) -> &'static str {
        match self {
            Provider::Anthropic => "claude-opus-5-5",
            Provider::OpenAI => "gpt-5",
            Provider::OpenRouter => "anthropic/claude-opus-5-5",
            Provider::Ollama => "llama3.3",
        }
    }

    fn default_base_url(self) -> &'static str {
        match self {
            Provider::Anthropic => "https://api.anthropic.com",
            Provider::OpenAI => "https://api.openai.com/v1",
            Provider::OpenRouter => "https://openrouter.ai/api/v1",
            Provider::Ollama => "http://localhost:11434/v1",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    pub provider: Provider,
    pub model: String,
    /// Override for self-hosted or proxy endpoints.
    #[serde(default)]
    pub base_url: Option<String>,
    /// Anthropic effort level: low | medium | high | xhigh | max.
    #[serde(default)]
    pub effort: Option<String>,
}

impl LlmConfig {
    pub fn base_url(&self) -> &str {
        self.base_url.as_deref().filter(|s| !s.is_empty()).unwrap_or(self.provider.default_base_url()).trim_end_matches('/')
    }
}

/// One structured-output request.
pub struct JsonRequest<'a> {
    pub system: &'a str,
    pub user: &'a str,
    /// Short identifier for the schema (required by OpenAI-style APIs).
    pub schema_name: &'a str,
    /// JSON Schema: every object needs `additionalProperties: false` and full `required`.
    pub schema: &'a serde_json::Value,
    pub max_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonResponse {
    pub value: serde_json::Value,
    /// The model that actually served the request (may differ after a server-side fallback).
    pub model: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
}

#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("an API key is required for {0:?}")]
    MissingKey(Provider),
    #[error("the API key was rejected ({0})")]
    Auth(String),
    #[error("rate limited by the provider; try again shortly")]
    RateLimited,
    #[error("the model declined this request{0}")]
    Refused(String),
    #[error("the response was cut off before it finished (raise max tokens)")]
    Truncated,
    #[error("the model returned invalid JSON: {0}")]
    BadJson(String),
    #[error("provider error {0}: {1}")]
    Api(u16, String),
    #[error("could not reach the provider: {0}")]
    Network(#[from] reqwest::Error),
}

pub struct Llm {
    http: reqwest::Client,
}

impl Default for Llm {
    fn default() -> Self {
        Self::new()
    }
}

impl Llm {
    pub fn new() -> Self {
        let http = reqwest::Client::builder().timeout(Duration::from_secs(600)).build().expect("reqwest client");
        Llm { http }
    }

    pub async fn complete_json(&self, cfg: &LlmConfig, api_key: Option<&str>, req: &JsonRequest<'_>) -> Result<JsonResponse, LlmError> {
        let key = api_key.filter(|k| !k.trim().is_empty());
        if cfg.provider.needs_key() && key.is_none() {
            return Err(LlmError::MissingKey(cfg.provider));
        }
        match cfg.provider {
            Provider::Anthropic => anthropic::complete_json(&self.http, cfg, key.unwrap_or_default(), req).await,
            _ => openai::complete_json(&self.http, cfg, key, req).await,
        }
    }
}

/// Map a non-success HTTP status to a typed error.
pub(crate) fn status_error(status: u16, body: &str) -> LlmError {
    let message = serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|v| v.pointer("/error/message").and_then(|m| m.as_str()).map(str::to_string))
        .unwrap_or_else(|| body.chars().take(300).collect());
    match status {
        401 | 403 => LlmError::Auth(message),
        429 => LlmError::RateLimited,
        _ => LlmError::Api(status, message),
    }
}

/// Parse model text as JSON, tolerating a stray code fence from providers without
/// strict schema enforcement.
pub(crate) fn parse_json_text(text: &str) -> Result<serde_json::Value, LlmError> {
    let t = text.trim();
    let t = t.strip_prefix("```json").or_else(|| t.strip_prefix("```")).map(|s| s.trim_end_matches("```")).unwrap_or(t).trim();
    serde_json::from_str(t).map_err(|e| LlmError::BadJson(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_fenced_json_and_maps_status() {
        assert_eq!(parse_json_text("```json\n{\"a\":1}\n```").unwrap()["a"], 1);
        assert!(matches!(status_error(401, r#"{"error":{"message":"invalid x-api-key"}}"#), LlmError::Auth(m) if m == "invalid x-api-key"));
        assert!(matches!(status_error(429, ""), LlmError::RateLimited));
    }

    #[tokio::test]
    async fn missing_key_is_reported_before_any_request() {
        let cfg = LlmConfig { provider: Provider::Anthropic, model: "claude-opus-5-5".into(), base_url: None, effort: None };
        let schema = serde_json::json!({});
        let req = JsonRequest { system: "", user: "", schema_name: "x", schema: &schema, max_tokens: 10 };
        assert!(matches!(Llm::new().complete_json(&cfg, None, &req).await, Err(LlmError::MissingKey(Provider::Anthropic))));
    }
}
