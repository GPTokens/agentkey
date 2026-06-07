# AgentKey

Use Codex and Claude Code desktop clients with API keys, without account login.

AgentKey is a local desktop bridge for AI coding agents. It lets users configure provider API keys and run desktop-client workflows through a local helper, while keeping account login out of the runtime path.

## Goals

- Use Codex desktop workflows through API keys.
- Add Claude Code support through the same desktop bridge model.
- Support OpenAI-compatible providers with custom base URLs.
- Keep setup simple for users who already have provider API keys.
- Protect the local helper with session-scoped authentication.
- Avoid exposing API keys to arbitrary web pages or untrusted local clients.

## Current Scope

AgentKey is being prepared from the local desktop bridge worktree. The first development track focuses on:

- Rebranding the public project identity to AgentKey.
- Hardening local helper authentication and CORS.
- Improving API key provider setup.
- Adding a Claude Code profile beside the existing Codex profile.
- Removing old project-specific release and sponsor metadata.

## Security Model

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

## Repository

```text
https://github.com/GPTokens/agentkey
```

## Keywords

Codex, Claude Code, API key, desktop client, AI agent, no-login, Tauri, Rust.
