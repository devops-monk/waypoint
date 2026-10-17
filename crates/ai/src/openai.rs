//! OpenAI-compatible chat completions: OpenAI, OpenRouter and Ollama all speak this shape.

use serde_json::{Value, json};

use crate::{JsonRequest, JsonResponse, LlmConfig, LlmError, Provider, parse_json_text, status_error};

pub(crate) async fn complete_json(http: &reqwest::Client, cfg: &LlmConfig, key: Option<&str>, req: &JsonRequest<'_>) -> Result<JsonResponse, LlmError> {
    let body = json!({
        "model": cfg.model,
        "max_tokens": req.max_tokens,
        "messages": [
            { "role": "system", "content": req.system },
            { "role": "user", "content": req.user },
        ],
        "response_format": {
            "type": "json_schema",
            "json_schema": { "name": req.schema_name, "strict": true, "schema": req.schema },
        },
    });
    let mut builder = http.post(format!("{}/chat/completions", cfg.base_url())).json(&body);
    if let Some(key) = key {
        builder = builder.bearer_auth(key);
    }
    if cfg.provider == Provider::OpenRouter {
        builder = builder.header("X-Title", "Waypoint");
    }
    let resp = builder.send().await?;
    let status = resp.status().as_u16();
    let text = resp.text().await?;
    if !(200..300).contains(&status) {
        return Err(status_error(status, &text));
    }
    parse_response(&text, &cfg.model)
}

pub(crate) fn parse_response(text: &str, requested_model: &str) -> Result<JsonResponse, LlmError> {
    let v: Value = serde_json::from_str(text).map_err(|e| LlmError::BadJson(e.to_string()))?;
    let choice = &v["choices"][0];
    match choice["finish_reason"].as_str() {
        Some("length") => return Err(LlmError::Truncated),
        Some("content_filter") => return Err(LlmError::Refused(String::new())),
        _ => {}
    }
    if let Some(refusal) = choice.pointer("/message/refusal").and_then(Value::as_str) {
        return Err(LlmError::Refused(format!(": {refusal}")));
    }
    let content = choice.pointer("/message/content").and_then(Value::as_str).unwrap_or_default();
    Ok(JsonResponse {
        value: parse_json_text(content)?,
        model: v["model"].as_str().unwrap_or(requested_model).to_string(),
        input_tokens: v.pointer("/usage/prompt_tokens").and_then(Value::as_i64).unwrap_or(0),
        output_tokens: v.pointer("/usage/completion_tokens").and_then(Value::as_i64).unwrap_or(0),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_choice_and_usage() {
        let ok = r#"{"model":"llama3.3","choices":[{"finish_reason":"stop","message":{"content":"{\"ok\":true}"}}],"usage":{"prompt_tokens":5,"completion_tokens":2}}"#;
        let r = parse_response(ok, "x").unwrap();
        assert_eq!((r.value["ok"].as_bool(), r.model.as_str(), r.input_tokens), (Some(true), "llama3.3", 5));
        let cut = r#"{"choices":[{"finish_reason":"length","message":{"content":"{"}}]}"#;
        assert!(matches!(parse_response(cut, "x"), Err(LlmError::Truncated)));
    }
}
