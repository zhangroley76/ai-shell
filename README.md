<h1 align="center">ai-shell</h1>

<p align="center">
  <b>A local-first, privacy-friendly natural-language Linux command assistant.</b><br>
  Ask in plain English (or Chinese). It generates, explains, and teaches shell commands — with your local LLM, no cloud required.
</p>

<p align="center">
  <img src="assets/demo.gif" alt="ai-shell demo" width="720">
</p>

<p align="center">
  <a href="#install">Install</a> ·
  <a href="#usage">Usage</a> ·
  <a href="#backends">Backends</a> ·
  <a href="#safety">Safety</a> ·
  <a href="#configuration">Config</a> ·
  <a href="#license">License</a>
</p>

<p align="center">English · <a href="README.zh-CN.md">简体中文</a></p>

---

## Why ai-shell?

Most "AI shell" tools send everything to a paid cloud API. **ai-shell runs against a local model by default** (via [Ollama](https://ollama.com)), so your queries never leave your machine — and it's free to run. When you want more power, you can switch to Claude/Codex CLIs or any OpenAI-compatible cloud API with a single flag.

It's not just a command generator — it's a **learning tool**. It reads the *real* `--help`/`man` pages on your machine, so explanations are grounded in your actual system instead of the model's (often outdated) memory.

## Features

- 🧠 **Understands intent automatically** — generate a command, explain a command, break one down, or answer a concept question. No modes to memorize.
- 📖 **Grounded explanations** — reads real `--help` / `man` output on *your* machine, so even obscure or third-party commands are explained correctly.
- 🛡️ **Safety first** — every command is shown before it runs. Three-tier confirmation: normal (Enter), `sudo` (type `y`), catastrophic like `rm -rf /` (must type `yes`).
- 📝 **Script generation** — `ai script "..."` produces a hardened Bash script (`set -euo pipefail`, quoted vars, dry-run for deletes), runs `shellcheck`, and warns on any destructive command before saving. Never auto-executes.
- 🔀 **Multiple backends** — local Ollama, Claude CLI, Codex CLI, or any OpenAI-compatible cloud API. Switch per-call or set a default.
- 🌍 **Follows your language** — ask in English, get English; ask in Chinese, get Chinese.
- 💬 **Interactive mode** — `ai -i` for a learning REPL with follow-up questions.
- 🦀 **Single static binary** — written in Rust, ~750 KB, two tiny dependencies, no runtime needed.

## Install

Works on **Linux and macOS**.

### Option A — one-line installer (recommended)

Detects/installs Rust, builds from source, installs the `ai` binary, and helps you pick a backend:

```bash
curl -fsSL https://raw.githubusercontent.com/zhangroley76/ai-shell/main/install.sh | bash
```

Or from a cloned repo: `git clone https://github.com/zhangroley76/ai-shell.git && cd ai-shell && ./install.sh`

### Option B — manual build

1. **Install Rust** (if you don't have `cargo`):

   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source "$HOME/.cargo/env"      # or just open a new terminal
   ```

   > On macOS you can also `brew install rust` (note: `rust`, **not** `--cask rust`).

2. **Build:**

   ```bash
   git clone https://github.com/zhangroley76/ai-shell.git
   cd ai-shell
   cargo build --release
   ```

3. **Put it on your PATH:**

   ```bash
   # Linux (or macOS with ~/.local/bin on PATH):
   install -Dm755 target/release/ai ~/.local/bin/ai
   # …or system-wide (works everywhere, asks for your password):
   sudo install -m755 target/release/ai /usr/local/bin/ai
   ```

4. **Verify:**

   ```bash
   ai --version      # → ai (ai-shell) 1.0.0
   ```

### Choose a backend

ai-shell needs a model to talk to. Pick one (the installer sets this up for you; to do it manually, create `~/.config/ai/config`):

| Your situation | Backend | Config |
|---|---|---|
| **Have a GPU** | Local Ollama (free, private) | `endpoint = http://127.0.0.1:11434`<br>then `ollama pull qwen3:14b` |
| **Laptop / no GPU, have Claude Code** | Claude CLI | `backend = claude` |
| **Have Codex** | Codex CLI | `backend = codex` |
| **Prefer a cloud API** | OpenAI-compatible | see [Configuration](#configuration) |

```bash
# example: laptop without a GPU, using your existing Claude Code login
mkdir -p ~/.config/ai && echo "backend = claude" > ~/.config/ai/config
```

> No GPU and no Claude/Codex? Install [Ollama](https://ollama.com) (it runs on CPU too, just slower), or point ai-shell at a remote Ollama over your LAN / an SSH tunnel (see [Configuration](#configuration)).

## Usage

```bash
ai "list the largest files in the current directory"   # generate a command → confirm → run
ai "how do I use tar"                                   # explain a command (reads real --help)
ai "explain this command: tar -xzvf backup.tar.gz"      # break a command down piece by piece
ai "what is an inode"                                   # answer a concept question
ai script "delete cache files older than 7 days"        # generate a hardened script (never auto-runs)
ai -i                                                   # interactive learning mode
ai --dry "..."                                          # show the command, don't run it
ai -h                                                   # full help
```

Intent is detected automatically — you never specify a mode.

## Backends

| Backend | Flag | Notes |
|---|---|---|
| **Local (Ollama)** | *default* / `--local` | Free, private, offline. |
| **Claude CLI** | `--claude` | Uses your existing [Claude Code](https://claude.com/claude-code) login. Higher quality, no API key. |
| **Codex CLI** | `--codex` | Uses your existing Codex login. |
| **Cloud API** | `--cloud` | Any OpenAI-compatible endpoint (OpenAI, DeepSeek, OpenRouter, …). Requires a key in config. |

Set a default in the config, or override per call:

```bash
ai --claude "explain epoll vs io_uring"   # borrow a stronger model for a hard question
ai --local  "list pdf files"              # force local even if a default cloud is set
```

The active backend is printed on every call (`Thinking (Claude CLI)...`), so you always know where your query is going.

## Safety

ai-shell is built to **never surprise you**:

- **Commands are never run without confirmation.** You see the exact command (and a one-line description) first.
- **Three-tier confirmation:**
  - normal command → `Enter` to run, `e` to edit, anything else cancels
  - `sudo` command → must type `y`/`yes` (yellow warning)
  - catastrophic pattern (`rm -rf /`, `dd of=/dev/…`, `mkfs`, fork bombs, `curl | sh`, …) → must type the full word `yes`
- **Generated scripts never auto-run.** They're shown, checked with `shellcheck`, scanned for destructive commands (`rm`/`mv`/`dd`/overwrite redirects — flagged line-by-line), and only written to disk after you confirm.

> ⚠️ ai-shell is a helpful assistant, not an infallible one. A local 14B model can be wrong. Always read the command before you run it — the confirmation step is your safety net.

## Configuration

Optional. Copy [`config.example`](config.example) to `~/.config/ai/config` and edit. Everything has sane defaults (local Ollama on `127.0.0.1:11434`).

```ini
endpoint    = http://127.0.0.1:11434     # local Ollama
backend     = local                      # local | claude | codex | cloud

# cloud (OpenAI-compatible), used when backend=cloud
# cloud_url   = https://api.deepseek.com/v1/chat/completions
# cloud_key   = sk-...
# cloud_model = deepseek-chat
```

Environment variables `AI_ENDPOINT` and `AI_MODEL` override the config.

## How it works

```
your question ──▶ intent router (local model)
                    ├─ generate command  ──▶ show + describe ──▶ confirm ──▶ run
                    ├─ explain command   ──▶ read real --help/man ──▶ summarize
                    ├─ break down command──▶ dissect piece by piece
                    └─ answer knowledge  ──▶ concise reply
```

Only the model inference talks to a backend; command execution and doc reading happen locally.

## Contributing

Issues and PRs welcome. The whole tool is a single `src/main.rs` (~400 lines) with two dependencies — easy to read and hack on.

## License

[MIT](LICENSE) © roley zhang

<sub>Built with the help of AI-assisted development. You are free to use, modify, and redistribute this software under the MIT license.</sub>
