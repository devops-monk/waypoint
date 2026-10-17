//! Claude Messages API over raw HTTP (Rust has no official Anthropic SDK).
//! Structured output via `output_config.format`; depth via `output_config.effort`.
//! On current Opus/Sonnet models a server-side refusal fallback is enabled with
//! `fallbacks: "default"`, so a policy decline is retried on another model in the same call.

use serde_json::{Value, json};

use crate::{JsonRequest, JsonResponse, LlmConfig, LlmError, parse_json_text, status_error};

const API_VERSION: &str = "2023-06-01";
const FALLBACK_BETA: &str = "server-side-fallback-2026-07-01";

fn supports_default_fallback(model: &str) -> bool {
    matches!(model, "claude-fable-5-1" | "claude-opus-5-5" | "claude-opus-5" | "claude-sonnet-5-5")
}

pub(crate) fn build_body(cfg: &LlmConfig, req: &JsonRequest<'_>) -> Value {
    let mut body = json!({
        "model": cfg.model,
        "max_tokens": req.max_tokens,
        "system": req.system,
        "messages": [{ "role": "user", "content": req.user }],
        "output_config": {
            // Claude Opus 5.5 defaults to "medium"; set it explicitly either way.
            "effort": cfg.effort.as_deref().unwrap_or("medium"),
            "format": { "type": "json_schema", "schema": req.schema },
        },
    });
    if supports_default_fallback(&cfg.model) {
        body["fallbacks"] = json!("default");
    }
    body
}

pub(crate) async fn complete_json(http: &reqwest::Client, cfg: &LlmConfig, key: &str, req: &JsonRequest<'_>) -> Result<JsonResponse, LlmError> {
    let mut builder = http
        .post(format!("{}/v1/messages", cfg.base_url()))
        .header("x-api-key", key)
        .header("anthropic-version", API_VERSION)
        .header("content-type", "application/json");
    if supports_default_fallback(&cfg.model) {
        builder = builder.header("anthropic-beta", FALLBACK_BETA);
    }
    let resp = builder.json(&build_body(cfg, req)).send().await?;
    let status = resp.status().as_u16();
    let text = resp.text().await?;
    if !(200..300).contains(&status) {
        return Err(status_error(status, &text));
    }
    parse_response(&text, &cfg.model)
}

pub(crate) fn parse_response(text: &str, requested_model: &str) -> Result<JsonResponse, LlmError> {
    let v: Value = serde_json::from_str(text).map_err(|e| LlmError::BadJson(e.to_string()))?;
    match v["stop_reason"].as_str() {
        Some("refusal") => {
            let detail = v.pointer("/stop_details/explanation").and_then(Value::as_str).map(|e| format!(": {e}")).unwrap_or_default();
            return Err(LlmError::Refused(detail));
        }
        Some("max_tokens") => return Err(LlmError::Truncated),
        _ => {}
    }
    // Content may include thinking/fallback blocks; the answer is the text blocks.
    let answer: String = v["content"].as_array().into_iter().flatten().filter(|b| b["type"] == "text").filter_map(|b| b["text"].as_str()).collect();
    Ok(JsonResponse {
        value: parse_json_text(&answer)?,
        model: v["model"].as_str().unwrap_or(requested_model).to_string(),
        input_tokens: v.pointer("/usage/input_tokens").and_then(Value::as_i64).unwrap_or(0),
        output_tokens: v.pointer("/usage/output_tokens").and_then(Value::as_i64).unwrap_or(0),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Provider;

    #[test]
    fn body_has_schema_effort_and_fallback() {
        let cfg = LlmConfig { provider: Provider::Anthropic, model: "claude-opus-5-5".into(), base_url: None, effort: None };
        let schema = json!({"type":"object","properties":{},"required":[],"additionalProperties":false});
        let req = JsonRequest { system: "s", user: "u", schema_name: "x", schema: &schema, max_tokens: 4000 };
        let b = build_body(&cfg, &req);
        assert_eq!(b["output_config"]["format"]["type"], "json_schema");
        assert_eq!(b["output_config"]["effort"], "medium");
        assert_eq!(b["fallbacks"], "default");
        assert!(b.get("thinking").is_none(), "thinking can't be disabled on Opus 5.5; leave it unset");

        let haiku = LlmConfig { model: "claude-haiku-5-5".into(), ..cfg };
        assert!(build_body(&haiku, &req).get("fallbacks").is_none());
    }

    #[test]
    fn parses_text_blocks_and_stop_reasons() {
        let ok = r#"{"model":"claude-opus-5-5","stop_reason":"end_turn","content":[{"type":"thinking","thinking":""},{"type":"text","text":"{\"a\":1}"}],"usage":{"input_tokens":12,"output_tokens":3}}"#;
        let r = parse_response(ok, "x").unwrap();
        assert_eq!((r.value["a"].as_i64(), r.input_tokens, r.output_tokens), (Some(1), 12, 3));

        let refused = r#"{"stop_reason":"refusal","stop_details":{"explanation":"policy"},"content":[]}"#;
        assert!(matches!(parse_response(refused, "x"), Err(LlmError::Refused(m)) if m == ": policy"));
        assert!(matches!(parse_response(r#"{"stop_reason":"max_tokens","content":[]}"#, "x"), Err(LlmError::Truncated)));
    }
}
