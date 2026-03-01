use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::tui::app::AppState;

const SHORTCUTS: &[(&str, &str)] = &[
    ("j / ↓", "Move down"),
    ("k / ↑", "Move up"),
    ("↵ / →", "Open detail"),
    ("n", "New secret"),
    ("e", "Edit secret"),
    ("d", "Delete secret"),
    ("c", "Copy password (auto-clears in 30 s)"),
    ("Space", "Toggle password visibility"),
    ("g", "Generate password (in password field)"),
    ("Esc", "Back / cancel / clear search"),
    ("?", "This help screen"),
    ("q / Ctrl+C", "Quit"),
];

pub fn render(f: &mut Frame, _app: &AppState) {
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(area);

    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Keyboard shortcuts",
            Style::default().fg(Color::Cyan),
        )),
        Line::from(""),
    ];

    for (key, desc) in SHORTCUTS {
        lines.push(Line::from(vec![
            Span::styled(format!("  {:16}", key), Style::default().fg(Color::Yellow)),
            Span::raw(*desc),
        ]));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Help ");

    f.render_widget(Paragraph::new(lines).block(block), chunks[0]);

    f.render_widget(
        Paragraph::new("[Esc] or [?]  Back to list").style(Style::default().fg(Color::DarkGray)),
        chunks[1],
    );
}
