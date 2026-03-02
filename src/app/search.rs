use std::collections::HashMap;
use std::process::Command;
use std::sync::mpsc;
use std::thread;

use crossterm::event::{KeyCode, KeyEvent};

use super::{App, InputMode, Mode, View};

impl App {
    pub(crate) fn handle_search_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.search.query.clear();
                self.refilter();
            }
            KeyCode::Enter => {
                self.input_mode = InputMode::Normal;
            }
            KeyCode::Backspace => {
                self.search.query.pop();
                self.refilter();
            }
            KeyCode::Char(c) => {
                self.search.query.push(c);
                self.refilter();
            }
            _ => {}
        }
    }

    pub(crate) fn handle_rg_input_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.search.rg_query.clear();
            }
            KeyCode::Enter => {
                let query = self.search.rg_query.trim().to_string();
                if !query.is_empty() {
                    self.run_rg_search(&query);
                }
                self.input_mode = InputMode::Normal;
            }
            KeyCode::Backspace => {
                self.search.rg_query.pop();
            }
            KeyCode::Char(c) => {
                self.search.rg_query.push(c);
            }
            _ => {}
        }
    }

    fn run_rg_search(&mut self, query: &str) {
        let Some(project) = self.selected_project() else { return };
        let claude_dir = project.claude_dir.clone();
        let query = query.to_string();
        let (tx, rx) = mpsc::channel();
        self.search.rg_rx = Some(rx);

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

    pub(crate) fn clear_rg_filter(&mut self) {
        self.search.rg_active = false;
        self.search.rg_matches.clear();
        self.search.rg_query.clear();
        self.search.rg_rx = None;
        self.search.session_indices = (0..self.sessions.len()).collect();
        self.session_idx = 0;
    }

    pub(crate) fn refilter(&mut self) {
        let query = self.search.query.to_lowercase();
        // ProjectSelect mode reuses the same project filtering
        if self.mode == Mode::ProjectSelect {
            if query.is_empty() {
                self.search.project_indices = (0..self.projects.len()).collect();
            } else {
                self.search.project_indices = self
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
                    self.search.project_indices = (0..self.projects.len()).collect();
                } else {
                    self.search.project_indices = self
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
                    self.search.session_indices = (0..self.sessions.len()).collect();
                } else {
                    self.search.session_indices = self
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
}
