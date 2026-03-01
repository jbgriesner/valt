use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serdevault::VaultFile;
use valt_core::{generate, GeneratorConfig, Secret, VaultManager};

use super::app::{AppState, AppView, FormMode, SecretDraft};

const CLIPBOARD_TIMEOUT: Duration = Duration::from_secs(30);

pub fn handle_key(app: &mut AppState, key: KeyEvent) {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.should_quit = true;
        return;
    }

    match &app.view {
        AppView::Locked { .. } => handle_locked(app, key),
        AppView::List { .. } => handle_list(app, key),
        AppView::Detail { .. } => handle_detail(app, key),
        AppView::Form { .. } => handle_form(app, key),
        AppView::Help => handle_help(app, key),
    }
}

fn handle_locked(app: &mut AppState, key: KeyEvent) {
    match key.code {
        KeyCode::Char(c) => {
            if let AppView::Locked { input, error } = &mut app.view {
                input.push(c);
                *error = None;
            }
        }
        KeyCode::Backspace => {
            if let AppView::Locked { input, .. } = &mut app.view {
                input.pop();
            }
        }
        KeyCode::Enter => {
            // Clone password before any mutation.
            let password = match &app.view {
                AppView::Locked { input, .. } => input.clone(),
                _ => return,
            };
            let vault_file = VaultFile::open(&app.vault_path, &password);
            match VaultManager::open_or_create(vault_file) {
                Ok(manager) => {
                    app.vault = Some(manager);
                    app.go_to_list();
                }
                Err(_) => {
                    if let AppView::Locked { error, input } = &mut app.view {
                        *error = Some("Wrong password or corrupted vault".to_string());
                        input.clear();
                    }
                }
            }
        }
        KeyCode::Esc => {
            app.should_quit = true;
        }
        _ => {}
    }
}

fn handle_list(app: &mut AppState, key: KeyEvent) {
    app.status = None;

    let (search_query, selected_idx) = match &app.view {
        AppView::List {
            search_query,
            selected_idx,
        } => (search_query.clone(), *selected_idx),
        _ => return,
    };

    let count = app
        .vault
        .as_ref()
        .map(|v| v.search(&search_query).len())
        .unwrap_or(0);

    match key.code {
        KeyCode::Char('q') => {
            app.should_quit = true;
        }
        KeyCode::Char('?') => {
            app.view = AppView::Help;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if count > 0 {
                let new_idx = (selected_idx + 1).min(count - 1);
                if let AppView::List { selected_idx, .. } = &mut app.view {
                    *selected_idx = new_idx;
                }
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if selected_idx > 0 {
                let new_idx = selected_idx - 1;
                if let AppView::List { selected_idx, .. } = &mut app.view {
                    *selected_idx = new_idx;
                }
            }
        }
        KeyCode::Enter | KeyCode::Right => {
            if count > 0 {
                let idx = selected_idx.min(count - 1);
                let secret_id = app
                    .vault
                    .as_ref()
                    .and_then(|v| v.search(&search_query).get(idx).map(|s| s.id));
                if let Some(id) = secret_id {
                    app.view = AppView::Detail {
                        secret_id: id,
                        show_password: false,
                    };
                }
            }
        }
        KeyCode::Char('n') => {
            app.view = AppView::Form {
                mode: FormMode::Add,
                draft: SecretDraft::empty(),
                focused_field: 0,
                show_password: false,
                error: None,
            };
        }
        KeyCode::Char('d') => {
            if count > 0 {
                let idx = selected_idx.min(count - 1);
                let secret_id = app
                    .vault
                    .as_ref()
                    .and_then(|v| v.search(&search_query).get(idx).map(|s| s.id));
                if let Some(id) = secret_id {
                    if let Some(vault) = &mut app.vault {
                        let _ = vault.delete(id);
                    }
                    let new_count = app.vault.as_ref().map(|v| v.list().len()).unwrap_or(0);
                    if let AppView::List { selected_idx, .. } = &mut app.view {
                        *selected_idx = (*selected_idx).min(new_count.saturating_sub(1));
                    }
                    app.status = Some("Secret deleted.".to_string());
                }
            }
        }
        KeyCode::Backspace => {
            if let AppView::List {
                search_query,
                selected_idx,
                ..
            } = &mut app.view
            {
                search_query.pop();
                *selected_idx = 0;
            }
        }
        KeyCode::Char(c) => {
            // Any printable character filters the list.
            if let AppView::List {
                search_query,
                selected_idx,
                ..
            } = &mut app.view
            {
                search_query.push(c);
                *selected_idx = 0;
            }
        }
        KeyCode::Esc => {
            if let AppView::List { search_query, .. } = &mut app.view {
                if !search_query.is_empty() {
                    *search_query = String::new();
                }
            }
        }
        _ => {}
    }
}

fn handle_detail(app: &mut AppState, key: KeyEvent) {
    app.status = None;

    let secret_id = match &app.view {
        AppView::Detail { secret_id, .. } => *secret_id,
        _ => return,
    };

    match key.code {
        KeyCode::Esc | KeyCode::Left => {
            app.go_to_list();
        }
        KeyCode::Char('?') => {
            app.view = AppView::Help;
        }
        KeyCode::Char(' ') => {
            if let AppView::Detail { show_password, .. } = &mut app.view {
                *show_password = !*show_password;
            }
        }
        KeyCode::Char('c') => {
            let password = app
                .vault
                .as_ref()
                .and_then(|v| v.get(secret_id))
                .map(|s| s.password.clone());

            if let Some(pwd) = password {
                match arboard::Clipboard::new() {
                    Ok(mut cb) => {
                        if cb.set_text(&pwd).is_ok() {
                            app.clipboard_clear_at = Some(Instant::now() + CLIPBOARD_TIMEOUT);
                            app.status = Some("Password copied — clears in 30s".to_string());
                        } else {
                            app.status = Some("Failed to copy to clipboard".to_string());
                        }
                    }
                    Err(_) => {
                        app.status = Some("Clipboard not available".to_string());
                    }
                }
            }
        }
        KeyCode::Char('e') => {
            let draft = app
                .vault
                .as_ref()
                .and_then(|v| v.get(secret_id))
                .map(SecretDraft::from_secret);
            if let Some(draft) = draft {
                app.view = AppView::Form {
                    mode: FormMode::Edit(secret_id),
                    draft,
                    focused_field: 0,
                    show_password: false,
                    error: None,
                };
            }
        }
        KeyCode::Char('d') => {
            if let Some(vault) = &mut app.vault {
                if vault.delete(secret_id).is_ok() {
                    app.go_to_list();
                    app.status = Some("Secret deleted.".to_string());
                }
            }
        }
        _ => {}
    }
}

const FORM_FIELD_COUNT: usize = 6;

fn handle_form(app: &mut AppState, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            let mode = match &app.view {
                AppView::Form { mode, .. } => mode.clone(),
                _ => return,
            };
            match mode {
                FormMode::Add => app.go_to_list(),
                FormMode::Edit(id) => {
                    app.view = AppView::Detail {
                        secret_id: id,
                        show_password: false,
                    };
                }
            }
        }
        KeyCode::Tab => {
            if let AppView::Form { focused_field, .. } = &mut app.view {
                *focused_field = (*focused_field + 1) % FORM_FIELD_COUNT;
            }
        }
        KeyCode::BackTab => {
            if let AppView::Form { focused_field, .. } = &mut app.view {
                *focused_field = (*focused_field + FORM_FIELD_COUNT - 1) % FORM_FIELD_COUNT;
            }
        }
        KeyCode::Enter => {
            save_form(app);
        }
        KeyCode::Char('g') => {
            let is_pwd = is_password_field(app);
            if is_pwd {
                if let Ok(pwd) = generate(&GeneratorConfig::default()) {
                    if let AppView::Form { draft, .. } = &mut app.view {
                        draft.password = pwd;
                    }
                }
            } else {
                type_char(app, 'g');
            }
        }
        KeyCode::Char(' ') => {
            let is_pwd = is_password_field(app);
            if is_pwd {
                if let AppView::Form { show_password, .. } = &mut app.view {
                    *show_password = !*show_password;
                }
            } else {
                type_char(app, ' ');
            }
        }
        KeyCode::Char(c) => {
            type_char(app, c);
        }
        KeyCode::Backspace => {
            if let AppView::Form {
                draft,
                focused_field,
                error,
                ..
            } = &mut app.view
            {
                *error = None;
                get_field_mut(draft, *focused_field).pop();
            }
        }
        _ => {}
    }
}

fn is_password_field(app: &AppState) -> bool {
    if let AppView::Form { focused_field, .. } = &app.view {
        *focused_field == 2
    } else {
        false
    }
}

fn type_char(app: &mut AppState, c: char) {
    if let AppView::Form {
        draft,
        focused_field,
        error,
        ..
    } = &mut app.view
    {
        *error = None;
        get_field_mut(draft, *focused_field).push(c);
    }
}

fn get_field_mut(draft: &mut SecretDraft, field: usize) -> &mut String {
    match field {
        0 => &mut draft.name,
        1 => &mut draft.username,
        2 => &mut draft.password,
        3 => &mut draft.url,
        4 => &mut draft.tags,
        5 => &mut draft.notes,
        _ => &mut draft.name,
    }
}

fn save_form(app: &mut AppState) {
    let (mode, draft) = match &app.view {
        AppView::Form { mode, draft, .. } => (mode.clone(), draft.clone()),
        _ => return,
    };

    if let Some(msg) = draft.validate() {
        if let AppView::Form { error, .. } = &mut app.view {
            *error = Some(msg.to_string());
        }
        return;
    }

    let mut secret = Secret::new(&draft.name, &draft.password);
    if !draft.username.is_empty() {
        secret.username = Some(draft.username.clone());
    }
    if !draft.url.is_empty() {
        secret.url = Some(draft.url.clone());
    }
    if !draft.notes.is_empty() {
        secret.notes = Some(draft.notes.clone());
    }
    secret.tags = draft
        .tags
        .split(',')
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .collect();

    let result = match &mode {
        FormMode::Add => app.vault.as_mut().map(|v| v.add(secret)),
        FormMode::Edit(id) => {
            let id = *id;
            app.vault.as_mut().map(|v| v.update(id, secret))
        }
    };

    match result {
        Some(Ok(())) => {
            app.go_to_list();
            app.status = Some("Secret saved.".to_string());
        }
        Some(Err(e)) => {
            if let AppView::Form { error, .. } = &mut app.view {
                *error = Some(format!("Save failed: {e}"));
            }
        }
        None => {}
    }
}

fn handle_help(app: &mut AppState, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') => {
            app.go_to_list();
        }
        _ => {}
    }
}
