pub mod ads;
pub mod app_paths;
pub mod assets;
pub mod bridge;
pub mod cdp;
pub mod claude_code;
pub mod cli_wrapper;
pub mod diagnostic_log;
pub mod http_client;
pub mod install;
pub mod launcher;
pub mod model_catalog;
pub mod models;
pub mod paths;
pub mod ports;
pub mod protocol_proxy;
pub mod provider_link;
pub mod proxy;
pub mod relay_config;
pub mod routes;
pub mod script_market;
pub mod settings;
pub mod status;
pub mod update;
pub mod upstream_worktree;
pub mod url_policy;
pub mod user_scripts;
pub mod version;
pub mod watcher;
#[cfg(windows)]
mod windows_integration;
pub mod zed_remote;

pub fn restrict_sensitive_file_access(path: &std::path::Path) -> anyhow::Result<()> {
    #[cfg(unix)]
    {
        use anyhow::Context;
        use std::fs;
        use std::os::unix::fs::PermissionsExt;

        if let Some(parent) = path.parent() {
            fs::set_permissions(parent, fs::Permissions::from_mode(0o700)).with_context(|| {
                format!(
                    "failed to restrict sensitive directory {}",
                    parent.display()
                )
            })?;
        }
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))
            .with_context(|| format!("failed to restrict sensitive file {}", path.display()))?;
    }
    #[cfg(windows)]
    {
        use anyhow::Context;

        windows_integration::restrict_path_to_current_user(path)
            .with_context(|| format!("failed to restrict sensitive file {}", path.display()))?;
    }
    Ok(())
}

pub fn harden_sensitive_file(path: &std::path::Path) -> anyhow::Result<()> {
    #[cfg(windows)]
    {
        use anyhow::Context;

        if let Some(parent) = path.parent() {
            windows_integration::restrict_path_to_current_user(parent).with_context(|| {
                format!(
                    "failed to restrict sensitive directory {}",
                    parent.display()
                )
            })?;
        }
    }
    restrict_sensitive_file_access(path)?;
    #[cfg(windows)]
    {
        use anyhow::Context;

        windows_integration::hide_file(path)
            .with_context(|| format!("failed to hide sensitive file {}", path.display()))?;
    }
    Ok(())
}

#[cfg(windows)]
pub fn windows_create_no_window() -> u32 {
    windows_integration::CREATE_NO_WINDOW
}

#[cfg(windows)]
pub fn windows_open_url(url: &str) -> anyhow::Result<()> {
    windows_integration::open_url(url)
}

#[cfg(windows)]
pub fn windows_activate_process_window(process_id: u32) -> bool {
    windows_integration::activate_process_window(process_id)
}
