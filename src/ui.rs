use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use unicode_width::UnicodeWidthStr;

use crate::app::{App, InputMode, Mode, TaskStatus, View};

pub fn draw(f: &mut Frame, app: &mut App) {
    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .split(f.area());

    // Title bar
    let cwd_suffix = if app.cwd.as_os_str().is_empty() {
        String::new()
    } else {
        format!(" | {}", app.cwd.display())
    };
    let title = match app.mode {
        Mode::ProjectSelect => "cc-companion | Select Project".to_string(),
        Mode::Chat => {
            if app.chat.session_id.is_some() {
                format!("cc-companion | Chat{cwd_suffix}")
            } else {
                format!("cc-companion | Chat (new session){cwd_suffix}")
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
        Mode::ProjectSelect => draw_project_select(f, app, chunks[1]),
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
        Mode::ProjectSelect => match app.input_mode {
            InputMode::Search => format!("Search: {}_ | Enter=accept Esc=cancel", app.search.query),
            InputMode::PathInput => format!("Path: {}_ | Enter=open Esc=cancel", app.path_input),
            _ => "j/k=move Enter=select /=search a=add path q=quit".to_string(),
        },
        Mode::Chat => match app.input_mode {
            InputMode::ChatInput => "Enter=send Alt+Enter=newline Ctrl+C=copy Esc=cancel | Shift+Tab=logs".to_string(),
            InputMode::TaskInput => "Enter=run Esc=cancel".to_string(),
            _ if app.tasks.pipeline_picker => "j/k=select Enter=choose Esc=cancel".to_string(),
            _ if app.tasks.show_panel => "j/k=select Enter=run d=delete p=pipeline Ctrl+d/u=scroll Esc=close".to_string(),
            _ => "i=type x=task X=tasks j/k=scroll n=new p=proj t=tone g=gw a=auto q=quit | Shift+Tab=logs".to_string(),
        },
        Mode::LogViewer => match (&app.view, &app.input_mode) {
            (_, InputMode::Search) => {
                format!("Search: {}_ | Enter=accept Esc=cancel", app.search.query)
            }
            (_, InputMode::RgInput) => {
                format!("rg: {}_ | Enter=search Esc=cancel", app.search.rg_query)
            }
            (View::ProjectList, _) => {
                "j/k=move Enter=open /=search c=CLAUDE.md q=quit | Shift+Tab=chat".to_string()
            }
            (View::SessionList, _) => "j/k=move Enter=open /=search r=rg R=clear y=copy id Esc=back".to_string(),
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
    let line_count = app.chat.input.matches('\n').count() + 1;
    let input_height = (line_count as u16 + 2).min(8);
    let chat_chunks = Layout::vertical([
        Constraint::Min(0),
        Constraint::Length(1),
        Constraint::Length(input_height),
    ])
    .split(area);

    // Messages area
    let mut lines: Vec<Line> = Vec::new();
    let msg_count = app.chat.messages.len();
    let blink_on = app.chat.new_msg_at.is_some_and(|t| {
        let elapsed = t.elapsed();
        elapsed.as_secs() < 4 && elapsed.as_millis() / 500 % 2 == 0
    });

    for (i, (role, content)) in app.chat.messages.iter().enumerate() {
        let is_new = blink_on && i == msg_count - 1;
        let (label, color) = match role.as_str() {
            "user" => ("You:", Color::Green),
            _ => ("cc-companion:", Color::Blue),
        };
        let bg = if is_new { Some(Color::Rgb(60, 60, 120)) } else { None };
        let label_style = Style::default().fg(color).add_modifier(Modifier::BOLD);
        let label_style = if let Some(bg) = bg { label_style.bg(bg) } else { label_style };
        lines.push(Line::from(Span::styled(label, label_style)));
        for text_line in content.lines() {
            let mut style = Style::default().fg(Color::White);
            if let Some(bg) = bg { style = style.bg(bg); }
            lines.push(Line::from(Span::styled(
                format!("  {}", text_line),
                style,
            )));
        }
        lines.push(Line::from(""));
    }

    if app.chat.waiting {
        let secs = app.chat.waiting_since
            .map(|t| t.elapsed().as_secs())
            .unwrap_or(0);
        lines.push(Line::from(Span::styled(
            format!("Thinking... {}s", secs),
            Style::default().fg(Color::Yellow),
        )));
    }

    if let Some(err) = &app.chat.error {
        lines.push(Line::from(Span::styled(
            format!("Error: {}", err),
            Style::default().fg(Color::Red),
        )));
    }

    if let Some(hint) = &app.chat.hint {
        lines.push(Line::from(Span::styled(
            hint.clone(),
            Style::default().fg(Color::Yellow),
        )));
    }

    let visible_height = chat_chunks[0].height as usize;
    let messages = Paragraph::new(lines).wrap(Wrap { trim: false });
    let total_rendered = messages.line_count(chat_chunks[0].width);
    let max_scroll = total_rendered.saturating_sub(visible_height) as u16;
    app.chat.scroll = app.chat.scroll.min(max_scroll);
    let messages = messages.scroll((app.chat.scroll, 0));
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
    let input = Paragraph::new(app.chat.input.as_str())
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
        let last_line = app.chat.input.rsplit('\n').next().unwrap_or(&app.chat.input);
        let cursor_x = chat_chunks[2].x + UnicodeWidthStr::width(last_line) as u16 + 1;
        let lines_above = app.chat.input.matches('\n').count() as u16;
        let cursor_y = chat_chunks[2].y + 1 + lines_above;
        f.set_cursor_position((cursor_x, cursor_y));
    }

    // Task popups (rendered over chat, last = on top)
    if app.tasks.show_input {
        draw_task_input_popup(f, app, area);
    }
    if app.tasks.show_panel {
        draw_task_list_popup(f, app, area);
    }
    if app.tasks.goal_input {
        draw_goal_input_popup(f, app, area);
    }
    if app.tasks.pipeline_picker {
        draw_pipeline_picker(f, app, area);
    }
}

fn draw_project_list(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let items: Vec<ListItem> = app.search.project_indices
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
                .title(format!(" {} projects ", app.search.project_indices.len())),
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

fn draw_project_select(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let items: Vec<ListItem> = app.search.project_indices
        .iter()
        .map(|&i| {
            let p = &app.projects[i];
            let line = Line::from(vec![
                Span::styled(&p.name, Style::default().fg(Color::White)),
                Span::raw("  "),
                Span::styled(&p.project_path, Style::default().fg(Color::DarkGray)),
            ]);
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::NONE)
                .title(format!(" Select a project ({}) ", app.search.project_indices.len())),
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
    let items: Vec<ListItem> = app.search.session_indices
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
            let mut spans = vec![
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
            ];
            if let Some(&count) = app.search.rg_matches.get(&s.session_id) {
                spans.push(Span::styled(
                    format!(" ({} hits)", count),
                    Style::default().fg(Color::Magenta),
                ));
            }
            spans.push(Span::raw("  "));
            spans.push(Span::styled(prompt, Style::default().fg(Color::White)));
            ListItem::new(Line::from(spans))
        })
        .collect();

    let title = if app.search.rg_active {
        let n = app.search.rg_matches.len();
        format!(
            " {} matches for '{}' ({} sessions)  R=clear ",
            app.search.session_indices.len(),
            app.search.rg_query,
            n
        )
    } else {
        match &app.clipboard_msg {
            Some(msg) => format!(" {} sessions  [{}] ", app.search.session_indices.len(), msg),
            None => format!(" {} sessions ", app.search.session_indices.len()),
        }
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
    let session_part = match &app.chat.session_id {
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
        let label = format!(" Tone:{}", app.chat.tone.label());
        vec![Span::styled(label, Style::default().fg(Color::Magenta))]
    };
    let tone_len: usize = tone_spans.iter().map(|s| s.content.len()).sum();

    let running_count = app.tasks.items.iter().filter(|t| matches!(t.status, TaskStatus::Running)).count();
    let task_spans: Vec<Span> = if !app.tasks.items.is_empty() {
        let label = if running_count > 0 {
            format!(" Tasks:{}/{}", running_count, app.tasks.items.len())
        } else {
            format!(" Tasks:{}", app.tasks.items.len())
        };
        let color = if running_count > 0 { Color::Yellow } else { Color::DarkGray };
        vec![Span::styled(label, Style::default().fg(color))]
    } else {
        vec![]
    };
    let task_len: usize = task_spans.iter().map(|s| s.content.len()).sum();

    let pipe_spans: Vec<Span> = {
        let label = format!(" P:{}", app.tasks.scheduler.pipeline.label());
        vec![Span::styled(label, Style::default().fg(Color::Cyan))]
    };
    let pipe_len: usize = pipe_spans.iter().map(|s| s.content.len()).sum();

    let auto_spans: Vec<Span> = {
        let (label, color) = if app.tasks.scheduler.enabled {
            (" Auto:ON", Color::Green)
        } else {
            (" Auto:OFF", Color::DarkGray)
        };
        vec![Span::styled(label, Style::default().fg(color))]
    };
    let auto_len: usize = auto_spans.iter().map(|s| s.content.len()).sum();

    let Some(usage) = &app.usage_status else {
        let left = " Loading usage...";
        let pad = (width as usize).saturating_sub(left.len() + gw_len + tone_len + task_len + pipe_len + auto_len + session_part.len());
        let mut spans = vec![
            Span::styled(left, Style::default().fg(Color::DarkGray)),
        ];
        spans.extend(gw_spans);
        spans.extend(tone_spans);
        spans.extend(task_spans);
        spans.extend(pipe_spans);
        spans.extend(auto_spans);
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

    let seven_day_countdown = match usage.seven_day_resets_at {
        Some(end) => {
            let now = chrono::Utc::now();
            let remaining = end.signed_duration_since(now);
            if remaining.num_seconds() <= 0 {
                "expired".to_string()
            } else {
                let d = remaining.num_days();
                let h = remaining.num_hours() % 24;
                format!(" {}d{:02}h", d, h)
            }
        }
        None => String::new(),
    };

    let sonnet_part = match usage.seven_day_sonnet_pct {
        Some(pct) => format!(" | Sonnet: {}%", pct as u32),
        None => String::new(),
    };

    let left = format!(
        " \u{23f1} {} | {}% used | 7d: {}%{}{}",
        countdown,
        usage.five_hour_pct as u32,
        usage.seven_day_pct as u32,
        seven_day_countdown,
        sonnet_part,
    );

    let pad = (width as usize).saturating_sub(left.len() + gw_len + tone_len + task_len + pipe_len + auto_len + session_part.len());
    let mut spans = vec![
        Span::styled(left, Style::default().fg(color)),
    ];
    spans.extend(gw_spans);
    spans.extend(tone_spans);
    spans.extend(task_spans);
    spans.extend(pipe_spans);
    spans.extend(auto_spans);
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
    let input = Paragraph::new(app.tasks.input.as_str()).block(block);
    f.render_widget(ratatui::widgets::Clear, popup_area);
    f.render_widget(input, popup_area);

    let cursor_x = popup_area.x + UnicodeWidthStr::width(app.tasks.input.as_str()) as u16 + 1;
    let cursor_y = popup_area.y + 1;
    f.set_cursor_position((cursor_x, cursor_y));
}

fn draw_goal_input_popup(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let popup_w = (area.width * 60 / 100).max(30);
    let popup_h = 3;
    let x = area.x + (area.width.saturating_sub(popup_w)) / 2;
    let y = area.y + (area.height.saturating_sub(popup_h)) / 2;
    let popup_area = ratatui::layout::Rect::new(x, y, popup_w, popup_h);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Issue label filter (empty=all issues) ");
    let input = Paragraph::new(app.tasks.goal_text.as_str()).block(block);
    f.render_widget(ratatui::widgets::Clear, popup_area);
    f.render_widget(input, popup_area);

    let cursor_x = popup_area.x + UnicodeWidthStr::width(app.tasks.goal_text.as_str()) as u16 + 1;
    let cursor_y = popup_area.y + 1;
    f.set_cursor_position((cursor_x, cursor_y));
}

fn draw_pipeline_picker(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let pipelines = crate::pipeline::Pipeline::all();
    // 2 lines per pipeline (name + description) + 1 blank between + border
    let content_h = pipelines.len() as u16 * 3;
    let popup_h = content_h + 2; // borders
    let popup_w = 70u16.min(area.width.saturating_sub(4)).max(1);
    let x = area.x + (area.width.saturating_sub(popup_w)) / 2;
    let y = area.y + (area.height.saturating_sub(popup_h)) / 2;
    let popup_area = ratatui::layout::Rect::new(x, y, popup_w, popup_h);

    f.render_widget(ratatui::widgets::Clear, popup_area);

    let mut lines: Vec<Line> = Vec::new();
    for (i, p) in pipelines.iter().enumerate() {
        let marker = if i == app.tasks.pipeline_idx { " > " } else { "   " };
        let name_style = if i == app.tasks.pipeline_idx {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        lines.push(Line::from(vec![
            Span::styled(marker, name_style),
            Span::styled(p.label(), name_style),
        ]));
        lines.push(Line::from(Span::styled(
            format!("   {}", p.description()),
            Style::default().fg(Color::DarkGray),
        )));
        if i + 1 < pipelines.len() {
            lines.push(Line::from(""));
        }
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Select Pipeline ");
    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, popup_area);
}

fn draw_task_list_popup(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let popup_w = (area.width * 80 / 100).max(40);
    let popup_h = (area.height * 80 / 100).max(10);
    let x = area.x + (area.width.saturating_sub(popup_w)) / 2;
    let y = area.y + (area.height.saturating_sub(popup_h)) / 2;
    let popup_area = ratatui::layout::Rect::new(x, y, popup_w, popup_h);

    f.render_widget(ratatui::widgets::Clear, popup_area);

    let list_h = (popup_h * 50 / 100).max(3);
    let output_h = popup_h.saturating_sub(list_h);
    let chunks = Layout::vertical([
        Constraint::Length(list_h),
        Constraint::Length(output_h),
    ])
    .split(popup_area);

    // Done + running + pending auto-tasks
    let max_cmd = (popup_w as usize).saturating_sub(8);
    let mut items: Vec<ListItem> = app
        .tasks.scheduler.done
        .iter()
        .map(|name| {
            let icon = Span::styled(" \u{2714} ", Style::default().fg(Color::Green));
            let label = Span::styled(
                truncate(name, max_cmd),
                Style::default().fg(Color::DarkGray),
            );
            ListItem::new(Line::from(vec![icon, label]))
        })
        .collect();
    if let Some(ref running) = app.tasks.scheduler.running {
        let icon = Span::styled(" \u{23f3} ", Style::default().fg(Color::Yellow));
        let label = Span::styled(
            truncate(running, max_cmd),
            Style::default().fg(Color::Yellow),
        );
        items.push(ListItem::new(Line::from(vec![icon, label])));
    }
    items.extend(app.tasks.scheduler.tasks.iter().map(|t| {
        let icon = Span::styled(" >> ", Style::default().fg(Color::Cyan));
        let name = Span::styled(
            truncate(&t.name, max_cmd),
            Style::default().fg(Color::White),
        );
        ListItem::new(Line::from(vec![icon, name]))
    }));

    let done_count = app.tasks.scheduler.done.len();
    let running_count: usize = if app.tasks.scheduler.running.is_some() { 1 } else { 0 };
    let pending_count = app.tasks.scheduler.tasks.len();
    let pipe_label = app.tasks.scheduler.pipeline.label();
    let list_title = format!(" {pipe_label} | {done_count} done / {running_count} running / {pending_count} pending ");
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
    let total = done_count + running_count + pending_count;
    if total > 0 {
        state.select(Some(app.tasks.selected_idx));
    }
    f.render_stateful_widget(list, chunks[0], &mut state);

    // Selected task prompt preview
    let output_text = if app.tasks.selected_idx < done_count {
        format!("[completed] {}", app.tasks.scheduler.done[app.tasks.selected_idx])
    } else if running_count > 0 && app.tasks.selected_idx == done_count {
        format!("[running] {}", app.tasks.scheduler.running.as_deref().unwrap_or(""))
    } else if let Some(task) = app.tasks.scheduler.tasks.get(app.tasks.selected_idx - done_count - running_count) {
        format!("[{}]\n{}", task.cwd, task.prompt)
    } else {
        "No scheduled tasks".to_string()
    };

    let output_title = if app.tasks.selected_idx < done_count {
        " Output "
    } else if running_count > 0 && app.tasks.selected_idx == done_count {
        " Running "
    } else {
        " Prompt "
    };
    let output = Paragraph::new(output_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(output_title),
        )
        .wrap(Wrap { trim: false })
        .scroll((app.tasks.scroll, 0));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{App, ChatTone, InputMode, Mode, UsageStatus, View};
    use crate::data::{ConversationMessage, SessionEntry};
    use crate::pipeline::AutoTask;
    use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};
    use std::path::PathBuf;
    use std::time::Instant;

    /// Scan all rows in the buffer and check if `text` appears in any row.
    fn buffer_contains(buf: &Buffer, text: &str) -> bool {
        let area = buf.area;
        for y in area.y..area.y + area.height {
            let row: String = (area.x..area.x + area.width)
                .map(|x| buf[(x, y)].symbol().to_string())
                .collect();
            if row.contains(text) {
                return true;
            }
        }
        false
    }

    #[test]
    fn test_render_project_select() {
        let mut app = App::test_default();
        app.mode = Mode::ProjectSelect;
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        let buf = terminal.backend().buffer();
        assert!(buffer_contains(buf, "Select Project"));
    }

    #[test]
    fn test_render_chat_empty() {
        let mut app = App::test_default();
        app.mode = Mode::Chat;
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        let buf = terminal.backend().buffer();
        assert!(buffer_contains(buf, "new session"));
    }

    #[test]
    fn test_render_chat_with_messages() {
        let mut app = App::test_default();
        app.mode = Mode::Chat;
        app.chat.messages.push(("user".into(), "hello world".into()));
        app.chat.messages.push(("assistant".into(), "hi there".into()));
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        let buf = terminal.backend().buffer();
        assert!(buffer_contains(buf, "hello world"));
        assert!(buffer_contains(buf, "hi there"));
    }

    #[test]
    fn test_render_project_list() {
        let mut app = App::test_default();
        app.mode = Mode::LogViewer;
        app.view = View::ProjectList;
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        let buf = terminal.backend().buffer();
        assert!(buffer_contains(buf, "Projects"));
    }

    #[test]
    fn test_render_session_list() {
        let mut app = App::test_default();
        app.mode = Mode::LogViewer;
        app.view = View::SessionList;
        app.sessions.push(SessionEntry {
            session_id: "abc-123".into(),
            first_prompt: "do stuff".into(),
            message_count: 5,
            modified: "2025-01-01 12:00".into(),
            git_branch: "main".into(),
            jsonl_path: PathBuf::from("/tmp/fake.jsonl"),
        });
        app.search.session_indices = vec![0];
        app.projects.push(crate::data::Project {
            name: "test-proj".into(),
            project_path: "/tmp/test".into(),
            claude_dir: PathBuf::from("/tmp"),
            session_count: 1,
            has_claude_md: false,
        });
        app.search.project_indices = vec![0];
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        let buf = terminal.backend().buffer();
        assert!(buffer_contains(buf, "abc-123"));
        assert!(buffer_contains(buf, "do stuff"));
    }

    #[test]
    fn test_render_chat_waiting() {
        let mut app = App::test_default();
        app.mode = Mode::Chat;
        app.chat.waiting = true;
        app.chat.waiting_since = Some(Instant::now());
        app.chat.messages.push(("user".into(), "generate overview".into()));
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        let buf = terminal.backend().buffer();
        assert!(buffer_contains(buf, "Thinking..."));
    }

    #[test]
    fn test_render_chat_error() {
        let mut app = App::test_default();
        app.mode = Mode::Chat;
        app.chat.error = Some("rate limit".into());
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        let buf = terminal.backend().buffer();
        assert!(buffer_contains(buf, "Error: rate limit"));
    }

    #[test]
    fn test_render_chat_hint() {
        let mut app = App::test_default();
        app.mode = Mode::Chat;
        app.chat.hint = Some("Use /help".into());
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        let buf = terminal.backend().buffer();
        assert!(buffer_contains(buf, "Use /help"));
    }

    #[test]
    fn test_render_status_bar_usage() {
        let mut app = App::test_default();
        app.mode = Mode::Chat;
        app.usage_status = Some(UsageStatus {
            five_hour_pct: 75.0,
            five_hour_resets_at: Some(chrono::Utc::now() + chrono::Duration::hours(2)),
            seven_day_pct: 40.0,
            seven_day_resets_at: Some(chrono::Utc::now() + chrono::Duration::days(3)),
            seven_day_sonnet_pct: None,
            last_fetched: Instant::now(),
        });
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        let buf = terminal.backend().buffer();
        assert!(buffer_contains(buf, "75% used"));
        assert!(buffer_contains(buf, "7d: 40%"));
    }

    #[test]
    fn test_render_status_bar_loading() {
        let mut app = App::test_default();
        app.mode = Mode::Chat;
        // usage_status is None by default
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        let buf = terminal.backend().buffer();
        assert!(buffer_contains(buf, "Loading usage"));
    }

    #[test]
    fn test_render_status_bar_gateway_tone() {
        let mut app = App::test_default();
        app.mode = Mode::Chat;
        app.gateway_url = Some("http://localhost:8080".into());
        app.gateway_enabled = true;
        app.chat.tone = ChatTone::Eric;
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        let buf = terminal.backend().buffer();
        assert!(buffer_contains(buf, "GW:ON"));
        assert!(buffer_contains(buf, "Tone:eric"));
    }

    #[test]
    fn test_render_pipeline_picker() {
        let mut app = App::test_default();
        app.mode = Mode::Chat;
        app.tasks.pipeline_picker = true;
        app.tasks.pipeline_idx = 1;
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        let buf = terminal.backend().buffer();
        assert!(buffer_contains(buf, "Select Pipeline"));
        assert!(buffer_contains(buf, "Example"));
        assert!(buffer_contains(buf, "Issue-Driven"));
    }

    #[test]
    fn test_render_task_panel() {
        let mut app = App::test_default();
        app.mode = Mode::Chat;
        app.tasks.show_panel = true;
        app.tasks.scheduler.done = vec!["Analyze".into()];
        app.tasks.scheduler.running = Some("Build".into());
        app.tasks.scheduler.tasks = vec![AutoTask {
            name: "Deploy".into(),
            prompt: "deploy it".into(),
            cwd: "/tmp".into(),
            read_only: false,
            resume: false,
            setup: None,
        }];
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        let buf = terminal.backend().buffer();
        assert!(buffer_contains(buf, "Analyze"));
        assert!(buffer_contains(buf, "Build"));
        assert!(buffer_contains(buf, "Deploy"));
        assert!(buffer_contains(buf, "Example"));
    }

    #[test]
    fn test_render_conversation_view() {
        let mut app = App::test_default();
        app.mode = Mode::LogViewer;
        app.view = View::Conversation;
        app.conversation = vec![
            ConversationMessage { role: "user".into(), text: "hello from user".into() },
            ConversationMessage { role: "assistant".into(), text: "hello from assistant".into() },
        ];
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        let buf = terminal.backend().buffer();
        assert!(buffer_contains(buf, "USER"));
        assert!(buffer_contains(buf, "ASSISTANT"));
        assert!(buffer_contains(buf, "hello from user"));
        assert!(buffer_contains(buf, "hello from assistant"));
    }

    #[test]
    fn test_render_help_bar_variants() {
        // Chat input mode
        let mut app = App::test_default();
        app.mode = Mode::Chat;
        app.input_mode = InputMode::ChatInput;
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        let buf = terminal.backend().buffer();
        assert!(buffer_contains(buf, "Enter=send"));

        // LogViewer + RgInput mode
        let mut app2 = App::test_default();
        app2.mode = Mode::LogViewer;
        app2.view = View::SessionList;
        app2.input_mode = InputMode::RgInput;
        let backend2 = TestBackend::new(80, 24);
        let mut terminal2 = Terminal::new(backend2).unwrap();
        terminal2.draw(|f| draw(f, &mut app2)).unwrap();
        let buf2 = terminal2.backend().buffer();
        assert!(buffer_contains(buf2, "rg:"));
    }
}
