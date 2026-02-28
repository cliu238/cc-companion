use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, InputMode, Mode, TaskStatus, View};

pub fn draw(f: &mut Frame, app: &mut App) {
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
            InputMode::TaskInput => "Enter=run Esc=cancel".to_string(),
            _ if app.show_task_list => "j/k=select Ctrl+d/u=scroll D=delete Esc=close".to_string(),
            _ => "i=type x=task X=tasks j/k=scroll n=new t=tone g=gw q=quit | Shift+Tab=logs".to_string(),
        },
        Mode::LogViewer => match (&app.view, &app.input_mode) {
            (_, InputMode::Search) => {
                format!("Search: {}_ | Enter=accept Esc=cancel", app.search_query)
            }
            (View::ProjectList, _) => {
                "j/k=move Enter=open /=search c=CLAUDE.md q=quit | Shift+Tab=chat".to_string()
            }
            (View::SessionList, _) => "j/k=move Enter=open /=search y=copy id Esc=back".to_string(),
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

fn draw_chat(f: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    let input_height = 3u16;
    let chat_chunks = Layout::vertical([
        Constraint::Min(0),
        Constraint::Length(1),
        Constraint::Length(input_height),
    ])
    .split(area);

    // Messages area
    let mut lines: Vec<Line> = Vec::new();
    let msg_count = app.chat_messages.len();
    let highlight_last = app.new_msg_at.is_some_and(|t| t.elapsed().as_secs() < 2);

    for (i, (role, content)) in app.chat_messages.iter().enumerate() {
        let is_new = highlight_last && i == msg_count - 1;
        let (label, color) = match role.as_str() {
            "user" => ("You:", Color::Green),
            _ => ("Claude:", if is_new { Color::Yellow } else { Color::Blue }),
        };
        let mut label_style = Style::default().fg(color).add_modifier(Modifier::BOLD);
        if is_new {
            label_style = label_style.add_modifier(Modifier::REVERSED);
        }
        lines.push(Line::from(Span::styled(label, label_style)));
        let text_color = if is_new { Color::Yellow } else { Color::White };
        for text_line in content.lines() {
            lines.push(Line::from(Span::styled(
                format!("  {}", text_line),
                Style::default().fg(text_color),
            )));
        }
        lines.push(Line::from(""));
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

    if let Some(hint) = &app.chat_hint {
        lines.push(Line::from(Span::styled(
            hint.clone(),
            Style::default().fg(Color::Yellow),
        )));
    }

    let max_scroll = (lines.len() as u16).saturating_sub(1);
    app.chat_scroll = app.chat_scroll.min(max_scroll);
    let messages = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((app.chat_scroll, 0));
    f.render_widget(messages, chat_chunks[0]);

    // Status bar: token usage + session ID
    let status_line = build_status_line(app, chat_chunks[1].width);
    f.render_widget(status_line, chat_chunks[1]);

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

    // Task popups (rendered over chat)
    if app.show_task_input {
        draw_task_input_popup(f, app, area);
    }
    if app.show_task_list {
        draw_task_list_popup(f, app, area);
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
            let short_id: String = s.session_id.chars().take(8).collect();
            let prompt = truncate(&s.first_prompt, 80);
            let line = Line::from(vec![
                Span::styled(date, Style::default().fg(Color::Yellow)),
                Span::styled(
                    format!(" {:>3}msg", s.message_count),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    format!(" {}", short_id),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(branch, Style::default().fg(Color::Cyan)),
                Span::raw("  "),
                Span::styled(prompt, Style::default().fg(Color::White)),
            ]);
            ListItem::new(line)
        })
        .collect();

    let title = match &app.clipboard_msg {
        Some(msg) => format!(" {} sessions  [{}] ", app.filtered_session_indices.len(), msg),
        None => format!(" {} sessions ", app.filtered_session_indices.len()),
    };
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::NONE)
                .title(title),
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

fn build_status_line(app: &App, width: u16) -> Paragraph<'static> {
    let session_part = match &app.chat_session_id {
        Some(id) => format!("Session: {} ", &id[..id.len().min(8)]),
        None => String::new(),
    };

    let gw_spans: Vec<Span> = {
        let (label, color) = if app.gateway_url.is_none() {
            ("GW:N/A", Color::DarkGray)
        } else if app.gateway_enabled {
            ("GW:ON", Color::Green)
        } else {
            ("GW:OFF", Color::Red)
        };
        vec![
            Span::raw(" "),
            Span::styled(label, Style::default().fg(color)),
        ]
    };
    let gw_len: usize = gw_spans.iter().map(|s| s.content.len()).sum();

    let tone_spans: Vec<Span> = {
        let label = format!(" Tone:{}", app.chat_tone.label());
        vec![Span::styled(label, Style::default().fg(Color::Magenta))]
    };
    let tone_len: usize = tone_spans.iter().map(|s| s.content.len()).sum();

    let running_count = app.tasks.iter().filter(|t| matches!(t.status, TaskStatus::Running)).count();
    let task_spans: Vec<Span> = if !app.tasks.is_empty() {
        let label = if running_count > 0 {
            format!(" Tasks:{}/{}", running_count, app.tasks.len())
        } else {
            format!(" Tasks:{}", app.tasks.len())
        };
        let color = if running_count > 0 { Color::Yellow } else { Color::DarkGray };
        vec![Span::styled(label, Style::default().fg(color))]
    } else {
        vec![]
    };
    let task_len: usize = task_spans.iter().map(|s| s.content.len()).sum();

    let Some(usage) = &app.usage_status else {
        let left = " Loading usage...";
        let pad = (width as usize).saturating_sub(left.len() + gw_len + tone_len + task_len + session_part.len());
        let mut spans = vec![
            Span::styled(left, Style::default().fg(Color::DarkGray)),
        ];
        spans.extend(gw_spans);
        spans.extend(tone_spans);
        spans.extend(task_spans);
        spans.push(Span::raw(" ".repeat(pad)));
        spans.push(Span::styled(session_part, Style::default().fg(Color::DarkGray)));
        return Paragraph::new(Line::from(spans));
    };

    let color = if usage.five_hour_pct >= 90.0 {
        Color::Red
    } else if usage.five_hour_pct >= 70.0 {
        Color::Yellow
    } else {
        Color::Green
    };

    let countdown = match usage.five_hour_resets_at {
        Some(end) => {
            let now = chrono::Utc::now();
            let remaining = end.signed_duration_since(now);
            if remaining.num_seconds() <= 0 {
                "expired".to_string()
            } else {
                let h = remaining.num_hours();
                let m = remaining.num_minutes() % 60;
                format!("{}h{:02}m left", h, m)
            }
        }
        None => "??".to_string(),
    };

    let sonnet_part = match usage.seven_day_sonnet_pct {
        Some(pct) => format!(" | Sonnet: {}%", pct as u32),
        None => String::new(),
    };

    let left = format!(
        " \u{23f1} {} | {}% used | 7d: {}%{}",
        countdown,
        usage.five_hour_pct as u32,
        usage.seven_day_pct as u32,
        sonnet_part,
    );

    let pad = (width as usize).saturating_sub(left.len() + gw_len + tone_len + task_len + session_part.len());
    let mut spans = vec![
        Span::styled(left, Style::default().fg(color)),
    ];
    spans.extend(gw_spans);
    spans.extend(tone_spans);
    spans.extend(task_spans);
    spans.push(Span::raw(" ".repeat(pad)));
    spans.push(Span::styled(session_part, Style::default().fg(Color::DarkGray)));
    Paragraph::new(Line::from(spans))
}

fn draw_task_input_popup(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let popup_w = (area.width * 60 / 100).max(30);
    let popup_h = 3;
    let x = area.x + (area.width.saturating_sub(popup_w)) / 2;
    let y = area.y + (area.height.saturating_sub(popup_h)) / 2;
    let popup_area = ratatui::layout::Rect::new(x, y, popup_w, popup_h);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .title(" Run task (sh -c) ");
    let input = Paragraph::new(app.task_input.as_str()).block(block);
    f.render_widget(ratatui::widgets::Clear, popup_area);
    f.render_widget(input, popup_area);

    let cursor_x = popup_area.x + app.task_input.len() as u16 + 1;
    let cursor_y = popup_area.y + 1;
    f.set_cursor_position((cursor_x, cursor_y));
}

fn draw_task_list_popup(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let popup_w = (area.width * 80 / 100).max(40);
    let popup_h = (area.height * 80 / 100).max(10);
    let x = area.x + (area.width.saturating_sub(popup_w)) / 2;
    let y = area.y + (area.height.saturating_sub(popup_h)) / 2;
    let popup_area = ratatui::layout::Rect::new(x, y, popup_w, popup_h);

    f.render_widget(ratatui::widgets::Clear, popup_area);

    let list_h = (popup_h * 35 / 100).max(3);
    let output_h = popup_h.saturating_sub(list_h);
    let chunks = Layout::vertical([
        Constraint::Length(list_h),
        Constraint::Length(output_h),
    ])
    .split(popup_area);

    // Task list
    let items: Vec<ListItem> = app
        .tasks
        .iter()
        .map(|t| {
            let icon = match t.status {
                TaskStatus::Running => Span::styled("... ", Style::default().fg(Color::Yellow)),
                TaskStatus::Done => Span::styled(" OK ", Style::default().fg(Color::Green)),
                TaskStatus::Error => Span::styled("ERR ", Style::default().fg(Color::Red)),
            };
            let cmd = Span::styled(
                truncate(&t.command, (popup_w as usize).saturating_sub(8)),
                Style::default().fg(Color::White),
            );
            ListItem::new(Line::from(vec![icon, cmd]))
        })
        .collect();

    let list_title = format!(" {} tasks ", app.tasks.len());
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow))
                .title(list_title),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );
    let mut state = ListState::default();
    if !app.tasks.is_empty() {
        state.select(Some(app.task_list_idx));
    }
    f.render_stateful_widget(list, chunks[0], &mut state);

    // Selected task output
    let output_text = if let Some(task) = app.tasks.get(app.task_list_idx) {
        match &task.output {
            Some(text) => text.clone(),
            None if matches!(task.status, TaskStatus::Running) => "Running...".to_string(),
            _ => String::new(),
        }
    } else {
        "No tasks".to_string()
    };

    let output = Paragraph::new(output_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(" Output "),
        )
        .wrap(Wrap { trim: false })
        .scroll((app.task_scroll, 0));
    f.render_widget(output, chunks[1]);
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
