use std::path::PathBuf;
use std::process::Command;
use std::sync::mpsc;
use std::thread;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::{App, BackgroundTask, InputMode, Mode, TaskStatus, View};

impl App {
    pub(crate) fn handle_project_select_key(&mut self, key: KeyEvent) {
        let len = self.search.project_indices.len();
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
                self.search.query.clear();
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
        if let Some(&idx) = self.search.project_indices.get(self.project_idx) {
            let path = self.projects[idx].project_path.clone();
            if !path.is_empty() {
                self.cwd = PathBuf::from(&path);
                self.mode = Mode::Chat;
                self.chat.messages.clear();
                self.chat.session_id = None;
                self.chat.error = None;
                self.chat.scroll = 0;
                self.chat.messages.push(("user".into(), format!("Project: {}", path)));
                self.tasks.scheduler = crate::pipeline::Scheduler::new(
                    crate::pipeline::Pipeline::Example, &path, "",
                );
                self.search.query.clear();
                self.refilter();
            }
        }
    }

    pub(crate) fn handle_path_input_key(&mut self, key: KeyEvent) {
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
                    self.chat.messages.clear();
                    self.chat.session_id = None;
                    self.chat.scroll = 0;
                }
            }
            KeyCode::Backspace => { self.path_input.pop(); }
            KeyCode::Char(c) => { self.path_input.push(c); }
            _ => {}
        }
    }

    pub(crate) fn handle_task_input_key(&mut self, key: KeyEvent) {
        // Goal input mode for SelfEvolve pipeline
        if self.tasks.goal_input {
            match key.code {
                KeyCode::Esc => {
                    self.tasks.goal_input = false;
                    self.tasks.goal_text.clear();
                    self.input_mode = InputMode::Normal;
                }
                KeyCode::Enter => {
                    let goal = self.tasks.goal_text.trim().to_string();
                    let cwd = self.cwd.display().to_string();
                    self.tasks.scheduler.switch_pipeline(
                        crate::pipeline::Pipeline::SelfEvolve, &cwd, &goal,
                    );
                    self.tasks.selected_idx = 0;
                    self.tasks.goal_input = false;
                    self.tasks.goal_text.clear();
                    self.input_mode = InputMode::Normal;
                }
                KeyCode::Backspace => { self.tasks.goal_text.pop(); }
                KeyCode::Char(c) => { self.tasks.goal_text.push(c); }
                _ => {}
            }
            return;
        }

        match key.code {
            KeyCode::Esc => {
                self.tasks.show_input = false;
                self.tasks.input.clear();
                self.input_mode = InputMode::Normal;
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if !self.tasks.input.is_empty() {
                    let ok = crate::platform::copy_to_clipboard(&self.tasks.input);
                    self.clipboard_msg = Some(
                        if ok { "Copied!".into() } else { "Copy failed".into() },
                    );
                }
            }
            KeyCode::Enter => {
                let cmd = self.tasks.input.trim().to_string();
                if !cmd.is_empty() {
                    self.spawn_task(cmd);
                }
                self.tasks.show_input = false;
                self.tasks.input.clear();
                self.input_mode = InputMode::Normal;
            }
            KeyCode::Backspace => {
                self.tasks.input.pop();
            }
            KeyCode::Char(c) => {
                self.tasks.input.push(c);
            }
            _ => {}
        }
    }

    fn spawn_task(&mut self, command: String) {
        let (tx, rx) = mpsc::channel();
        self.tasks.items.push(BackgroundTask {
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

    pub(crate) fn handle_project_list_key(&mut self, key: KeyEvent) {
        let len = self.search.project_indices.len();
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
                self.search.query.clear();
            }
            KeyCode::Char('c') => self.view_claude_md(),
            _ => {}
        }
    }

    pub(crate) fn handle_session_list_key(&mut self, key: KeyEvent) {
        // Clear clipboard feedback on any key
        self.clipboard_msg = None;

        let len = self.search.session_indices.len();
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Backspace => {
                self.view = View::ProjectList;
                self.search.query.clear();
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
                self.search.query.clear();
            }
            KeyCode::Char('r') => {
                self.input_mode = InputMode::RgInput;
                self.search.rg_query.clear();
            }
            KeyCode::Char('R') => self.clear_rg_filter(),
            _ => {}
        }
    }

    pub(crate) fn handle_conversation_key(&mut self, key: KeyEvent) {
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

    pub(crate) fn handle_claude_md_key(&mut self, key: KeyEvent) {
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
        if let Some(&real_idx) = self.search.project_indices.get(self.project_idx) {
            let project = &self.projects[real_idx];
            self.sessions = crate::data::load_sessions(&project.claude_dir);
            self.search.session_indices = (0..self.sessions.len()).collect();
            self.session_idx = 0;
            self.search.query.clear();
            self.search.rg_active = false;
            self.search.rg_matches.clear();
            self.search.rg_query.clear();
            self.view = View::SessionList;
        }
    }

    fn enter_conversation(&mut self) {
        if let Some(&real_idx) = self.search.session_indices.get(self.session_idx) {
            let session = &self.sessions[real_idx];
            self.conversation = crate::data::load_conversation(&session.jsonl_path);
            self.scroll_offset = 0;
            self.view = View::Conversation;
        }
    }

    fn view_claude_md(&mut self) {
        if let Some(&real_idx) = self.search.project_indices.get(self.project_idx) {
            let project = &self.projects[real_idx];
            if let Some(content) = crate::data::load_claude_md(project) {
                self.claude_md_content = content;
                self.scroll_offset = 0;
                self.view = View::ClaudeMdViewer;
            }
        }
    }
}
