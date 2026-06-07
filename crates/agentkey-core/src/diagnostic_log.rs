use std::io::{Error, Write};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use serde_json::{Value, json};

static TEST_LOG_PATH: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();

#[derive(Debug, Clone, Serialize)]
struct DiagnosticRecord {
    timestamp_ms: u64,
    pid: u32,
    event: String,
    detail: Value,
}

pub fn append_diagnostic_log(event: &str, detail: impl Serialize) -> std::io::Result<()> {
    let path = diagnostic_log_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let detail = redact_diagnostic_value(serde_json::to_value(detail).unwrap_or_else(|error| {
        json!({
            "serialization_error": error.to_string()
        })
    }));
    let record = DiagnosticRecord {
        timestamp_ms: now_ms(),
        pid: std::process::id(),
        event: event.to_string(),
        detail,
    };
    let line = serde_json::to_string(&record).unwrap_or_else(|error| {
        json!({
            "timestamp_ms": now_ms(),
            "pid": std::process::id(),
            "event": "diagnostic_log.serialization_failed",
            "detail": {
                "message": error.to_string()
            }
        })
        .to_string()
    });

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    crate::harden_sensitive_file(&path).map_err(|error| Error::other(error.to_string()))?;
    writeln!(file, "{line}")?;
    Ok(())
}

pub fn diagnostic_log_path() -> PathBuf {
    if let Some(lock) = TEST_LOG_PATH.get() {
        if let Ok(guard) = lock.lock() {
            if let Some(path) = &*guard {
                return path.clone();
            }
        }
    }
    crate::paths::default_diagnostic_log_path()
}

#[doc(hidden)]
pub fn set_diagnostic_log_path_for_tests(path: Option<PathBuf>) {
    let lock = TEST_LOG_PATH.get_or_init(|| Mutex::new(None));
    *lock.lock().expect("test log path lock poisoned") = path;
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

pub fn redact_diagnostic_value(value: Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.into_iter()
                .map(|(key, value)| {
                    if is_sensitive_key(&key) {
                        (key, Value::String("[REDACTED]".to_string()))
                    } else {
                        (key, redact_diagnostic_value(value))
                    }
                })
                .collect(),
        ),
        Value::Array(items) => {
            Value::Array(items.into_iter().map(redact_diagnostic_value).collect())
        }
        Value::String(value) => Value::String(redact_diagnostic_string(&value)),
        other => other,
    }
}

fn is_sensitive_key(key: &str) -> bool {
    let normalized = key
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect::<String>();
    normalized.contains("apikey")
        || normalized.contains("bearertoken")
        || normalized.contains("authtoken")
        || normalized.contains("accesstoken")
        || normalized.contains("refreshtoken")
        || normalized.contains("authorization")
        || normalized.contains("authcontents")
        || normalized.contains("configcontents")
        || normalized.contains("extraenv")
        || normalized.contains("token")
        || normalized == "password"
        || normalized == "secret"
}

fn redact_diagnostic_string(value: &str) -> String {
    let mut redacted = redact_after_markers(value);
    redacted = redact_url_query_params(&redacted);
    redacted = redact_key_value_markers(&redacted);
    for prefix in [
        "sk-",
        "sk_",
        "gsk_",
        "xai-",
        "hf_",
        "AIza",
        "ya29.",
        "gho_",
        "ghp_",
        "github_pat_",
        "xoxb-",
        "xoxp-",
        "AKIA",
    ] {
        redacted = redact_prefixed_token(&redacted, prefix);
    }
    redacted
}

fn redact_after_markers(value: &str) -> String {
    let mut output = value.to_string();
    for marker in ["Bearer ", "bearer ", "Authorization: ", "authorization: "] {
        output = redact_marker_value(&output, marker);
    }
    output
}

fn redact_marker_value(value: &str, marker: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut remaining = value;
    while let Some(index) = remaining.find(marker) {
        let (before, after_before) = remaining.split_at(index);
        output.push_str(before);
        output.push_str(marker);
        let token_start = marker.len();
        let after_marker = &after_before[token_start..];
        if let Some(quote) = after_marker
            .chars()
            .next()
            .filter(|ch| matches!(ch, '"' | '\''))
        {
            output.push(quote);
            output.push_str("[REDACTED]");
            let after_quote = &after_marker[quote.len_utf8()..];
            let token_end = after_quote.find(quote).unwrap_or(after_quote.len());
            remaining = &after_quote[token_end..];
            continue;
        }
        output.push_str("[REDACTED]");
        let token_end = after_marker
            .find(|ch: char| {
                ch.is_whitespace() || matches!(ch, '"' | '\'' | ',' | ';' | '}' | '&' | '#' | '?')
            })
            .unwrap_or(after_marker.len());
        remaining = &after_marker[token_end..];
    }
    output.push_str(remaining);
    output
}

fn redact_url_query_params(value: &str) -> String {
    let mut output = value.to_string();
    for marker in [
        "?api_key=",
        "&api_key=",
        "?apiKey=",
        "&apiKey=",
        "?key=",
        "&key=",
        "?token=",
        "&token=",
        "?access_token=",
        "&access_token=",
        "?refresh_token=",
        "&refresh_token=",
        "?id_token=",
        "&id_token=",
        "?authToken=",
        "&authToken=",
        "?code=",
        "&code=",
        "?session=",
        "&session=",
        "?secret=",
        "&secret=",
        "#token=",
        "#access_token=",
    ] {
        output = redact_url_param_value(&output, marker);
    }
    output
}

fn redact_url_param_value(value: &str, marker: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut remaining = value;
    while let Some(index) = remaining.find(marker) {
        let (before, after_before) = remaining.split_at(index);
        output.push_str(before);
        output.push_str(marker);
        output.push_str("[REDACTED]");
        let after_marker = &after_before[marker.len()..];
        let token_end = after_marker
            .find(|ch: char| {
                matches!(
                    ch,
                    '&' | '#' | '?' | '"' | '\'' | '<' | '>' | ')' | ' ' | '\n' | '\r' | '\t'
                )
            })
            .unwrap_or(after_marker.len());
        remaining = &after_marker[token_end..];
    }
    output.push_str(remaining);
    output
}

fn redact_key_value_markers(value: &str) -> String {
    let mut output = value.to_string();
    for marker in [
        "api_key=",
        "api_key = ",
        "apiKey=",
        "apiKey = ",
        "apiKey:",
        "apiKey\": \"",
        "apiKey\":\"",
        "OPENAI_API_KEY=",
        "OPENAI_API_KEY = ",
        "OPENAI_API_KEY\": \"",
        "OPENAI_API_KEY\":\"",
        "ANTHROPIC_API_KEY=",
        "ANTHROPIC_API_KEY = ",
        "ANTHROPIC_API_KEY\": \"",
        "ANTHROPIC_API_KEY\":\"",
        "ANTHROPIC_AUTH_TOKEN=",
        "ANTHROPIC_AUTH_TOKEN = ",
        "ANTHROPIC_AUTH_TOKEN\": \"",
        "ANTHROPIC_AUTH_TOKEN\":\"",
        "experimental_bearer_token=",
        "experimental_bearer_token = ",
        "experimental_bearer_token = \"",
        "authorization=",
        "authorization = ",
        "Authorization=",
        "Authorization = ",
        "password=",
        "password = ",
        "secret=",
        "secret = ",
    ] {
        output = redact_marker_value(&output, marker);
    }
    output
}

fn redact_prefixed_token(value: &str, prefix: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut remaining = value;
    while let Some(index) = remaining.find(prefix) {
        let (before, after_before) = remaining.split_at(index);
        output.push_str(before);
        output.push_str(prefix);
        output.push_str("[REDACTED]");
        let after_prefix = &after_before[prefix.len()..];
        let token_end = after_prefix
            .find(|ch: char| !(ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | ':')))
            .unwrap_or(after_prefix.len());
        remaining = &after_prefix[token_end..];
    }
    output.push_str(remaining);
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn redacts_sensitive_object_keys_recursively() {
        let redacted = redact_diagnostic_value(json!({
            "apiKey": "sk-real",
            "nested": {
                "Authorization": "Bearer secret"
            },
            "safe": "visible"
        }));

        assert_eq!(redacted["apiKey"], "[REDACTED]");
        assert_eq!(redacted["nested"]["Authorization"], "[REDACTED]");
        assert_eq!(redacted["safe"], "visible");
    }

    #[test]
    fn redacts_tokens_embedded_in_strings() {
        let redacted = redact_diagnostic_value(json!({
            "message": "Authorization: Bearer abc123 sk-live-token gho_secret"
        }));
        let text = redacted["message"].as_str().unwrap();

        assert!(text.contains("[REDACTED]"));
        assert!(!text.contains("abc123"));
        assert!(!text.contains("live-token"));
        assert!(!text.contains("gho_secret"));
    }

    #[test]
    fn redacts_common_third_party_api_key_prefixes() {
        let redacted = redact_diagnostic_value(json!({
            "message": "gemini=AIzaSyExample groq=gsk_live_secret xai=xai-test-secret hf=hf_test_secret oauth=ya29.secret-token"
        }));
        let text = redacted["message"].as_str().unwrap();

        assert!(text.matches("[REDACTED]").count() >= 5);
        assert!(!text.contains("SyExample"));
        assert!(!text.contains("live_secret"));
        assert!(!text.contains("test-secret"));
        assert!(!text.contains("test_secret"));
        assert!(!text.contains("secret-token"));
    }

    #[test]
    fn redacts_relay_file_contents_fields() {
        let redacted = redact_diagnostic_value(json!({
            "configContents": "experimental_bearer_token = \"plain-secret\"",
            "authContents": "{\"OPENAI_API_KEY\":\"plain-secret\"}",
            "safe": "visible"
        }));

        assert_eq!(redacted["configContents"], "[REDACTED]");
        assert_eq!(redacted["authContents"], "[REDACTED]");
        assert_eq!(redacted["safe"], "visible");
    }

    #[test]
    fn redacts_extra_env_fields() {
        let redacted = redact_diagnostic_value(json!({
            "claudeCodeExtraEnv": "CUSTOM_SECRET=plain-secret",
            "nested": {
                "extraEnv": "VENDOR_KEY=plain-secret"
            },
            "safe": "visible"
        }));

        assert_eq!(redacted["claudeCodeExtraEnv"], "[REDACTED]");
        assert_eq!(redacted["nested"]["extraEnv"], "[REDACTED]");
        assert_eq!(redacted["safe"], "visible");
    }

    #[test]
    fn redacts_plain_key_value_secrets_in_strings() {
        let redacted = redact_diagnostic_value(json!({
            "message": "api_key=plain-secret experimental_bearer_token = \"toml-secret\" {\"OPENAI_API_KEY\":\"json-secret\"} ANTHROPIC_AUTH_TOKEN = claude-secret"
        }));
        let text = redacted["message"].as_str().unwrap();

        assert!(text.matches("[REDACTED]").count() >= 4);
        assert!(!text.contains("plain-secret"));
        assert!(!text.contains("toml-secret"));
        assert!(!text.contains("json-secret"));
        assert!(!text.contains("claude-secret"));
    }

    #[test]
    fn redacts_secret_url_query_params() {
        let redacted = redact_diagnostic_value(json!({
            "location": "https://client.example/callback?code=oauth-code&state=visible#access_token=access-secret",
            "message": "next=http://localhost/?apiKey=query-secret&model=gpt"
        }));

        let location = redacted["location"].as_str().unwrap();
        let message = redacted["message"].as_str().unwrap();
        assert!(location.contains("code=[REDACTED]"));
        assert!(location.contains("#access_token=[REDACTED]"));
        assert!(location.contains("state=visible"));
        assert!(message.contains("apiKey=[REDACTED]"));
        assert!(message.contains("model=gpt"));
        assert!(!location.contains("oauth-code"));
        assert!(!location.contains("access-secret"));
        assert!(!message.contains("query-secret"));
    }

    #[test]
    fn append_diagnostic_log_writes_redacted_hardened_file() {
        let temp = tempfile::tempdir().unwrap();
        let log_path = temp.path().join("agentkey.log");
        set_diagnostic_log_path_for_tests(Some(log_path.clone()));

        append_diagnostic_log(
            "test.event",
            json!({
                "apiKey": "sk-real-secret",
                "safe": "visible"
            }),
        )
        .unwrap();

        let contents = std::fs::read_to_string(&log_path).unwrap();
        assert!(contents.contains("test.event"));
        assert!(contents.contains("visible"));
        assert!(contents.contains("[REDACTED]"));
        assert!(!contents.contains("sk-real-secret"));

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let file_mode = std::fs::metadata(&log_path).unwrap().permissions().mode() & 0o777;
            let dir_mode = std::fs::metadata(temp.path()).unwrap().permissions().mode() & 0o777;
            assert_eq!(file_mode, 0o600);
            assert_eq!(dir_mode, 0o700);
        }

        set_diagnostic_log_path_for_tests(None);
    }
}
