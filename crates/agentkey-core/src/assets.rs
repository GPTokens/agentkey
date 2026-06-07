const RENDERER_SCRIPT: &str = include_str!("../../../assets/inject/renderer-inject.js");
pub const DIAGNOSTIC_BUILD_ID: &str = "diag-20260518-1";

pub fn renderer_script() -> &'static str {
    RENDERER_SCRIPT
}

pub fn injection_script(helper_port: u16, helper_token: &str) -> String {
    let helper_url = format!("http://127.0.0.1:{helper_port}");
    format!(
        "window.__AGENTKEY_HELPER_BASE__ = {};\nwindow.__AGENTKEY_HELPER_TOKEN__ = {};\nwindow.__AGENTKEY_VERSION__ = {};\nwindow.__AGENTKEY_BUILD__ = {};\n{}",
        serde_json::to_string(&helper_url).expect("helper URL should serialize"),
        serde_json::to_string(helper_token).expect("helper token should serialize"),
        serde_json::to_string(crate::version::VERSION).expect("version should serialize"),
        serde_json::to_string(DIAGNOSTIC_BUILD_ID).expect("build id should serialize"),
        renderer_script(),
    )
}
