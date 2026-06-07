use std::path::PathBuf;

use agentkey_core::cli_wrapper::{
    build_wrapper_config, build_wrapper_source, install_cli_wrapper_to,
    parse_wrapper_config_settings, parse_wrapper_source_settings,
    resolve_real_codex_from_candidates, should_refresh_cli_wrapper, wrapper_dir_from_roaming,
    wrapper_settings_for_refresh,
};
use agentkey_core::settings::BackendSettings;

#[test]
fn wrapper_source_embeds_absolute_real_codex_path() {
    let settings = BackendSettings {
        cli_wrapper_enabled: true,
        cli_wrapper_base_url: "https://proxy.example/v1".to_string(),
        cli_wrapper_api_key: "sk-test".to_string(),
        cli_wrapper_api_key_env: "CUSTOM_KEY".to_string(),
        ..BackendSettings::default()
    };
    let source = build_wrapper_source(
        &PathBuf::from(r"C:\AgentKey\Runtime\codex.exe"),
        &PathBuf::from(r"C:\Users\me\.agentkey-cli"),
        &settings,
    );

    assert!(source.contains(r#"class AgentKeyCliBridge"#));
    assert!(!source.contains(r#"class CodexWrapper"#));
    assert!(source.contains(r#"string realCodex = @"C:\AgentKey\Runtime\codex.exe";"#));
    assert!(!source.contains(r#"string realCodex = @"codex";"#));
    assert!(source.contains("ReadConfig(configPath)"));
    assert!(source.contains("agentkey-cli-wrapper.env"));
    assert!(source.contains(r#"startInfo.EnvironmentVariables["OPENAI_BASE_URL"]"#));
    assert!(!source.contains("https://proxy.example/v1"));
    assert!(!source.contains("sk-test"));
    assert!(!source.contains("CUSTOM_KEY"));
}

#[test]
fn wrapper_source_redacts_logged_arguments() {
    let source = build_wrapper_source(
        &PathBuf::from(r"C:\AgentKey\Runtime\codex.exe"),
        &PathBuf::from(r"C:\Users\me\.agentkey-cli"),
        &BackendSettings::default(),
    );

    assert!(source.contains("agentkey-cli-wrapper.log"));
    assert!(source.contains("RedactArguments(args)"));
    assert!(source.contains("LooksLikeSecret"));
    assert!(!source.contains("start args=\" + string.Join(\" \", args)"));
}

#[test]
fn wrapper_source_omits_remote_http_base_url() {
    let settings = BackendSettings {
        cli_wrapper_enabled: true,
        cli_wrapper_base_url: "http://gateway.example.test/v1".to_string(),
        cli_wrapper_api_key: "sk-test".to_string(),
        ..BackendSettings::default()
    };
    let source = build_wrapper_source(
        &PathBuf::from(r"C:\AgentKey\codex.exe"),
        &PathBuf::from(r"C:\Users\me\.agentkey-cli"),
        &settings,
    );

    assert!(source.contains("OPENAI_BASE_URL"));
    assert!(!source.contains("http://gateway.example.test/v1"));
    assert!(!source.contains("sk-test"));
}

#[test]
fn wrapper_config_contains_api_settings_outside_generated_source() {
    let settings = BackendSettings {
        cli_wrapper_enabled: true,
        cli_wrapper_base_url: "https://proxy.example/v1/".to_string(),
        cli_wrapper_api_key: "sk-test".to_string(),
        cli_wrapper_api_key_env: "CUSTOM_KEY".to_string(),
        ..BackendSettings::default()
    };
    let config = build_wrapper_config(&settings).unwrap();
    let parsed = parse_wrapper_config_settings(&config).unwrap();

    assert_eq!(parsed.cli_wrapper_api_key_env, "CUSTOM_KEY");
    assert_eq!(parsed.cli_wrapper_base_url, "https://proxy.example/v1");
    assert_eq!(parsed.cli_wrapper_api_key, "sk-test");
}

#[test]
fn wrapper_install_rejects_remote_http_base_url_before_compilation() {
    let temp = tempfile::tempdir().unwrap();
    let real_codex = temp.path().join("codex.exe");
    std::fs::write(&real_codex, "").unwrap();
    let settings = BackendSettings {
        cli_wrapper_enabled: true,
        cli_wrapper_base_url: "http://gateway.example.test/v1".to_string(),
        cli_wrapper_api_key: "sk-test".to_string(),
        ..BackendSettings::default()
    };

    let error = install_cli_wrapper_to(
        &temp.path().join("wrapper"),
        &real_codex,
        &temp.path().join("home"),
        &settings,
    )
    .unwrap_err();

    assert!(error.to_string().contains("HTTP 仅允许"));
}

#[test]
fn resolves_user_runtime_before_packaged_resources_codex() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp
        .path()
        .join("OpenAI.Codex_1.0.0.0_x64__abc")
        .join("app");
    let packaged = app_dir.join("resources").join("codex.exe");
    let user_runtime = temp
        .path()
        .join("OpenAI")
        .join("Codex")
        .join("bin")
        .join("codex.exe");
    std::fs::create_dir_all(packaged.parent().unwrap()).unwrap();
    std::fs::create_dir_all(user_runtime.parent().unwrap()).unwrap();
    std::fs::write(&packaged, "").unwrap();
    std::fs::write(&user_runtime, "").unwrap();

    let resolved = resolve_real_codex_from_candidates(Some(&app_dir), &[user_runtime.clone()])
        .expect("user runtime codex should be preferred");

    assert_eq!(resolved, user_runtime);
}

#[test]
fn resolves_packaged_resources_when_user_runtime_is_missing() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp
        .path()
        .join("OpenAI.Codex_1.0.0.0_x64__abc")
        .join("app");
    let packaged = app_dir.join("resources").join("codex.exe");
    std::fs::create_dir_all(&app_dir).unwrap();
    std::fs::create_dir_all(packaged.parent().unwrap()).unwrap();
    std::fs::write(&packaged, "").unwrap();

    let resolved = resolve_real_codex_from_candidates(Some(&app_dir), &[])
        .expect("packaged codex should be used as fallback");

    assert_eq!(resolved, packaged);
}

#[test]
fn wrapper_dir_uses_roaming_agentkey() {
    assert_eq!(
        wrapper_dir_from_roaming(&PathBuf::from(r"C:\Users\me\AppData\Roaming")),
        PathBuf::from(r"C:\Users\me\AppData\Roaming\AgentKey")
    );
}

#[test]
fn repair_refreshes_when_wrapper_already_exists_even_if_setting_is_disabled() {
    let temp = tempfile::tempdir().unwrap();
    let wrapper_dir = temp.path().join("AgentKey");
    std::fs::create_dir_all(&wrapper_dir).unwrap();
    std::fs::write(wrapper_dir.join("agentkey-cli-wrapper.exe"), "").unwrap();

    assert!(should_refresh_cli_wrapper(
        &BackendSettings::default(),
        &wrapper_dir
    ));
}

#[test]
fn repair_refreshes_legacy_wrapper_when_setting_is_disabled() {
    let temp = tempfile::tempdir().unwrap();
    let wrapper_dir = temp.path().join("AgentKey");
    std::fs::create_dir_all(&wrapper_dir).unwrap();
    std::fs::write(wrapper_dir.join("codex-wrapper.exe"), "").unwrap();

    assert!(should_refresh_cli_wrapper(
        &BackendSettings::default(),
        &wrapper_dir
    ));
}

#[test]
fn repair_skips_when_wrapper_is_disabled_and_absent() {
    let temp = tempfile::tempdir().unwrap();

    assert!(!should_refresh_cli_wrapper(
        &BackendSettings::default(),
        temp.path()
    ));
}

#[test]
fn repair_preserves_existing_wrapper_api_settings_when_global_setting_is_disabled() {
    let temp = tempfile::tempdir().unwrap();
    let wrapper_dir = temp.path().join("AgentKey");
    std::fs::create_dir_all(&wrapper_dir).unwrap();
    std::fs::write(
        wrapper_dir.join("codex-wrapper.cs"),
        r#"class AgentKeyCliBridge
{
    static int Main(string[] args)
    {
        string apiKeyEnv = @"CUSTOM_KEY";
        string apiKey = @"sk-old";
        startInfo.EnvironmentVariables["OPENAI_BASE_URL"] = @"https://old.example/v1";
        startInfo.EnvironmentVariables[apiKeyEnv] = apiKey;
    }
}"#,
    )
    .unwrap();

    let settings = wrapper_settings_for_refresh(&BackendSettings::default(), &wrapper_dir);

    assert_eq!(settings.cli_wrapper_api_key_env, "CUSTOM_KEY");
    assert_eq!(settings.cli_wrapper_api_key, "sk-old");
    assert_eq!(
        settings.cli_wrapper_base_url,
        "https://old.example/v1".to_string()
    );
}

#[test]
fn repair_reads_new_wrapper_config_before_legacy_source() {
    let temp = tempfile::tempdir().unwrap();
    let wrapper_dir = temp.path().join("AgentKey");
    std::fs::create_dir_all(&wrapper_dir).unwrap();
    std::fs::write(
        wrapper_dir.join("agentkey-cli-wrapper.env"),
        "apiKeyEnv=CONFIG_KEY\nbaseUrl=https://config.example/v1\napiKey=sk-config\n",
    )
    .unwrap();
    std::fs::write(
        wrapper_dir.join("codex-wrapper.cs"),
        r#"string apiKeyEnv = @"LEGACY_KEY";
startInfo.EnvironmentVariables["OPENAI_BASE_URL"] = @"https://legacy.example/v1";
startInfo.EnvironmentVariables[apiKeyEnv] = @"sk-legacy";"#,
    )
    .unwrap();

    let settings = wrapper_settings_for_refresh(&BackendSettings::default(), &wrapper_dir);

    assert_eq!(settings.cli_wrapper_api_key_env, "CONFIG_KEY");
    assert_eq!(settings.cli_wrapper_base_url, "https://config.example/v1");
    assert_eq!(settings.cli_wrapper_api_key, "sk-config");
}

#[test]
fn parses_new_wrapper_source_api_settings() {
    let parsed = parse_wrapper_source_settings(
        r#"string apiKeyEnv = @"CUSTOM_KEY";
startInfo.EnvironmentVariables["OPENAI_BASE_URL"] = @"https://new.example/v1";
startInfo.EnvironmentVariables[apiKeyEnv] = @"sk-new";"#,
    )
    .unwrap();

    assert_eq!(parsed.cli_wrapper_api_key_env, "CUSTOM_KEY");
    assert_eq!(parsed.cli_wrapper_api_key, "sk-new");
    assert_eq!(parsed.cli_wrapper_base_url, "https://new.example/v1");
}
