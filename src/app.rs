use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Instant;

use chrono::{DateTime, Utc};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::data::{self, ConversationMessage, Project, SessionEntry};
use crate::scheduler::Scheduler;

const BASE_SYSTEM_PROMPT: &str = include_str!("../system_prompt.md");

const OVERVIEW_PROMPT: &str = "Read the project files (CLAUDE.md, main source) to understand this project. Give me your assessment of what the project is about, where it stands, what you'd prioritize working on next, and where you think this project could ultimately go long-term.";

#[derive(Clone, Copy)]
pub enum ChatTone {
    Advisor,
    Eric,
    EricChinese,
}

impl ChatTone {
    pub fn suffix(self) -> &'static str {
        match self {
            Self::Advisor => "",
            Self::Eric => "\n\nTone override: Be a mean advisor. Sometimes use personal attacks (only when it makes sense, not too often). If there is nothing to complain about, talk normally.",
            Self::EricChinese => "\n\nTone override: Be a mean advisor. Sometimes use personal attacks (only when it makes sense, not too often). If there is nothing to complain about, talk normally. Output in Chinese.",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Advisor => "advisor",
            Self::Eric => "eric",
            Self::EricChinese => "eric-chinese",
        }
    }

    fn next(self) -> Self {
        match self {
            Self::Advisor => Self::Eric,
            Self::Eric => Self::EricChinese,
            Self::EricChinese => Self::Advisor,
        }
    }
}

pub struct UsageStatus {
    pub five_hour_pct: f64,
    pub five_hour_resets_at: Option<DateTime<Utc>>,
    pub seven_day_pct: f64,
    pub seven_day_resets_at: Option<DateTime<Utc>>,
    pub seven_day_sonnet_pct: Option<f64>,
    pub last_fetched: Instant,
}

#[derive(PartialEq)]
pub enum Mode {
    ProjectSelect,
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
    TaskInput,
    RgInput,
    PathInput,
}

pub enum TaskStatus {
    Running,
    Done,
    Error,
}

pub struct BackgroundTask {
    pub command: String,
    pub status: TaskStatus,
    pub output: Option<String>,
    rx: Receiver<Result<String, String>>,
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
    pub chat_waiting_since: Option<Instant>,
    pub chat_error: Option<String>,
    pub chat_session_id: Option<String>,
    response_rx: Option<Receiver<Result<(String, String), String>>>,

    // Usage status
    pub usage_status: Option<UsageStatus>,
    usage_rx: Option<Receiver<Option<UsageStatus>>>,
    usage_fetching: bool,

    // Clipboard feedback
    pub clipboard_msg: Option<String>,

    // Chat hint (transient, clears on next key)
    pub chat_hint: Option<String>,

    // Gateway
    pub gateway_enabled: bool,
    pub gateway_url: Option<String>,
    pub gateway_headers: Option<String>,

    // Chat tone
    pub chat_tone: ChatTone,

    // New message highlight
    pub new_msg_at: Option<Instant>,

    // Background tasks
    pub tasks: Vec<BackgroundTask>,
    pub show_task_input: bool,
    pub task_input: String,
    pub show_task_list: bool,
    pub task_list_idx: usize,
    pub task_scroll: u16,

    // Auto-task scheduler
    pub scheduler: Scheduler,

    // Ripgrep search in session list
    pub rg_query: String,
    pub rg_matches: HashMap<String, usize>,
    pub rg_active: bool,
    rg_rx: Option<Receiver<Result<HashMap<String, usize>, String>>>,

    // Working directory (selected project)
    pub cwd: PathBuf,
    pub path_input: String,
}

impl App {
    pub fn new() -> Self {
        let projects = data::load_projects();
        let filtered_project_indices: Vec<usize> = (0..projects.len()).collect();
        let mut app = Self {
            mode: Mode::ProjectSelect,
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
            chat_waiting_since: None,
            chat_error: None,
            chat_session_id: None,
            response_rx: None,
            usage_status: None,
            usage_rx: None,
            usage_fetching: false,
            clipboard_msg: None,
            chat_hint: None,
            gateway_url: std::env::var("ANTHROPIC_BASE_URL").ok(),
            gateway_headers: std::env::var("ANTHROPIC_CUSTOM_HEADERS").ok(),
            gateway_enabled: std::env::var("ANTHROPIC_BASE_URL").is_ok(),
            chat_tone: ChatTone::Advisor,
            new_msg_at: None,
            tasks: Vec::new(),
            show_task_input: false,
            task_input: String::new(),
            show_task_list: false,
            task_list_idx: 0,
            task_scroll: 0,
            scheduler: Scheduler::new(),
            rg_query: String::new(),
            rg_matches: HashMap::new(),
            rg_active: false,
            rg_rx: None,
            cwd: PathBuf::new(),
            path_input: String::new(),
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
                        self.new_msg_at = Some(Instant::now());
                    }
                    Err(e) => self.chat_error = Some(e),
                }
                self.chat_scroll = u16::MAX;
                self.chat_waiting = false;
                self.chat_waiting_since = None;
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

        // Check background tasks
        for task in &mut self.tasks {
            if matches!(task.status, TaskStatus::Running) {
                if let Ok(result) = task.rx.try_recv() {
                    match result {
                        Ok(output) => {
                            task.status = TaskStatus::Done;
                            task.output = Some(output);
                        }
                        Err(e) => {
                            task.status = TaskStatus::Error;
                            task.output = Some(e);
                        }
                    }
                }
            }
        }

        // Check rg search results
        if let Some(rx) = &self.rg_rx {
            if let Ok(result) = rx.try_recv() {
                self.rg_rx = None;
                if let Ok(matches) = result {
                    self.rg_matches = matches;
                    self.rg_active = true;
                    // Clear any text search filter
                    self.search_query.clear();
                    // Filter session list to only matching sessions
                    self.filtered_session_indices = self
                        .sessions
                        .iter()
                        .enumerate()
                        .filter(|(_, s)| self.rg_matches.contains_key(&s.session_id))
                        .map(|(i, _)| i)
                        .collect();
                    self.session_idx = 0;
                }
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

        // Auto-task scheduler
        if let Some(usage) = &self.usage_status {
            if self.scheduler.should_launch(usage, self.chat_waiting) {
                let task = self.scheduler.next_task();
                let name = task.name.to_string();
                let prompt = task.prompt.to_string();
                let cwd = Some(task.cwd);
                self.chat_messages
                    .push(("user".into(), format!("[Auto] {}", name)));
                self.spawn_claude(prompt, true, true, cwd.as_deref(), None);
            }
        }
    }

    fn fetch_usage(&mut self) {
        self.usage_fetching = true;
        let (tx, rx) = mpsc::channel();
        self.usage_rx = Some(rx);

        thread::spawn(move || {
            let status = fetch_oauth_usage();
            let _ = tx.send(status);
        });
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        match self.input_mode {
            InputMode::Search => self.handle_search_key(key),
            InputMode::ChatInput => self.handle_chat_input_key(key),
            InputMode::TaskInput => self.handle_task_input_key(key),
            InputMode::RgInput => self.handle_rg_input_key(key),
            InputMode::PathInput => self.handle_path_input_key(key),
            InputMode::Normal => {
                // Shift+Tab toggles mode between Chat and LogViewer only
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
                    Mode::ProjectSelect => self.handle_project_select_key(key),
                    Mode::Chat => self.handle_chat_normal_key(key),
                    Mode::LogViewer => self.handle_normal_key(key),
                }
            }
        }
    }

    // -- Chat key handlers --

    fn handle_chat_normal_key(&mut self, key: KeyEvent) {
        self.chat_hint = None;

        // Task list popup intercepts keys when visible
        if self.show_task_list {
            match key.code {
                KeyCode::Char('j') | KeyCode::Down => {
                    if !self.tasks.is_empty() {
                        self.task_list_idx = (self.task_list_idx + 1).min(self.tasks.len() - 1);
                        self.task_scroll = 0;
                    }
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    self.task_list_idx = self.task_list_idx.saturating_sub(1);
                    self.task_scroll = 0;
                }
                KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.task_scroll = self.task_scroll.saturating_add(10);
                }
                KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.task_scroll = self.task_scroll.saturating_sub(10);
                }
                KeyCode::Char('D') => {
                    if let Some(task) = self.tasks.get(self.task_list_idx) {
                        if !matches!(task.status, TaskStatus::Running) {
                            self.tasks.remove(self.task_list_idx);
                            if self.task_list_idx > 0 && self.task_list_idx >= self.tasks.len() {
                                self.task_list_idx = self.tasks.len().saturating_sub(1);
                            }
                        }
                    }
                }
                KeyCode::Esc | KeyCode::Char('X') => self.show_task_list = false,
                _ => {}
            }
            return;
        }

        match key.code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('i') | KeyCode::Enter => {
                if !self.chat_waiting {
                    self.input_mode = InputMode::ChatInput;
                }
            }
            KeyCode::Char('x') => {
                self.show_task_input = true;
                self.task_input.clear();
                self.input_mode = InputMode::TaskInput;
            }
            KeyCode::Char('X') => {
                self.show_task_list = !self.show_task_list;
                self.task_scroll = 0;
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
            KeyCode::Char('g') => {
                if self.gateway_url.is_some() {
                    self.gateway_enabled = !self.gateway_enabled;
                } else {
                    self.chat_messages.push(("system".into(),
                        "No gateway configured. Add to your shell profile:\n  export ANTHROPIC_BASE_URL=https://dev.sites.idies.jhu.edu/litellm\n  export ANTHROPIC_CUSTOM_HEADERS=\"x-litellm-api-key: Bearer sk-litellm-d2591383180bdbe94246734943cdd6a1\"".into()
                    ));
                    self.chat_scroll = u16::MAX;
                }
            }
            KeyCode::Char('t') => {
                self.chat_tone = self.chat_tone.next();
            }
            KeyCode::Char('n') => {
                if !self.chat_waiting {
                    self.chat_messages.clear();
                    self.chat_session_id = None;
                    self.chat_error = None;
                    self.chat_scroll = 0;
                }
            }
            KeyCode::Char('p') => {
                if !self.chat_waiting {
                    self.mode = Mode::ProjectSelect;
                    self.search_query.clear();
                    self.refilter();
                }
            }
            KeyCode::Char('a') => self.scheduler.toggle(),
            _ => {}
        }
    }

    fn handle_project_select_key(&mut self, key: KeyEvent) {
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
            KeyCode::Char('/') => {
                self.input_mode = InputMode::Search;
                self.search_query.clear();
            }
            KeyCode::Char('a') => {
                self.input_mode = InputMode::PathInput;
                self.path_input.clear();
            }
            KeyCode::Enter => self.select_project_and_enter_chat(),
            _ => {}
        }
    }

    fn select_project_and_enter_chat(&mut self) {
        if let Some(&idx) = self.filtered_project_indices.get(self.project_idx) {
            let path = &self.projects[idx].project_path;
            if !path.is_empty() {
                self.cwd = PathBuf::from(path);
                self.mode = Mode::Chat;
                self.chat_messages.clear();
                self.chat_session_id = None;
                self.chat_error = None;
                self.chat_scroll = 0;
                self.chat_messages.push(("user".into(), format!("Project: {}", path)));
                self.send_overview();
                self.search_query.clear();
                self.refilter();
            }
        }
    }

    fn handle_path_input_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.path_input.clear();
            }
            KeyCode::Enter => {
                let path = self.path_input.trim().to_string();
                if !path.is_empty() && std::path::Path::new(&path).is_dir() {
                    self.cwd = PathBuf::from(&path);
                    self.mode = Mode::Chat;
                    self.path_input.clear();
                    self.input_mode = InputMode::Normal;
                    self.chat_messages.clear();
                    self.chat_session_id = None;
                    self.chat_scroll = 0;
                    self.send_overview();
                }
            }
            KeyCode::Backspace => { self.path_input.pop(); }
            KeyCode::Char(c) => { self.path_input.push(c); }
            _ => {}
        }
    }

    fn handle_chat_input_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if !self.chat_input.is_empty() {
                    let ok = crate::platform::copy_to_clipboard(&self.chat_input);
                    self.clipboard_msg = Some(
                        if ok { "Copied!".into() } else { "Copy failed".into() },
                    );
                }
            }
            KeyCode::Enter if key.modifiers.contains(KeyModifiers::ALT) => {
                self.chat_input.push('\n');
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

    fn handle_task_input_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.show_task_input = false;
                self.task_input.clear();
                self.input_mode = InputMode::Normal;
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if !self.task_input.is_empty() {
                    let ok = crate::platform::copy_to_clipboard(&self.task_input);
                    self.clipboard_msg = Some(
                        if ok { "Copied!".into() } else { "Copy failed".into() },
                    );
                }
            }
            KeyCode::Enter => {
                let cmd = self.task_input.trim().to_string();
                if !cmd.is_empty() {
                    self.spawn_task(cmd);
                }
                self.show_task_input = false;
                self.task_input.clear();
                self.input_mode = InputMode::Normal;
            }
            KeyCode::Backspace => {
                self.task_input.pop();
            }
            KeyCode::Char(c) => {
                self.task_input.push(c);
            }
            _ => {}
        }
    }

    fn spawn_task(&mut self, command: String) {
        let (tx, rx) = mpsc::channel();
        self.tasks.push(BackgroundTask {
            command: command.clone(),
            status: TaskStatus::Running,
            output: None,
            rx,
        });
        thread::spawn(move || {
            let (shell, flag) = crate::platform::shell_cmd();
            let result = Command::new(shell)
                .arg(flag)
                .arg(&command)
                .output();
            let _ = tx.send(match result {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                    if output.status.success() {
                        Ok(if stderr.is_empty() { stdout } else { format!("{}\n{}", stdout, stderr) })
                    } else {
                        Err(if stderr.is_empty() {
                            format!("exit code: {}", output.status)
                        } else {
                            stderr
                        })
                    }
                }
                Err(e) => Err(format!("Failed to spawn: {}", e)),
            });
        });
    }

    fn send_overview(&mut self) {
        self.chat_messages
            .push(("user".into(), "Generating project overview...".into()));
        let cwd = if self.cwd.as_os_str().is_empty() { None } else { Some(self.cwd.display().to_string()) };
        self.spawn_claude(OVERVIEW_PROMPT.to_string(), false, false, cwd.as_deref(), None);
    }

    fn send_chat_message(&mut self, msg: String) {
        self.chat_messages.push(("user".into(), msg.clone()));
        let cwd = if self.cwd.as_os_str().is_empty() { None } else { Some(self.cwd.display().to_string()) };
        self.spawn_claude(msg, true, true, cwd.as_deref(), None);
    }

    /// Shared helper: spawn a background `claude` CLI call.
    /// `resume` — attach to existing session; `read_only` — disallow write tools.
    /// `cwd` — optional working directory for the claude process.
    fn spawn_claude(&mut self, msg: String, resume: bool, read_only: bool, cwd: Option<&str>, model: Option<&str>) {
        self.chat_error = None;
        self.chat_waiting = true;
        self.chat_waiting_since = Some(Instant::now());

        let session_id = if resume { self.chat_session_id.clone() } else { None };
        let gw_enabled = self.gateway_enabled;
        let gw_url = self.gateway_url.clone();
        let gw_headers = self.gateway_headers.clone();
        let system_prompt = format!("{}{}", BASE_SYSTEM_PROMPT, self.chat_tone.suffix());
        let work_dir = cwd.map(|s| s.to_string());
        let model_flag = model.map(|s| s.to_string());

        let (tx, rx) = mpsc::channel();
        self.response_rx = Some(rx);

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
        // Clear clipboard feedback on any key
        self.clipboard_msg = None;

        let len = self.filtered_session_indices.len();
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Backspace => {
                self.view = View::ProjectList;
                self.search_query.clear();
                self.clear_rg_filter();
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
            KeyCode::Char('y') => {
                if let Some(session) = self.selected_session() {
                    let id = session.session_id.clone();
                    let ok = crate::platform::copy_to_clipboard(&id);
                    self.clipboard_msg = Some(if ok { "Copied!".into() } else { "Copy failed".into() });
                }
            }
            KeyCode::Enter => self.enter_conversation(),
            KeyCode::Char('/') => {
                self.clear_rg_filter();
                self.input_mode = InputMode::Search;
                self.search_query.clear();
            }
            KeyCode::Char('r') => {
                self.input_mode = InputMode::RgInput;
                self.rg_query.clear();
            }
            KeyCode::Char('R') => self.clear_rg_filter(),
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
            self.rg_active = false;
            self.rg_matches.clear();
            self.rg_query.clear();
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

    fn handle_rg_input_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.rg_query.clear();
            }
            KeyCode::Enter => {
                let query = self.rg_query.trim().to_string();
                if !query.is_empty() {
                    self.run_rg_search(&query);
                }
                self.input_mode = InputMode::Normal;
            }
            KeyCode::Backspace => {
                self.rg_query.pop();
            }
            KeyCode::Char(c) => {
                self.rg_query.push(c);
            }
            _ => {}
        }
    }

    fn run_rg_search(&mut self, query: &str) {
        let Some(project) = self.selected_project() else { return };
        let claude_dir = project.claude_dir.clone();
        let query = query.to_string();
        let (tx, rx) = mpsc::channel();
        self.rg_rx = Some(rx);

        thread::spawn(move || {
            let result = Command::new("rg")
                .arg("--count-matches")
                .arg("--glob")
                .arg("*.jsonl")
                .arg(&query)
                .arg(&claude_dir)
                .output();

            let _ = tx.send(match result {
                Ok(output) => {
                    let mut map = HashMap::new();
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    for line in stdout.lines() {
                        // format: /path/to/session_id.jsonl:count
                        if let Some((path, count)) = line.rsplit_once(':') {
                            if let Ok(n) = count.parse::<usize>() {
                                // Extract session_id from filename
                                if let Some(fname) = std::path::Path::new(path)
                                    .file_stem()
                                    .and_then(|s| s.to_str())
                                {
                                    map.insert(fname.to_string(), n);
                                }
                            }
                        }
                    }
                    Ok(map)
                }
                Err(e) => Err(format!("rg failed: {}", e)),
            });
        });
    }

    fn clear_rg_filter(&mut self) {
        self.rg_active = false;
        self.rg_matches.clear();
        self.rg_query.clear();
        self.rg_rx = None;
        self.filtered_session_indices = (0..self.sessions.len()).collect();
        self.session_idx = 0;
    }

    fn refilter(&mut self) {
        let query = self.search_query.to_lowercase();
        // ProjectSelect mode reuses the same project filtering
        if self.mode == Mode::ProjectSelect {
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
            return;
        }
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

    pub fn handle_paste(&mut self, text: String) {
        match self.input_mode {
            InputMode::ChatInput => self.chat_input.push_str(&text),
            InputMode::TaskInput => self.task_input.push_str(&text),
            InputMode::Search => {
                self.search_query.push_str(&text);
                self.refilter();
            }
            InputMode::RgInput => self.rg_query.push_str(&text),
            InputMode::PathInput => self.path_input.push_str(&text),
            InputMode::Normal => {}
        }
    }
}

/// Fetch usage from the Anthropic OAuth API.
fn fetch_oauth_usage() -> Option<UsageStatus> {
    let token = crate::platform::get_oauth_token()?;

    // Call the usage API (anthropic-beta header is required)
    let api_output = Command::new("curl")
        .args([
            "-s",
            "-H", "Accept: application/json",
            "-H", "Content-Type: application/json",
            "-H", "User-Agent: claude-code/2.0.32",
            "-H", &format!("Authorization: Bearer {}", token),
            "-H", "anthropic-beta: oauth-2025-04-20",
            "https://api.anthropic.com/api/oauth/usage",
        ])
        .output()
        .ok()?;
    if !api_output.status.success() {
        return None;
    }
    let raw = String::from_utf8_lossy(&api_output.stdout);
    let val: serde_json::Value = serde_json::from_str(&raw).ok()?;

    let five_hour = val.get("five_hour")?;
    let five_hour_pct = five_hour.get("utilization")?.as_f64()?;
    let five_hour_resets_at = five_hour
        .get("resets_at")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<DateTime<Utc>>().ok());

    let seven_day = val.get("seven_day")?;
    let seven_day_pct = seven_day.get("utilization")?.as_f64()?;
    let seven_day_resets_at = seven_day
        .get("resets_at")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<DateTime<Utc>>().ok());

    let seven_day_sonnet_pct = val
        .get("seven_day_sonnet")
        .and_then(|v| v.get("utilization"))
        .and_then(|v| v.as_f64());

    Some(UsageStatus {
        five_hour_pct,
        five_hour_resets_at,
        seven_day_pct,
        seven_day_resets_at,
        seven_day_sonnet_pct,
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
