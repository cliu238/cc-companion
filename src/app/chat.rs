use std::process::Command;
use std::sync::mpsc;
use std::thread;
use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::{App, InputMode, BASE_SYSTEM_PROMPT};

impl App {
    pub(crate) fn handle_chat_normal_key(&mut self, key: KeyEvent) {
        self.chat.hint = None;

        // Task list popup intercepts keys when visible
        if self.tasks.show_panel {
            match key.code {
                KeyCode::Char('j') | KeyCode::Down => {
                    if !self.tasks.items.is_empty() {
                        self.tasks.selected_idx = (self.tasks.selected_idx + 1).min(self.tasks.items.len() - 1);
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
                KeyCode::Char('D') => {
                    if let Some(task) = self.tasks.items.get(self.tasks.selected_idx) {
                        if !matches!(task.status, super::TaskStatus::Running) {
                            self.tasks.items.remove(self.tasks.selected_idx);
                            if self.tasks.selected_idx > 0 && self.tasks.selected_idx >= self.tasks.items.len() {
                                self.tasks.selected_idx = self.tasks.items.len().saturating_sub(1);
                            }
                        }
                    }
                }
                KeyCode::Esc | KeyCode::Char('X') => self.tasks.show_panel = false,
                _ => {}
            }
            return;
        }

        match key.code {
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

    pub(crate) fn send_overview(&mut self) {
        self.chat.messages
            .push(("user".into(), "Generating project overview...".into()));
        let cwd = if self.cwd.as_os_str().is_empty() { None } else { Some(self.cwd.display().to_string()) };
        self.spawn_claude(super::OVERVIEW_PROMPT.to_string(), false, false, cwd.as_deref(), None);
    }

    fn send_chat_message(&mut self, msg: String) {
        self.chat.messages.push(("user".into(), msg.clone()));
        let cwd = if self.cwd.as_os_str().is_empty() { None } else { Some(self.cwd.display().to_string()) };
        self.spawn_claude(msg, true, true, cwd.as_deref(), None);
    }

    /// Shared helper: spawn a background `claude` CLI call.
    /// `resume` — attach to existing session; `read_only` — disallow write tools.
    /// `cwd` — optional working directory for the claude process.
    pub(crate) fn spawn_claude(&mut self, msg: String, resume: bool, read_only: bool, cwd: Option<&str>, model: Option<&str>) {
        self.chat.error = None;
        self.chat.waiting = true;
        self.chat.waiting_since = Some(Instant::now());

        let session_id = if resume { self.chat.session_id.clone() } else { None };
        let gw_enabled = self.gateway_enabled;
        let gw_url = self.gateway_url.clone();
        let gw_headers = self.gateway_headers.clone();
        let system_prompt = format!("{}{}", BASE_SYSTEM_PROMPT, self.chat.tone.suffix());
        let work_dir = cwd.map(|s| s.to_string());
        let model_flag = model.map(|s| s.to_string());

        let (tx, rx) = mpsc::channel();
        self.chat.response_rx = Some(rx);

        thread::spawn(move || {
            let mut cmd = Command::new("claude");
            if let Some(dir) = &work_dir {
                if !dir.is_empty() {
                    cmd.current_dir(dir);
                    cmd.arg("--add-dir").arg(dir);
                }
            }
            if gw_enabled {
                if let Some(url) = &gw_url {
                    cmd.env("ANTHROPIC_BASE_URL", url);
                }
                if let Some(headers) = &gw_headers {
                    cmd.env("ANTHROPIC_CUSTOM_HEADERS", headers);
                }
            } else {
                cmd.env_remove("ANTHROPIC_BASE_URL");
                cmd.env_remove("ANTHROPIC_CUSTOM_HEADERS");
            }
            if let Some(m) = &model_flag {
                cmd.arg("--model").arg(m);
            }
            cmd.arg("--system-prompt").arg(&system_prompt);
            cmd.arg("-p").arg(&msg);
            cmd.arg("--output-format").arg("json");
            cmd.arg("--permission-mode").arg("dontAsk");
            if read_only {
                cmd.arg("--disallowedTools")
                    .arg("Write,Edit,MultiEdit,TodoWrite");
            }
            if let Some(id) = &session_id {
                cmd.arg("--resume").arg(id);
            }

            let result = match cmd.output() {
                Ok(output) => {
                    if output.status.success() {
                        let raw = String::from_utf8_lossy(&output.stdout);
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
                Err(e) => Err(format!("Failed to run claude: {}", e)),
            };
            let _ = tx.send(result);
        });
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
