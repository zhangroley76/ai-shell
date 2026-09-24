#!/usr/bin/env python3
"""Build an asciinema v2 .cast from real captured ai-shell outputs.
Produces a snappy, scripted demo (English) → feed to `agg` for the GIF.
"""
import json, os, sys

CAP = os.path.join(os.path.dirname(__file__), "captures")
COLS, ROWS = 96, 30
PS1 = "\x1b[38;5;147mai-shell\x1b[0m:\x1b[38;5;114mdemo\x1b[0m$ "

events = []
t = [0.0]
def emit(s, dt=0.0):
    t[0] += dt
    events.append([round(t[0], 3), "o", s])

def typed(cmd, dt=0.045):
    # simulate typing char by char
    for ch in cmd:
        emit(ch, dt)
    emit("\r\n", 0.35)

def cat(fname):
    with open(os.path.join(CAP, fname)) as f:
        return f.read()

def prompt():
    emit(PS1, 0.5)

def thinking(label="local qwen3:14b", dt=0.9):
    emit(f"\x1b[2m🤔 Thinking ({label})...\x1b[0m\r\n", 0.3)
    emit("\r\n", dt)

def block(text, pause=2.2):
    # print a captured block (normalize \n to \r\n for terminal)
    emit(text.replace("\n", "\r\n"), 0.15)
    emit("\r\n", pause)

def clear():
    emit("\x1b[H\x1b[2J\x1b[3J", 0.4)

# ---- intro ----
clear()
emit("\x1b[1;36m ai-shell \x1b[0m\x1b[2m — natural-language Linux command assistant (local LLM)\x1b[0m\r\n\r\n", 1.4)

# 1) generate command + description + run
prompt(); typed('ai "list the 3 largest files here"')
thinking()
block(cat("cmd1.txt").strip(), 0.6)
emit("\x1b[2mEnter to run / e to edit / any key cancels:\x1b[0m\r\n", 0.8)
emit("\x1b[2m$ find . -type f -exec du -s {} + | sort -hr | head -n 3\x1b[0m\r\n", 0.4)
block(cat("cmd1_run.txt").strip(), 2.2)

# 2) explain a command (reads real --help)
prompt(); typed('ai "how do I use the tar command"')
emit("\x1b[2m📖 reading real --help of tar...\x1b[0m\r\n", 0.3); emit("\r\n", 0.9)
block(cat("cmd2.txt").strip(), 2.6)
clear()

# 3) break a command down
prompt(); typed('ai "explain this command: tar -xzvf backup.tar.gz -C /tmp"')
emit("\x1b[2m🔍 breaking down command...\x1b[0m\r\n", 0.3); emit("\r\n", 0.9)
block(cat("cmd4.txt").strip(), 2.6)
clear()

# 4) concept question
prompt(); typed('ai "what is an inode"')
thinking()
block(cat("cmd5.txt").strip(), 2.6)

# 5) sudo guard
prompt(); typed('ai "install htop"')
thinking()
block(cat("cmd6.txt").strip(), 0.4)
emit("\x1b[1;33m⚠  This command uses sudo and will modify the system.\x1b[0m\r\n", 0.3)
emit("\x1b[2mConfirm? type y or yes (e to edit / any key cancels):\x1b[0m\r\n", 2.2)
clear()

# 6) script generation + shellcheck + safety lock
prompt(); typed('ai script "delete cache files older than 7 days"')
emit("\x1b[2m🛠  generating script...\x1b[0m\r\n", 0.3); emit("\r\n", 1.0)
block(cat("cmd3.txt").rstrip(), 0.3)
emit("\r\n\x1b[1;32m✓ shellcheck: no warnings\x1b[0m\r\n", 0.3)
emit("\x1b[1;41m SAFETY LOCK: script contains destructive commands \x1b[0m\r\n", 0.2)
emit("\x1b[1;33m  [dry-run/commented] line 22 (rm): rm -f \"$file\"\x1b[0m\r\n", 0.2)
emit("\x1b[2mConfirm to save? type y/yes: \x1b[0m\r\n", 2.6)

# ---- outro ----
prompt()
emit("\x1b[2m# local · private · safe — github.com/YOUR_USERNAME/ai-shell\x1b[0m\r\n", 2.0)

header = {"version": 2, "width": COLS, "height": ROWS}
out = os.path.join(os.path.dirname(__file__), "demo.cast")
with open(out, "w") as f:
    f.write(json.dumps(header) + "\n")
    for e in events:
        f.write(json.dumps(e, ensure_ascii=False) + "\n")
print(f"wrote {out} · duration {t[0]:.1f}s · {len(events)} events")
