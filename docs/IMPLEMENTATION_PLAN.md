# Implementation Plan

AgentKey is being built from the local desktop bridge worktree into a desktop bridge for Codex and Claude Code API-key workflows.

## Phase 1: Project Identity

- Rename visible product strings to AgentKey.
- Replace repository metadata, package metadata, window titles, app identifiers, updater labels, and user-facing docs.
- Remove or rewrite copy that makes the codebase look like a direct Codex-branded project.
- Keep technical compatibility names only where changing them would break runtime behavior.

## Phase 2: Local Helper Security

- Require a random session token for privileged helper requests.
- Inject the token only into trusted desktop-client renderer contexts.
- Reject missing or invalid helper tokens with 401 responses.
- Restrict CORS to trusted desktop-client origins.
- Remove wildcard CORS from privileged helper endpoints.
- Keep protocol proxy compatibility only where the desktop client requires it.

## Phase 3: API Key Provider UX

- Provide a clear settings flow for API key configuration.
- Support OpenAI-compatible providers through base URL, API key, and model fields.
- Add Claude Code provider configuration through Anthropic-compatible API key settings.
- Validate provider settings before launch.
- Redact API keys in diagnostics, logs, and UI error messages.

## Phase 4: Claude Code Support

- Add a Claude Code launch profile beside the existing Codex profile.
- Start Claude Code with `ANTHROPIC_API_KEY` or `ANTHROPIC_AUTH_TOKEN`.
- Support `ANTHROPIC_BASE_URL`, `ANTHROPIC_MODEL`, `ANTHROPIC_SMALL_FAST_MODEL`, and extra environment variables.
- Open Claude Code in a visible terminal on Windows so interactive workflows remain usable.
- Continue identifying desktop-client injection and local protocol requirements.
- Share common helper, security, provider, and diagnostics code.
- Keep product-specific bridge code isolated behind provider/client adapters.

## Phase 5: Updater and Script Hardening

- Verify update artifacts before execution through signatures or trusted hashes.
- Enforce script integrity checks before enabling installed scripts.
- Disable newly installed community scripts by default unless explicitly trusted.
- Harden Content Security Policy for the desktop shell.

## Phase 6: Verification

- Run JavaScript syntax checks for injected scripts.
- Run Rust formatting and tests when Rust tooling is available.
- Run frontend type checks when TypeScript tooling is available.
- Add targeted tests for token rejection, CORS handling, secret redaction, and provider validation.
- Verify desktop launch flows manually for Codex and Claude Code.
