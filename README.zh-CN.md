<h1 align="center">ai-shell</h1>

<p align="center">
  <b>本地优先、注重隐私的自然语言 Linux 命令助手。</b><br>
  用大白话提问(中文或英文),它帮你生成、讲解、教学 shell 命令 —— 由你的本地大模型驱动,无需云端。
</p>

<p align="center">
  <img src="assets/demo.gif" alt="ai-shell 演示" width="720">
</p>

<p align="center">
  <a href="#安装">安装</a> ·
  <a href="#用法">用法</a> ·
  <a href="#后端">后端</a> ·
  <a href="#安全">安全</a> ·
  <a href="#配置">配置</a> ·
  <a href="#许可证">许可证</a>
</p>

<p align="center"><a href="README.md">English</a> · 简体中文</p>

---

## 为什么用 ai-shell?

大多数"AI shell"工具把一切都发给付费云端 API。**ai-shell 默认跑本地模型**(通过 [Ollama](https://ollama.com)),你的查询永不离开本机 —— 而且免费。想要更强时,一个参数就能切到 Claude/Codex CLI 或任意 OpenAI 兼容的云端 API。

它不只是命令生成器,更是一个**学习工具**。它读取你机器上**真实的** `--help`/`man` 文档,所以讲解基于你的实际系统,而不是模型(常常过时)的记忆。

## 功能

- 🧠 **自动识别意图** —— 生成命令、讲解命令、拆解命令、回答概念问题,无需记忆任何模式。
- 📖 **讲解有据可依** —— 读取你机器上真实的 `--help` / `man`,即使冷门或第三方命令也能讲对。
- 🛡️ **安全优先** —— 每条命令执行前都先展示。三档确认:普通(回车)、`sudo`(输 `y`)、灾难级如 `rm -rf /`(必须输完整 `yes`)。
- 📝 **脚本生成** —— `ai script "..."` 生成加固过的 Bash 脚本(`set -euo pipefail`、变量加引号、删除类默认 dry-run),自动跑 `shellcheck`,并在保存前对任何破坏性命令红字警告。绝不自动执行。
- 🔀 **多种后端** —— 本地 Ollama、Claude CLI、Codex CLI,或任意 OpenAI 兼容云端 API。可临时切换或设为默认。
- 🌍 **跟随你的语言** —— 用中文问就中文答,用英文问就英文答。
- 💬 **交互模式** —— `ai -i` 进入学习式 REPL,可连续追问。
- 🦀 **单个静态二进制** —— Rust 编写,约 750 KB,两个极小依赖,无需运行时。

## 安装

### 从源码编译(推荐)

需要 [Rust 工具链](https://rustup.rs)(`cargo`)。

```bash
git clone https://github.com/zhangroley76/ai-shell.git
cd ai-shell
cargo build --release
install -Dm755 target/release/ai ~/.local/bin/ai
```

支持 Linux 和 macOS(Rust 跨平台;讲解读取的是你本机系统的 man 文档)。

### 配置后端

默认后端是**本地 Ollama** 模型。安装 Ollama 并拉取模型:

```bash
# https://ollama.com
ollama pull qwen3:14b        # ai-shell 默认使用的模型
```

> 没有显卡?改用 `--claude` / `--codex` 后端(见 [后端](#后端)),无需本地模型。

## 用法

```bash
ai "列出当前目录最大的文件"          # 生成命令 → 确认 → 执行
ai "tar 命令怎么用"                  # 讲解命令(读真实 --help)
ai "解释这条命令:tar -xzvf a.tgz"   # 逐段拆解命令
ai "什么是 inode"                   # 回答概念问题
ai script "删除7天前的缓存文件"      # 生成加固脚本(绝不自动执行)
ai -i                              # 交互学习模式
ai --dry "..."                     # 只显示命令,不执行
ai -h                              # 完整帮助
```

意图会自动识别 —— 你无需指定模式。

## 后端

| 后端 | 参数 | 说明 |
|---|---|---|
| **本地(Ollama)** | *默认* / `--local` | 免费、私密、离线。 |
| **Claude CLI** | `--claude` | 用你已有的 [Claude Code](https://claude.com/claude-code) 登录态。质量更高,无需 API key。 |
| **Codex CLI** | `--codex` | 用你已有的 Codex 登录态。 |
| **云端 API** | `--cloud` | 任意 OpenAI 兼容端点(OpenAI、DeepSeek、OpenRouter…),需在配置里填 key。 |

在配置里设默认后端,或每次调用临时覆盖:

```bash
ai --claude "解释 epoll 和 io_uring 的区别"   # 难题临时借更强的模型
ai --local  "列出 pdf 文件"                    # 强制本地(即使配了默认云端)
```

每次调用都会显示当前后端(`思考中(Claude CLI)...`),你随时知道查询去了哪里。

## 安全

ai-shell 的设计原则是**绝不让你意外**:

- **命令未经确认绝不执行。** 你先看到确切的命令(和一句作用说明)。
- **三档确认:**
  - 普通命令 → 回车执行,`e` 编辑,其它键取消
  - `sudo` 命令 → 必须输 `y`/`yes`(黄色警告)
  - 灾难级模式(`rm -rf /`、`dd of=/dev/…`、`mkfs`、fork 炸弹、`curl | sh` 等)→ 必须输完整单词 `yes`
- **生成的脚本绝不自动运行。** 先展示,用 `shellcheck` 检查,扫描破坏性命令(`rm`/`mv`/`dd`/覆盖重定向 —— 逐行标出),确认后才写入磁盘。

> ⚠️ ai-shell 是得力助手,但不是万无一失。本地 14B 模型可能出错。执行前请务必看一眼命令 —— 那道确认就是你的安全网。

## 配置

可选。把 [`config.example`](config.example) 复制到 `~/.config/ai/config` 后编辑。一切都有合理默认值(本地 Ollama `127.0.0.1:11434`)。

```ini
endpoint    = http://127.0.0.1:11434     # 本地 Ollama
backend     = local                      # local | claude | codex | cloud

# 云端(OpenAI 兼容),backend=cloud 时使用
# cloud_url   = https://api.deepseek.com/v1/chat/completions
# cloud_key   = sk-...
# cloud_model = deepseek-chat
```

环境变量 `AI_ENDPOINT`、`AI_MODEL` 会覆盖配置文件。

## 工作原理

```
你的问题 ──▶ 意图路由(本地模型)
              ├─ 生成命令   ──▶ 展示+说明 ──▶ 确认 ──▶ 执行
              ├─ 讲解命令   ──▶ 读真实 --help/man ──▶ 总结
              ├─ 拆解命令   ──▶ 逐段解释
              └─ 回答知识   ──▶ 简洁作答
```

只有模型推理会连接后端;命令执行和文档读取都在本地完成。

## 参与贡献

欢迎 Issue 和 PR。整个工具就是一个 `src/main.rs`(约 400 行)加两个依赖,易读易改。

## 许可证

[MIT](LICENSE) © roley zhang

<sub>本项目借助 AI 辅助开发完成。你可以在 MIT 许可证下自由使用、修改和再分发本软件。</sub>
