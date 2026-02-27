use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, InputMode, Mode, View};

pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .split(f.area());

    // Title bar
    let title = match app.mode {
        Mode::Chat => {
            if app.chat_session_id.is_some() {
                "cc-companion | Chat".to_string()
            } else {
                "cc-companion | Chat (new session)".to_string()
            }
        }
        Mode::LogViewer => match &app.view {
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
        },
    };
    let title_bar = Paragraph::new(title).style(
        Style::default()
            .fg(Color::Black)
            .bg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    );
    f.render_widget(title_bar, chunks[0]);

    // Main content
    match app.mode {
        Mode::Chat => draw_chat(f, app, chunks[1]),
        Mode::LogViewer => match &app.view {
            View::ProjectList => draw_project_list(f, app, chunks[1]),
            View::SessionList => draw_session_list(f, app, chunks[1]),
            View::Conversation => draw_conversation(f, app, chunks[1]),
            View::ClaudeMdViewer => draw_claude_md(f, app, chunks[1]),
        },
    }

    // Help bar
    let help = match app.mode {
        Mode::Chat => match app.input_mode {
            InputMode::ChatInput => "Enter=send Esc=cancel | Shift+Tab=logs".to_string(),
            _ => "i=type j/k=scroll n=new session q=quit | Shift+Tab=logs".to_string(),
        },
        Mode::LogViewer => match (&app.view, &app.input_mode) {
            (_, InputMode::Search) => {
                format!("Search: {}_ | Enter=accept Esc=cancel", app.search_query)
            }
            (View::ProjectList, _) => {
                "j/k=move Enter=open /=search c=CLAUDE.md q=quit | Shift+Tab=chat".to_string()
            }
            (View::SessionList, _) => "j/k=move Enter=open /=search Esc=back".to_string(),
            (View::Conversation, _) => {
                "j/k=scroll Ctrl+d/u=page g/G=top/bottom Esc=back".to_string()
            }
            (View::ClaudeMdViewer, _) => "j/k=scroll Esc=back".to_string(),
        },
    };
    let help_bar =
        Paragraph::new(help).style(Style::default().fg(Color::White).bg(Color::DarkGray));
    f.render_widget(help_bar, chunks[2]);
}

fn draw_chat(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let input_height = 3u16;
    let chat_chunks = Layout::vertical([
        Constraint::Min(0),
        Constraint::Length(1),
        Constraint::Length(input_height),
    ])
    .split(area);

    // Messages area
    let mut lines: Vec<Line> = Vec::new();

    if app.chat_messages.is_empty() && !app.chat_waiting {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Hello! How can I help?",
            Style::default().fg(Color::Cyan),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Press 'i' or Enter to start typing.",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        for (role, content) in &app.chat_messages {
            let (label, color) = match role.as_str() {
                "user" => ("You:", Color::Green),
                _ => ("Claude:", Color::Blue),
            };
            lines.push(Line::from(Span::styled(
                label,
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            )));
            for text_line in content.lines() {
                lines.push(Line::from(Span::styled(
                    format!("  {}", text_line),
                    Style::default().fg(Color::White),
                )));
            }
            lines.push(Line::from(""));
        }
    }

    if app.chat_waiting {
        lines.push(Line::from(Span::styled(
            "Thinking...",
            Style::default().fg(Color::Yellow),
        )));
    }

    if let Some(err) = &app.chat_error {
        lines.push(Line::from(Span::styled(
            format!("Error: {}", err),
            Style::default().fg(Color::Red),
        )));
    }

    let messages = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((app.chat_scroll, 0));
    f.render_widget(messages, chat_chunks[0]);

    // Session bar
    let session_text = match &app.chat_session_id {
        Some(id) => format!(" Session: {}", &id[..id.len().min(8)]),
        None => " Session: (new)".to_string(),
    };
    let session_bar =
        Paragraph::new(session_text).style(Style::default().fg(Color::DarkGray));
    f.render_widget(session_bar, chat_chunks[1]);

    // Input box
    let is_input_active = matches!(app.input_mode, InputMode::ChatInput);
    let border_color = if is_input_active {
        Color::Cyan
    } else {
        Color::DarkGray
    };
    let input = Paragraph::new(app.chat_input.as_str())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color))
                .title(" Message "),
        )
        .wrap(Wrap { trim: false });
    f.render_widget(input, chat_chunks[2]);

    // Show cursor in input box when active
    if is_input_active {
        let cursor_x = chat_chunks[2].x + app.chat_input.len() as u16 + 1;
        let cursor_y = chat_chunks[2].y + 1;
        f.set_cursor_position((cursor_x, cursor_y));
    }
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
