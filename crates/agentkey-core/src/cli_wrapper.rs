use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::Context;

use crate::settings::BackendSettings;

pub const WRAPPER_EXE: &str = "agentkey-cli-wrapper.exe";
pub const WRAPPER_SOURCE: &str = "agentkey-cli-wrapper.cs";
pub const WRAPPER_CONFIG: &str = "agentkey-cli-wrapper.env";
const LEGACY_WRAPPER_EXE: &str = "codex-wrapper.exe";
const LEGACY_WRAPPER_SOURCE: &str = "codex-wrapper.cs";
const CLI_HOME_DIR: &str = ".agentkey-cli";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WrapperInstall {
    pub wrapper_path: PathBuf,
    pub source_path: PathBuf,
    pub config_path: PathBuf,
    pub real_codex: PathBuf,
    pub codex_home: PathBuf,
}

pub fn ensure_cli_wrapper(settings: &BackendSettings) -> anyhow::Result<Option<WrapperInstall>> {
    let wrapper_dir = wrapper_dir();
    if !should_refresh_cli_wrapper(settings, &wrapper_dir) {
        return Ok(None);
    }
    let real_codex = resolve_real_codex_for_settings(settings)
        .ok_or_else(|| anyhow::anyhow!("未找到桌面 CLI 运行时，可先启动一次桌面客户端或重新安装"))?;
    let codex_home = cli_home_dir();
    let wrapper_settings = wrapper_settings_for_refresh(settings, &wrapper_dir);
    install_cli_wrapper_to(&wrapper_dir, &real_codex, &codex_home, &wrapper_settings).map(Some)
}

pub fn should_refresh_cli_wrapper(settings: &BackendSettings, wrapper_dir: &Path) -> bool {
    settings.cli_wrapper_enabled
        || wrapper_dir.join(WRAPPER_EXE).is_file()
        || wrapper_dir.join(LEGACY_WRAPPER_EXE).is_file()
}

pub fn wrapper_settings_for_refresh(
    settings: &BackendSettings,
    wrapper_dir: &Path,
) -> BackendSettings {
    if settings.cli_wrapper_enabled {
        return settings.clone();
    }

    read_wrapper_config_for_refresh(wrapper_dir)
        .ok()
        .and_then(|source| parse_wrapper_config_settings(&source))
        .or_else(|| {
            read_wrapper_source_for_refresh(wrapper_dir)
                .ok()
                .and_then(|source| parse_wrapper_source_settings(&source))
        })
        .unwrap_or_else(|| settings.clone())
}

fn read_wrapper_config_for_refresh(wrapper_dir: &Path) -> anyhow::Result<String> {
    std::fs::read_to_string(wrapper_dir.join(WRAPPER_CONFIG))
        .with_context(|| format!("failed to read wrapper config in {}", wrapper_dir.display()))
}

fn read_wrapper_source_for_refresh(wrapper_dir: &Path) -> anyhow::Result<String> {
    std::fs::read_to_string(wrapper_dir.join(WRAPPER_SOURCE))
        .or_else(|_| std::fs::read_to_string(wrapper_dir.join(LEGACY_WRAPPER_SOURCE)))
        .with_context(|| format!("failed to read wrapper source in {}", wrapper_dir.display()))
}

pub fn parse_wrapper_config_settings(source: &str) -> Option<BackendSettings> {
    let mut settings = BackendSettings::default();
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };
        match key.trim() {
            "apiKeyEnv" => settings.cli_wrapper_api_key_env = value.trim().to_string(),
            "baseUrl" => settings.cli_wrapper_base_url = value.trim().to_string(),
            "apiKey" => settings.cli_wrapper_api_key = value.trim().to_string(),
            _ => {}
        }
    }
    if settings.cli_wrapper_api_key_env.trim().is_empty() {
        settings.cli_wrapper_api_key_env = crate::settings::default_api_key_env();
    }
    if settings.cli_wrapper_base_url.is_empty() && settings.cli_wrapper_api_key.is_empty() {
        None
    } else {
        Some(settings)
    }
}

pub fn parse_wrapper_source_settings(source: &str) -> Option<BackendSettings> {
    let mut settings = BackendSettings::default();
    settings.cli_wrapper_api_key_env =
        csharp_string_assignment(source, "apiKeyEnv").unwrap_or(settings.cli_wrapper_api_key_env);
    settings.cli_wrapper_base_url =
        csharp_environment_assignment(source, r#"["OPENAI_BASE_URL"]"#).unwrap_or_default();
    settings.cli_wrapper_api_key = csharp_environment_assignment(source, "[apiKeyEnv]")
        .or_else(|| csharp_string_assignment(source, "apiKey"))
        .unwrap_or_default();

    if settings.cli_wrapper_base_url.is_empty() && settings.cli_wrapper_api_key.is_empty() {
        None
    } else {
        Some(settings)
    }
}

fn validate_wrapper_config_value(label: &str, value: &str) -> anyhow::Result<()> {
    if value.contains('\0') || value.contains('\n') || value.contains('\r') {
        anyhow::bail!("{label} 不能包含换行或 NUL 字符");
    }
    Ok(())
}

pub fn build_wrapper_config(settings: &BackendSettings) -> anyhow::Result<String> {
    let api_key_env = settings.cli_wrapper_api_key_env.trim();
    let api_key_env = if api_key_env.is_empty() {
        crate::settings::default_api_key_env()
    } else {
        api_key_env.to_string()
    };
    validate_wrapper_config_value("Desktop CLI Bridge API Key Env", &api_key_env)?;
    let api_key = settings.cli_wrapper_api_key.trim();
    validate_wrapper_config_value("Desktop CLI Bridge API Key", api_key)?;
    let base_url = crate::url_policy::validate_optional_api_base_url(
        "Desktop CLI Bridge Base URL",
        &settings.cli_wrapper_base_url,
    )?
    .unwrap_or_default();
    Ok(format!(
        "apiKeyEnv={api_key_env}\nbaseUrl={base_url}\napiKey={api_key}\n"
    ))
}

fn write_wrapper_config_to(path: &Path, settings: &BackendSettings) -> anyhow::Result<()> {
    let config = build_wrapper_config(settings)?;
    std::fs::write(path, config).with_context(|| format!("failed to write {}", path.display()))?;
    crate::harden_sensitive_file(path)
}

pub fn install_cli_wrapper_to(
    wrapper_dir: &Path,
    real_codex: &Path,
    codex_home: &Path,
    settings: &BackendSettings,
) -> anyhow::Result<WrapperInstall> {
    if !settings.cli_wrapper_base_url.trim().is_empty() {
        crate::url_policy::validate_api_base_url(
            "Desktop CLI Bridge Base URL",
            &settings.cli_wrapper_base_url,
        )?;
    }
    std::fs::create_dir_all(wrapper_dir)
        .with_context(|| format!("failed to create wrapper dir {}", wrapper_dir.display()))?;
    std::fs::create_dir_all(codex_home)
        .with_context(|| format!("failed to create AgentKey CLI home {}", codex_home.display()))?;

    let source_path = wrapper_dir.join(WRAPPER_SOURCE);
    let wrapper_path = wrapper_dir.join(WRAPPER_EXE);
    let config_path = wrapper_dir.join(WRAPPER_CONFIG);
    let source = build_wrapper_source(real_codex, codex_home, settings);
    std::fs::write(&source_path, source)
        .with_context(|| format!("failed to write {}", source_path.display()))?;
    write_wrapper_config_to(&config_path, settings)?;

    compile_wrapper(&source_path, &wrapper_path)?;
    Ok(WrapperInstall {
        wrapper_path,
        source_path,
        config_path,
        real_codex: real_codex.to_path_buf(),
        codex_home: codex_home.to_path_buf(),
    })
}

pub fn resolve_real_codex() -> Option<PathBuf> {
    let app_dir = crate::app_paths::resolve_codex_app_dir(None);
    resolve_real_codex_from_candidates(app_dir.as_deref(), &default_user_runtime_candidates())
}

pub fn resolve_real_codex_for_settings(settings: &BackendSettings) -> Option<PathBuf> {
    let app_dir = crate::app_paths::resolve_codex_app_dir_with_saved(
        None,
        Some(settings.codex_app_path.as_str()),
    );
    resolve_real_codex_from_candidates(app_dir.as_deref(), &default_user_runtime_candidates())
}

pub fn resolve_real_codex_from_candidates(
    app_dir: Option<&Path>,
    user_runtime_candidates: &[PathBuf],
) -> Option<PathBuf> {
    user_runtime_candidates
        .iter()
        .chain(packaged_codex_candidates(app_dir).iter())
        .find(|path| path.is_file())
        .cloned()
}

pub fn default_user_runtime_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if cfg!(windows) {
        if let Some(local_app_data) = std::env::var_os("LOCALAPPDATA").map(PathBuf::from) {
            candidates.push(
                local_app_data
                    .join("OpenAI")
                    .join("Codex")
                    .join("bin")
                    .join("codex.exe"),
            );
        }
    }
    candidates
}

pub fn packaged_codex_candidates(app_dir: Option<&Path>) -> Vec<PathBuf> {
    let Some(app_dir) = app_dir else {
        return Vec::new();
    };
    vec![
        app_dir.join("resources").join("codex.exe"),
        app_dir.join("resources").join("codex"),
    ]
}

pub fn wrapper_dir() -> PathBuf {
    if cfg!(windows) {
        if let Some(roaming) = std::env::var_os("APPDATA").map(PathBuf::from) {
            return wrapper_dir_from_roaming(&roaming);
        }
    }
    crate::paths::default_app_state_dir().join("cli-wrapper")
}

pub fn wrapper_dir_from_roaming(roaming: &Path) -> PathBuf {
    let roaming_text = roaming.as_os_str().to_string_lossy();
    if roaming_text.contains('\\') && !roaming_text.contains('/') {
        return PathBuf::from(format!("{roaming_text}\\AgentKey"));
    }
    roaming.join("AgentKey")
}

pub fn cli_home_dir() -> PathBuf {
    directories::BaseDirs::new()
        .map(|dirs| dirs.home_dir().join(CLI_HOME_DIR))
        .unwrap_or_else(|| PathBuf::from(CLI_HOME_DIR))
}

pub fn build_wrapper_source(
    real_codex: &Path,
    codex_home: &Path,
    _settings: &BackendSettings,
) -> String {
    format!(
        r#"using System;
using System.Diagnostics;
using System.IO;
using System.Text;

class AgentKeyCliBridge
{{
    static int Main(string[] args)
    {{
        string desktopClientCli = @{real_codex};
        string desktopClientHome = @{codex_home};
        string configPath = Path.Combine(AppDomain.CurrentDomain.BaseDirectory, "agentkey-cli-wrapper.env");
        WrapperConfig config = ReadConfig(configPath);
        string apiKeyEnv = String.IsNullOrWhiteSpace(config.ApiKeyEnv) ? "CUSTOM_OPENAI_API_KEY" : config.ApiKeyEnv.Trim();
        Directory.CreateDirectory(desktopClientHome);
        string logPath = Path.Combine(desktopClientHome, "agentkey-cli-wrapper.log");
        AppendLog(logPath, "agentkey-cli-wrapper start args=" + RedactArguments(args));
        AppendLog(logPath, "target_cli=" + desktopClientCli);
        AppendLog(logPath, "CODEX_HOME=" + desktopClientHome);
        AppendLog(logPath, "api_key_env=" + apiKeyEnv + " api_key_present=" + (!String.IsNullOrWhiteSpace(config.ApiKey)).ToString().ToLowerInvariant());
        var startInfo = new ProcessStartInfo(desktopClientCli);
        startInfo.UseShellExecute = false;
        startInfo.RedirectStandardInput = false;
        startInfo.RedirectStandardOutput = false;
        startInfo.RedirectStandardError = false;
        startInfo.EnvironmentVariables["CODEX_HOME"] = desktopClientHome;
        if (!String.IsNullOrWhiteSpace(config.BaseUrl)) startInfo.EnvironmentVariables["OPENAI_BASE_URL"] = config.BaseUrl.Trim();
        if (!String.IsNullOrWhiteSpace(config.ApiKey)) startInfo.EnvironmentVariables[apiKeyEnv] = config.ApiKey.Trim();
        foreach (string arg in args) startInfo.Arguments += QuoteArgument(arg) + " ";
        using (var process = Process.Start(startInfo))
        {{
            process.WaitForExit();
            AppendLog(logPath, "exit_code=" + process.ExitCode);
            return process.ExitCode;
        }}
    }}

    static void AppendLog(string path, string message)
    {{
        File.AppendAllText(path, "[" + DateTime.Now.ToString("yyyy-MM-dd HH:mm:ss") + "] " + message + Environment.NewLine, Encoding.UTF8);
    }}

    static WrapperConfig ReadConfig(string path)
    {{
        WrapperConfig config = new WrapperConfig();
        if (!File.Exists(path)) return config;
        foreach (string rawLine in File.ReadAllLines(path, Encoding.UTF8))
        {{
            string line = (rawLine ?? "").Trim();
            if (line.Length == 0 || line.StartsWith("#")) continue;
            int equalsIndex = line.IndexOf('=');
            if (equalsIndex < 0) continue;
            string key = line.Substring(0, equalsIndex).Trim();
            string value = line.Substring(equalsIndex + 1).Trim();
            if (key == "apiKeyEnv") config.ApiKeyEnv = value;
            else if (key == "baseUrl") config.BaseUrl = value;
            else if (key == "apiKey") config.ApiKey = value;
        }}
        return config;
    }}

    static string RedactArguments(string[] args)
    {{
        string[] redacted = new string[args.Length];
        bool redactNext = false;
        for (int i = 0; i < args.Length; i++)
        {{
            string arg = args[i] ?? "";
            string lower = arg.ToLowerInvariant();
            if (redactNext)
            {{
                redacted[i] = "[REDACTED]";
                redactNext = false;
                continue;
            }}
            int equalsIndex = arg.IndexOf('=');
            string key = equalsIndex >= 0 ? lower.Substring(0, equalsIndex) : lower;
            if (IsSecretName(key))
            {{
                redacted[i] = equalsIndex >= 0 ? arg.Substring(0, equalsIndex + 1) + "[REDACTED]" : arg;
                redactNext = equalsIndex < 0;
            }}
            else if (LooksLikeSecret(arg))
            {{
                redacted[i] = "[REDACTED]";
            }}
            else
            {{
                redacted[i] = arg;
            }}
        }}
        return string.Join(" ", redacted);
    }}

    static bool IsSecretName(string value)
    {{
        return value.Contains("key") || value.Contains("token") || value.Contains("secret") || value.Contains("password") || value.Contains("authorization");
    }}

    static bool LooksLikeSecret(string value)
    {{
        return value.StartsWith("sk-", StringComparison.OrdinalIgnoreCase) || value.StartsWith("sess-", StringComparison.OrdinalIgnoreCase);
    }}

    static string QuoteArgument(string value)
    {{
        if (value.Length == 0) return "\"\"";
        if (value.IndexOfAny(new char[] {{ ' ', '\t', '\n', '\r', '\"' }}) < 0) return value;
        return "\"" + value.Replace("\\", "\\\\").Replace("\"", "\\\"") + "\"";
    }}

    class WrapperConfig
    {{
        public string ApiKeyEnv = "";
        public string BaseUrl = "";
        public string ApiKey = "";
    }}
}}
"#,
        real_codex = cs_string_literal(&real_codex.to_string_lossy()),
        codex_home = cs_string_literal(&codex_home.to_string_lossy()),
    )
}

fn compile_wrapper(source_path: &Path, wrapper_path: &Path) -> anyhow::Result<()> {
    let csc =
        find_csc().ok_or_else(|| anyhow::anyhow!("未找到 csc.exe，无法编译 AgentKey wrapper"))?;
    let output_arg = format!("/out:{}", wrapper_path.display());
    let mut command = Command::new(&csc);
    command
        .args(["/nologo", "/target:exe"])
        .arg(output_arg)
        .arg(source_path);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(crate::windows_integration::CREATE_NO_WINDOW);
    }
    let status = command
        .status()
        .with_context(|| format!("failed to run {}", csc.display()))?;
    if status.success() {
        Ok(())
    } else {
        anyhow::bail!("csc.exe exited with {status}")
    }
}

fn find_csc() -> Option<PathBuf> {
    if let Some(windir) = std::env::var_os("WINDIR").map(PathBuf::from) {
        for relative in [
            ["Microsoft.NET", "Framework64", "v4.0.30319", "csc.exe"],
            ["Microsoft.NET", "Framework", "v4.0.30319", "csc.exe"],
        ] {
            let path = relative
                .iter()
                .fold(windir.clone(), |path, segment| path.join(segment));
            if path.is_file() {
                return Some(path);
            }
        }
    }
    Some(PathBuf::from("csc.exe"))
}

fn cs_string_literal(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

fn csharp_string_assignment(source: &str, variable: &str) -> Option<String> {
    let marker = format!("string {variable} = @\"");
    let rest = source.split_once(&marker)?.1;
    parse_csharp_verbatim_string(rest)
}

fn csharp_environment_assignment(source: &str, key: &str) -> Option<String> {
    let marker = format!("startInfo.EnvironmentVariables{key} = @\"");
    let rest = source.split_once(&marker)?.1;
    parse_csharp_verbatim_string(rest)
}

fn parse_csharp_verbatim_string(rest: &str) -> Option<String> {
    let mut value = String::new();
    let mut chars = rest.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '"' {
            if chars.peek() == Some(&'"') {
                chars.next();
                value.push('"');
                continue;
            }
            return Some(value);
        }
        value.push(ch);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn wrapper_config_write_restricts_file_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let temp = tempfile::tempdir().unwrap();
        let wrapper_dir = temp.path().join("AgentKey");
        std::fs::create_dir_all(&wrapper_dir).unwrap();
        let path = wrapper_dir.join(WRAPPER_CONFIG);
        let settings = BackendSettings {
            cli_wrapper_api_key: "sk-test".to_string(),
            cli_wrapper_base_url: "https://proxy.example/v1".to_string(),
            ..BackendSettings::default()
        };

        write_wrapper_config_to(&path, &settings).unwrap();

        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(
            std::fs::metadata(&wrapper_dir)
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o700
        );
    }
}
