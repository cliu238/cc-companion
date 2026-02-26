use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, InputMode, View};

pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .split(f.area());

    // Title bar
    let title = match &app.view {
        View::ProjectList => "cc-companion | Projects".to_string(),
        View::SessionList => {
            let name = app
                .selected_project()
                .map(|p| p.name.as_str())
                .unwrap_or("?");
            format!("cc-companion | {}", name)
        }
        View::Conversation => {
            let prompt = app
                .selected_session()
                .map(|s| truncate(&s.first_prompt, 60))
                .unwrap_or_default();
            format!("cc-companion | {}", prompt)
        }
        View::ClaudeMdViewer => {
            let name = app
                .selected_project()
                .map(|p| p.name.as_str())
                .unwrap_or("?");
            format!("cc-companion | CLAUDE.md - {}", name)
        }
    };
    let title_bar = Paragraph::new(title).style(
        Style::default()
            .fg(Color::Black)
            .bg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    );
    f.render_widget(title_bar, chunks[0]);

    // Main content
    match &app.view {
        View::ProjectList => draw_project_list(f, app, chunks[1]),
        View::SessionList => draw_session_list(f, app, chunks[1]),
        View::Conversation => draw_conversation(f, app, chunks[1]),
        View::ClaudeMdViewer => draw_claude_md(f, app, chunks[1]),
    }

    // Help bar
    let help = match (&app.view, &app.input_mode) {
        (_, InputMode::Search) => {
            format!("Search: {}_ | Enter=accept Esc=cancel", app.search_query)
        }
        (View::ProjectList, _) => {
            "j/k=move Enter=open /=search c=CLAUDE.md q=quit".to_string()
        }
        (View::SessionList, _) => {
            "j/k=move Enter=open /=search Esc=back".to_string()
        }
        (View::Conversation, _) => {
            "j/k=scroll Ctrl+d/u=page g/G=top/bottom Esc=back".to_string()
        }
        (View::ClaudeMdViewer, _) => "j/k=scroll Esc=back".to_string(),
    };
    let help_bar =
        Paragraph::new(help).style(Style::default().fg(Color::White).bg(Color::DarkGray));
    f.render_widget(help_bar, chunks[2]);
}

fn draw_project_list(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let items: Vec<ListItem> = app
        .filtered_project_indices
        .iter()
        .map(|&i| {
            let p = &app.projects[i];
            let md_marker = if p.has_claude_md { " [MD]" } else { "" };
            let line = Line::from(vec![
                Span::styled(
                    format!("{:>3} sessions", p.session_count),
                    Style::default().fg(Color::Yellow),
                ),
                Span::raw("  "),
                Span::styled(&p.name, Style::default().fg(Color::White)),
                Span::styled(md_marker, Style::default().fg(Color::Green)),
            ]);
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::NONE)
                .title(format!(" {} projects ", app.filtered_project_indices.len())),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );

    let mut state = ListState::default();
    state.select(Some(app.project_idx));
    f.render_stateful_widget(list, area, &mut state);
}

fn draw_session_list(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let items: Vec<ListItem> = app
        .filtered_session_indices
        .iter()
        .map(|&i| {
            let s = &app.sessions[i];
            let date = &s.modified;
            let branch = if s.git_branch.is_empty() {
                String::new()
            } else {
                format!(" [{}]", s.git_branch)
            };
            let prompt = truncate(&s.first_prompt, 80);
            let line = Line::from(vec![
                Span::styled(date, Style::default().fg(Color::Yellow)),
                Span::styled(
                    format!(" {:>3}msg", s.message_count),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(branch, Style::default().fg(Color::Cyan)),
                Span::raw("  "),
                Span::styled(prompt, Style::default().fg(Color::White)),
            ]);
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::NONE)
                .title(format!(" {} sessions ", app.filtered_session_indices.len())),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );

    let mut state = ListState::default();
    state.select(Some(app.session_idx));
    f.render_stateful_widget(list, area, &mut state);
}

fn draw_conversation(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let mut lines: Vec<Line> = Vec::new();

    for msg in &app.conversation {
        let (label, color) = match msg.role.as_str() {
            "user" => ("USER", Color::Green),
            "assistant" => ("ASSISTANT", Color::Blue),
            _ => ("SYSTEM", Color::DarkGray),
        };

        lines.push(Line::from(Span::styled(
            format!("--- {} ---", label),
            Style::default()
                .fg(color)
                .add_modifier(Modifier::BOLD),
        )));

        for text_line in msg.text.lines() {
            let style = if text_line.starts_with("[tool:") || text_line == "[tool result]" {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default().fg(Color::White)
            };
            lines.push(Line::from(Span::styled(text_line, style)));
        }
        lines.push(Line::from(""));
    }

    let paragraph = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((app.scroll_offset, 0));
    f.render_widget(paragraph, area);
}

fn draw_claude_md(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let paragraph = Paragraph::new(app.claude_md_content.as_str())
        .wrap(Wrap { trim: false })
        .scroll((app.scroll_offset, 0));
    f.render_widget(paragraph, area);
}

fn truncate(s: &str, max: usize) -> String {
    let clean: String = s.chars().map(|c| if c == '\n' { ' ' } else { c }).collect();
    if clean.chars().count() > max {
        let truncated: String = clean.chars().take(max).collect();
        format!("{}...", truncated)
    } else {
        clean
    }
}
