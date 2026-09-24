# vhs script — generates the ai-shell demo GIF (English).
# Run from repo root:  vhs assets/demo.tape
Output assets/demo.gif

Set Shell "bash"
Set FontSize 19
Set Width 1180
Set Height 940
Set Theme "Catppuccin Mocha"
Set TypingSpeed 55ms
Set Padding 20

Hide
Type `alias ai="$PWD/assets/demo-ai"` Enter
Type `cd assets/demo-sandbox` Enter
Type `export PS1='\[\e[38;5;147m\]ai-shell\[\e[0m\]:\[\e[38;5;114m\]demo\[\e[0m\]$ '` Enter
Type `clear` Enter
Show

Sleep 700ms

# 1) Generate a command (with description + run)
Type `ai "list the 3 largest files here"` Sleep 400ms Enter
Sleep 2600ms

# 2) Explain a command (reads real --help)
Type `ai "how do I use the tar command"` Sleep 400ms Enter
Sleep 2800ms
Type `clear` Enter

# 3) Break a command down piece by piece
Type `ai "explain this command: tar -xzvf backup.tar.gz -C /tmp"` Sleep 400ms Enter
Sleep 2800ms
Type `clear` Enter

# 4) Answer a concept question
Type `ai "what is an inode"` Sleep 400ms Enter
Sleep 2600ms

# 5) sudo command -> confirmation guard
Type `ai "install htop"` Sleep 400ms Enter
Sleep 2400ms
Type `clear` Enter

# 6) Generate a hardened script (shellcheck + safety lock)
Type `ai script "delete cache files older than 7 days"` Sleep 400ms Enter
Sleep 3200ms

Sleep 1500ms
