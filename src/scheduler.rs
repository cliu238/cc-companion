use chrono::Utc;
use crate::app::UsageStatus;

const TRIGGER_MINUTES_5H: i64 = 30;
const TRIGGER_MINUTES_7D: i64 = 24 * 60;
const TRIGGER_MAX_5H_PCT: f64 = 90.0;
const TRIGGER_MAX_7D_PCT: f64 = 95.0;

pub struct AutoTask {
    pub name: String,
    pub prompt: String,
    pub cwd: String,
}

pub struct Scheduler {
    pub enabled: bool,
    pub tasks: Vec<AutoTask>,
    pub running: Option<String>,
    pub done: Vec<String>,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            enabled: false,
            tasks: default_tasks(),
            running: None,
            done: Vec::new(),
        }
    }

    pub fn should_launch(&self, usage: &UsageStatus, chat_waiting: bool) -> bool {
        if !self.enabled || self.tasks.is_empty() || chat_waiting {
            return false;
        }

        let now = Utc::now();

        let five_hour_trigger = usage
            .five_hour_resets_at
            .map(|reset| {
                let mins_left = (reset - now).num_minutes();
                mins_left <= TRIGGER_MINUTES_5H && usage.five_hour_pct < TRIGGER_MAX_5H_PCT
            })
            .unwrap_or(false);

        let seven_day_trigger = usage
            .seven_day_resets_at
            .map(|reset| {
                let mins_left = (reset - now).num_minutes();
                mins_left <= TRIGGER_MINUTES_7D && usage.seven_day_pct < TRIGGER_MAX_7D_PCT
            })
            .unwrap_or(false);

        five_hour_trigger || seven_day_trigger
    }

    pub fn next_task(&mut self) -> AutoTask {
        let task = self.tasks.remove(0);
        self.running = Some(task.name.clone());
        task
    }

    pub fn complete_running(&mut self) {
        if let Some(name) = self.running.take() {
            self.done.push(name);
        }
    }

    pub fn toggle(&mut self) {
        self.enabled = !self.enabled;
    }
}

fn default_tasks() -> Vec<AutoTask> {
    vec![
        AutoTask {
            name: "code review".into(),
            prompt: "Review this project's codebase. Focus on bugs, error handling gaps, and logic issues. Be concise.".into(),
            cwd: "/Users/ericliu/projects5/cc-companion".into(),
        },
        AutoTask {
            name: "write tests".into(),
            prompt: "Identify the most critical untested code paths in this project and write unit tests for them.".into(),
            cwd: "/Users/ericliu/projects5/cc-companion".into(),
        },
        AutoTask {
            name: "refactor suggestions".into(),
            prompt: "Identify code duplication, overly complex functions, or structural issues in this project. Suggest specific refactoring changes.".into(),
            cwd: "/Users/ericliu/projects5/cc-companion".into(),
        },
        AutoTask {
            name: "update docs".into(),
            prompt: "Review the README and CLAUDE.md for this project. Suggest updates to reflect the current codebase state.".into(),
            cwd: "/Users/ericliu/projects5/cc-companion".into(),
        },
    ]
}
