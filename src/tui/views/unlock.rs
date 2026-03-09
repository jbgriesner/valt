use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::tui::app::{AppState, AppView};

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn render(f: &mut Frame, app: &AppState) {
    let AppView::Locked { input, error } = &app.view else {
        return;
    };

    let area = f.area();

    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Length(15),
            Constraint::Min(0),
        ])
        .split(area);

    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(60),
            Constraint::Percentage(20),
        ])
        .split(vertical[1]);

    let dialog = horizontal[1];
    f.render_widget(Clear, dialog);

    let masked: String = "•".repeat(input.len());
    let vault_path = app.vault_path.display().to_string();

    let (status_line, path_color) = if app.vault_exists() {
        (
            Line::from(Span::styled(
                "  ✓ Vault found",
                Style::default().fg(Color::Green),
            )),
            Color::DarkGray,
        )
    } else {
        (
            Line::from(Span::styled(
                "  + No vault — will be created",
                Style::default().fg(Color::Yellow),
            )),
            Color::Yellow,
        )
    };

    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  🔐  VALT  v{VERSION}"),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  ────────────────────────────────────",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        status_line,
        Line::from(Span::styled(
            format!("  {vault_path}"),
            Style::default().fg(path_color).add_modifier(Modifier::DIM),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Master password  ", Style::default().fg(Color::Gray)),
            Span::styled(masked, Style::default().fg(Color::Yellow)),
            Span::styled(
                "█",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::SLOW_BLINK),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(
                "Enter",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Unlock", Style::default().fg(Color::DarkGray)),
            Span::styled("  ·  ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                "Esc",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Quit", Style::default().fg(Color::DarkGray)),
        ]),
    ];

    if let Some(err) = error {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("  ✗ {err}"),
            Style::default().fg(Color::Red),
        )));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Blue));

    f.render_widget(Paragraph::new(lines).block(block), dialog);
}
