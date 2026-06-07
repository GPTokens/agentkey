use agentkey_core::update::{
    Release, download_asset_to, is_newer_version, parse_version_tag, release_from_github_payload,
    release_from_latest_json_payload, safe_asset_name, select_update_asset, sha256_hex,
    update_url_allowed, verify_release_asset_sha256,
};
use serde_json::json;

#[test]
fn parse_version_tag_accepts_prefix_and_suffix() {
    assert_eq!(parse_version_tag("v1.2.3").unwrap(), vec![1, 2, 3]);
    assert_eq!(parse_version_tag("1.2.3").unwrap(), vec![1, 2, 3]);
    assert_eq!(parse_version_tag("v1.2.3-beta.1").unwrap(), vec![1, 2, 3]);
}

#[test]
fn version_comparison_uses_numeric_segments() {
    assert!(is_newer_version("v1.0.10", "1.0.4").unwrap());
    assert!(!is_newer_version("v1.0.4", "1.0.4").unwrap());
    assert!(!is_newer_version("v1.0.3", "1.0.4").unwrap());
}

#[test]
fn github_payload_selects_platform_installer() {
    let release = release_from_github_payload(&json!({
        "tag_name": "v1.0.9",
        "html_url": "https://github.com/GPTokens/agentkey/releases/tag/v1.0.9",
        "body": "fixes",
        "assets": [
            {"name": "source.zip", "browser_download_url": "https://example.test/source.zip"},
            {"name": "agentkey-manager.exe", "browser_download_url": "https://example.test/manager.exe"},
            {"name": "AgentKey_1.0.9_x64-setup.exe", "browser_download_url": "https://example.test/setup.exe", "digest": "sha256:bef57ec7f53a6d40beb640a780a639c83bc29ac8a9816f1fc6c5c6dcd93c4721"},
            {"name": "AgentKey_1.0.9_x64.dmg", "browser_download_url": "https://example.test/app.dmg"}
        ]
    }))
    .unwrap();

    assert_eq!(release.version, "v1.0.9");
    if cfg!(windows) {
        assert_eq!(
            release.asset_name.as_deref(),
            Some("AgentKey_1.0.9_x64-setup.exe")
        );
        assert_eq!(
            release.asset_sha256.as_deref(),
            Some("bef57ec7f53a6d40beb640a780a639c83bc29ac8a9816f1fc6c5c6dcd93c4721")
        );
    } else if cfg!(target_os = "macos") {
        assert_eq!(
            release.asset_name.as_deref(),
            Some("AgentKey_1.0.9_x64.dmg")
        );
    } else {
        assert_eq!(release.asset_name.as_deref(), None);
    }
}

#[test]
fn latest_json_payload_selects_platform_installer_without_github_api_shape() {
    let release = release_from_latest_json_payload(&json!({
        "version": "v1.1.6",
        "url": "https://github.com/GPTokens/agentkey/releases/tag/v1.1.6",
        "body": "静态更新描述",
        "assets": [
            {"name": "source.zip", "url": "https://example.test/source.zip"},
            {"name": "AgentKey-1.1.6-windows-x64-setup.exe", "url": "https://example.test/setup.exe", "sha256": "bef57ec7f53a6d40beb640a780a639c83bc29ac8a9816f1fc6c5c6dcd93c4721"},
            {"name": "AgentKey-1.1.6-macos-x64.dmg", "url": "https://example.test/app.dmg"}
        ]
    }))
    .unwrap();

    assert_eq!(release.version, "v1.1.6");
    assert_eq!(release.body, "静态更新描述");
    if cfg!(windows) {
        assert_eq!(
            release.asset_name.as_deref(),
            Some("AgentKey-1.1.6-windows-x64-setup.exe")
        );
        assert_eq!(
            release.asset_sha256.as_deref(),
            Some("bef57ec7f53a6d40beb640a780a639c83bc29ac8a9816f1fc6c5c6dcd93c4721")
        );
    } else if cfg!(target_os = "macos") {
        assert_eq!(
            release.asset_name.as_deref(),
            Some("AgentKey-1.1.6-macos-x64.dmg")
        );
    } else {
        assert_eq!(release.asset_name.as_deref(), None);
    }
}

#[test]
fn asset_selection_prefers_current_platform_artifacts() {
    let assets = vec![
        (
            "AgentKey.zip".to_string(),
            "https://example.test/source.zip".to_string(),
        ),
        (
            "agentkey-manager.exe".to_string(),
            "https://example.test/manager.exe".to_string(),
        ),
        (
            "AgentKey_1.0.9_x64-setup.exe".to_string(),
            "https://example.test/setup.exe".to_string(),
        ),
        (
            "AgentKey_1.0.9_x64.dmg".to_string(),
            "https://example.test/app.dmg".to_string(),
        ),
    ];

    if cfg!(windows) {
        let selected = select_update_asset(&assets).unwrap();
        assert_eq!(selected.name, "AgentKey_1.0.9_x64-setup.exe");
    } else if cfg!(target_os = "macos") {
        let selected = select_update_asset(&assets).unwrap();
        assert_eq!(selected.name, "AgentKey_1.0.9_x64.dmg");
    } else {
        assert!(select_update_asset(&assets).is_none());
    }
}

#[test]
fn update_urls_must_use_https() {
    assert!(update_url_allowed("https://example.test/pkg.zip"));
    assert!(!update_url_allowed("http://example.test/pkg.zip"));
    assert!(!update_url_allowed("file:///tmp/pkg.zip"));
}

#[test]
fn asset_selection_ignores_non_https_urls() {
    let insecure_prefix = concat!("http", "://");
    let assets = vec![
        (
            "AgentKey_1.0.9_x64-setup.exe".to_string(),
            format!("{insecure_prefix}example.test/setup.exe"),
        ),
        (
            "AgentKey_1.0.9_x64.dmg".to_string(),
            format!("{insecure_prefix}example.test/app.dmg"),
        ),
    ];

    assert!(select_update_asset(&assets).is_none());
}

#[test]
fn safe_asset_name_rejects_path_traversal() {
    assert_eq!(safe_asset_name("pkg.zip").unwrap(), "pkg.zip");
    assert!(safe_asset_name("../pkg.zip").is_err());
    assert!(safe_asset_name("").is_err());
}

#[test]
fn download_asset_to_writes_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let release = Release {
        version: "v1.0.9".to_string(),
        url: "https://example.test".to_string(),
        body: "fixes".to_string(),
        asset_name: Some("pkg.zip".to_string()),
        asset_url: Some("https://example.test/pkg.zip".to_string()),
        asset_sha256: None,
    };

    let path = download_asset_to(&release, b"abcdef", dir.path()).unwrap();

    assert_eq!(path, dir.path().join("pkg.zip"));
    assert_eq!(std::fs::read(path).unwrap(), b"abcdef");
}

#[test]
fn sha256_hex_matches_known_vector() {
    assert_eq!(
        sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn release_asset_sha256_is_required_before_execution() {
    let release = Release {
        version: "v1.0.9".to_string(),
        url: "https://example.test".to_string(),
        body: "fixes".to_string(),
        asset_name: Some("pkg.zip".to_string()),
        asset_url: Some("https://example.test/pkg.zip".to_string()),
        asset_sha256: None,
    };

    assert!(verify_release_asset_sha256(&release, b"abcdef").is_err());
}

#[test]
fn release_asset_sha256_rejects_mismatch() {
    let release = Release {
        version: "v1.0.9".to_string(),
        url: "https://example.test".to_string(),
        body: "fixes".to_string(),
        asset_name: Some("pkg.zip".to_string()),
        asset_url: Some("https://example.test/pkg.zip".to_string()),
        asset_sha256: Some("0".repeat(64)),
    };

    assert!(verify_release_asset_sha256(&release, b"abcdef").is_err());
}

#[test]
fn release_asset_sha256_accepts_match() {
    let release = Release {
        version: "v1.0.9".to_string(),
        url: "https://example.test".to_string(),
        body: "fixes".to_string(),
        asset_name: Some("pkg.zip".to_string()),
        asset_url: Some("https://example.test/pkg.zip".to_string()),
        asset_sha256: Some(sha256_hex(b"abcdef")),
    };

    assert_eq!(
        verify_release_asset_sha256(&release, b"abcdef").unwrap(),
        sha256_hex(b"abcdef")
    );
}
