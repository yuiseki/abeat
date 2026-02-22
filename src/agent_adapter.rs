use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};

pub struct AgentAdapterRequest<'a> {
    pub agent: &'a str,
    pub workspace: &'a Path,
    pub prompt: &'a str,
}

pub struct AgentAdapterOutput {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub exit_code: Option<i32>,
    pub success: bool,
    pub normalized_text: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AgentKind {
    Codex,
    Gemini,
    Claude,
    Copilot,
    Opencode,
}

pub fn run_one_shot(req: AgentAdapterRequest<'_>) -> Result<AgentAdapterOutput> {
    let kind = parse_agent_kind(req.agent)?;
    let spec = build_command_spec(kind, req.workspace, req.prompt)?;

    let mut cmd = Command::new(&spec.program);
    cmd.current_dir(req.workspace);
    for (k, v) in &spec.envs {
        cmd.env(k, v);
    }
    for arg in &spec.args {
        cmd.arg(arg);
    }

    let output = cmd
        .output()
        .with_context(|| format!("failed to run agent CLI `{}`", spec.program))?;

    let normalized_text = extract_normalized_text(kind, &output.stdout, &output.stderr);
    let message = if output.status.success() {
        None
    } else {
        Some(format!("agent CLI exited with status {}", output.status))
    };

    Ok(AgentAdapterOutput {
        stdout: output.stdout,
        stderr: output.stderr,
        exit_code: output.status.code(),
        success: output.status.success(),
        normalized_text,
        message,
    })
}

fn parse_agent_kind(agent: &str) -> Result<AgentKind> {
    match agent.trim().to_ascii_lowercase().as_str() {
        "codex" => Ok(AgentKind::Codex),
        "gemini" => Ok(AgentKind::Gemini),
        "claude" => Ok(AgentKind::Claude),
        "copilot" => Ok(AgentKind::Copilot),
        "opencode" => Ok(AgentKind::Opencode),
        other => bail!(
            "unsupported agent `{other}`; supported: codex, gemini, claude, copilot, opencode"
        ),
    }
}

struct CommandSpec {
    program: String,
    args: Vec<String>,
    envs: Vec<(String, String)>,
}

fn build_command_spec(kind: AgentKind, workspace: &Path, prompt: &str) -> Result<CommandSpec> {
    match kind {
        AgentKind::Codex => Ok(CommandSpec {
            program: env_var_or("ABEAT_CODEX_BIN", "codex"),
            args: vec![
                "exec".to_string(),
                "--json".to_string(),
                "--dangerously-bypass-approvals-and-sandbox".to_string(),
                "--skip-git-repo-check".to_string(),
                "--cd".to_string(),
                workspace.display().to_string(),
                prompt.to_string(),
            ],
            envs: Vec::new(),
        }),
        AgentKind::Gemini => Ok(CommandSpec {
            program: env_var_or("ABEAT_GEMINI_BIN", "gemini"),
            args: vec![
                "--approval-mode".to_string(),
                "yolo".to_string(),
                "--output-format".to_string(),
                "json".to_string(),
                "-p".to_string(),
                prompt.to_string(),
            ],
            envs: Vec::new(),
        }),
        AgentKind::Claude => Ok(CommandSpec {
            program: resolve_claude_bin(),
            args: vec![
                "--dangerously-skip-permissions".to_string(),
                "--print".to_string(),
                "--output-format".to_string(),
                "json".to_string(),
                prompt.to_string(),
            ],
            envs: Vec::new(),
        }),
        AgentKind::Copilot => Ok(CommandSpec {
            program: env_var_or("ABEAT_COPILOT_BIN", "copilot"),
            args: vec![
                "-p".to_string(),
                prompt.to_string(),
                "--allow-all".to_string(),
            ],
            envs: Vec::new(),
        }),
        AgentKind::Opencode => {
            const DEFAULT_OPENCODE_PERMISSION: &str = r#"{"*":"allow"}"#;
            let opencode_bin = env_var_or("ABEAT_OPENCODE_BIN", "opencode");
            let opencode_agent = env_var_or("ABEAT_OPENCODE_AGENT", "build");
            let opencode_permission = env::var("ABEAT_OPENCODE_PERMISSION")
                .ok()
                .or_else(|| env::var("OPENCODE_PERMISSION").ok())
                .filter(|v| !v.trim().is_empty())
                .unwrap_or_else(|| DEFAULT_OPENCODE_PERMISSION.to_string());
            let default_opencode_config_content = serde_json::json!({
                "agent": {
                    opencode_agent.clone(): {
                        "permission": { "*": "allow" }
                    }
                }
            })
            .to_string();
            let opencode_config_content = env::var("ABEAT_OPENCODE_CONFIG_CONTENT")
                .ok()
                .or_else(|| env::var("OPENCODE_CONFIG_CONTENT").ok())
                .filter(|v| !v.trim().is_empty())
                .unwrap_or(default_opencode_config_content);

            Ok(CommandSpec {
                program: opencode_bin,
                args: vec![
                    "run".to_string(),
                    "--agent".to_string(),
                    opencode_agent,
                    "--format".to_string(),
                    "json".to_string(),
                    prompt.to_string(),
                ],
                envs: vec![
                    ("OPENCODE_PERMISSION".to_string(), opencode_permission),
                    (
                        "OPENCODE_CONFIG_CONTENT".to_string(),
                        opencode_config_content,
                    ),
                ],
            })
        }
    }
}

fn env_var_or(name: &str, default_value: &str) -> String {
    env::var(name)
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| default_value.to_string())
}

fn extract_normalized_text(kind: AgentKind, stdout: &[u8], stderr: &[u8]) -> Option<String> {
    match kind {
        AgentKind::Codex => extract_codex_agent_message(stdout)
            .or_else(|| extract_json_response_text(stdout))
            .or_else(|| extract_json_response_text(stderr))
            .or_else(|| normalize_plain_text(stdout))
            .or_else(|| normalize_plain_text(stderr)),
        AgentKind::Copilot => normalize_plain_text(stdout)
            .or_else(|| normalize_plain_text(stderr))
            .or_else(|| extract_json_response_text(stdout))
            .or_else(|| extract_json_response_text(stderr)),
        _ => extract_json_response_text(stdout)
            .or_else(|| extract_json_response_text(stderr))
            .or_else(|| normalize_plain_text(stdout))
            .or_else(|| normalize_plain_text(stderr)),
    }
}

fn extract_codex_agent_message(bytes: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(bytes);
    let mut last: Option<String> = None;
    for line in text.lines() {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if value.get("type").and_then(|v| v.as_str()) != Some("item.completed") {
            continue;
        }
        let Some(item) = value.get("item") else {
            continue;
        };
        if item.get("type").and_then(|v| v.as_str()) != Some("agent_message") {
            continue;
        }
        if let Some(t) = item.get("text").and_then(|v| v.as_str()) {
            let trimmed = t.trim();
            if !trimmed.is_empty() {
                last = Some(trimmed.to_string());
            }
        }
    }
    last
}

fn normalize_plain_text(bytes: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(bytes);
    let trimmed = text.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn extract_json_response_text(bytes: &[u8]) -> Option<String> {
    for keys in [
        &["response"][..],
        &["output_text"][..],
        &["text"][..],
        &["content"][..],
        &["message"][..],
    ] {
        if let Some(text) = extract_string_field_from_json_output(bytes, keys) {
            let t = text.trim();
            if !t.is_empty() {
                return Some(t.to_string());
            }
        }
    }
    None
}

fn extract_string_field_from_json_output(bytes: &[u8], keys: &[&str]) -> Option<String> {
    let text = String::from_utf8_lossy(bytes);
    let trimmed = text.trim();

    if let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) {
        if let Some(id) = find_string_field_recursive(&value, keys) {
            return Some(id);
        }
    }

    for line in text.lines() {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(line) {
            if let Some(id) = find_string_field_recursive(&value, keys) {
                return Some(id);
            }
        }
    }

    if let (Some(start), Some(end)) = (text.find('{'), text.rfind('}')) {
        let candidate = &text[start..=end];
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(candidate) {
            if let Some(id) = find_string_field_recursive(&value, keys) {
                return Some(id);
            }
        }
    }

    None
}

fn find_string_field_recursive(value: &serde_json::Value, keys: &[&str]) -> Option<String> {
    match value {
        serde_json::Value::Object(map) => {
            for key in keys {
                if let Some(id) = map.get(*key).and_then(|v| v.as_str()) {
                    return Some(id.to_string());
                }
            }
            for v in map.values() {
                if let Some(id) = find_string_field_recursive(v, keys) {
                    return Some(id);
                }
            }
            None
        }
        serde_json::Value::Array(items) => {
            for v in items {
                if let Some(id) = find_string_field_recursive(v, keys) {
                    return Some(id);
                }
            }
            None
        }
        _ => None,
    }
}

fn resolve_claude_bin() -> String {
    if let Ok(bin) = env::var("ABEAT_CLAUDE_BIN") {
        if !bin.trim().is_empty() {
            return bin;
        }
    }
    if Command::new("claude")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        return "claude".to_string();
    }
    if let Some(path) = find_asdf_claude_bin() {
        return path;
    }
    "claude".to_string()
}

fn find_asdf_claude_bin() -> Option<String> {
    let home = env::var("HOME").ok()?;
    let installs = PathBuf::from(home)
        .join(".asdf")
        .join("installs")
        .join("nodejs");
    let mut candidates: Vec<(Vec<u32>, String)> = Vec::new();

    for entry in fs::read_dir(installs).ok()? {
        let entry = entry.ok()?;
        let file_type = entry.file_type().ok()?;
        if !file_type.is_dir() {
            continue;
        }
        let version = entry.file_name().to_string_lossy().to_string();
        let bin = entry.path().join("bin").join("claude");
        if !bin.exists() {
            continue;
        }
        let key = version
            .split(|c: char| !c.is_ascii_digit())
            .filter(|s| !s.is_empty())
            .map(|s| s.parse::<u32>().unwrap_or(0))
            .collect::<Vec<_>>();
        candidates.push((key, bin.to_string_lossy().to_string()));
    }

    if candidates.is_empty() {
        return None;
    }
    candidates.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    candidates.pop().map(|(_, path)| path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_agent_kind_names() {
        assert!(matches!(
            parse_agent_kind("codex").unwrap(),
            AgentKind::Codex
        ));
        assert!(matches!(
            parse_agent_kind("Gemini").unwrap(),
            AgentKind::Gemini
        ));
        assert!(parse_agent_kind("unknown").is_err());
    }

    #[test]
    fn extracts_response_from_json_object_and_jsonl() {
        let obj = br#"{"response":"HEARTBEAT_OK"}"#;
        assert_eq!(
            extract_json_response_text(obj).as_deref(),
            Some("HEARTBEAT_OK")
        );

        let jsonl = br#"{"type":"step_start","sessionID":"x"}
{"type":"message","text":"summary line"}"#;
        assert_eq!(
            extract_json_response_text(jsonl).as_deref(),
            Some("summary line")
        );
    }

    #[test]
    fn extracts_codex_agent_message_from_jsonl_events() {
        let jsonl = br#"{"type":"item.completed","item":{"type":"reasoning","text":"plan"}}
{"type":"item.completed","item":{"type":"agent_message","text":"first"}}
{"type":"item.completed","item":{"type":"agent_message","text":"final answer"}}"#;
        assert_eq!(
            extract_codex_agent_message(jsonl).as_deref(),
            Some("final answer")
        );
    }

    #[test]
    fn builds_opencode_defaults_with_allow_permission() {
        let cwd = Path::new("/tmp");
        let spec = build_command_spec(AgentKind::Opencode, cwd, "prompt").unwrap();
        assert_eq!(spec.args[0], "run");
        assert!(spec.args.windows(2).any(|w| w == ["--agent", "build"]));
        let perm = spec
            .envs
            .iter()
            .find(|(k, _)| k == "OPENCODE_PERMISSION")
            .map(|(_, v)| v.clone())
            .unwrap();
        let cfg = spec
            .envs
            .iter()
            .find(|(k, _)| k == "OPENCODE_CONFIG_CONTENT")
            .map(|(_, v)| v.clone())
            .unwrap();
        assert_eq!(perm, r#"{"*":"allow"}"#);
        assert!(cfg.contains(r#""build""#));
        assert!(cfg.contains(r#""*":"allow""#));
    }
}
