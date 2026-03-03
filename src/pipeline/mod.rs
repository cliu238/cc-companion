mod example;
mod self_evolve;

use chrono::Utc;
use crate::app::UsageStatus;

const TRIGGER_MINUTES_5H: i64 = 30;
const TRIGGER_MINUTES_7D: i64 = 24 * 60;
const TRIGGER_MAX_5H_PCT: f64 = 90.0;
const TRIGGER_MAX_7D_PCT: f64 = 95.0;

#[derive(Clone)]
pub struct AutoTask {
    pub name: String,
    pub prompt: String,
    pub cwd: String,
    pub read_only: bool,
    pub resume: bool,
    pub setup: Option<String>,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Pipeline {
    Example,
    SelfEvolve,
}

impl Pipeline {
    pub fn label(&self) -> &str {
        match self {
            Self::Example => "Example",
            Self::SelfEvolve => "Self-Evolve",
        }
    }

    pub fn initial_tasks(&self, project_cwd: &str, goal: &str) -> Vec<AutoTask> {
        match self {
            Self::Example => example::tasks(project_cwd),
            Self::SelfEvolve => self_evolve::initial_tasks(project_cwd, goal),
        }
    }

    pub fn on_complete(&self, task_name: &str, output: &str, project_cwd: &str, goal: &str) -> Vec<AutoTask> {
        match self {
            Self::Example => vec![],
            Self::SelfEvolve => self_evolve::on_complete(task_name, output, project_cwd, goal),
        }
    }
}

pub struct Scheduler {
    pub enabled: bool,
    pub pipeline: Pipeline,
    pub goal: String,
    pub tasks: Vec<AutoTask>,
    pub running: Option<String>,
    pub running_resume: bool,
    pub done: Vec<String>,
}

impl Scheduler {
    pub fn new(pipeline: Pipeline, project_cwd: &str, goal: &str) -> Self {
        Self {
            enabled: false,
            pipeline,
            goal: goal.to_string(),
            tasks: pipeline.initial_tasks(project_cwd, goal),
            running: None,
            running_resume: false,
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
        self.running_resume = task.resume;
        task
    }

    pub fn run_task(&mut self, idx: usize) -> AutoTask {
        let task = self.tasks.remove(idx);
        self.running = Some(task.name.clone());
        self.running_resume = task.resume;
        task
    }

    pub fn complete_running(&mut self, output: &str, project_cwd: &str) {
        if let Some(name) = self.running.take() {
            let new_tasks = self.pipeline.on_complete(&name, output, project_cwd, &self.goal);
            self.tasks.extend(new_tasks);
            self.done.push(name);
        }
    }

    pub fn switch_pipeline(&mut self, pipeline: Pipeline, project_cwd: &str, goal: &str) {
        self.pipeline = pipeline;
        self.goal = goal.to_string();
        self.tasks = pipeline.initial_tasks(project_cwd, goal);
        self.running = None;
        self.running_resume = false;
        self.done.clear();
    }

    pub fn toggle(&mut self) {
        self.enabled = !self.enabled;
    }
}
