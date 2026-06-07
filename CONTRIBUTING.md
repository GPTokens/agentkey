# Contributing to AgentKey

AgentKey is a local desktop bridge for Codex and Claude Code API-key workflows.

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

- Do not commit real API keys, bearer tokens, cookies, or local auth files.
- Redact secrets in logs, diagnostics, screenshots, and issue reports.
- Treat local helper routes as privileged because they can proxy API-key traffic.
- Keep helper authentication, CORS restrictions, and updater verification enabled.

## Repository Layout

```text
apps/
  agentkey-launcher/
  agentkey-manager/
crates/
  agentkey-core/
  agentkey-data/
assets/
  inject/
scripts/
  installer/
```
