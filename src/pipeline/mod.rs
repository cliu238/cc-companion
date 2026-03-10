mod example;
mod issue_driven;

use chrono::Utc;
use crate::app::UsageStatus;

const TRIGGER_MINUTES_5H: i64 = 30;
const TRIGGER_MINUTES_7D: i64 = 24 * 60;
const TRIGGER_MAX_5H_PCT: f64 = 90.0;
const TRIGGER_MAX_7D_PCT: f64 = 95.0;

const AUTONOMOUS_PREAMBLE: &str = "\
You are running in an automated pipeline with no human operator.\n\
Complete the task fully and autonomously. Do not ask questions or wait for confirmation.\n\
Make decisions yourself. When done, output your final result and stop.\n\
If you are blocked and cannot proceed, output FAILED=<reason> and stop.\n\n";

#[derive(Clone)]
pub struct AutoTask {
    pub name: String,
    pub prompt: String,
    pub cwd: String,
    pub read_only: bool,
    pub resume: bool,
    pub setup: Option<String>,
    pub use_advisor: bool,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Pipeline {
    Example,
    IssueDriven,
}

impl Pipeline {
    pub fn label(&self) -> &str {
        match self {
            Self::Example => "Example",
            Self::IssueDriven => "Issue-Driven",
        }
    }

    pub fn description(&self) -> &str {
        match self {
            Self::Example => "Read-only analysis tasks (code review, tests, refactor, docs)",
            Self::IssueDriven => "Fetch GitHub issues, implement via TDD + worktree PRs",
        }
    }

    pub fn all() -> &'static [Pipeline] {
        &[Pipeline::Example, Pipeline::IssueDriven]
    }

    pub fn initial_tasks(&self, project_cwd: &str, goal: &str) -> Vec<AutoTask> {
        match self {
            Self::Example => example::tasks(project_cwd),
            Self::IssueDriven => issue_driven::initial_tasks(project_cwd, goal),
        }
    }

    pub fn on_complete(&self, task_name: &str, output: &str, project_cwd: &str, goal: &str) -> Vec<AutoTask> {
        match self {
            Self::Example => vec![],
            Self::IssueDriven => issue_driven::on_complete(task_name, output, project_cwd, goal),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};

    fn make_usage(five_pct: f64, five_mins_left: i64, seven_pct: f64, seven_mins_left: i64) -> UsageStatus {
        let now = Utc::now();
        UsageStatus {
            five_hour_pct: five_pct,
            five_hour_resets_at: Some(now + Duration::minutes(five_mins_left)),
            seven_day_pct: seven_pct,
            seven_day_resets_at: Some(now + Duration::minutes(seven_mins_left)),
            seven_day_sonnet_pct: None,
            last_fetched: Utc::now(),
        }
    }

    #[test]
    fn test_pipeline_labels() {
        assert_eq!(Pipeline::Example.label(), "Example");
        assert_eq!(Pipeline::IssueDriven.label(), "Issue-Driven");
        assert!(!Pipeline::Example.description().is_empty());
        assert!(!Pipeline::IssueDriven.description().is_empty());
    }

    #[test]
    fn test_should_launch_disabled() {
        let mut sched = Scheduler::new(Pipeline::Example, "/tmp", "");
        sched.enabled = false;
        let usage = make_usage(10.0, 5, 10.0, 60);
        assert!(!sched.should_launch(&usage, false));
    }

    #[test]
    fn test_should_launch_triggers() {
        let mut sched = Scheduler::new(Pipeline::Example, "/tmp", "");
        sched.enabled = true;
        // Low usage, 10 mins to 5h reset (within 30min trigger)
        let usage = make_usage(50.0, 10, 50.0, 60);
        assert!(sched.should_launch(&usage, false));
    }

    #[test]
    fn test_should_launch_too_high_pct() {
        let mut sched = Scheduler::new(Pipeline::Example, "/tmp", "");
        sched.enabled = true;
        // High usage even though close to reset
        let usage = make_usage(95.0, 10, 96.0, 60);
        assert!(!sched.should_launch(&usage, false));
    }

    #[test]
    fn test_complete_running_cycles_issue_driven() {
        let mut sched = Scheduler::new(Pipeline::IssueDriven, "/tmp", "");
        // Simulate running the final "finish" task
        sched.tasks.clear();
        sched.running = Some("finish".into());
        sched.complete_running("Done", "/tmp");
        // Should cycle: queue gets refilled with initial tasks
        assert!(!sched.tasks.is_empty(), "IssueDriven should restart after normal completion");
    }

    #[test]
    fn test_complete_running_halts_on_issues_empty() {
        let mut sched = Scheduler::new(Pipeline::IssueDriven, "/tmp", "");
        sched.tasks.clear();
        sched.running = Some("implement-issue".into());
        sched.complete_running("ISSUES_EMPTY", "/tmp");
        // Should NOT cycle: halt signal stops restart
        assert!(sched.tasks.is_empty(), "IssueDriven must halt when ISSUES_EMPTY");
    }

    #[test]
    fn test_complete_running_halts_on_failed() {
        let mut sched = Scheduler::new(Pipeline::IssueDriven, "/tmp", "");
        sched.tasks.clear();
        sched.running = Some("run-tests".into());
        sched.complete_running("FAILED=compilation error", "/tmp");
        // Should NOT cycle: halt signal stops restart
        assert!(sched.tasks.is_empty(), "IssueDriven must halt when FAILED=");
    }

    #[test]
    fn test_issue_driven_tasks_skip_advisor() {
        let tasks = Pipeline::IssueDriven.initial_tasks("/tmp/proj", "");
        for task in &tasks {
            assert!(!task.use_advisor, "IssueDriven task '{}' should not use advisor", task.name);
        }
    }

    #[test]
    fn test_example_tasks_use_advisor() {
        let tasks = Pipeline::Example.initial_tasks("/tmp/proj", "");
        for task in &tasks {
            assert!(task.use_advisor, "Example task '{}' should use advisor", task.name);
        }
    }

    #[test]
    fn test_should_launch_too_far() {
        let mut sched = Scheduler::new(Pipeline::Example, "/tmp", "");
        sched.enabled = true;
        // Low usage but far from reset
        let usage = make_usage(50.0, 200, 50.0, 2000);
        assert!(!sched.should_launch(&usage, false));
    }

    #[test]
    fn test_next_task_prepends_preamble() {
        let mut sched = Scheduler::new(Pipeline::Example, "/tmp", "");
        sched.enabled = true;
        assert!(!sched.tasks.is_empty());
        let task = sched.next_task();
        assert!(task.prompt.starts_with("You are running in an automated pipeline"),
            "next_task() should prepend autonomous preamble to prompt");
    }

    #[test]
    fn test_run_task_prepends_preamble() {
        let mut sched = Scheduler::new(Pipeline::Example, "/tmp", "");
        sched.enabled = true;
        assert!(!sched.tasks.is_empty());
        let task = sched.run_task(0);
        assert!(task.prompt.starts_with("You are running in an automated pipeline"),
            "run_task() should prepend autonomous preamble to prompt");
    }

    #[test]
    fn test_should_launch_fallback_low_usage() {
        let mut sched = Scheduler::new(Pipeline::Example, "/tmp", "");
        sched.enabled = true;
        // Low usage, but reset is far away (2 hours for 5h, 2 days for 7d)
        // Neither near-reset trigger fires, but fallback should
        let usage = make_usage(20.0, 120, 30.0, 2 * 24 * 60);
        assert!(sched.should_launch(&usage, false), "fallback should fire when usage is low");
    }

    #[test]
    fn test_should_launch_fallback_blocked_by_high_5h() {
        let mut sched = Scheduler::new(Pipeline::Example, "/tmp", "");
        sched.enabled = true;
        // 5h usage above fallback cap, 7d low, reset far away
        let usage = make_usage(85.0, 120, 30.0, 2 * 24 * 60);
        assert!(!sched.should_launch(&usage, false), "fallback must not fire when 5h usage high");
    }

    #[test]
    fn test_should_launch_fallback_blocked_by_high_7d() {
        let mut sched = Scheduler::new(Pipeline::Example, "/tmp", "");
        sched.enabled = true;
        // 5h low, 7d usage above fallback cap, reset far away
        let usage = make_usage(20.0, 120, 92.0, 2 * 24 * 60);
        assert!(!sched.should_launch(&usage, false), "fallback must not fire when 7d usage high");
    }

    #[test]
    fn test_should_launch_fallback_none_resets_at() {
        let mut sched = Scheduler::new(Pipeline::Example, "/tmp", "");
        sched.enabled = true;
        // resets_at is None (0% usage, API returns null) — fallback should still fire
        let usage = UsageStatus {
            five_hour_pct: 0.0,
            five_hour_resets_at: None,
            seven_day_pct: 22.0,
            seven_day_resets_at: Some(Utc::now() + Duration::minutes(2 * 24 * 60)),
            seven_day_sonnet_pct: Some(4.0),
            last_fetched: Utc::now(),
        };
        assert!(sched.should_launch(&usage, false), "fallback should fire even when resets_at is None");
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
        self.prepare_task(0)
    }

    pub fn run_task(&mut self, idx: usize) -> AutoTask {
        self.prepare_task(idx)
    }

    fn prepare_task(&mut self, idx: usize) -> AutoTask {
        let mut task = self.tasks.remove(idx);
        self.running = Some(task.name.clone());
        self.running_resume = task.resume;
        task.prompt = format!("{}{}", AUTONOMOUS_PREAMBLE, task.prompt);
        task
    }

    pub fn complete_running(&mut self, output: &str, project_cwd: &str) {
        if let Some(name) = self.running.take() {
            let new_tasks = self.pipeline.on_complete(&name, output, project_cwd, &self.goal);
            self.tasks.extend(new_tasks);
            self.done.push(name);
            // Cycle: start new round when queue is empty (IssueDriven only),
            // but not when halted by signals.
            let halted = output.contains("ISSUES_EMPTY") || output.contains("FAILED=");
            if self.tasks.is_empty() && self.pipeline == Pipeline::IssueDriven && !halted {
                self.tasks = self.pipeline.initial_tasks(project_cwd, &self.goal);
            }
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
