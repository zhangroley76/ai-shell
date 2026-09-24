#!/usr/bin/env bash
# ai-shell installer — builds from source and installs the `ai` binary.
# Works on Linux and macOS. Safe to re-run.
#
#   curl -fsSL https://raw.githubusercontent.com/zhangroley76/ai-shell/main/install.sh | bash
# or, from a cloned repo:
#   ./install.sh
set -euo pipefail

REPO="https://github.com/zhangroley76/ai-shell.git"
BLUE='\033[1;34m'; GRN='\033[1;32m'; YEL='\033[1;33m'; RED='\033[1;31m'; DIM='\033[2m'; RST='\033[0m'
info(){ printf "${BLUE}==>${RST} %s\n" "$1"; }
ok(){ printf "${GRN}✓${RST} %s\n" "$1"; }
warn(){ printf "${YEL}!${RST} %s\n" "$1"; }
err(){ printf "${RED}✗${RST} %s\n" "$1" >&2; }

# --- 1. Ensure Rust/cargo ---
if ! command -v cargo >/dev/null 2>&1; then
  warn "Rust (cargo) not found."
  read -r -p "Install Rust now via rustup? [Y/n] " a </dev/tty || a="y"
  case "${a:-y}" in
    n|N) err "Rust is required. Install it from https://rustup.rs then re-run."; exit 1;;
    *) info "Installing Rust via rustup..."
       curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
       # shellcheck disable=SC1091
       . "$HOME/.cargo/env";;
  esac
fi
command -v cargo >/dev/null 2>&1 || { err "cargo still not on PATH. Open a new terminal and re-run."; exit 1; }
ok "cargo $(cargo --version | awk '{print $2}')"

# --- 2. Get the source (clone if not already inside the repo) ---
if [ -f "Cargo.toml" ] && grep -q 'name = "ai-shell"' Cargo.toml 2>/dev/null; then
  SRC="$(pwd)"
else
  need_git() { command -v git >/dev/null 2>&1 || { err "git is required to fetch the source."; exit 1; }; }
  need_git
  SRC="${TMPDIR:-/tmp}/ai-shell-src"
  info "Cloning $REPO ..."
  rm -rf "$SRC"; git clone --depth 1 "$REPO" "$SRC"
fi

# --- 3. Build ---
info "Building (release)..."
( cd "$SRC" && cargo build --release )
BIN="$SRC/target/release/ai"
[ -x "$BIN" ] || { err "Build did not produce $BIN"; exit 1; }
ok "Built $BIN"

# --- 4. Install to PATH ---
DEST=""
for d in "$HOME/.local/bin" "/usr/local/bin"; do
  if [ -d "$d" ] && [ -w "$d" ]; then DEST="$d"; break; fi
done
if [ -z "$DEST" ]; then
  info "Installing to /usr/local/bin (may ask for your password)..."
  sudo install -m 755 "$BIN" /usr/local/bin/ai
  DEST="/usr/local/bin"
else
  install -m 755 "$BIN" "$DEST/ai"
fi
ok "Installed to $DEST/ai"

# Warn if DEST not on PATH
case ":$PATH:" in
  *":$DEST:"*) : ;;
  *) warn "$DEST is not on your PATH. Add this to your shell rc:"
     printf "    ${DIM}export PATH=\"%s:\$PATH\"${RST}\n" "$DEST";;
esac

# Warn if another `ai` earlier on PATH would shadow the one we just installed
RESOLVED="$(command -v ai || true)"
if [ -n "$RESOLVED" ] && [ "$RESOLVED" != "$DEST/ai" ]; then
  warn "Another 'ai' is earlier on your PATH and will be used instead:"
  printf "    ${DIM}%s${RST}\n" "$RESOLVED"
  printf "  Remove it, or install over it:  ${DIM}install -m755 \"%s\" \"%s\"${RST}\n" "$BIN" "$RESOLVED"
fi

# --- 5. Pick a backend ---
CFG_DIR="$HOME/.config/ai"; CFG="$CFG_DIR/config"
if [ ! -f "$CFG" ]; then
  mkdir -p "$CFG_DIR"
  echo
  info "Choose a backend:"
  echo "  1) Local Ollama   (free & private; needs a GPU + 'ollama pull qwen3:14b')"
  echo "  2) Claude CLI     (uses your existing Claude Code login; great on laptops)"
  echo "  3) Codex CLI      (uses your existing Codex login)"
  echo "  4) Skip           (configure later in $CFG)"
  read -r -p "Selection [1-4]: " sel </dev/tty || sel="1"
  case "${sel:-1}" in
    2) echo "backend = claude" > "$CFG"; ok "Configured Claude CLI backend";;
    3) echo "backend = codex"  > "$CFG"; ok "Configured Codex CLI backend";;
    4) warn "Skipped. Copy config.example to $CFG when ready.";;
    *) printf "endpoint = http://127.0.0.1:11434\nbackend = local\n" > "$CFG"
       ok "Configured local Ollama backend"
       command -v ollama >/dev/null 2>&1 || warn "Ollama not found — install it from https://ollama.com and run: ollama pull qwen3:14b";;
  esac
fi

echo
ok "Done. Try:  ai \"how do I use the find command\""
