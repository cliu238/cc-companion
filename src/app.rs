use std::process::Command;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Instant;

use chrono::{DateTime, Utc};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::data::{self, ConversationMessage, Project, SessionEntry};

pub struct UsageStatus {
    pub window_end: Option<DateTime<Utc>>,
    pub total_tokens: u64,
    pub token_limit: u64,
    pub percent_used: f64,
    pub last_fetched: Instant,
}

#[derive(PartialEq)]
pub enum Mode {
    Chat,
    LogViewer,
}

#[derive(PartialEq)]
pub enum View {
    ProjectList,
    SessionList,
    Conversation,
    ClaudeMdViewer,
}

pub enum InputMode {
    Normal,
    Search,
    ChatInput,
}

pub struct App {
    pub mode: Mode,
    pub view: View,
    pub input_mode: InputMode,
    pub should_quit: bool,

    // Log viewer data
    pub projects: Vec<Project>,
    pub sessions: Vec<SessionEntry>,
    pub conversation: Vec<ConversationMessage>,
    pub claude_md_content: String,

    // Selection
    pub project_idx: usize,
    pub session_idx: usize,
    pub scroll_offset: u16,

    // Search
    pub search_query: String,
    pub filtered_project_indices: Vec<usize>,
    pub filtered_session_indices: Vec<usize>,

    // Chat
    pub chat_messages: Vec<(String, String)>, // (role, content)
    pub chat_input: String,
    pub chat_scroll: u16,
    pub chat_waiting: bool,
    pub chat_error: Option<String>,
    pub chat_session_id: Option<String>,
    response_rx: Option<Receiver<Result<(String, String), String>>>,

    // Usage status
    pub usage_status: Option<UsageStatus>,
    usage_rx: Option<Receiver<Option<UsageStatus>>>,
    usage_fetching: bool,
}

impl App {
    pub fn new() -> Self {
        let projects = data::load_projects();
        let filtered_project_indices: Vec<usize> = (0..projects.len()).collect();
        let mut app = Self {
            mode: Mode::Chat,
            view: View::ProjectList,
            input_mode: InputMode::Normal,
            should_quit: false,
            projects,
            sessions: Vec::new(),
            conversation: Vec::new(),
            claude_md_content: String::new(),
            project_idx: 0,
            session_idx: 0,
            scroll_offset: 0,
            search_query: String::new(),
            filtered_project_indices,
            filtered_session_indices: Vec::new(),
            chat_messages: Vec::new(),
            chat_input: String::new(),
            chat_scroll: 0,
            chat_waiting: false,
            chat_error: None,
            chat_session_id: None,
            response_rx: None,
            usage_status: None,
            usage_rx: None,
            usage_fetching: false,
        };
        app.fetch_usage();
        app
    }

    pub fn tick(&mut self) {
        // Check chat response
        if let Some(rx) = &self.response_rx {
            if let Ok(result) = rx.try_recv() {
                match result {
                    Ok((session_id, text)) => {
                        self.chat_session_id = Some(session_id);
                        self.chat_messages.push(("assistant".into(), text));
                    }
                    Err(e) => self.chat_error = Some(e),
                }
                self.chat_waiting = false;
                self.response_rx = None;
            }
        }

        // Check usage status response
        if let Some(rx) = &self.usage_rx {
            if let Ok(result) = rx.try_recv() {
                self.usage_status = result;
                self.usage_rx = None;
                self.usage_fetching = false;
            }
        }

        // Periodic usage refetch (every 60s)
        if !self.usage_fetching {
            let should_fetch = match &self.usage_status {
                Some(s) => s.last_fetched.elapsed().as_secs() >= 60,
                None => self.usage_rx.is_none(), // retry if no data and not fetching
            };
            if should_fetch {
                self.fetch_usage();
            }
        }
    }

    fn fetch_usage(&mut self) {
        self.usage_fetching = true;
        let (tx, rx) = mpsc::channel();
        self.usage_rx = Some(rx);

        thread::spawn(move || {
            let result = Command::new("npx")
                .args(["ccusage@latest", "blocks", "--active", "--json", "--token-limit", "max"])
                .output();

            let status = match result {
                Ok(output) if output.status.success() => {
                    let raw = String::from_utf8_lossy(&output.stdout);
                    parse_usage_json(&raw)
                }
                _ => None,
            };
            let _ = tx.send(status);
        });
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        match self.input_mode {
            InputMode::Search => self.handle_search_key(key),
            InputMode::ChatInput => self.handle_chat_input_key(key),
            InputMode::Normal => {
                // Shift+Tab toggles mode at top-level views
                if key.code == KeyCode::BackTab {
                    match self.mode {
                        Mode::Chat => self.mode = Mode::LogViewer,
                        Mode::LogViewer if self.view == View::ProjectList => {
                            self.mode = Mode::Chat;
                        }
                        _ => {}
                    }
                    return;
                }
                match self.mode {
                    Mode::Chat => self.handle_chat_normal_key(key),
                    Mode::LogViewer => self.handle_normal_key(key),
                }
            }
        }
    }

    // -- Chat key handlers --

    fn handle_chat_normal_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('i') | KeyCode::Enter => {
                if !self.chat_waiting {
                    self.input_mode = InputMode::ChatInput;
                }
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.chat_scroll = self.chat_scroll.saturating_add(1);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.chat_scroll = self.chat_scroll.saturating_sub(1);
            }
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.chat_scroll = self.chat_scroll.saturating_add(20);
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.chat_scroll = self.chat_scroll.saturating_sub(20);
            }
            KeyCode::Char('G') => self.chat_scroll = u16::MAX,
            KeyCode::Char('g') => self.chat_scroll = 0,
            KeyCode::Char('n') => {
                if !self.chat_waiting {
                    self.chat_messages.clear();
                    self.chat_session_id = None;
                    self.chat_error = None;
                    self.chat_scroll = 0;
                }
            }
            _ => {}
        }
    }

    fn handle_chat_input_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
            }
            KeyCode::Enter => {
                let msg = self.chat_input.trim().to_string();
                if !msg.is_empty() {
                    self.send_chat_message(msg);
                    self.chat_input.clear();
                    self.input_mode = InputMode::Normal;
                }
            }
            KeyCode::Backspace => {
                self.chat_input.pop();
            }
            KeyCode::Char(c) => {
                self.chat_input.push(c);
            }
            _ => {}
        }
    }

    fn send_chat_message(&mut self, msg: String) {
        self.chat_messages.push(("user".into(), msg.clone()));
        self.chat_error = None;
        self.chat_waiting = true;

        let session_id = self.chat_session_id.clone();

        let (tx, rx) = mpsc::channel();
        self.response_rx = Some(rx);

        thread::spawn(move || {
            let mut cmd = Command::new("claude");
            cmd.arg("-p").arg(&msg);
            cmd.arg("--output-format").arg("json");
            cmd.arg("--disallowedTools")
                .arg("Write,Edit,MultiEdit,TodoWrite");
            cmd.arg("--permission-mode").arg("dontAsk");
            if let Some(id) = &session_id {
                cmd.arg("--resume").arg(id);
            }

            let result = match cmd.output() {
                Ok(output) => {
                    if output.status.success() {
                        let raw = String::from_utf8_lossy(&output.stdout);
                        parse_claude_json(&raw)
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

    // -- Log viewer key handlers (unchanged) --

    fn handle_search_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.search_query.clear();
                self.refilter();
            }
            KeyCode::Enter => {
                self.input_mode = InputMode::Normal;
            }
            KeyCode::Backspace => {
                self.search_query.pop();
                self.refilter();
            }
            KeyCode::Char(c) => {
                self.search_query.push(c);
                self.refilter();
            }
            _ => {}
        }
    }

    fn handle_normal_key(&mut self, key: KeyEvent) {
        match self.view {
            View::ProjectList => self.handle_project_list_key(key),
            View::SessionList => self.handle_session_list_key(key),
            View::Conversation => self.handle_conversation_key(key),
            View::ClaudeMdViewer => self.handle_claude_md_key(key),
        }
    }

    fn handle_project_list_key(&mut self, key: KeyEvent) {
        let len = self.filtered_project_indices.len();
        match key.code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('j') | KeyCode::Down => {
                if len > 0 {
                    self.project_idx = (self.project_idx + 1).min(len - 1);
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.project_idx = self.project_idx.saturating_sub(1);
            }
            KeyCode::Char('g') => self.project_idx = 0,
            KeyCode::Char('G') => {
                if len > 0 {
                    self.project_idx = len - 1;
                }
            }
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if len > 0 {
                    self.project_idx = (self.project_idx + 10).min(len - 1);
                }
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.project_idx = self.project_idx.saturating_sub(10);
            }
            KeyCode::Enter => self.enter_session_list(),
            KeyCode::Char('/') => {
                self.input_mode = InputMode::Search;
                self.search_query.clear();
            }
            KeyCode::Char('c') => self.view_claude_md(),
            _ => {}
        }
    }

    fn handle_session_list_key(&mut self, key: KeyEvent) {
        let len = self.filtered_session_indices.len();
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Backspace => {
                self.view = View::ProjectList;
                self.search_query.clear();
                self.refilter();
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if len > 0 {
                    self.session_idx = (self.session_idx + 1).min(len - 1);
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.session_idx = self.session_idx.saturating_sub(1);
            }
            KeyCode::Char('g') => self.session_idx = 0,
            KeyCode::Char('G') => {
                if len > 0 {
                    self.session_idx = len - 1;
                }
            }
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if len > 0 {
                    self.session_idx = (self.session_idx + 10).min(len - 1);
                }
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.session_idx = self.session_idx.saturating_sub(10);
            }
            KeyCode::Enter => self.enter_conversation(),
            KeyCode::Char('/') => {
                self.input_mode = InputMode::Search;
                self.search_query.clear();
            }
            _ => {}
        }
    }

    fn handle_conversation_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Backspace => {
                self.view = View::SessionList;
                self.scroll_offset = 0;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.scroll_offset = self.scroll_offset.saturating_add(1);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.scroll_offset = self.scroll_offset.saturating_sub(1);
            }
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.scroll_offset = self.scroll_offset.saturating_add(20);
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.scroll_offset = self.scroll_offset.saturating_sub(20);
            }
            KeyCode::Char('g') => self.scroll_offset = 0,
            KeyCode::Char('G') => self.scroll_offset = u16::MAX,
            _ => {}
        }
    }

    fn handle_claude_md_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Backspace => {
                self.view = View::ProjectList;
                self.scroll_offset = 0;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.scroll_offset = self.scroll_offset.saturating_add(1);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.scroll_offset = self.scroll_offset.saturating_sub(1);
            }
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.scroll_offset = self.scroll_offset.saturating_add(20);
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.scroll_offset = self.scroll_offset.saturating_sub(20);
            }
            _ => {}
        }
    }

    fn enter_session_list(&mut self) {
        if let Some(&real_idx) = self.filtered_project_indices.get(self.project_idx) {
            let project = &self.projects[real_idx];
            self.sessions = data::load_sessions(&project.claude_dir);
            self.filtered_session_indices = (0..self.sessions.len()).collect();
            self.session_idx = 0;
            self.search_query.clear();
            self.view = View::SessionList;
        }
    }

    fn enter_conversation(&mut self) {
        if let Some(&real_idx) = self.filtered_session_indices.get(self.session_idx) {
            let session = &self.sessions[real_idx];
            self.conversation = data::load_conversation(&session.jsonl_path);
            self.scroll_offset = 0;
            self.view = View::Conversation;
        }
    }

    fn view_claude_md(&mut self) {
        if let Some(&real_idx) = self.filtered_project_indices.get(self.project_idx) {
            let project = &self.projects[real_idx];
            if let Some(content) = data::load_claude_md(project) {
                self.claude_md_content = content;
                self.scroll_offset = 0;
                self.view = View::ClaudeMdViewer;
            }
        }
    }

    fn refilter(&mut self) {
        let query = self.search_query.to_lowercase();
        match self.view {
            View::ProjectList => {
                if query.is_empty() {
                    self.filtered_project_indices = (0..self.projects.len()).collect();
                } else {
                    self.filtered_project_indices = self
                        .projects
                        .iter()
                        .enumerate()
                        .filter(|(_, p)| p.name.to_lowercase().contains(&query))
                        .map(|(i, _)| i)
                        .collect();
                }
                self.project_idx = 0;
            }
            View::SessionList => {
                if query.is_empty() {
                    self.filtered_session_indices = (0..self.sessions.len()).collect();
                } else {
                    self.filtered_session_indices = self
                        .sessions
                        .iter()
                        .enumerate()
                        .filter(|(_, s)| {
                            s.first_prompt.to_lowercase().contains(&query)
                                || s.git_branch.to_lowercase().contains(&query)
                        })
                        .map(|(i, _)| i)
                        .collect();
                }
                self.session_idx = 0;
            }
            _ => {}
        }
    }

    pub fn selected_project(&self) -> Option<&Project> {
        self.filtered_project_indices
            .get(self.project_idx)
            .map(|&i| &self.projects[i])
    }

    pub fn selected_session(&self) -> Option<&SessionEntry> {
        self.filtered_session_indices
            .get(self.session_idx)
            .map(|&i| &self.sessions[i])
    }
}

fn parse_usage_json(raw: &str) -> Option<UsageStatus> {
    let val: serde_json::Value = serde_json::from_str(raw).ok()?;
    let block = val.get("blocks")?.as_array()?.first()?;

    let end_time_str = block.get("endTime")?.as_str()?;
    let window_end = end_time_str.parse::<DateTime<Utc>>().ok();

    let total_tokens = block.get("totalTokens")?.as_u64().unwrap_or(0);

    let limit_status = block.get("tokenLimitStatus")?;
    let token_limit = limit_status.get("limit")?.as_u64().unwrap_or(0);
    let percent_used = limit_status.get("percentUsed")?.as_f64().unwrap_or(0.0);
    Some(UsageStatus {
        window_end,
        total_tokens,
        token_limit,
        percent_used,
        last_fetched: Instant::now(),
    })
}

/// Parse Claude JSON output array, extract session_id and result text from the "result" event.
fn parse_claude_json(raw: &str) -> Result<(String, String), String> {
    let arr: Vec<serde_json::Value> =
        serde_json::from_str(raw).map_err(|e| format!("JSON parse error: {}", e))?;

    for obj in arr.iter().rev() {
        if obj.get("type").and_then(|v| v.as_str()) == Some("result") {
            let session_id = obj
                .get("session_id")
                .and_then(|v| v.as_str())
                .ok_or("missing session_id in result")?
                .to_string();
            let result_text = obj
                .get("result")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            return Ok((session_id, result_text));
        }
    }
    Err("no result event found in response".into())
}
