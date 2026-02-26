use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::data::{self, ConversationMessage, Project, SessionEntry};

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
}

pub struct App {
    pub view: View,
    pub input_mode: InputMode,
    pub should_quit: bool,

    // Data
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
}

impl App {
    pub fn new() -> Self {
        let projects = data::load_projects();
        let filtered_project_indices: Vec<usize> = (0..projects.len()).collect();
        Self {
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
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        match self.input_mode {
            InputMode::Search => self.handle_search_key(key),
            InputMode::Normal => self.handle_normal_key(key),
        }
    }

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
