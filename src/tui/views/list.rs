use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::tui::app::{AppState, AppView};

pub fn render(f: &mut Frame, app: &AppState) {
    let AppView::List {
        search_query,
        selected_idx,
    } = &app.view
    else {
        return;
    };

    let vault = match &app.vault {
        Some(v) => v,
        None => return,
    };

    let secrets = vault.search(search_query);
    let count = secrets.len();
    let total = vault.list().len();
    let selected = (*selected_idx).min(count.saturating_sub(1));

    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // search bar
            Constraint::Min(1),    // secret list
            Constraint::Length(1), // status bar
        ])
        .split(area);

    let search_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(format!(
            " VALT ── {total} secret{} ",
            if total == 1 { "" } else { "s" }
        ));

    let (search_text, search_style) = if search_query.is_empty() {
        (
            "Search… (type to filter)".to_string(),
            Style::default().fg(Color::DarkGray),
        )
    } else {
        (search_query.clone(), Style::default().fg(Color::White))
    };

    f.render_widget(
        Paragraph::new(search_text)
            .style(search_style)
            .block(search_block),
        chunks[0],
    );

    let items: Vec<ListItem> = secrets
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let is_sel = i == selected;

            let base_bg = if is_sel { Color::Cyan } else { Color::Reset };
            let base_fg = if is_sel { Color::Black } else { Color::White };
            let dim_fg = if is_sel {
                Color::Black
            } else {
                Color::DarkGray
            };
            let tag_fg = if is_sel { Color::Black } else { Color::Blue };

            let tags_str = if s.tags.is_empty() {
                String::new()
            } else {
                format!(" [{}]", s.tags.join(", "))
            };

            let name_style = Style::default()
                .fg(base_fg)
                .bg(base_bg)
                .add_modifier(if is_sel {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                });

            let line = Line::from(vec![
                Span::styled(format!(" {:<28}", &s.name), name_style),
                Span::styled(
                    format!("{:<25}", s.url.as_deref().unwrap_or("")),
                    Style::default().fg(dim_fg).bg(base_bg),
                ),
                Span::styled(tags_str, Style::default().fg(tag_fg).bg(base_bg)),
            ]);
            ListItem::new(line)
        })
        .collect();

    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let mut list_state = ListState::default();
    if !secrets.is_empty() {
        list_state.select(Some(selected));
    }

    f.render_stateful_widget(
        List::new(items).block(list_block),
        chunks[1],
        &mut list_state,
    );

    let status_text = if let Some(msg) = &app.status {
        msg.clone()
    } else {
        let clip = app
            .clipboard_secs_remaining()
            .map(|s| format!("  [clipboard clears in {s}s]"))
            .unwrap_or_default();
        format!("[↑↓/jk] Navigate  [↵] Open  [n] New  [d] Delete  [q] Quit  [?] Help{clip}")
    };

    f.render_widget(
        Paragraph::new(status_text).style(Style::default().fg(Color::DarkGray)),
        chunks[2],
    );
}
