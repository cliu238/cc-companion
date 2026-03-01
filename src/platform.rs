use std::process::{Command, Stdio};

/// Copy text to the system clipboard (platform-aware).
pub fn copy_to_clipboard(text: &str) -> bool {
    let (cmd, args): (&str, &[&str]) = if cfg!(target_os = "macos") {
        ("pbcopy", &[])
    } else if cfg!(target_os = "windows") {
        ("clip", &[])
    } else {
        ("xclip", &["-selection", "clipboard"])
    };
    Command::new(cmd)
        .args(args)
        .stdin(Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            if let Some(stdin) = child.stdin.as_mut() {
                stdin.write_all(text.as_bytes())?;
            }
            child.wait()
        })
        .is_ok()
}

/// Get OAuth token: macOS tries Keychain first, all platforms fall back to credentials file.
pub fn get_oauth_token() -> Option<String> {
    if cfg!(target_os = "macos") {
        if let Some(token) = get_token_from_keychain() {
            return Some(token);
        }
    }
    // Fallback: read from ~/.claude/.credentials.json
    let cred_path = dirs::home_dir()?.join(".claude").join(".credentials.json");
    let content = std::fs::read_to_string(cred_path).ok()?;
    let cred: serde_json::Value = serde_json::from_str(&content).ok()?;
    cred.get("claudeAiOauth")
        .and_then(|o| o.get("accessToken"))
        .and_then(|t| t.as_str())
        .map(|s| s.to_string())
}

/// Return shell command and argument prefix for the current platform.
pub fn shell_cmd() -> (&'static str, &'static str) {
    if cfg!(target_os = "windows") {
        ("cmd", "/c")
    } else {
        ("sh", "-c")
    }
}

/// Try to read the OAuth token from the macOS Keychain.
fn get_token_from_keychain() -> Option<String> {
    let output = Command::new("security")
        .args(["find-generic-password", "-s", "Claude Code-credentials", "-w"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let json = String::from_utf8_lossy(&output.stdout);
    let cred: serde_json::Value = serde_json::from_str(json.trim()).ok()?;
    cred.get("claudeAiOauth")
        .and_then(|o| o.get("accessToken"))
        .and_then(|t| t.as_str())
        .map(|s| s.to_string())
}
