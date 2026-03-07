use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicU32;
use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;
use std::thread;
use std::time::Instant;

use chrono::{DateTime, Utc};
use crossterm::event::{KeyCode, KeyEvent};

use crate::data::{self, ConversationMessage, Project, SessionEntry};
use crate::pipeline::{Pipeline, Scheduler};

mod chat;
mod nav;
mod search;

const BASE_SYSTEM_PROMPT: &str = include_str!("../../system_prompt.md");

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

    pub(crate) fn next(self) -> Self {
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
    pub(crate) rx: Receiver<Result<String, String>>,
}

pub struct ChatState {
    pub messages: Vec<(String, String)>,
    pub input: String,
    pub scroll: u16,
    pub waiting: bool,
    pub waiting_since: Option<Instant>,
    pub error: Option<String>,
    pub session_id: Option<String>,
    pub(crate) response_rx: Option<Receiver<Result<(String, String), String>>>,
    pub tone: ChatTone,
    pub hint: Option<String>,
    pub new_msg_at: Option<Instant>,
    /// PID of running claude process (0 = none), shared with spawned thread.
    pub(crate) child_pid: Arc<AtomicU32>,
}

pub struct TaskState {
    pub items: Vec<BackgroundTask>,
    pub show_input: bool,
    pub input: String,
    pub show_panel: bool,
    pub selected_idx: usize,
    pub scroll: u16,
    pub scheduler: Scheduler,
    pub goal_input: bool,
    pub goal_text: String,
    pub pipeline_picker: bool,
    pub pipeline_idx: usize,
}

pub struct SearchState {
    pub query: String,
    pub project_indices: Vec<usize>,
    pub session_indices: Vec<usize>,
    pub rg_query: String,
    pub rg_matches: HashMap<String, usize>,
    pub rg_active: bool,
    pub(crate) rg_rx: Option<Receiver<Result<HashMap<String, usize>, String>>>,
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

    // Clipboard feedback
    pub clipboard_msg: Option<String>,

    // Usage status
    pub usage_status: Option<UsageStatus>,
    usage_rx: Option<Receiver<Option<UsageStatus>>>,
    usage_fetching: bool,

    // Gateway
    pub gateway_enabled: bool,
    pub gateway_url: Option<String>,
    pub gateway_headers: Option<String>,

    // Working directory (selected project)
    pub cwd: PathBuf,
    pub path_input: String,

    // Sub-states
    pub chat: ChatState,
    pub tasks: TaskState,
    pub search: SearchState,
}

impl App {
    pub fn new() -> Self {
        let projects = data::load_projects();
        let project_indices: Vec<usize> = (0..projects.len()).collect();
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
            clipboard_msg: None,
            usage_status: None,
            usage_rx: None,
            usage_fetching: false,
            gateway_url: std::env::var("ANTHROPIC_BASE_URL").ok(),
            gateway_headers: std::env::var("ANTHROPIC_CUSTOM_HEADERS").ok(),
            gateway_enabled: std::env::var("ANTHROPIC_BASE_URL").is_ok(),
            cwd: PathBuf::new(),
            path_input: String::new(),
            chat: ChatState {
                messages: Vec::new(),
                input: String::new(),
                scroll: 0,
                waiting: false,
                waiting_since: None,
                error: None,
                session_id: None,
                response_rx: None,
                tone: ChatTone::Advisor,
                hint: None,
                new_msg_at: None,
                child_pid: Arc::new(AtomicU32::new(0)),
            },
            tasks: TaskState {
                items: Vec::new(),
                show_input: false,
                input: String::new(),
                show_panel: false,
                selected_idx: 0,
                scroll: 0,
                scheduler: Scheduler::new(Pipeline::Example, "", ""),
                goal_input: false,
                goal_text: String::new(),
                pipeline_picker: false,
                pipeline_idx: 0,
            },
            search: SearchState {
                query: String::new(),
                project_indices,
                session_indices: Vec::new(),
                rg_query: String::new(),
                rg_matches: HashMap::new(),
                rg_active: false,
                rg_rx: None,
            },
        };
        app.fetch_usage();
        app
    }

    pub fn tick(&mut self) {
        // Check chat response
        if let Some(rx) = &self.chat.response_rx {
            if let Ok(result) = rx.try_recv() {
                match result {
                    Ok((session_id, text)) => {
                        self.chat.session_id = Some(session_id);
                        self.chat.messages.push(("assistant".into(), text));
                        self.chat.new_msg_at = Some(Instant::now());
                    }
                    Err(e) => self.chat.error = Some(e),
                }
                self.chat.scroll = u16::MAX;
                self.chat.waiting = false;
                self.chat.waiting_since = None;
                self.chat.response_rx = None;
                if self.tasks.scheduler.running.is_some() {
                    let output = self.chat.messages.last()
                        .map(|(_, t)| t.as_str()).unwrap_or("");
                    let cwd = self.cwd.display().to_string();
                    let keep_session = self.tasks.scheduler.running_resume;
                    self.tasks.scheduler.complete_running(output, &cwd);
                    if !keep_session {
                        // Non-resuming tasks are independent; keep existing session_id
                    }
                }
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
        for task in &mut self.tasks.items {
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
        if let Some(rx) = &self.search.rg_rx {
            if let Ok(result) = rx.try_recv() {
                self.search.rg_rx = None;
                if let Ok(matches) = result {
                    self.search.rg_matches = matches;
                    self.search.rg_active = true;
                    self.search.query.clear();
                    self.search.session_indices = self
                        .sessions
                        .iter()
                        .enumerate()
                        .filter(|(_, s)| self.search.rg_matches.contains_key(&s.session_id))
                        .map(|(i, _)| i)
                        .collect();
                    self.session_idx = 0;
                }
            }
        }

        // Periodic usage refetch (every 5 min)
        if !self.usage_fetching {
            let should_fetch = match &self.usage_status {
                Some(s) => s.last_fetched.elapsed().as_secs() >= 300,
                None => self.usage_rx.is_none(),
            };
            if should_fetch {
                self.fetch_usage();
            }
        }

        // Auto-task scheduler
        if let Some(ref usage) = self.usage_status {
            if !self.tasks.show_panel && self.tasks.scheduler.should_launch(usage, self.chat.waiting) {
                let task = self.tasks.scheduler.next_task();
                self.chat.messages
                    .push(("user".into(), format!("[Auto] {}", task.name)));
                let config = self.config_for_task(&task);
                self.spawn_claude(config);
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

    fn handle_normal_key(&mut self, key: KeyEvent) {
        match self.view {
            View::ProjectList => self.handle_project_list_key(key),
            View::SessionList => self.handle_session_list_key(key),
            View::Conversation => self.handle_conversation_key(key),
            View::ClaudeMdViewer => self.handle_claude_md_key(key),
        }
    }

    pub fn selected_project(&self) -> Option<&Project> {
        self.search.project_indices
            .get(self.project_idx)
            .map(|&i| &self.projects[i])
    }

    pub fn selected_session(&self) -> Option<&SessionEntry> {
        self.search.session_indices
            .get(self.session_idx)
            .map(|&i| &self.sessions[i])
    }
}

/// Fetch usage from the Anthropic OAuth API.
fn fetch_oauth_usage() -> Option<UsageStatus> {
    let token = crate::platform::get_oauth_token()?;

    let api_output = std::process::Command::new("curl")
        .args([
            "-s",
            "-H", "Accept: application/json",
            "-H", "Content-Type: application/json",
            "-H", "User-Agent: claude-code",
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

#[cfg(test)]
impl App {
    /// Side-effect-free constructor for tests: no file I/O, no threads.
    pub(crate) fn test_default() -> Self {
        Self {
            mode: Mode::ProjectSelect,
            view: View::ProjectList,
            input_mode: InputMode::Normal,
            should_quit: false,
            projects: Vec::new(),
            sessions: Vec::new(),
            conversation: Vec::new(),
            claude_md_content: String::new(),
            project_idx: 0,
            session_idx: 0,
            scroll_offset: 0,
            clipboard_msg: None,
            usage_status: None,
            usage_rx: None,
            usage_fetching: false,
            gateway_url: None,
            gateway_headers: None,
            gateway_enabled: false,
            cwd: PathBuf::new(),
            path_input: String::new(),
            chat: ChatState {
                messages: Vec::new(),
                input: String::new(),
                scroll: 0,
                waiting: false,
                waiting_since: None,
                error: None,
                session_id: None,
                response_rx: None,
                tone: ChatTone::Advisor,
                hint: None,
                new_msg_at: None,
                child_pid: Arc::new(AtomicU32::new(0)),
            },
            tasks: TaskState {
                items: Vec::new(),
                show_input: false,
                input: String::new(),
                show_panel: false,
                selected_idx: 0,
                scroll: 0,
                scheduler: Scheduler::new(Pipeline::Example, "", ""),
                goal_input: false,
                goal_text: String::new(),
                pipeline_picker: false,
                pipeline_idx: 0,
            },
            search: SearchState {
                query: String::new(),
                project_indices: Vec::new(),
                session_indices: Vec::new(),
                rg_query: String::new(),
                rg_matches: HashMap::new(),
                rg_active: false,
                rg_rx: None,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_claude_json_valid() {
        let json = r#"[
            {"type":"init","session_id":"sess-1"},
            {"type":"result","session_id":"sess-1","result":"all done"}
        ]"#;
        let (sid, text) = parse_claude_json(json).unwrap();
        assert_eq!(sid, "sess-1");
        assert_eq!(text, "all done");
    }

    #[test]
    fn test_parse_claude_json_no_result() {
        let json = r#"[{"type":"init","session_id":"sess-1"}]"#;
        assert!(parse_claude_json(json).is_err());
    }

    #[test]
    fn test_parse_claude_json_invalid() {
        assert!(parse_claude_json("not json").is_err());
    }

    #[test]
    fn test_parse_claude_json_missing_session_id() {
        let json = r#"[{"type":"result","result":"ok"}]"#;
        assert!(parse_claude_json(json).is_err());
    }
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
