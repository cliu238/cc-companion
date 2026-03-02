use chrono::Utc;

use crate::app::UsageStatus;

const TRIGGER_MINUTES_5H: i64 = 30;
const TRIGGER_MINUTES_7D: i64 = 24 * 60; // 24 hours
const TRIGGER_MAX_5H_PCT: f64 = 90.0;
const TRIGGER_MAX_7D_PCT: f64 = 95.0;

pub struct AutoTaskDef {
    pub name: &'static str,
    pub prompt: &'static str,
    pub cwd: &'static str,
}

pub const AUTO_TASKS: &[AutoTaskDef] = &[
    AutoTaskDef {
        name: "code review",
        prompt: "Review this project's codebase. Focus on bugs, error handling gaps, and logic issues. Be concise.",
        cwd: "/Users/ericliu/projects5/cc-companion",
    },
    AutoTaskDef {
        name: "write tests",
        prompt: "Identify the most critical untested code paths in this project and write unit tests for them.",
        cwd: "/Users/ericliu/projects5/cc-companion",
    },
    AutoTaskDef {
        name: "refactor suggestions",
        prompt: "Identify code duplication, overly complex functions, or structural issues in this project. Suggest specific refactoring changes.",
        cwd: "/Users/ericliu/projects5/cc-companion",
    },
    AutoTaskDef {
        name: "update docs",
        prompt: "Review the README and CLAUDE.md for this project. Suggest updates to reflect the current codebase state.",
        cwd: "/Users/ericliu/projects5/cc-companion",
    },
];

pub struct Scheduler {
    pub enabled: bool,
    pub task_idx: usize,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            enabled: false,
            task_idx: 0,
        }
    }

    pub fn should_launch(&self, usage: &UsageStatus, chat_waiting: bool) -> bool {
        if !self.enabled || AUTO_TASKS.is_empty() || chat_waiting {
            return false;
        }

        let now = Utc::now();

        // 5h window: resets soon + low utilization
        let five_hour_trigger = match usage.five_hour_resets_at {
            Some(end) => {
                let remaining = end.signed_duration_since(now).num_minutes();
                remaining >= 0
                    && remaining <= TRIGGER_MINUTES_5H
                    && usage.five_hour_pct < TRIGGER_MAX_5H_PCT
            }
            None => false,
        };

        // 7d window: resets soon + low utilization
        let seven_day_trigger = match usage.seven_day_resets_at {
            Some(end) => {
                let remaining = end.signed_duration_since(now).num_minutes();
                remaining >= 0
                    && remaining <= TRIGGER_MINUTES_7D
                    && usage.seven_day_pct < TRIGGER_MAX_7D_PCT
            }
            None => false,
        };

        five_hour_trigger || seven_day_trigger
    }

    pub fn next_task(&mut self) -> &'static AutoTaskDef {
        let task = &AUTO_TASKS[self.task_idx % AUTO_TASKS.len()];
        self.task_idx = (self.task_idx + 1) % AUTO_TASKS.len();
        task
    }

    pub fn toggle(&mut self) {
        self.enabled = !self.enabled;
    }
}
