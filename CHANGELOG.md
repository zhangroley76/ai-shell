# Changelog

## v1.0.0 — 2026-09-24

First public release.

- Auto-detected intents: generate a command, explain a command (reads real `--help`/`man`), break a command down, answer a concept question.
- Three-tier command confirmation (normal / `sudo` / catastrophic) — commands never run without you seeing them first.
- `ai script` — generates hardened Bash (`set -euo pipefail`, quoted vars, dry-run deletes), runs `shellcheck`, and warns on destructive commands before saving. Never auto-executes.
- Four backends: local Ollama (default), Claude CLI, Codex CLI, and OpenAI-compatible cloud APIs. Switch per-call or set a default.
- Follows the user's language (English / Chinese).
- Interactive learning mode (`ai -i`), `--dry` preview, config file, `--version`.
- Single ~750 KB Rust binary, two dependencies.
