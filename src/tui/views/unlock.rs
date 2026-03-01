use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::tui::app::{AppState, AppView};

pub fn render(f: &mut Frame, app: &AppState) {
    let AppView::Locked { input, error } = &app.view else {
        return;
    };

    let area = f.area();

    // Centre a dialog box vertically and horizontally.
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Length(11),
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

    let vault_hint = if app.vault_exists() {
        "Vault found — enter master password"
    } else {
        "No vault found — will be created"
    };

    let masked: String = "•".repeat(input.len());

    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  VALT",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "  password manager",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Master password: ", Style::default().fg(Color::DarkGray)),
            Span::styled(masked, Style::default().fg(Color::Yellow)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            format!("  {vault_hint}"),
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  [Enter] Unlock   [Esc/Ctrl+C] Quit",
            Style::default().fg(Color::DarkGray),
        )),
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
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Valt ");

    f.render_widget(Paragraph::new(lines).block(block), dialog);
}
