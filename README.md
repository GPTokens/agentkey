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

## Supported Provider Settings

- OpenAI-compatible: `base_url`, API key, model, Responses or Chat Completions protocol.
- Claude Code: `ANTHROPIC_API_KEY` or `ANTHROPIC_AUTH_TOKEN`, `ANTHROPIC_BASE_URL`, `ANTHROPIC_MODEL`, `ANTHROPIC_SMALL_FAST_MODEL`.
- Local development: loopback HTTP is allowed for `localhost`, `127.0.0.1`, and `::1`; remote providers must use HTTPS.

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
