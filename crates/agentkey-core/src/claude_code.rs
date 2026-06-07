use std::path::PathBuf;
use std::process::Command;

use anyhow::Context;
use serde::Serialize;

use crate::settings::{BackendSettings, ClaudeCodeAuthMode};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeCodeLaunchResult {
    pub pid: u32,
    pub command: String,
    pub working_directory: String,
    pub auth_env: String,
    pub base_url_configured: bool,
    pub model_configured: bool,
    pub small_fast_model_configured: bool,
    pub nonessential_traffic_disabled: bool,
}

pub fn launch_claude_code(settings: &BackendSettings) -> anyhow::Result<ClaudeCodeLaunchResult> {
    let command_line = validated_command(&settings.claude_code_command)?;
    let api_key = validated_env_value("Claude Code API Key", &settings.claude_code_api_key)?;
    let base_url = optional_url("Claude Code Base URL", &settings.claude_code_base_url)?;
    let model = optional_env_value("ANTHROPIC_MODEL", &settings.claude_code_model)?;
    let small_fast_model = optional_env_value(
        "ANTHROPIC_SMALL_FAST_MODEL",
        &settings.claude_code_small_fast_model,
    )?;
    let working_directory = optional_working_directory(&settings.claude_code_working_directory)?;
    let extra_env = parse_extra_env(&settings.claude_code_extra_env)?;
    let auth_env = match settings.claude_code_auth_mode {
        ClaudeCodeAuthMode::ApiKey => "ANTHROPIC_API_KEY",
        ClaudeCodeAuthMode::AuthToken => "ANTHROPIC_AUTH_TOKEN",
    };

    let mut command = platform_command(&command_line);
    if let Some(path) = &working_directory {
        command.current_dir(path);
    }
    command.env(auth_env, &api_key);
    if let Some(value) = &base_url {
        command.env("ANTHROPIC_BASE_URL", value);
    }
    if let Some(value) = &model {
        command.env("ANTHROPIC_MODEL", value);
    }
    if let Some(value) = &small_fast_model {
        command.env("ANTHROPIC_SMALL_FAST_MODEL", value);
    }
    if settings.claude_code_disable_nonessential_traffic {
        command.env("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC", "1");
    }
    for (key, value) in extra_env {
        command.env(key, value);
    }

    let child = command
        .spawn()
        .with_context(|| format!("无法启动 Claude Code 命令：{command_line}"))?;

    Ok(ClaudeCodeLaunchResult {
        pid: child.id(),
        command: command_line,
        working_directory: working_directory
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_default(),
        auth_env: auth_env.to_string(),
        base_url_configured: base_url.is_some(),
        model_configured: model.is_some(),
        small_fast_model_configured: small_fast_model.is_some(),
        nonessential_traffic_disabled: settings.claude_code_disable_nonessential_traffic,
    })
}

pub fn validate_claude_code_extra_env(contents: &str) -> anyhow::Result<()> {
    parse_extra_env(contents).map(|_| ())
}

fn platform_command(command_line: &str) -> Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;

        const CREATE_NEW_CONSOLE: u32 = 0x00000010;
        let mut command = Command::new("cmd.exe");
        command.arg("/K").arg(command_line);
        command.creation_flags(CREATE_NEW_CONSOLE);
        command
    }

    #[cfg(not(windows))]
    {
        let mut command = Command::new("sh");
        command.arg("-lc").arg(command_line);
        command
    }
}

fn validated_command(value: &str) -> anyhow::Result<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        anyhow::bail!("Claude Code 命令不能为空");
    }
    validate_process_value("Claude Code 命令", trimmed)?;
    Ok(trimmed.to_string())
}

fn optional_working_directory(value: &str) -> anyhow::Result<Option<PathBuf>> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    validate_process_value("Claude Code 工作目录", trimmed)?;
    let path = PathBuf::from(trimmed);
    if !path.exists() {
        anyhow::bail!("Claude Code 工作目录不存在：{trimmed}");
    }
    if !path.is_dir() {
        anyhow::bail!("Claude Code 工作目录不是目录：{trimmed}");
    }
    Ok(Some(path))
}

fn optional_url(label: &str, value: &str) -> anyhow::Result<Option<String>> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    validate_process_value(label, trimmed)?;
    crate::url_policy::validate_optional_api_base_url(label, trimmed)
}

fn optional_env_value(name: &str, value: &str) -> anyhow::Result<Option<String>> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    validate_process_value(name, trimmed)?;
    Ok(Some(trimmed.to_string()))
}

fn validated_env_value(label: &str, value: &str) -> anyhow::Result<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        anyhow::bail!("{label} 不能为空");
    }
    validate_process_value(label, trimmed)?;
    Ok(trimmed.to_string())
}

fn validate_process_value(label: &str, value: &str) -> anyhow::Result<()> {
    if value.contains('\0') || value.contains('\n') || value.contains('\r') {
        anyhow::bail!("{label} 不能包含换行或 NUL 字符");
    }
    Ok(())
}

fn parse_extra_env(contents: &str) -> anyhow::Result<Vec<(String, String)>> {
    let mut vars = Vec::new();
    for (index, line) in contents.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            anyhow::bail!("Claude Code 额外环境变量第 {} 行缺少 =", index + 1);
        };
        let key = key.trim();
        if !is_valid_env_key(key) {
            anyhow::bail!("Claude Code 额外环境变量名无效：{key}");
        }
        if is_managed_env_key(key) {
            anyhow::bail!(
                "Claude Code 额外环境变量不能覆盖 AgentKey 托管变量：{key}"
            );
        }
        validate_process_value(key, value)?;
        vars.push((key.to_string(), value.trim().to_string()));
    }
    Ok(vars)
}

fn is_managed_env_key(value: &str) -> bool {
    matches!(
        value,
        "ANTHROPIC_API_KEY"
            | "ANTHROPIC_AUTH_TOKEN"
            | "ANTHROPIC_BASE_URL"
            | "ANTHROPIC_MODEL"
            | "ANTHROPIC_SMALL_FAST_MODEL"
            | "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC"
    )
}

fn is_valid_env_key(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '_' || first.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extra_env_rejects_bad_lines() {
        let error = parse_extra_env("GOOD=value\nbad-name=value").unwrap_err();
        assert!(error.to_string().contains("bad-name"));
    }

    #[test]
    fn extra_env_rejects_managed_keys() {
        let error = parse_extra_env("ANTHROPIC_BASE_URL=http://example.test/v1").unwrap_err();
        assert!(error.to_string().contains("ANTHROPIC_BASE_URL"));
    }

    #[test]
    fn optional_url_requires_http_scheme() {
        let error = optional_url("Base URL", "ftp://example.test").unwrap_err();
        assert!(error.to_string().contains("HTTPS"));
    }

    #[test]
    fn optional_url_allows_local_http() {
        assert_eq!(
            optional_url("Base URL", "http://127.0.0.1:4000/v1/").unwrap(),
            Some("http://127.0.0.1:4000/v1".to_string())
        );
    }

    #[test]
    fn optional_url_rejects_remote_http() {
        let error = optional_url("Base URL", "http://gateway.example.test/v1").unwrap_err();
        assert!(error.to_string().contains("HTTP 仅允许"));
    }
}
