# AgentKey

Use Codex and Claude Code desktop workflows with API keys, without account login.

AgentKey is a local desktop bridge for AI coding agents. It lets users configure third-party provider API keys, route desktop-client traffic through a protected loopback helper, and keep account login out of the runtime path.

## What It Does

- Runs Codex desktop workflows with OpenAI-compatible API providers.
- Launches Claude Code with Anthropic-compatible API key or auth-token settings.
- Supports custom base URLs, model names, and provider profiles.
- Preserves desktop-client features through local bridge injection.
- Protects privileged helper routes with a random session token and restricted CORS.
- Redacts API keys, bearer tokens, and auth values from diagnostics.

## Quick Start

1. Download the latest installer from [GitHub Releases](https://github.com/GPTokens/agentkey/releases).
2. Open AgentKey Manager.
3. Keep or choose the Pure API profile.
4. Enter your provider Base URL, API key, and model.
5. Save the profile and launch AgentKey.
6. For Claude Code, enable Claude Code settings, enter the command, Base URL, API key or auth token, and model, then launch it from AgentKey Manager.

Pure API mode is the default no-login path. Official-account login is optional and only used when a user deliberately selects an official provider profile.

## Supported Provider Settings

- OpenAI-compatible: `base_url`, API key, model, Responses or Chat Completions protocol.
- Claude Code: `ANTHROPIC_API_KEY` or `ANTHROPIC_AUTH_TOKEN`, `ANTHROPIC_BASE_URL`, `ANTHROPIC_MODEL`, `ANTHROPIC_SMALL_FAST_MODEL`.
- Local development: loopback HTTP is allowed for `localhost`, `127.0.0.1`, and `::1`; remote providers must use HTTPS.

## Release Builds

Expected release assets:

- Windows: `AgentKey-*-windows-x64-setup.exe`.
- macOS Intel: `AgentKey-*-macos-x64.dmg`.
- macOS Apple Silicon: `AgentKey-*-macos-arm64.dmg`.

Update assets must include SHA-256 metadata and pass verification before execution.

## Development

```bash
cd apps/agentkey-manager
npm install
npm run check
npm run vite:build

cd ../..
cargo test --workspace
```

## Security

AgentKey treats the local helper as privileged because it can proxy requests that use user-provided API keys.

Required controls:

- Bind helper services to loopback only.
- Generate a random session token at launch.
- Inject the token only into trusted desktop-client renderer contexts.
- Reject helper requests that do not present the token.
- Restrict CORS to trusted desktop-client origins.
- Redact API keys and bearer tokens from diagnostics.
- Verify downloaded update assets before execution.
- Validate script integrity before enabling installed scripts.

See [docs/SECURITY_MODEL.md](docs/SECURITY_MODEL.md) for the full model.

## Repository

```text
https://github.com/GPTokens/agentkey
```

## Keywords

Codex, Claude Code, API key, desktop client, AI agent, no-login, Tauri, Rust.
