use super::AutoTask;

const LOAD_SKILLS: &str = "load-skills";
const RUN_TESTS: &str = "run-tests";
const IMPLEMENT: &str = "implement-issue";
const VERIFY: &str = "verify";
const FINISH: &str = "finish";

pub fn initial_tasks(project_cwd: &str, _goal: &str) -> Vec<AutoTask> {
    vec![AutoTask {
        name: LOAD_SKILLS.into(),
        prompt: "Load `/domain-knowledge` skills (if not found, skip but display a warning \
                 that domain-knowledge skills should exist for this project).".into(),
        cwd: project_cwd.to_string(),
        read_only: false,
        resume: false,
        setup: None,
        use_advisor: false,
    }]
}

pub fn on_complete(task_name: &str, output: &str, project_cwd: &str, goal: &str) -> Vec<AutoTask> {
    // Halt signals
    if output.contains("ISSUES_EMPTY") || output.contains("FAILED=") {
        return vec![];
    }

    match task_name {
        LOAD_SKILLS => vec![AutoTask {
            name: RUN_TESTS.into(),
            prompt: "Use the Skill tool to invoke the `test` skill to run all tests. \
                     If the `test` skill is missing, create it at \
                     .claude/skills/test/SKILL.md that auto-detects the project's test framework.".into(),
            cwd: project_cwd.to_string(),
            read_only: false,
            resume: false,
            setup: None,
            use_advisor: false,
        }],

        RUN_TESTS => {
            let label_filter = if goal.is_empty() {
                String::new()
            } else {
                format!(" Use `gh issue list --label {}` to filter.", goal)
            };
            vec![AutoTask {
                name: IMPLEMENT.into(),
                prompt: format!(
                    "/superpowers:verification-before-completion\n\n\
                     - First, check for the next open GitHub issue (gh cli).{label_filter} \
                     If there are no open issues, output ISSUES_EMPTY and stop.\n\n\
                     - Create a worktree for this issue: \
                     `git worktree add .worktrees/issue-<NUMBER> -b issue-driven-<NUMBER>` \
                     (where <NUMBER> is the issue number), then cd into it.\n\n\
                     - Use `superpowers:test-driven-development` skill and the `test` skill to fix it \
                     following strict red->green->refactor TDD.\n\n\
                     - Unit tests alone are NOT sufficient for user-visible changes. \
                     Choose test level based on what changed: \
                     logic/data → unit tests, UI rendering → TestBackend render tests, \
                     user interaction flows → PTY E2E tests (expectrl). \
                     At minimum, add one E2E test proving the feature works end-to-end.\n\n\
                     - Use the Skill tool to invoke the `test` skill to verify all tests pass.\n\n\
                     - Submit the PR and use `superpowers:requesting-code-review` to request a review.\n\n\
                     Previous test output:\n{output}"
                ),
                cwd: project_cwd.to_string(),
                read_only: false,
                resume: true,
                setup: None,
                use_advisor: false,
            }]
        }

        IMPLEMENT => vec![AutoTask {
            name: VERIFY.into(),
            prompt: "/superpowers:verification-before-completion\n\n\
                     - Use the Skill tool to invoke the `test` skill to run the full test suite, including e2e tests and all new tests added.\n\n\
                     - If any tests cannot be run (e.g. #[ignore] tests requiring API keys or \
                     manual terminal interaction), list them clearly with the exact command \
                     the user should run manually. Do NOT block the pipeline for these.\n\n\
                     - Update the `test` skill only if: new test types were introduced, \
                     existing commands no longer work, or coverage gaps were discovered.".into(),
            cwd: project_cwd.to_string(),
            read_only: false,
            resume: true,
            setup: None,
            use_advisor: false,
        }],

        VERIFY => vec![AutoTask {
            name: FINISH.into(),
            prompt: "/superpowers:finishing-a-development-branch".into(),
            cwd: project_cwd.to_string(),
            read_only: false,
            resume: true,
            setup: None,
            use_advisor: false,
        }],

        FINISH => vec![],

        _ => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_tasks() {
        let tasks = initial_tasks("/tmp/proj", "");
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].name, LOAD_SKILLS);
        assert!(!tasks[0].read_only);
        assert!(!tasks[0].resume);
    }

    #[test]
    fn test_chain_load_skills_to_run_tests() {
        let tasks = on_complete(LOAD_SKILLS, "Skills loaded", "/tmp/proj", "");
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].name, RUN_TESTS);
        assert!(!tasks[0].read_only);
        assert!(!tasks[0].resume);
    }

    #[test]
    fn test_chain_run_tests_to_implement() {
        let output = "39 passed; 0 failed";
        let tasks = on_complete(RUN_TESTS, output, "/tmp/proj", "bug");
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].name, IMPLEMENT);
        assert!(!tasks[0].read_only);
        assert!(tasks[0].resume);
        assert!(tasks[0].setup.is_none());
        assert!(tasks[0].prompt.contains(output));
        assert!(tasks[0].prompt.contains("bug"));
        assert!(tasks[0].prompt.contains("NOT sufficient"));
        assert!(tasks[0].prompt.contains("ISSUES_EMPTY"));
        assert!(tasks[0].prompt.contains("git worktree add"));
    }

    #[test]
    fn test_chain_implement_to_verify() {
        let tasks = on_complete(IMPLEMENT, "PR created", "/tmp/proj", "");
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].name, VERIFY);
        assert!(!tasks[0].read_only);
        assert!(tasks[0].resume);
    }

    #[test]
    fn test_chain_verify_to_finish() {
        let tasks = on_complete(VERIFY, "All tests pass", "/tmp/proj", "");
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].name, FINISH);
        assert!(!tasks[0].read_only);
        assert!(tasks[0].resume);
    }

    #[test]
    fn test_chain_finish_returns_empty() {
        let tasks = on_complete(FINISH, "Done", "/tmp/proj", "");
        assert!(tasks.is_empty());
    }

    #[test]
    fn test_issues_empty_halts() {
        let tasks = on_complete(IMPLEMENT, "ISSUES_EMPTY", "/tmp/proj", "");
        assert!(tasks.is_empty());
    }

    #[test]
    fn test_failed_halts() {
        let tasks = on_complete(RUN_TESTS, "FAILED=compilation error", "/tmp/proj", "");
        assert!(tasks.is_empty());
    }

    #[test]
    fn test_failed_halts_mid_output() {
        // FAILED= can appear anywhere in output, not just at the start
        let tasks = on_complete(RUN_TESTS, "Some preamble\nFAILED=build error", "/tmp/proj", "");
        assert!(tasks.is_empty());
    }

    #[test]
    fn test_no_setup_worktree_leak() {
        let output = "39 passed";
        let tasks = on_complete(RUN_TESTS, output, "/tmp/proj", "");
        assert!(tasks[0].setup.is_none(), "worktree must be created by agent, not setup");
        assert!(tasks[0].prompt.contains("git worktree add"));
    }

    #[test]
    fn test_goal_as_label_filter_empty() {
        let tasks = on_complete(RUN_TESTS, "ok", "/tmp/proj", "");
        assert_eq!(tasks.len(), 1);
        assert!(!tasks[0].prompt.contains("--label"));
    }

    #[test]
    fn test_goal_as_label_filter_set() {
        let tasks = on_complete(RUN_TESTS, "ok", "/tmp/proj", "enhancement");
        assert_eq!(tasks.len(), 1);
        assert!(tasks[0].prompt.contains("--label"));
        assert!(tasks[0].prompt.contains("enhancement"));
    }
}
