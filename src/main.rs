// ai-shell — a local-first, privacy-friendly natural-language Linux command assistant.
// https://github.com/YOUR_USERNAME/ai-shell   MIT License
//
// Intents (auto-detected): generate a command / explain a command (reads real --help) /
// break a command down / answer a concept. Commands run locally; only inference is remote.
// Backends: local Ollama (default), Claude CLI, Codex CLI, or any OpenAI-compatible cloud.
use std::collections::HashMap;
use std::io::{self, Write};
use std::process::Command;

const LOCAL_BASE: &str = "qwen3:14b";       // 本地基础模型(内联system)
const ENDPOINT_DEFAULT: &str = "http://127.0.0.1:11434";

// 意图路由系统提示词(本地云端共用)
const SYS_SHELL: &str = "你是 Linux 命令行学习助手,环境:Arch Linux + zsh。判断用户意图,按格式输出(必须以标记开头):\n【A. 执行某个具体操作】输出两行:\nCMD: <一条shell命令>\nDESC: <一句话说明这条命令做什么>\n规则:遍历文件系统的命令(find/du/grep -r/ls -R)后跟 2>/dev/null;当前目录用 . ;未明确递归时find加-maxdepth 1;在家目录查找时排除噪音目录(-not -path '*/.local/share/Trash/*' -not -path '*/.cache/*' -not -path '*/repos/*');分清文件与目录;破坏性操作选最保守写法。\n【B. 想了解某命令用法/参数】(如\"tar怎么用\")输出:HELP: <命令名>\n【C. 询问\"有哪些命令/工具\"、\"什么是X\"、概念定义等一般知识】——问\"哪些命令/什么工具\"是了解有哪些工具不是查找文件!\"什么是/解释概念\"要直接讲解!输出:ANSWER: <回答;若问工具则列举相关命令及用途>\n【无法判断】输出:CLARIFY: <原因>\n语言:DESC、ANSWER、CLARIFY 的内容用与用户提问相同的语言(用户用英文就用英文)。\n严格:只以 CMD:/HELP:/ANSWER:/CLARIFY: 之一开头;CMD必跟一行DESC;不要思考过程、不要markdown。";

const SYS_SCRIPT: &str = "你是 Bash 脚本生成助手,环境:Arch Linux + bash。根据需求生成完整健壮的脚本。硬性规则:1.首行 #!/usr/bin/env bash,次行 set -euo pipefail;2.所有变量引用加双引号;3.破坏性操作前校验目标非空、路径存在、非根目录,删除类默认用echo打印(dry-run),注释说明取消注释才真删;4.参数不足时打印用法并退出;5.关键步骤加简短注释(用与用户需求相同的语言);6.只输出脚本代码,不要markdown标记、不要解释、不要思考过程。";

const DANGER: &[(&str, &str)] = &[
    ("rm -rf /", "递归删除根目录"), ("rm -rf ~", "递归删除家目录"),
    ("rm -rf *", "递归删除通配"), ("rm -fr /", "递归删除根目录"),
    ("dd of=/dev/", "直接写入块设备(毁盘)"), ("dd if=", "dd 块设备操作"),
    ("mkfs", "格式化文件系统"), ("wipefs", "擦除文件系统签名"),
    ("> /dev/sd", "覆写块设备"), ("> /dev/nvme", "覆写块设备"),
    (":(){", "fork炸弹"), ("chmod -r 777 /", "递归改根目录权限"),
    ("shutdown", "关机"), ("reboot", "重启"), ("poweroff", "关机"),
    ("mkswap", "交换分区操作"), ("fdisk", "分区表操作"),
    ("parted", "分区表操作"), ("gdisk", "分区表操作"),
];

fn load_config() -> HashMap<String, String> {
    let mut cfg = HashMap::new();
    if let Ok(home) = std::env::var("HOME") {
        if let Ok(content) = std::fs::read_to_string(format!("{home}/.config/ai/config")) {
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') { continue; }
                if let Some((k, v)) = line.split_once('=') {
                    cfg.insert(k.trim().to_string(), v.trim().trim_matches('"').to_string());
                }
            }
        }
    }
    if let Ok(v) = std::env::var("AI_ENDPOINT") { cfg.insert("endpoint".into(), v); }
    if let Ok(v) = std::env::var("AI_MODEL") { cfg.insert("model".into(), v); }
    cfg
}

fn b64(input: &str) -> String {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let b = input.as_bytes();
    let mut out = String::new();
    for chunk in b.chunks(3) {
        let n = chunk.len();
        let (b0, b1, b2) = (chunk[0] as u32,
            if n > 1 { chunk[1] as u32 } else { 0 },
            if n > 2 { chunk[2] as u32 } else { 0 });
        let t = (b0 << 16) | (b1 << 8) | b2;
        out.push(T[((t >> 18) & 63) as usize] as char);
        out.push(T[((t >> 12) & 63) as usize] as char);
        out.push(if n > 1 { T[((t >> 6) & 63) as usize] as char } else { '=' });
        out.push(if n > 2 { T[(t & 63) as usize] as char } else { '=' });
    }
    out
}

// 选择后端: 强制本地 > 显式backend > 有cloud_key > 本地
fn pick_backend(cfg: &HashMap<String, String>) -> &'static str {
    if cfg.get("force_local").map_or(false, |v| v == "1") { return "local"; }
    match cfg.get("backend").map(|s| s.as_str()) {
        Some("claude") => return "claude",
        Some("codex") => return "codex",
        Some("cloud") => return "cloud",
        Some("local") => return "local",
        _ => {}
    }
    if cfg.contains_key("cloud_key") { return "cloud"; }
    "local"
}

fn backend_name(cfg: &HashMap<String, String>) -> String {
    match pick_backend(cfg) {
        "claude" => "Claude CLI".into(),
        "codex" => "Codex CLI".into(),
        "cloud" => format!("云端 {}", cfg.get("cloud_model").map(|s| s.as_str()).unwrap_or("?")),
        _ => format!("本地 {LOCAL_BASE}"),
    }
}

// 调用本地 CLI(claude/codex),合并 system+user 为一个 prompt,过滤噪音
fn call_cli(tool: &str, system: &str, user: &str) -> Result<String, String> {
    let full = if system.is_empty() { user.to_string() }
               else { format!("{system}\n\n{user}") };
    let output = match tool {
        "claude" => Command::new("claude").arg("-p").arg(&full).output(),
        "codex" => Command::new("codex").arg("exec").arg(&full).output(),
        _ => return Err("未知CLI".into()),
    }.map_err(|e| format!("调用 {tool} 失败: {e}"))?;
    let raw = String::from_utf8_lossy(&output.stdout);
    if tool == "codex" {
        // 过滤 codex 的 hook/tokens/codex 等噪音行,取实质内容
        let noise = ["hook:", "tokens used", "codex", "user", "thinking"];
        let lines: Vec<&str> = raw.lines()
            .filter(|l| { let t = l.trim();
                !t.is_empty() && !t.chars().all(|c| c.is_ascii_digit() || c == ',')
                && !noise.iter().any(|n| t.eq_ignore_ascii_case(n) || t.starts_with(n)) })
            .collect();
        Ok(lines.join("\n").trim().to_string())
    } else {
        Ok(raw.trim().to_string())
    }
}

fn call(system: &str, user: &str, cfg: &HashMap<String, String>) -> Result<String, String> {
    let backend = pick_backend(cfg);
    let mut out = if backend == "claude" || backend == "codex" {
        call_cli(backend, system, user)?
    } else if backend == "cloud" {
        // 云端:OpenAI 兼容 /chat/completions
        let url = cfg.get("cloud_url").map(|s| s.as_str())
            .unwrap_or("https://api.openai.com/v1/chat/completions");
        let model = cfg.get("cloud_model").map(|s| s.as_str()).unwrap_or("gpt-4o-mini");
        let key = cfg.get("cloud_key").unwrap();
        let mut messages = Vec::new();
        if !system.is_empty() {
            messages.push(serde_json::json!({"role":"system","content":system}));
        }
        messages.push(serde_json::json!({"role":"user","content":user}));
        let body = serde_json::json!({"model": model, "messages": messages, "temperature": 0});
        let resp = ureq::post(url).set("Authorization", &format!("Bearer {key}"))
            .send_json(body).map_err(|e| format!("调用云端失败({url}): {e}"))?;
        let v: serde_json::Value = resp.into_json().map_err(|e| e.to_string())?;
        v["choices"][0]["message"]["content"].as_str().unwrap_or("").trim().to_string()
    } else {
        // 本地:Ollama /api/generate + 内联 system + 关思考
        let endpoint = cfg.get("endpoint").map(|s| s.as_str()).unwrap_or(ENDPOINT_DEFAULT);
        let url = format!("{}/api/generate", endpoint.trim_end_matches('/'));
        let body = serde_json::json!({
            "model": LOCAL_BASE, "system": system, "prompt": user,
            "stream": false, "think": false, "options": {"temperature": 0, "num_ctx": 16384}
        });
        let mut req = ureq::post(&url);
        if let (Some(u), Some(p)) = (cfg.get("user"), cfg.get("password")) {
            req = req.set("Authorization", &format!("Basic {}", b64(&format!("{u}:{p}"))));
        }
        let resp = req.send_json(body).map_err(|e| format!("调用本地模型失败({url}): {e}"))?;
        let v: serde_json::Value = resp.into_json().map_err(|e| e.to_string())?;
        v["response"].as_str().unwrap_or("").trim().to_string()
    };
    if out.starts_with("```") {
        out = out.trim_start_matches("```").to_string();
        if let Some(pos) = out.find('\n') { out = out[pos + 1..].to_string(); }
        out = out.trim_end_matches("```").trim().to_string();
    }
    for p in ["ANSWER:", "EXPLAIN:", "CMD:"] {
        if let Some(s) = out.strip_prefix(p) { out = s.trim().to_string(); }
    }
    Ok(out.trim().to_string())
}

fn fetch_help(cmd: &str) -> Option<String> {
    let name: String = cmd.trim().split_whitespace().next().unwrap_or("")
        .chars().filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_' || *c == '.').collect();
    if name.is_empty() { return None; }
    if let Ok(out) = Command::new(&name).arg("--help").output() {
        let mut t = String::from_utf8_lossy(&out.stdout).to_string();
        t.push_str(&String::from_utf8_lossy(&out.stderr));
        if t.trim().len() > 30 { return Some(t.trim().chars().take(4000).collect()); }
    }
    if let Ok(out) = Command::new("sh").arg("-c")
        .arg(format!("man {name} 2>/dev/null | col -b")).output() {
        let t = String::from_utf8_lossy(&out.stdout);
        if t.trim().len() > 30 { return Some(t.trim().chars().take(4000).collect()); }
    }
    None
}

// 彩色打印:参数flag(-x/--xx)青色,反引号内容绿色
fn print_colored(text: &str) {
    for line in text.lines() {
        let mut out = String::new();
        let mut in_code = false;
        let mut token = String::new();
        let flush = |token: &mut String, out: &mut String| {
            if token.is_empty() { return; }
            if token.starts_with('-') && token.len() > 1
                && token.chars().nth(1).map_or(false, |c| c.is_alphabetic() || c == '-') {
                let end = token.find(|c: char|
                    !(c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '='))
                    .unwrap_or(token.len());
                let (flag, rest) = token.split_at(end);
                out.push_str(&format!("\x1b[1;36m{flag}\x1b[0m{rest}"));
            } else { out.push_str(token); }
            token.clear();
        };
        for c in line.chars() {
            if c == '`' {
                if in_code { out.push_str(&token); token.clear(); out.push_str("\x1b[0m"); }
                else { flush(&mut token, &mut out); out.push_str("\x1b[0;32m"); }
                in_code = !in_code;
            } else if c == ' ' && !in_code {
                flush(&mut token, &mut out); out.push(' ');
            } else { token.push(c); }
        }
        if in_code { out.push_str(&token); out.push_str("\x1b[0m"); }
        else { flush(&mut token, &mut out); }
        println!("{out}");
    }
}

fn prompt(msg: &str) -> String {
    print!("{msg}"); io::stdout().flush().ok();
    let mut s = String::new();
    io::stdin().read_line(&mut s).ok();
    s.trim().to_string()
}

fn scan_danger(cmd: &str) -> Vec<&'static str> {
    let norm: String = cmd.to_lowercase().split_whitespace().collect::<Vec<_>>().join(" ");
    let mut hits = Vec::new();
    for (pat, desc) in DANGER { if norm.contains(pat) { hits.push(*desc); } }
    if (norm.contains("curl") || norm.contains("wget"))
        && (norm.contains("| sh") || norm.contains("| bash")
            || norm.contains("|sh") || norm.contains("|bash")) {
        hits.push("下载即执行(供应链风险)");
    }
    hits
}

// 命令是否已安装
fn is_installed(cmd: &str) -> bool {
    Command::new("which").arg(cmd).output().map(|o| o.status.success()).unwrap_or(false)
}

// 工具端检测"某命令用法"意图:含用法关键词 + 有已安装的命令名 → 该命令名
// 解决:模型不认识冷门命令就 CLARIFY,而不去读真实文档的问题
fn help_target(input: &str) -> Option<String> {
    let kw = ["用法", "怎么用", "怎么使用", "如何使用", "参数", "命令", "说明", "讲解",
              "how to use", "how do i use", "usage", "options", "parameters", "how to"];
    let li = input.to_lowercase(); let input_l = li.as_str();
    if !kw.iter().any(|k| input.contains(k) || input_l.contains(k)) { return None; }
    // 抽取所有 ASCII 命令样式的片段
    let mut runs = Vec::new();
    let mut cur = String::new();
    for c in input.chars() {
        if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' { cur.push(c); }
        else { if cur.len() >= 2 { runs.push(cur.clone()); } cur.clear(); }
    }
    if cur.len() >= 2 { runs.push(cur); }
    // 返回第一个"已安装的命令"
    runs.into_iter().find(|r| r.chars().any(|c| c.is_ascii_alphabetic()) && is_installed(r))
}

// 工具端检测"解释某条命令"意图,提取出待解释的命令(比让模型回显更可靠)
fn explain_target(input: &str) -> Option<String> {
    let t = input.trim();
    let looks_cmd = |s: &str| {
        let s = s.trim();
        // 必须是纯命令行(不含中文),否则交给 help_target 处理
        !s.is_empty() && s.chars().next().map_or(false, |c| c.is_ascii_alphanumeric())
            && !s.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c))
            && (s.contains(' ') || s.contains('-') || s.contains('/') || s.contains('|'))
    };
    for p in ["解释这条命令", "解释一下这条命令", "解释一下命令", "解释命令",
              "解释下这条命令", "解释下", "解释一下", "解释", "拆解这条命令",
              "拆解命令", "拆解", "explain this command", "explain the command", "explain"] {
        if let Some(rest) = t.strip_prefix(p) {
            let rest = rest.trim().trim_start_matches([':', '：']).trim();
            if looks_cmd(rest) { return Some(rest.to_string()); }
        }
    }
    for suf in ["是什么意思", "什么意思", "怎么理解", "是啥意思"] {
        if let Some(head) = t.strip_suffix(suf) {
            let head = head.trim();
            if looks_cmd(head) { return Some(head.to_string()); }
        }
    }
    None
}

// 处理一条输入。dry=只看不执行。context=交互模式下的上下文。
// 读命令真实文档并讲解(HELP 流程,供模型路由和工具端检测共用)
// 输入是否含中日韩文字(判断该用中文还是英文回答)
fn is_cjk(s: &str) -> bool {
    s.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c))
}
// 返回目标语言的强指令(用目标语言本身写,最有效)
fn lang_directive(ask: &str) -> &'static str {
    if is_cjk(ask) { "请只用简体中文回答。" }
    else { "Answer ONLY in English. " }
}

fn do_help(name: &str, ask: &str, cfg: &HashMap<String, String>) {
    eprintln!("📖 读取 {name} 的帮助文档...");
    let sys = "You are a Linux command assistant. No thinking, no reasoning aloud.";
    let d = lang_directive(ask);
    let expl = match fetch_help(name) {
        Some(h) => call(sys, &format!("{d}Below is the real --help of `{name}`. Concisely explain what it does and its common options (one option per line with meaning), then give 1-2 usage examples. Wrap command examples in backticks. Keep it focused.\n\n{h}"), cfg),
        None => call(sys, &format!("{d}Concisely explain the `{name}` command: what it does, common options, and 1-2 examples. Wrap command examples in backticks."), cfg),
    }.unwrap_or_else(|e| format!("(explain failed: {e})"));
    println!(); print_colored(expl.trim());
}

fn process(input: &str, cfg: &HashMap<String, String>, dry: bool, context: &str) {
    // 优先工具端识别"解释命令"意图,直接拆解,不经模型路由(更稳)
    if let Some(target) = explain_target(input) {
        eprintln!("🔍 拆解命令...");
        let help = fetch_help(&target).map(|h| format!("\n(参考首个命令的帮助文档:\n{h}\n)")).unwrap_or_default();
        let expl = call("You are a Linux assistant. No thinking aloud.", &format!("{}Break down and explain each part of this Linux command for a beginner. End with a one-line summary of what the whole command does. Wrap command fragments in backticks.\n\nCommand: {target}{help}", lang_directive(&target)), cfg)
            .unwrap_or_else(|e| format!("(拆解失败: {e})"));
        println!(); print_colored(expl.trim()); return;
    }
    // 工具端识别"某命令用法":含用法词+已安装命令 → 直接读文档,不问模型认不认识(更稳)
    if let Some(name) = help_target(input) {
        do_help(&name, input, cfg); return;
    }

    let query = if context.is_empty() { input.to_string() }
                else { format!("{context}\n当前问题:{input}") };
    eprintln!("🤔 思考中({})...", backend_name(cfg));
    let raw = match call(SYS_SHELL, &query, cfg) {
        Ok(c) => c,
        Err(e) => { eprintln!("{e}"); return; }
    };

    // ANSWER:直接答  HELP:读文档  EXPLAIN:拆命令  CLARIFY:澄清  CMD:生成
    if let Some(a) = raw.strip_prefix("ANSWER:") {
        println!(); print_colored(a.trim()); return;
    }
    if let Some(name) = raw.strip_prefix("HELP:") {
        do_help(name.trim(), input, cfg); return;
    }
    if let Some(c) = raw.strip_prefix("EXPLAIN:") {
        let target = c.trim();
        eprintln!("🔍 拆解命令...");
        let help = fetch_help(target).map(|h| format!("\n(参考首个命令的帮助文档:\n{h}\n)")).unwrap_or_default();
        let expl = call("You are a Linux assistant. No thinking aloud.", &format!("{}Break down and explain each part of this Linux command for a beginner. End with a one-line summary of what the whole command does. Wrap command fragments in backticks.\n\nCommand: {target}{help}", lang_directive(&target)), cfg)
            .unwrap_or_else(|e| format!("(拆解失败: {e})"));
        println!(); print_colored(expl.trim()); return;
    }
    if let Some(c) = raw.strip_prefix("CLARIFY:") {
        println!("\n\x1b[1;33m⚠️  需要澄清:\x1b[0m{}", c.trim()); return;
    }

    // CMD 分支:解析 CMD: 和 DESC:
    let mut cmd = String::new();
    let mut desc = String::new();
    for line in raw.lines() {
        if let Some(s) = line.strip_prefix("CMD:") { cmd = s.trim().to_string(); }
        else if let Some(s) = line.strip_prefix("DESC:") { desc = s.trim().to_string(); }
    }
    // 兜底:模型漏了 CMD: 前缀时,取第一个非 DESC 行作为命令
    if cmd.is_empty() {
        // 若输出像"人话"(含中文句号/很长),当作 ANSWER 讲解,不当命令执行
        let looks_prose = raw.contains('。') || raw.chars().count() > 120
            || raw.lines().count() > 4;
        if looks_prose {
            println!(); print_colored(raw.trim()); return;
        }
        cmd = raw.lines().find(|l| !l.trim().is_empty() && !l.starts_with("DESC:"))
            .unwrap_or("").trim().to_string();
    }

    println!("\n\x1b[1;36m建议命令:\x1b[0m\n  {cmd}");
    if !desc.is_empty() { println!("\x1b[2m  作用:{desc}\x1b[0m"); } // 浅色作用说明
    println!();
    if dry { return; }

    let danger = scan_danger(&cmd);
    let needs_sudo = {
        let n: String = cmd.to_lowercase().split_whitespace().collect::<Vec<_>>().join(" ");
        n == "sudo" || n.starts_with("sudo ") || n.contains("| sudo ") || n.contains("&& sudo ") || n.contains("; sudo ")
    };
    let mut final_cmd = cmd.clone();
    if !danger.is_empty() {
        println!("\x1b[1;41m 危险操作警告 \x1b[0m 匹配到高危模式:");
        for d in &danger { println!("  ⚠️  {d}"); }
        if prompt("\n确认执行?必须输入 yes(其它取消): ") != "yes" { println!("已取消。"); return; }
    } else if needs_sudo {
        println!("\x1b[1;33m⚠️  该命令使用 sudo,将以管理员权限修改系统。\x1b[0m");
        let a = prompt("确认执行?输入 y 或 yes(e 编辑 / 其它取消): ").to_lowercase();
        if a == "e" { let ed = prompt("编辑命令: "); if !ed.is_empty() { final_cmd = ed; } }
        else if a != "y" && a != "yes" { println!("已取消。"); return; }
    } else {
        let a = prompt("回车执行 / e 编辑 / 其它键取消: ");
        if a == "e" { let ed = prompt("编辑命令: "); if !ed.is_empty() { final_cmd = ed; } }
        else if !a.is_empty() { println!("已取消。"); return; }
    }
    println!("\n\x1b[2m$ {final_cmd}\x1b[0m");
    let _ = Command::new("sh").arg("-c").arg(&final_cmd).status();
}

// 生成脚本:role-script 生成 → shellcheck 静态检查 → 展示 → 存文件(不自动执行)
fn gen_script(desc: &str, cfg: &HashMap<String, String>) {
    eprintln!("🛠  生成脚本中...");
    let mut script = match call(SYS_SCRIPT, desc, cfg) {
        Ok(s) => s,
        Err(e) => { eprintln!("{e}"); return; }
    };
    // 去掉可能的 markdown 围栏
    if script.starts_with("```") {
        script = script.trim_start_matches("```bash").trim_start_matches("```sh")
            .trim_start_matches("```").trim_end_matches("```").trim().to_string();
    }
    println!("\n\x1b[1;36m生成的脚本:\x1b[0m");
    println!("\x1b[2m{}\x1b[0m", "─".repeat(50));
    print_colored(&script);
    println!("\x1b[2m{}\x1b[0m", "─".repeat(50));

    // shellcheck 静态检查
    eprintln!("\n🔍 shellcheck 静态检查...");
    let mut tmp = std::env::temp_dir();
    tmp.push(format!("ai_script_{}.sh", std::process::id()));
    let _ = std::fs::write(&tmp, &script);
    match Command::new("shellcheck").arg("-S").arg("warning").arg(&tmp).output() {
        Ok(out) if out.stdout.is_empty() && out.stderr.is_empty() =>
            println!("\x1b[1;32m✓ shellcheck 未发现 warning 级以上问题\x1b[0m"),
        Ok(out) => {
            println!("\x1b[1;33m⚠ shellcheck 提示:\x1b[0m");
            print!("{}", String::from_utf8_lossy(&out.stdout));
        }
        Err(_) => println!("(shellcheck 未安装,跳过检查)"),
    }

    // 安全锁:扫描脚本中的破坏性命令(含注释掉的dry-run,标注状态)
    let mut risks: Vec<String> = Vec::new();
    for (i, line) in script.lines().enumerate() {
        let t = line.trim();
        let commented = t.starts_with('#');
        let body = t.trim_start_matches('#').trim();
        let tag = if commented { "\x1b[2m[dry-run/已注释]\x1b[0m" } else { "\x1b[1;31m[生效]\x1b[0m" };
        let toks: Vec<&str> = body.split(|c: char| c.is_whitespace() || c == '|' || c == ';' || c == '&').collect();
        let mut hit = None;
        for w in ["rm", "rmdir", "mv", "dd", "shred", "mkfs", "truncate"] {
            if toks.iter().any(|tok| *tok == w) { hit = Some(w); break; }
        }
        if let Some(w) = hit {
            risks.push(format!("  {tag} 第{}行(含 {w}): {}", i + 1, body.chars().take(55).collect::<String>()));
        } else if (body.contains(" > ") || body.contains(">/")) && !body.contains(">>") && !body.contains("/dev/null") {
            risks.push(format!("  {tag} 第{}行(含 > 覆盖): {}", i + 1, body.chars().take(55).collect::<String>()));
        }
    }
    if !risks.is_empty() {
        println!("\n\x1b[1;41m 安全锁:脚本含破坏性命令 \x1b[0m");
        for r in &risks { println!("\x1b[1;33m{r}\x1b[0m"); }
        println!("\x1b[2m  提示:删除/移动类操作请务必先看清路径变量、先跑 dry-run 验证。\x1b[0m");
        let ans = prompt("\n确认这些命令没问题?输入 y 或 yes 继续保存(其它键放弃): ").to_lowercase();
        if ans != "y" && ans != "yes" {
            let _ = std::fs::remove_file(&tmp);
            println!("已放弃保存。");
            return;
        }
    }

    // 存文件(不执行)
    let raw_path = prompt("\n保存到文件路径(回车放弃保存): ");
    // 展开 ~ / ~/ 到家目录(shell才会展开,程序需自己处理)
    let path = if raw_path == "~" {
        std::env::var("HOME").unwrap_or(raw_path.clone())
    } else if let Some(rest) = raw_path.strip_prefix("~/") {
        format!("{}/{}", std::env::var("HOME").unwrap_or_default(), rest)
    } else {
        raw_path.clone()
    };
    if raw_path.is_empty() {
        let _ = std::fs::remove_file(&tmp);
        println!("未保存。");
    } else {
        match std::fs::copy(&tmp, &path) {
            Ok(_) => {
                let _ = Command::new("chmod").arg("+x").arg(&path).status();
                let _ = std::fs::remove_file(&tmp);
                println!("\x1b[1;32m✓ 已保存到 {path}(已加执行权限)\x1b[0m");
                println!("\x1b[2m  运行前请先自己过一遍;删除类脚本默认dry-run,确认无误再按注释启用真正删除。\x1b[0m");
            }
            Err(e) => println!("保存失败: {e}"),
        }
    }
}

fn print_help() {
    println!("\x1b[1;36mai\x1b[0m (ai-shell) v{} — Linux 命令行学习助手(本地模型驱动)\n", env!("CARGO_PKG_VERSION"));
    println!("\x1b[1m用法:\x1b[0m");
    println!("  ai \"<自然语言>\"          自动判断意图:生成命令 / 讲解 / 拆解 / 答疑");
    println!("  ai script \"<描述>\"        生成 bash 脚本(shellcheck+安全锁,不自动执行)");
    println!("  ai -i                     交互学习模式(可连续追问,exit 退出)");
    println!("  ai --dry \"<...>\"          只生成命令,不执行");
    println!("  ai -h | --help            显示本帮助\n");
    println!("\x1b[1m后端选择(默认本地,可临时切换):\x1b[0m");
    println!("  --local   本地 Ollama(免费/私密)   --cloud   云端API(需配key)");
    println!("  --claude  本地 Claude CLI(质量高)  --codex   本地 Codex CLI");
    println!("  \x1b[2m默认后端可在配置里设 backend=claude/codex/cloud/local\x1b[0m\n");
    println!("\x1b[1m四种意图(自动识别,无需指定):\x1b[0m");
    println!("  执行操作   \x1b[2m如\x1b[0m ai \"列出当前目录最大的文件\"   \x1b[2m→ 生成命令+作用说明,确认后执行\x1b[0m");
    println!("  讲解命令   \x1b[2m如\x1b[0m ai \"tar 怎么用\"              \x1b[2m→ 读真实 --help 讲解\x1b[0m");
    println!("  拆解命令   \x1b[2m如\x1b[0m ai \"解释这条命令 tar -xzvf a.tgz\"  \x1b[2m→ 逐段拆解含义\x1b[0m");
    println!("  知识问答   \x1b[2m如\x1b[0m ai \"什么是 inode\" / \"有哪些pdf工具\"\n");
    println!("\x1b[1m安全机制:\x1b[0m");
    println!("  \x1b[2m命令三档确认:普通(回车)/ sudo(输y)/ 灾难级rm-rf等(必须输yes)\x1b[0m");
    println!("  \x1b[2m脚本安全锁:含 rm/mv/dd 等破坏命令会红字警告并要确认才保存\x1b[0m\n");
    println!("\x1b[1m配置:\x1b[0m ~/.config/ai/config  \x1b[2m(endpoint= 模型地址,可选 user=/password=)\x1b[0m");
}

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let mut cfg = load_config();

    if args.iter().any(|a| a == "-V" || a == "--version") {
        println!("ai (ai-shell) {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    if args.iter().any(|a| a == "-h" || a == "--help") {
        print_help();
        return;
    }
    // 后端覆盖开关
    if args.iter().any(|a| a == "--local") {
        cfg.insert("force_local".into(), "1".into());
        cfg.remove("backend");
        args.retain(|a| a != "--local");
    }
    for (flag, be) in [("--claude", "claude"), ("--codex", "codex"), ("--cloud", "cloud")] {
        if args.iter().any(|a| a == flag) {
            cfg.insert("backend".into(), be.into());
            cfg.remove("force_local");
            args.retain(|a| a != flag);
        }
    }

    // 子命令: ai script "描述" —— 生成脚本
    if args.first().map(|s| s.as_str()) == Some("script") {
        let desc = args[1..].join(" ");
        if desc.is_empty() { eprintln!("用法: ai script \"脚本要做什么\""); std::process::exit(1); }
        gen_script(&desc, &cfg);
        return;
    }

    let dry = args.iter().any(|a| a == "--dry");
    let repl = args.iter().any(|a| a == "-i" || a == "--repl");
    args.retain(|a| a != "--dry" && a != "-i" && a != "--repl");

    if repl {
        println!("\x1b[1;36mai 交互学习模式\x1b[0m — 输入问题,exit/quit 退出。");
        let mut context = String::new();
        loop {
            let line = prompt("\n\x1b[1;32mai>\x1b[0m ");
            if line.is_empty() { continue; }
            if line == "exit" || line == "quit" || line == "q" { break; }
            process(&line, &cfg, dry, &context);
            // 保留最近一轮作为上下文,支持追问
            context = format!("上一个问题:{line}");
        }
        return;
    }

    if args.is_empty() {
        eprintln!("用法: ai \"描述你想做的事\"   (ai -h 查看完整帮助)");
        std::process::exit(1);
    }
    process(&args.join(" "), &cfg, dry, "");
}
