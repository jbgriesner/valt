use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::tui::app::{AppState, AppView};

pub fn render(f: &mut Frame, app: &AppState) {
    let AppView::Detail {
        secret_id,
        show_password,
    } = &app.view
    else {
        return;
    };

    let vault = match &app.vault {
        Some(v) => v,
        None => return,
    };

    let secret = match vault.get(*secret_id) {
        Some(s) => s,
        None => return,
    };

    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(area);

    let pwd_display = if *show_password {
        secret.password.clone()
    } else {
        "•".repeat(secret.password.len().min(30))
    };
    let pwd_hint = if *show_password {
        "[Space] Hide"
    } else {
        "[Space] Show"
    };

    let clip_hint = match app.clipboard_secs_remaining() {
        Some(secs) => format!("[c] Copy  (clears in {secs}s)"),
        None => "[c] Copy password".to_string(),
    };

    let tags_str = if secret.tags.is_empty() {
        "—".to_string()
    } else {
        secret.tags.join(", ")
    };

    let lines = vec![
        Line::from(""),
        field_line("Name    ", &secret.name, Color::White),
        field_line(
            "Username",
            secret.username.as_deref().unwrap_or("—"),
            Color::White,
        ),
        Line::from(vec![
            Span::styled("  Password : ", Style::default().fg(Color::DarkGray)),
            Span::styled(&pwd_display, Style::default().fg(Color::Yellow)),
            Span::raw("  "),
            Span::styled(pwd_hint, Style::default().fg(Color::DarkGray)),
        ]),
        field_line(
            "URL     ",
            secret.url.as_deref().unwrap_or("—"),
            Color::White,
        ),
        field_line("Tags    ", &tags_str, Color::Blue),
        field_line(
            "Notes   ",
            secret.notes.as_deref().unwrap_or("—"),
            Color::White,
        ),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Created : ", Style::default().fg(Color::DarkGray)),
            Span::raw(secret.created_at.format("%Y-%m-%d").to_string()),
        ]),
        Line::from(vec![
            Span::styled("  Updated : ", Style::default().fg(Color::DarkGray)),
            Span::raw(secret.updated_at.format("%Y-%m-%d").to_string()),
        ]),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(format!(" {} ", secret.name));

    f.render_widget(Paragraph::new(lines).block(block), chunks[0]);

    let status = format!("[e] Edit  {clip_hint}  [d] Delete  [Esc] Back  [?] Help");
    f.render_widget(
        Paragraph::new(status).style(Style::default().fg(Color::DarkGray)),
        chunks[1],
    );
}

fn field_line<'a>(label: &'a str, value: &'a str, value_color: Color) -> Line<'a> {
    Line::from(vec![
        Span::styled(
            format!("  {label} : "),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(value, Style::default().fg(value_color)),
    ])
}
