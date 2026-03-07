use std::process::Command;
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::pipeline::Pipeline;

use super::{App, InputMode, BASE_SYSTEM_PROMPT};

pub(crate) struct GatewayConfig {
    pub url: String,
    pub headers: Option<String>,
}

pub(crate) struct SpawnConfig {
    pub msg: String,
    pub resume_session: Option<String>,
    pub read_only: bool,
    pub cwd: Option<String>,
    pub model: Option<String>,
    pub setup: Option<String>,
    pub system_prompt: Option<String>,
    pub gateway: Option<GatewayConfig>,
}

pub(crate) fn build_claude_cmd(config: &SpawnConfig) -> Command {
    let mut cmd = Command::new("claude");
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());
    if let Some(dir) = &config.cwd {
        if !dir.is_empty() {
            cmd.current_dir(dir);
            cmd.arg("--add-dir").arg(dir);
        }
    }
    if let Some(gw) = &config.gateway {
        cmd.env("ANTHROPIC_BASE_URL", &gw.url);
        if let Some(h) = &gw.headers {
            cmd.env("ANTHROPIC_CUSTOM_HEADERS", h);
        }
    } else {
        cmd.env_remove("ANTHROPIC_BASE_URL");
        cmd.env_remove("ANTHROPIC_CUSTOM_HEADERS");
    }
    if let Some(m) = &config.model {
        cmd.arg("--model").arg(m);
    }
    if let Some(sp) = &config.system_prompt {
        cmd.arg("--system-prompt").arg(sp);
    }
    cmd.arg("-p").arg(&config.msg);
    cmd.arg("--output-format").arg("json");
    if config.system_prompt.is_some() {
        cmd.arg("--permission-mode").arg("dontAsk");
    } else {
        cmd.arg("--permission-mode").arg("bypassPermissions");
    }
    if config.read_only {
        cmd.arg("--disallowedTools")
            .arg("Write,Edit,MultiEdit,TodoWrite");
    }
    if let Some(id) = &config.resume_session {
        cmd.arg("--resume").arg(id);
    }
    cmd
}

impl App {
    pub(crate) fn handle_chat_normal_key(&mut self, key: KeyEvent) {
        self.chat.hint = None;

        // Pipeline picker popup intercepts keys when visible
        if self.tasks.pipeline_picker {
            let pipelines = Pipeline::all();
            match key.code {
                KeyCode::Char('j') | KeyCode::Down => {
                    self.tasks.pipeline_idx = (self.tasks.pipeline_idx + 1).min(pipelines.len() - 1);
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    self.tasks.pipeline_idx = self.tasks.pipeline_idx.saturating_sub(1);
                }
                KeyCode::Enter => {
                    let selected = pipelines[self.tasks.pipeline_idx];
                    self.tasks.pipeline_picker = false;
                    if selected == Pipeline::IssueDriven {
                        self.tasks.goal_input = true;
                        self.tasks.goal_text.clear();
                        self.input_mode = InputMode::TaskInput;
                    } else {
                        let cwd = self.cwd.display().to_string();
                        self.tasks.scheduler.switch_pipeline(selected, &cwd, "");
                        self.tasks.selected_idx = 0;
                    }
                }
                KeyCode::Esc => { self.tasks.pipeline_picker = false; }
                _ => {}
            }
            return;
        }

        // Task list popup intercepts keys when visible
        if self.tasks.show_panel {
            let done_len = self.tasks.scheduler.done.len();
            let running_offset: usize = if self.tasks.scheduler.running.is_some() { 1 } else { 0 };
            let total = done_len + running_offset + self.tasks.scheduler.tasks.len();
            match key.code {
                KeyCode::Char('j') | KeyCode::Down => {
                    if total > 0 {
                        self.tasks.selected_idx = (self.tasks.selected_idx + 1).min(total - 1);
                        self.tasks.scroll = 0;
                    }
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    self.tasks.selected_idx = self.tasks.selected_idx.saturating_sub(1);
                    self.tasks.scroll = 0;
                }
                KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.tasks.scroll = self.tasks.scroll.saturating_add(10);
                }
                KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.tasks.scroll = self.tasks.scroll.saturating_sub(10);
                }
                KeyCode::Char('D') | KeyCode::Char('d') => {
                    // Only delete pending items (past done + running sections)
                    let pending_start = done_len + running_offset;
                    if self.tasks.selected_idx >= pending_start {
                        let pending_idx = self.tasks.selected_idx - pending_start;
                        if pending_idx < self.tasks.scheduler.tasks.len() {
                            self.tasks.scheduler.tasks.remove(pending_idx);
                            if self.tasks.selected_idx > 0 && self.tasks.selected_idx >= total - 1 {
                                self.tasks.selected_idx = (total - 2).max(0);
                            }
                        }
                    }
                }
                KeyCode::Enter => {
                    if !self.chat.waiting {
                        let pending_start = done_len + running_offset;
                        if self.tasks.selected_idx >= pending_start {
                            let pending_idx = self.tasks.selected_idx - pending_start;
                            if pending_idx < self.tasks.scheduler.tasks.len() {
                                let task = self.tasks.scheduler.run_task(pending_idx);
                                self.chat.messages
                                    .push(("user".into(), format!("[Manual] {}", task.name)));
                                let config = self.config_for_task(&task);
                                self.spawn_claude(config);
                            }
                        }
                    }
                }
                KeyCode::Char('P') | KeyCode::Char('p') => {
                    self.tasks.pipeline_picker = true;
                    self.tasks.pipeline_idx = 0;
                }
                KeyCode::Esc | KeyCode::Char('X') => self.tasks.show_panel = false,
                _ => {}
            }
            return;
        }

        match key.code {
            KeyCode::Esc => {
                if self.chat.waiting {
                    self.cancel_running();
                }
            }
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('i') | KeyCode::Enter => {
                if !self.chat.waiting {
                    self.input_mode = InputMode::ChatInput;
                }
            }
            KeyCode::Char('x') => {
                self.tasks.show_input = true;
                self.tasks.input.clear();
                self.input_mode = InputMode::TaskInput;
            }
            KeyCode::Char('X') => {
                self.tasks.show_panel = !self.tasks.show_panel;
                self.tasks.scroll = 0;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.chat.scroll = self.chat.scroll.saturating_add(1);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.chat.scroll = self.chat.scroll.saturating_sub(1);
            }
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.chat.scroll = self.chat.scroll.saturating_add(20);
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.chat.scroll = self.chat.scroll.saturating_sub(20);
            }
            KeyCode::Char('G') => self.chat.scroll = u16::MAX,
            KeyCode::Char('g') => {
                if self.gateway_url.is_some() {
                    self.gateway_enabled = !self.gateway_enabled;
                } else {
                    self.chat.messages.push(("system".into(),
                        "No gateway configured. Add to your shell profile:\n  export ANTHROPIC_BASE_URL=https://dev.sites.idies.jhu.edu/litellm\n  export ANTHROPIC_CUSTOM_HEADERS=\"x-litellm-api-key: Bearer sk-litellm-d2591383180bdbe94246734943cdd6a1\"".into()
                    ));
                    self.chat.scroll = u16::MAX;
                }
            }
            KeyCode::Char('t') => {
                self.chat.tone = self.chat.tone.next();
            }
            KeyCode::Char('n') => {
                if !self.chat.waiting {
                    self.chat.messages.clear();
                    self.chat.session_id = None;
                    self.chat.error = None;
                    self.chat.scroll = 0;
                }
            }
            KeyCode::Char('p') => {
                if !self.chat.waiting {
                    self.mode = super::Mode::ProjectSelect;
                    self.search.query.clear();
                    self.refilter();
                }
            }
            KeyCode::Char('a') => self.tasks.scheduler.toggle(),
            _ => {}
        }
    }

    pub(crate) fn handle_chat_input_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if !self.chat.input.is_empty() {
                    let ok = crate::platform::copy_to_clipboard(&self.chat.input);
                    self.clipboard_msg = Some(
                        if ok { "Copied!".into() } else { "Copy failed".into() },
                    );
                }
            }
            KeyCode::Enter if key.modifiers.contains(KeyModifiers::ALT) => {
                self.chat.input.push('\n');
            }
            KeyCode::Enter => {
                let msg = self.chat.input.trim().to_string();
                if !msg.is_empty() {
                    self.send_chat_message(msg);
                    self.chat.input.clear();
                    self.input_mode = InputMode::Normal;
                }
            }
            KeyCode::Backspace => {
                self.chat.input.pop();
            }
            KeyCode::Char(c) => {
                self.chat.input.push(c);
            }
            _ => {}
        }
    }


    fn send_chat_message(&mut self, msg: String) {
        self.chat.messages.push(("user".into(), msg.clone()));
        let system_prompt = format!("{}{}", BASE_SYSTEM_PROMPT, self.chat.tone.suffix());
        self.spawn_claude(SpawnConfig {
            msg,
            resume_session: self.chat.session_id.clone(),
            read_only: true,
            cwd: if self.cwd.as_os_str().is_empty() { None } else { Some(self.cwd.display().to_string()) },
            model: None,
            setup: None,
            system_prompt: Some(system_prompt),
            gateway: self.build_gateway_config(),
        });
    }

    /// Shared helper: spawn a background `claude` CLI call.
    pub(crate) fn spawn_claude(&mut self, config: SpawnConfig) {
        self.chat.error = None;
        self.chat.waiting = true;
        self.chat.waiting_since = Some(Instant::now());
        self.chat.child_pid.store(0, Ordering::Relaxed);

        let pid_handle = Arc::clone(&self.chat.child_pid);

        let (tx, rx) = mpsc::channel();
        self.chat.response_rx = Some(rx);

        thread::spawn(move || {
            if let Some(setup_cmd) = &config.setup {
                let _ = Command::new("sh").arg("-c").arg(setup_cmd).output();
            }
            let mut cmd = build_claude_cmd(&config);

            let result = match cmd.spawn() {
                Ok(child) => {
                    pid_handle.store(child.id(), Ordering::Relaxed);
                    match child.wait_with_output() {
                        Ok(output) => {
                            pid_handle.store(0, Ordering::Relaxed);
                            if output.status.success() {
                                // claude --output-format json may write to
                                // stdout or stderr depending on version/context.
                                let stdout = String::from_utf8_lossy(&output.stdout);
                                let raw = if stdout.trim_ascii().is_empty() {
                                    String::from_utf8_lossy(&output.stderr)
                                } else {
                                    stdout
                                };
                                super::parse_claude_json(&raw)
                            } else {
                                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                                Err(if stderr.is_empty() {
                                    "claude exited with error".into()
                                } else {
                                    stderr
                                })
                            }
                        }
                        Err(e) => {
                            pid_handle.store(0, Ordering::Relaxed);
                            Err(format!("Failed waiting for claude: {}", e))
                        }
                    }
                }
                Err(e) => Err(format!("Failed to run claude: {}", e)),
            };
            let _ = tx.send(result);
        });
    }

    fn build_gateway_config(&self) -> Option<GatewayConfig> {
        if self.gateway_enabled {
            self.gateway_url.as_ref().map(|url| GatewayConfig {
                url: url.clone(),
                headers: self.gateway_headers.clone(),
            })
        } else {
            None
        }
    }

    pub(crate) fn config_for_task(&self, task: &crate::pipeline::AutoTask) -> SpawnConfig {
        let cwd = if task.cwd.is_empty() {
            if self.cwd.as_os_str().is_empty() { None } else { Some(self.cwd.display().to_string()) }
        } else {
            Some(task.cwd.clone())
        };
        let system_prompt = if task.use_advisor {
            Some(BASE_SYSTEM_PROMPT.to_string())
        } else {
            None
        };
        SpawnConfig {
            msg: task.prompt.clone(),
            resume_session: if task.resume { self.chat.session_id.clone() } else { None },
            read_only: task.read_only,
            cwd,
            model: None,
            setup: task.setup.clone(),
            system_prompt,
            gateway: self.build_gateway_config(),
        }
    }

    /// Cancel the currently running claude process.
    pub(crate) fn cancel_running(&mut self) {
        let pid = self.chat.child_pid.swap(0, Ordering::Relaxed);
        if pid != 0 {
            // Kill the process and its children
            let _ = Command::new("kill").arg(pid.to_string()).output();
        }
        self.chat.waiting = false;
        self.chat.waiting_since = None;
        self.chat.response_rx = None;
        self.chat.messages.push(("system".into(), "[Cancelled]".into()));
        self.chat.scroll = u16::MAX;
        if self.tasks.scheduler.running.is_some() {
            self.tasks.scheduler.running = None;
        }
    }

    pub fn handle_paste(&mut self, text: String) {
        match self.input_mode {
            InputMode::ChatInput => self.chat.input.push_str(&text),
            InputMode::TaskInput => self.tasks.input.push_str(&text),
            InputMode::Search => {
                self.search.query.push_str(&text);
                self.refilter();
            }
            InputMode::RgInput => self.search.rg_query.push_str(&text),
            InputMode::PathInput => self.path_input.push_str(&text),
            InputMode::Normal => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_config() -> SpawnConfig {
        SpawnConfig {
            msg: "hello".into(),
            resume_session: None,
            read_only: false,
            cwd: None,
            model: None,
            setup: None,
            system_prompt: None,
            gateway: None,
        }
    }

    #[test]
    fn test_build_cmd_no_system_prompt() {
        let config = base_config();
        let cmd = build_claude_cmd(&config);
        let args: Vec<_> = cmd.get_args().collect();
        assert!(!args.contains(&"--system-prompt".as_ref()),
            "should not have --system-prompt when system_prompt is None");
        assert!(args.contains(&"-p".as_ref()));
        assert!(args.contains(&"hello".as_ref()));
    }

    #[test]
    fn test_build_cmd_with_system_prompt() {
        let mut config = base_config();
        config.system_prompt = Some("You are an advisor.".into());
        let cmd = build_claude_cmd(&config);
        let args: Vec<_> = cmd.get_args().collect();
        assert!(args.contains(&"--system-prompt".as_ref()));
        assert!(args.contains(&"You are an advisor.".as_ref()));
    }

    #[test]
    fn test_build_cmd_read_only() {
        let mut config = base_config();
        config.read_only = true;
        let cmd = build_claude_cmd(&config);
        let args: Vec<_> = cmd.get_args().collect();
        assert!(args.contains(&"--disallowedTools".as_ref()));
    }

    #[test]
    fn test_build_cmd_not_read_only() {
        let config = base_config();
        let cmd = build_claude_cmd(&config);
        let args: Vec<_> = cmd.get_args().collect();
        assert!(!args.contains(&"--disallowedTools".as_ref()));
    }

    #[test]
    fn test_build_cmd_with_resume() {
        let mut config = base_config();
        config.resume_session = Some("sess-123".into());
        let cmd = build_claude_cmd(&config);
        let args: Vec<_> = cmd.get_args().collect();
        assert!(args.contains(&"--resume".as_ref()));
        assert!(args.contains(&"sess-123".as_ref()));
    }

    #[test]
    fn test_build_cmd_with_model() {
        let mut config = base_config();
        config.model = Some("claude-sonnet-4-6".into());
        let cmd = build_claude_cmd(&config);
        let args: Vec<_> = cmd.get_args().collect();
        assert!(args.contains(&"--model".as_ref()));
        assert!(args.contains(&"claude-sonnet-4-6".as_ref()));
    }

    #[test]
    fn test_build_cmd_advisor_uses_dontask() {
        let mut config = base_config();
        config.system_prompt = Some("You are an advisor.".into());
        let cmd = build_claude_cmd(&config);
        let args: Vec<_> = cmd.get_args().collect();
        assert!(args.contains(&"dontAsk".as_ref()),
            "advisor mode should use dontAsk permission mode");
    }

    #[test]
    fn test_build_cmd_execution_uses_bypass() {
        let config = base_config(); // system_prompt is None
        let cmd = build_claude_cmd(&config);
        let args: Vec<_> = cmd.get_args().collect();
        assert!(args.contains(&"bypassPermissions".as_ref()),
            "execution mode should use bypassPermissions");
    }

    #[test]
    fn test_build_cmd_with_cwd() {
        let mut config = base_config();
        config.cwd = Some("/tmp/proj".into());
        let cmd = build_claude_cmd(&config);
        let args: Vec<_> = cmd.get_args().collect();
        assert!(args.contains(&"--add-dir".as_ref()));
        assert!(args.contains(&"/tmp/proj".as_ref()));
    }
}
