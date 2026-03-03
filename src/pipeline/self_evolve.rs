use super::AutoTask;

pub fn initial_tasks(project_cwd: &str, goal: &str) -> Vec<AutoTask> {
    let goal_part = if goal.is_empty() {
        String::new()
    } else {
        format!("\n\nGoal: {}", goal)
    };

    vec![AutoTask {
        name: "overview".into(),
        prompt: format!(
            "Analyze this project thoroughly. Read CLAUDE.md, main source files, and understand the architecture.\n\
             Then output a JSON array of 3-5 concrete improvements you'd make. Each item should have:\n\
             - \"name\": short slug (lowercase, hyphens, e.g. \"fix-error-handling\")\n\
             - \"description\": one paragraph describing what to implement\n\n\
             Output ONLY the JSON array, no other text.{goal_part}"
        ),
        cwd: project_cwd.to_string(),
        read_only: true,
        resume: true,
        setup: None,
    }]
}

pub fn on_complete(task_name: &str, output: &str, project_cwd: &str, _goal: &str) -> Vec<AutoTask> {
    if task_name != "overview" {
        return vec![];
    }

    let improvements = parse_improvements(output);
    if improvements.is_empty() {
        return vec![AutoTask {
            name: "overview-failed".into(),
            prompt: "The previous overview task did not produce valid JSON. Please try again: analyze the project and output a JSON array of improvements with \"name\" and \"description\" fields.".into(),
            cwd: project_cwd.to_string(),
            read_only: true,
            resume: true,
            setup: None,
        }];
    }

    improvements
        .into_iter()
        .map(|(slug, description)| {
            let worktree_path = format!("{}/.worktrees/{}", project_cwd, slug);
            let setup_cmd = format!(
                "git -C {} worktree add {} -b improve/{}",
                project_cwd, worktree_path, slug
            );

            AutoTask {
                name: slug.clone(),
                prompt: format!(
                    "Implement this improvement:\n\n{}\n\n\
                     After implementing, run:\n\
                     git add -A && git commit -m \"improve: {}\"\n\
                     gh pr create --base main --title \"improve: {}\" --body \"{}\"",
                    description, slug, slug,
                    description.replace('"', "'"),
                ),
                cwd: worktree_path,
                read_only: false,
                resume: false,
                setup: Some(setup_cmd),
            }
        })
        .collect()
}

fn parse_improvements(output: &str) -> Vec<(String, String)> {
    // Find JSON array in output (may be surrounded by text)
    let start = output.find('[');
    let end = output.rfind(']');
    let (start, end) = match (start, end) {
        (Some(s), Some(e)) if e > s => (s, e),
        _ => return vec![],
    };

    let json_str = &output[start..=end];
    let arr: Vec<serde_json::Value> = match serde_json::from_str(json_str) {
        Ok(v) => v,
        Err(_) => return vec![],
    };

    arr.iter()
        .filter_map(|item| {
            let name = item.get("name")?.as_str()?.to_string();
            let desc = item.get("description")?.as_str()?.to_string();
            Some((name, desc))
        })
        .collect()
}
