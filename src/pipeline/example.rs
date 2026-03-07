use super::AutoTask;

pub fn tasks(project_cwd: &str) -> Vec<AutoTask> {
    let cwd = project_cwd.to_string();
    vec![
        AutoTask {
            name: "code review".into(),
            prompt: "Review this project's codebase. Focus on bugs, error handling gaps, and logic issues. Be concise.".into(),
            cwd: cwd.clone(),
            read_only: true,
            resume: true,
            setup: None,
            use_advisor: true,
        },
        AutoTask {
            name: "write tests".into(),
            prompt: "Identify the most critical untested code paths in this project and write unit tests for them.".into(),
            cwd: cwd.clone(),
            read_only: true,
            resume: true,
            setup: None,
            use_advisor: true,
        },
        AutoTask {
            name: "refactor suggestions".into(),
            prompt: "Identify code duplication, overly complex functions, or structural issues in this project. Suggest specific refactoring changes.".into(),
            cwd: cwd.clone(),
            read_only: true,
            resume: true,
            setup: None,
            use_advisor: true,
        },
        AutoTask {
            name: "update docs".into(),
            prompt: "Review the README and CLAUDE.md for this project. Suggest updates to reflect the current codebase state.".into(),
            cwd,
            read_only: true,
            resume: true,
            setup: None,
            use_advisor: true,
        },
    ]
}
