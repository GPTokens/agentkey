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
- Verify downloaded update assets before execution.
- Validate script integrity before enabling installed scripts.

## Local Helper Token

The helper token is session-scoped. It is generated at launch and passed to the trusted renderer injection. Browser requests to helper routes must include the token through a dedicated header or equivalent authenticated channel.

## CORS

CORS is not an authentication mechanism. It is a browser access boundary. AgentKey uses CORS restrictions together with the session token so unrelated web pages cannot call privileged local helper endpoints.

## Updater

Updater code must verify release artifacts before execution. Acceptable verification options include detached signatures, trusted checksums, or platform-native signing where available.

## Diagnostics

Diagnostics may include environment details, configuration paths, and provider settings. Diagnostics must redact secrets before sending or saving reports.
