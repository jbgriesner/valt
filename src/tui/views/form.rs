use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::tui::app::{AppState, AppView, FormMode};

const LABELS: [&str; 6] = ["Name", "Username", "Password", "URL", "Tags", "Notes"];

pub fn render(f: &mut Frame, app: &AppState) {
    let AppView::Form {
        mode,
        draft,
        focused_field,
        show_password,
        error,
    } = &app.view
    else {
        return;
    };

    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(area);

    let title = match mode {
        FormMode::Add => " New Secret ",
        FormMode::Edit(_) => " Edit Secret ",
    };

    let field_values: [&str; 6] = [
        &draft.name,
        &draft.username,
        &draft.password,
        &draft.url,
        &draft.tags,
        &draft.notes,
    ];

    let mut lines = vec![Line::from("")];

    for (i, (&label, &value)) in LABELS.iter().zip(field_values.iter()).enumerate() {
        let is_focused = i == *focused_field;

        // Mask password unless show_password is active.
        let display: String = if i == 2 && !show_password {
            "•".repeat(value.len())
        } else {
            value.to_string()
        };

        let label_style = if is_focused {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let value_style = if is_focused {
            Style::default().fg(Color::White).bg(Color::DarkGray)
        } else {
            Style::default().fg(Color::White)
        };

        let mut spans = vec![
            Span::styled(format!("  {:10}: ", label), label_style),
            Span::styled(format!("{display:<40}"), value_style),
        ];

        if i == 2 && is_focused {
            spans.push(Span::styled(
                "  [g] Generate  [Space] Toggle",
                Style::default().fg(Color::DarkGray),
            ));
        }

        lines.push(Line::from(spans));
        lines.push(Line::from(""));
    }

    if let Some(err) = error {
        lines.push(Line::from(Span::styled(
            format!("  ✗ {err}"),
            Style::default().fg(Color::Red),
        )));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(title);

    f.render_widget(Paragraph::new(lines).block(block), chunks[0]);

    f.render_widget(
        Paragraph::new("[Tab] Next field  [Shift+Tab] Prev  [Enter] Save  [Esc] Cancel")
            .style(Style::default().fg(Color::DarkGray)),
        chunks[1],
    );
}
