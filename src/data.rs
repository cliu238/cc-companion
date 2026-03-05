use serde_json::Value;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct Project {
    pub name: String,
    pub project_path: String,
    pub claude_dir: PathBuf,
    pub session_count: usize,
    pub has_claude_md: bool,
}

#[derive(Clone)]
pub struct SessionEntry {
    pub session_id: String,
    pub first_prompt: String,
    pub message_count: usize,
    pub modified: String,
    pub git_branch: String,
    pub jsonl_path: PathBuf,
}

pub struct ConversationMessage {
    pub role: String,
    pub text: String,
}

pub fn load_projects() -> Vec<Project> {
    let claude_dir = match dirs::home_dir() {
        Some(h) => h.join(".claude").join("projects"),
        None => return Vec::new(),
    };

    let mut projects = Vec::new();
    let entries = match fs::read_dir(&claude_dir) {
        Ok(e) => e,
        Err(_) => return projects,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let dir_name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };

        // Count JSONL files as sessions
        let jsonl_count = fs::read_dir(&path)
            .map(|rd| {
                rd.flatten()
                    .filter(|e| {
                        e.path()
                            .extension()
                            .map(|ext| ext == "jsonl")
                            .unwrap_or(false)
                    })
                    .count()
            })
            .unwrap_or(0);

        if jsonl_count == 0 {
            continue;
        }

        // Try to get projectPath from sessions-index.json or first JSONL
        let project_path = get_project_path(&path);

        let display_name = if !project_path.is_empty() {
            project_path.clone()
        } else {
            decode_dir_name(&dir_name)
        };

        let has_claude_md = if !project_path.is_empty() {
            Path::new(&project_path).join("CLAUDE.md").exists()
        } else {
            // Try decoding dir name as path
            let decoded = decode_dir_name(&dir_name);
            Path::new(&decoded).join("CLAUDE.md").exists()
        };

        projects.push(Project {
            name: display_name,
            project_path,
            claude_dir: path,
            session_count: jsonl_count,
            has_claude_md,
        });
    }

    projects.sort_by(|a, b| b.session_count.cmp(&a.session_count));
    projects
}

fn get_project_path(claude_dir: &Path) -> String {
    // Try sessions-index.json first
    let index_path = claude_dir.join("sessions-index.json");
    if let Ok(data) = fs::read_to_string(&index_path) {
        if let Ok(val) = serde_json::from_str::<Value>(&data) {
            if let Some(path) = val
                .get("entries")
                .and_then(|e| e.as_array())
                .and_then(|a| a.first())
                .and_then(|e| e.get("projectPath"))
                .and_then(|p| p.as_str())
            {
                if !path.is_empty() {
                    return path.to_string();
                }
            }
        }
    }

    // Fall back: read cwd from first JSONL file
    if let Ok(rd) = fs::read_dir(claude_dir) {
        for entry in rd.flatten() {
            let p = entry.path();
            if p.extension().map(|e| e == "jsonl").unwrap_or(false) {
                if let Some(cwd) = read_cwd_from_jsonl(&p) {
                    return cwd;
                }
            }
        }
    }
    String::new()
}

fn read_cwd_from_jsonl(path: &Path) -> Option<String> {
    let file = fs::File::open(path).ok()?;
    let reader = BufReader::new(file);
    for line in reader.lines().take(20) {
        let line = line.ok()?;
        let val: Value = serde_json::from_str(&line).ok()?;
        if let Some(cwd) = val.get("cwd").and_then(|c| c.as_str()) {
            if !cwd.is_empty() {
                return Some(cwd.to_string());
            }
        }
    }
    None
}

fn decode_dir_name(name: &str) -> String {
    // Encoded as: leading slash replaced with -, then all / replaced with -
    // e.g. "-Users-ericliu-projects5-foo" -> "/Users/ericliu/projects5/foo"
    if name.starts_with('-') {
        format!("/{}", name[1..].replace('-', "/"))
    } else {
        name.replace('-', "/")
    }
}

pub fn load_sessions(claude_dir: &Path) -> Vec<SessionEntry> {
    let mut sessions = Vec::new();

    let entries = match fs::read_dir(claude_dir) {
        Ok(e) => e,
        Err(_) => return sessions,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map(|e| e == "jsonl").unwrap_or(false) {
            if let Some(session) = scan_session_jsonl(&path) {
                sessions.push(session);
            }
        }
    }

    sessions.sort_by(|a, b| b.modified.cmp(&a.modified));
    sessions
}

fn scan_session_jsonl(path: &Path) -> Option<SessionEntry> {
    let session_id = path.file_stem()?.to_str()?.to_string();

    let file = fs::File::open(path).ok()?;
    let metadata = file.metadata().ok()?;
    let modified = format_system_time(metadata.modified().ok()?);

    let reader = BufReader::new(file);
    let mut first_prompt = String::new();
    let mut git_branch = String::new();
    let mut message_count: usize = 0;

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };
        if line.trim().is_empty() {
            continue;
        }
        let val: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        if git_branch.is_empty() {
            if let Some(b) = val.get("gitBranch").and_then(|b| b.as_str()) {
                if !b.is_empty() {
                    git_branch = b.to_string();
                }
            }
        }

        let msg_type = val.get("type").and_then(|t| t.as_str()).unwrap_or("");
        if msg_type == "user" || msg_type == "assistant" {
            message_count += 1;
            if first_prompt.is_empty() && msg_type == "user" {
                if let Some(msg) = val.get("message") {
                    first_prompt = extract_text_preview(msg.get("content"));
                }
            }
        }
    }

    if message_count == 0 {
        return None;
    }

    Some(SessionEntry {
        session_id,
        first_prompt,
        message_count,
        modified,
        git_branch,
        jsonl_path: path.to_path_buf(),
    })
}

fn extract_text_preview(content: Option<&Value>) -> String {
    match content {
        Some(Value::String(s)) => s.chars().take(200).collect(),
        Some(Value::Array(blocks)) => {
            for block in blocks {
                if block.get("type").and_then(|t| t.as_str()) == Some("text") {
                    if let Some(t) = block.get("text").and_then(|t| t.as_str()) {
                        return t.chars().take(200).collect();
                    }
                }
            }
            String::new()
        }
        _ => String::new(),
    }
}

fn format_system_time(time: std::time::SystemTime) -> String {
    let duration = time
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs() as i64;
    let dt = chrono::DateTime::from_timestamp(secs, 0).unwrap_or_default();
    dt.format("%Y-%m-%d %H:%M").to_string()
}

pub fn load_conversation(jsonl_path: &Path) -> Vec<ConversationMessage> {
    let data = match fs::read_to_string(jsonl_path) {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };

    let mut messages = Vec::new();
    for line in data.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let val: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let msg_type = val.get("type").and_then(|t| t.as_str()).unwrap_or("");
        if msg_type != "user" && msg_type != "assistant" {
            continue;
        }

        let message = match val.get("message") {
            Some(m) => m,
            None => continue,
        };
        let role = message
            .get("role")
            .and_then(|r| r.as_str())
            .unwrap_or(msg_type)
            .to_string();
        let content = match message.get("content") {
            Some(c) => c,
            None => continue,
        };
        let text = extract_text(content);
        if !text.is_empty() {
            messages.push(ConversationMessage { role, text });
        }
    }
    messages
}

fn extract_text(content: &Value) -> String {
    match content {
        Value::String(s) => s.clone(),
        Value::Array(blocks) => {
            let mut parts = Vec::new();
            for block in blocks {
                let block_type = block.get("type").and_then(|t| t.as_str()).unwrap_or("");
                match block_type {
                    "text" => {
                        if let Some(t) = block.get("text").and_then(|t| t.as_str()) {
                            parts.push(t.to_string());
                        }
                    }
                    "tool_use" => {
                        let name = block
                            .get("name")
                            .and_then(|n| n.as_str())
                            .unwrap_or("unknown");
                        parts.push(format!("[tool: {}]", name));
                    }
                    "tool_result" => {
                        parts.push("[tool result]".to_string());
                    }
                    "thinking" => {} // skip
                    _ => {}
                }
            }
            parts.join("\n")
        }
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_decode_dir_name_absolute() {
        assert_eq!(decode_dir_name("-Users-eric-foo"), "/Users/eric/foo");
    }

    #[test]
    fn test_decode_dir_name_relative() {
        assert_eq!(decode_dir_name("some-path"), "some/path");
    }

    #[test]
    fn test_extract_text_string() {
        let val = serde_json::json!("hello world");
        assert_eq!(extract_text(&val), "hello world");
    }

    #[test]
    fn test_extract_text_blocks() {
        let val = serde_json::json!([
            {"type": "thinking", "thinking": "hmm"},
            {"type": "text", "text": "visible"},
            {"type": "tool_use", "name": "Bash"},
            {"type": "tool_result", "content": "ok"}
        ]);
        let result = extract_text(&val);
        assert!(result.contains("visible"));
        assert!(result.contains("[tool: Bash]"));
        assert!(result.contains("[tool result]"));
        assert!(!result.contains("hmm"));
    }

    #[test]
    fn test_extract_text_empty() {
        assert_eq!(extract_text(&serde_json::json!(null)), "");
        assert_eq!(extract_text(&serde_json::json!(42)), "");
    }

    #[test]
    fn test_extract_text_preview_truncates() {
        let long = "a".repeat(300);
        let val = serde_json::json!(long);
        let result = extract_text_preview(Some(&val));
        assert_eq!(result.len(), 200);
    }

    #[test]
    fn test_extract_text_preview_none() {
        assert_eq!(extract_text_preview(None), "");
    }

    #[test]
    fn test_format_system_time() {
        use std::time::{Duration, UNIX_EPOCH};
        // 2024-01-15 12:00:00 UTC = 1705320000
        let time = UNIX_EPOCH + Duration::from_secs(1705320000);
        let result = format_system_time(time);
        assert_eq!(result, "2024-01-15 12:00");
    }

    #[test]
    fn test_load_conversation() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.jsonl");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, r#"{{"type":"user","message":{{"role":"user","content":"hi"}}}}"#).unwrap();
        writeln!(f, r#"{{"type":"assistant","message":{{"role":"assistant","content":"hello"}}}}"#).unwrap();
        writeln!(f, r#"{{"type":"system","message":{{"role":"system","content":"ignore"}}}}"#).unwrap();

        let msgs = load_conversation(&path);
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].role, "user");
        assert_eq!(msgs[0].text, "hi");
        assert_eq!(msgs[1].role, "assistant");
        assert_eq!(msgs[1].text, "hello");
    }

    #[test]
    fn test_scan_session_jsonl() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("abc-123.jsonl");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, r#"{{"type":"user","gitBranch":"main","message":{{"role":"user","content":"do stuff"}}}}"#).unwrap();
        writeln!(f, r#"{{"type":"assistant","message":{{"role":"assistant","content":"done"}}}}"#).unwrap();

        let entry = scan_session_jsonl(&path).unwrap();
        assert_eq!(entry.session_id, "abc-123");
        assert_eq!(entry.message_count, 2);
        assert_eq!(entry.git_branch, "main");
        assert_eq!(entry.first_prompt, "do stuff");
    }
}

pub fn load_claude_md(project: &Project) -> Option<String> {
    // Try project_path first
    if !project.project_path.is_empty() {
        let path = Path::new(&project.project_path).join("CLAUDE.md");
        if let Ok(content) = fs::read_to_string(&path) {
            return Some(content);
        }
    }
    // Try decoded dir name
    if let Some(dir_name) = project.claude_dir.file_name().and_then(|n| n.to_str()) {
        let decoded = decode_dir_name(dir_name);
        let path = Path::new(&decoded).join("CLAUDE.md");
        if let Ok(content) = fs::read_to_string(&path) {
            return Some(content);
        }
    }
    None
}
