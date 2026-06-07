pub fn api_base_url_allowed(url: &str) -> bool {
    let Ok(parsed) = reqwest::Url::parse(url.trim()) else {
        return false;
    };
    match parsed.scheme() {
        "https" => true,
        "http" => match parsed.host_str() {
            Some(host) => matches!(host, "localhost" | "127.0.0.1" | "::1" | "[::1]"),
            None => false,
        },
        _ => false,
    }
}

pub fn validate_api_base_url(label: &str, url: &str) -> anyhow::Result<String> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        anyhow::bail!("{label} 不能为空");
    }
    if !api_base_url_allowed(trimmed) {
        anyhow::bail!("{label} 必须使用 HTTPS；HTTP 仅允许 localhost、127.0.0.1 或 ::1")
    }
    Ok(trimmed.trim_end_matches('/').to_string())
}

pub fn validate_optional_api_base_url(label: &str, url: &str) -> anyhow::Result<Option<String>> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    validate_api_base_url(label, trimmed).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_base_url_allows_https_and_loopback_http() {
        assert!(api_base_url_allowed("https://gateway.example.test/v1"));
        assert!(api_base_url_allowed("http://127.0.0.1:4000/v1"));
        assert!(api_base_url_allowed("http://localhost:4000/v1"));
        assert!(api_base_url_allowed("http://[::1]:4000/v1"));
    }

    #[test]
    fn api_base_url_rejects_cleartext_remote_hosts() {
        assert!(!api_base_url_allowed("http://192.168.1.10:4000/v1"));
        assert!(!api_base_url_allowed("http://gateway.example.test/v1"));
        assert!(!api_base_url_allowed("ftp://gateway.example.test/v1"));
        assert!(!api_base_url_allowed("not a url"));
    }
}
