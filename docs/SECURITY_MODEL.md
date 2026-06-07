# Security Model

AgentKey exposes local desktop-client functionality through a loopback helper. The helper must be treated as a privileged component because it can proxy requests that use user-provided API keys.

## Required Controls

- Bind helper services to loopback only.
- Generate a random session token at launch.
- Inject the session token only into trusted desktop-client pages.
- Reject helper requests that do not present the session token.
- Restrict CORS to trusted desktop-client origins.
- Never allow wildcard CORS on privileged helper routes.
- Redact API keys and bearer tokens from diagnostics.
- Require HTTPS for API provider base URLs unless the URL is loopback HTTP.
- Verify downloaded update assets before execution.
- Require HTTPS for update metadata and release asset downloads.
- Require HTTPS for script market indexes and script downloads.
- Validate script integrity before enabling installed scripts.

## Local Helper Token

The helper token is session-scoped. It is generated at launch and passed to the trusted renderer injection. Browser requests to helper routes must include the token through a dedicated header or equivalent authenticated channel.

## CORS

CORS is not an authentication mechanism. It is a browser access boundary. AgentKey uses CORS restrictions together with the session token so unrelated web pages cannot call privileged local helper endpoints.

## Updater

Updater code must use HTTPS for update metadata and release asset downloads. It must also verify release artifacts before execution. Acceptable verification options include detached signatures, trusted checksums, or platform-native signing where available.

AgentKey currently requires a SHA-256 checksum for the selected release asset. The checksum can be supplied as `sha256`, `checksum`, or a GitHub-style `digest` value such as `sha256:<hex>`. Assets without a valid checksum are downloaded but not executed. Assets with non-HTTPS URLs are ignored.

## Diagnostics

Diagnostics may include environment details, configuration paths, and provider settings. Diagnostics must redact secrets before sending or saving reports.

## Claude Code Launch

Claude Code is started with API credentials in the child process environment. API keys must not be written to command-line arguments or diagnostic events. Extra environment variables are accepted only as `KEY=value` lines with validated variable names.

Relay providers, Claude Code, model-catalog fetches, Chat Completions proxy upstreams, and Codex CLI wrapper provider base URLs must use HTTPS. HTTP is accepted only for loopback development endpoints such as `localhost`, `127.0.0.1`, or `::1`.

The Codex CLI wrapper must not write raw command arguments that look like keys, tokens, secrets, passwords, or authorization values to its log file.

## Script Market

Market scripts must provide a valid SHA-256 checksum before installation. AgentKey verifies the downloaded bytes before writing the script file. Newly installed market scripts are disabled by default and must be enabled explicitly.

Script market index URLs and script download URLs must use HTTPS. Manifest entries with non-HTTPS script URLs are ignored, and non-HTTPS homepage URLs are omitted from the UI payload.
