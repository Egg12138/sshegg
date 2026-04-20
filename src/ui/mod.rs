pub mod config;
pub mod filter;
pub mod highlight;
pub mod ordering;
mod state;

use crate::auth::resolve_session_password;
use crate::model::{PasswdUnsafeMode, Session};
use crate::password;
use crate::ssh::{AuthConfig, SshConnection};
use crate::store::SessionStore;
use crate::ui::state::{
    AddField, AddSessionForm, AppState, FormEditMode, InputMode, MonitorEntry, ScpDirection,
    ScpField, ScpForm,
};
use anyhow::Result;
use crossterm::cursor::{Hide, Show};
use crossterm::event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, TableState, Wrap};
use std::env;
use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub use config::{ThemeConfig, UiConfig, load_ui_config};
pub use highlight::SessionHighlight;
pub use ordering::sort_sessions;

struct Theme {
    logo: Color,
    header: Color,
    highlight: Color,
    border: Color,
    help: Color,
    status: Color,
    text: Color,
}

const PAGE_STEP: usize = 5;
const FIELD_LABEL_WIDTH: usize = 10;
const CARET_BLINK_MS: u128 = 600;
const NAVIGATION_NOTES: &str = "↑/↓ or j/k | gg/G | / search | ? for help";
const HELP_PANEL_LINES: &[&str] = &[
    "?             Toggle this help panel",
    "Enter         Connect to selected session",
    "o / O         Add session form",
    "e             Edit selected session",
    "dd            Delete selected session (confirm name)",
    "yy            Yank selected session",
    "p             Paste yanked session as a new draft",
    "/             Search (type to filter)",
    "s             Open SCP form",
    "m             Toggle monitor view",
    "Ctrl-d / Ctrl-u  Page down/up",
    "j / k / ↑ / ↓  Move selection",
    "gg / G        Jump top or bottom",
    "Esc           Close help, cancel, or dismiss error details",
];

struct PopupCursor {
    line: u16,
    column: u16,
}

struct PopupPanel<'a> {
    title: Line<'a>,
    body_lines: Vec<String>,
    accent_lines: Vec<usize>,
    width_percent: u16,
    height_percent: u16,
    cursor: Option<PopupCursor>,
    wrap: bool,
}

enum TextEntryAction {
    Cancel,
    Submit,
    Continue,
}

impl Theme {
    fn from_config(config: &UiConfig) -> Self {
        Self {
            logo: parse_color(&config.theme.logo),
            header: parse_color(&config.theme.header),
            highlight: parse_color(&config.theme.highlight),
            border: parse_color(&config.theme.border),
            help: parse_color(&config.theme.help),
            status: parse_color(&config.theme.status),
            text: parse_color(&config.theme.text),
        }
    }
}

pub fn run_tui(store: &dyn SessionStore, config: &UiConfig) -> Result<Option<Session>> {
    let mut sessions = store.list()?;

    // Apply ordering based on config
    sort_sessions(&mut sessions, config.ordering.mode);

    let mut app = AppState::new(&sessions);
    app.set_monitor_enabled(config.layout.show_monitor);
    app.set_form_default_mode(match config.input.form_default_mode {
        config::FormStartMode::Normal => FormEditMode::Normal,
        config::FormStartMode::Insert => FormEditMode::Insert,
    });
    let theme = Theme::from_config(config);
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture, Hide)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal, &mut app, store, config, &theme);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture,
        Show
    )?;
    terminal.show_cursor()?;

    result
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut AppState,
    store: &dyn SessionStore,
    config: &UiConfig,
    theme: &Theme,
) -> Result<Option<Session>> {
    loop {
        terminal.draw(|frame| draw_ui(frame, app, config, theme))?;

        if event::poll(Duration::from_millis(200))?
            && let Event::Key(key) = event::read()?
        {
            match handle_key(app, store, key) {
                Ok(Some(selection)) => return Ok(selection),
                Ok(None) => {}
                Err(err) => show_error_popup(
                    app,
                    "Operation failed (Esc closes error details)",
                    format!("{err:#}"),
                ),
            }
        }
    }
}

fn handle_key(
    app: &mut AppState,
    store: &dyn SessionStore,
    key: KeyEvent,
) -> Result<Option<Option<Session>>> {
    if key.modifiers.contains(event::KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return Ok(Some(None));
    }

    if app.has_error_popup() {
        if key.code == KeyCode::Esc {
            app.clear_error_popup();
        }
        return Ok(None);
    }

    match app.mode() {
        InputMode::Normal => handle_normal_key(app, key),
        InputMode::Search => handle_search_key(app, key),
        InputMode::ConfirmDelete => handle_confirm_delete_key(app, store, key),
        InputMode::AddSession => handle_add_session_key(app, store, key),
        InputMode::EditSession => handle_edit_session_key(app, store, key),
        InputMode::Help => handle_help_key(app, key),
        InputMode::Scp => handle_scp_key(app, store, key),
    }
}

fn show_error_popup(app: &mut AppState, reminder: &str, details: impl Into<String>) {
    app.set_error(reminder, details.into());
    app.set_pending(None);
}

fn handle_text_entry_key(
    entry: &mut crate::ui::state::TextEntryPanel,
    key: KeyEvent,
) -> TextEntryAction {
    match key.code {
        KeyCode::Esc => TextEntryAction::Cancel,
        KeyCode::Enter => TextEntryAction::Submit,
        KeyCode::Backspace => {
            entry.pop();
            TextEntryAction::Continue
        }
        KeyCode::Char(ch)
            if !key.modifiers.contains(event::KeyModifiers::CONTROL)
                && !key.modifiers.contains(event::KeyModifiers::ALT) =>
        {
            entry.push(ch);
            TextEntryAction::Continue
        }
        _ => TextEntryAction::Continue,
    }
}

fn handle_normal_key(app: &mut AppState, key: KeyEvent) -> Result<Option<Option<Session>>> {
    let mut handled = true;
    match key.code {
        KeyCode::Char('q') => return Ok(Some(None)),
        KeyCode::Enter => return Ok(Some(app.selected_session().cloned())),
        KeyCode::Char('d') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
            app.page_down(PAGE_STEP)
        }
        KeyCode::Char('u') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
            app.page_up(PAGE_STEP)
        }
        KeyCode::Char('d') => {
            if app.pending() == Some('d') {
                if app.start_delete() {
                    app.set_status("Type session name to confirm deletion");
                } else {
                    app.set_status("No session selected to delete");
                }
                app.set_pending(None);
            } else {
                app.set_pending(Some('d'));
            }
        }
        KeyCode::Char('y') => {
            if app.pending() == Some('y') {
                if let Some(name) = app.yank_selected() {
                    app.set_status(format!("Yanked session: {}", name));
                } else {
                    app.set_status("No session selected to yank");
                }
                app.set_pending(None);
            } else {
                app.set_pending(Some('y'));
            }
        }
        KeyCode::Char('p') => {
            if let Some(new_name) = app.next_copy_name_for_yank() {
                if app.start_paste_session(new_name.clone()) {
                    app.set_status(format!(
                        "Paste session draft: {} (edit and press Enter on Tags to save)",
                        new_name
                    ));
                } else {
                    app.set_status("No yanked session to paste");
                }
            } else {
                app.set_status("No yanked session to paste");
            }
        }
        KeyCode::Char('a') => {
            app.start_add_session(default_user());
            set_form_mode_status(app, "Add");
        }
        KeyCode::Char('o') | KeyCode::Char('O') => {
            app.start_add_session(default_user());
            set_form_mode_status(app, "Add");
        }
        KeyCode::Char('e') => {
            if let Some(session) = app.selected_session().cloned() {
                app.start_edit_session(&session);
                set_form_mode_status(app, "Edit");
            } else {
                app.set_status("No session selected to edit");
            }
        }
        KeyCode::Char('s') => {
            if let Some(session) = app.selected_session().cloned() {
                app.start_scp(session);
                app.set_status("SCP: Enter/Tab move fields, Esc cancel");
            } else {
                app.set_status("No session selected for SCP");
            }
        }
        KeyCode::Char('m') => app.toggle_monitor(),
        KeyCode::Up | KeyCode::Char('k') => app.move_prev(),
        KeyCode::Down | KeyCode::Char('j') => app.move_next(),
        KeyCode::Home => app.select_first(),
        KeyCode::End => app.select_last(),
        KeyCode::Char('G') => app.select_last(),
        KeyCode::Char('g') => {
            if app.pending() == Some('g') {
                app.select_first();
                app.set_pending(None);
            } else {
                app.set_pending(Some('g'));
            }
        }
        KeyCode::Char('/') => {
            app.filter.clear();
            app.refresh_filter();
            app.set_mode(InputMode::Search);
            app.set_status("Search mode: type to filter, Enter/Esc to exit");
        }
        KeyCode::Char('?') => {
            app.set_mode(InputMode::Help);
        }
        KeyCode::Char('n') => app.move_next(),
        KeyCode::Char('N') => app.move_prev(),
        KeyCode::Esc => {
            app.set_pending(None);
            app.clear_status();
        }
        _ => handled = false,
    }

    if handled {
        match key.code {
            KeyCode::Char('g') | KeyCode::Char('d') | KeyCode::Char('y') => {}
            _ => app.set_pending(None),
        }
    } else {
        app.set_pending(None);
    }

    Ok(None)
}

fn handle_search_key(app: &mut AppState, key: KeyEvent) -> Result<Option<Option<Session>>> {
    match key.code {
        KeyCode::Esc | KeyCode::Enter => {
            app.set_mode(InputMode::Normal);
            app.clear_status();
        }
        KeyCode::Backspace => app.backspace(),
        KeyCode::Up | KeyCode::Char('k') => app.move_prev(),
        KeyCode::Down | KeyCode::Char('j') => app.move_next(),
        KeyCode::Char(ch)
            if !key.modifiers.contains(event::KeyModifiers::CONTROL)
                && !key.modifiers.contains(event::KeyModifiers::ALT) =>
        {
            app.on_char(ch)
        }
        _ => {}
    }
    Ok(None)
}

fn handle_help_key(app: &mut AppState, key: KeyEvent) -> Result<Option<Option<Session>>> {
    match key.code {
        KeyCode::Char('?') | KeyCode::Esc => {
            app.set_mode(InputMode::Normal);
            app.set_pending(None);
        }
        KeyCode::Char('q') => return Ok(Some(None)),
        _ => {}
    }
    Ok(None)
}

fn handle_confirm_delete_key(
    app: &mut AppState,
    store: &dyn SessionStore,
    key: KeyEvent,
) -> Result<Option<Option<Session>>> {
    let Some(dialog) = app.delete_dialog_mut() else {
        app.cancel_delete();
        return Ok(None);
    };

    match handle_text_entry_key(dialog.entry_mut(), key) {
        TextEntryAction::Cancel => {
            app.cancel_delete();
            app.clear_status();
        }
        TextEntryAction::Submit => {
            if app.confirm_delete_matches() {
                if let Some(target) = app.delete_target().map(str::to_string) {
                    store.remove(&target)?;
                    app.remove_by_name(&target);
                    app.set_status(format!("Deleted session: {}", target));
                }
                app.cancel_delete();
            } else {
                app.set_status("Delete confirmation does not match session name");
            }
        }
        TextEntryAction::Continue => {}
    }
    Ok(None)
}

fn handle_add_session_key(
    app: &mut AppState,
    store: &dyn SessionStore,
    key: KeyEvent,
) -> Result<Option<Option<Session>>> {
    let Some(form) = app.add_form_mut() else {
        app.cancel_add_session();
        return Ok(None);
    };

    match form.edit_mode() {
        FormEditMode::Normal => match key.code {
            KeyCode::Esc => {
                app.cancel_add_session();
                app.clear_status();
            }
            KeyCode::Tab | KeyCode::Down | KeyCode::Char('j') => form.next_field(),
            KeyCode::BackTab | KeyCode::Up | KeyCode::Char('k') => form.prev_field(),
            KeyCode::Enter => {
                if form.field() == AddField::Tags {
                    submit_add_session(app, store)?;
                } else {
                    form.next_field();
                }
            }
            KeyCode::Char('i') => {
                form.enter_insert_before();
                app.set_status("Add session [INSERT]: type text, Esc normal, Enter next/save");
            }
            KeyCode::Char('a') => {
                form.enter_insert_after();
                app.set_status("Add session [INSERT]: type text, Esc normal, Enter next/save");
            }
            // Handle cursor movement and password mode cycling
            KeyCode::Left | KeyCode::Char('h') => {
                if form.field() == AddField::PasswdMode && form.show_passwd_mode() {
                    form.cycle_passwd_mode();
                } else {
                    form.move_cursor_left();
                }
            }
            KeyCode::Right | KeyCode::Char('l') => {
                if form.field() == AddField::PasswdMode && form.show_passwd_mode() {
                    form.cycle_passwd_mode();
                } else {
                    form.move_cursor_right();
                }
            }
            KeyCode::Char(' ')
                if form.field() == AddField::PasswdMode && form.show_passwd_mode() =>
            {
                form.cycle_passwd_mode();
            }
            _ => {}
        },
        FormEditMode::Insert => match key.code {
            KeyCode::Esc => {
                form.set_edit_mode(FormEditMode::Normal);
                app.set_status(
                    "Add session [NORMAL]: i/a insert, h/l move cursor, j/k change field, Esc cancel",
                );
            }
            KeyCode::Tab | KeyCode::Down => form.next_field(),
            KeyCode::BackTab | KeyCode::Up => form.prev_field(),
            KeyCode::Left => form.move_cursor_left(),
            KeyCode::Right => form.move_cursor_right(),
            KeyCode::Enter => {
                if form.field() == AddField::Tags {
                    submit_add_session(app, store)?;
                } else {
                    form.next_field();
                }
            }
            KeyCode::Backspace => {
                let active_field = form.field();
                form.backspace_char();
                if active_field == AddField::Identity {
                    update_identity_state(form);
                }
            }
            KeyCode::Char(ch)
                if !key.modifiers.contains(event::KeyModifiers::CONTROL)
                    && !key.modifiers.contains(event::KeyModifiers::ALT) =>
            {
                let active_field = form.field();
                form.insert_char(ch);
                if active_field == AddField::Identity {
                    update_identity_state(form);
                }
            }
            _ => {}
        },
    }

    Ok(None)
}

fn handle_edit_session_key(
    app: &mut AppState,
    store: &dyn SessionStore,
    key: KeyEvent,
) -> Result<Option<Option<Session>>> {
    let Some(form) = app.add_form_mut() else {
        app.cancel_add_session();
        return Ok(None);
    };

    match form.edit_mode() {
        FormEditMode::Normal => match key.code {
            KeyCode::Esc => {
                app.cancel_add_session();
                app.clear_status();
            }
            KeyCode::Tab | KeyCode::Down | KeyCode::Char('j') => form.next_field(),
            KeyCode::BackTab | KeyCode::Up | KeyCode::Char('k') => form.prev_field(),
            KeyCode::Enter => {
                if form.field() == AddField::Tags {
                    submit_edit_session(app, store)?;
                } else {
                    form.next_field();
                }
            }
            KeyCode::Char('i') => {
                form.enter_insert_before();
                app.set_status("Edit session [INSERT]: type text, Esc normal, Enter next/save");
            }
            KeyCode::Char('a') => {
                form.enter_insert_after();
                app.set_status("Edit session [INSERT]: type text, Esc normal, Enter next/save");
            }
            // Handle cursor movement and password mode cycling
            KeyCode::Left | KeyCode::Char('h') => {
                if form.field() == AddField::PasswdMode && form.show_passwd_mode() {
                    form.cycle_passwd_mode();
                } else {
                    form.move_cursor_left();
                }
            }
            KeyCode::Right | KeyCode::Char('l') => {
                if form.field() == AddField::PasswdMode && form.show_passwd_mode() {
                    form.cycle_passwd_mode();
                } else {
                    form.move_cursor_right();
                }
            }
            KeyCode::Char(' ')
                if form.field() == AddField::PasswdMode && form.show_passwd_mode() =>
            {
                form.cycle_passwd_mode();
            }
            _ => {}
        },
        FormEditMode::Insert => match key.code {
            KeyCode::Esc => {
                form.set_edit_mode(FormEditMode::Normal);
                app.set_status(
                    "Edit session [NORMAL]: i/a insert, h/l move cursor, j/k change field, Esc cancel",
                );
            }
            KeyCode::Tab | KeyCode::Down => form.next_field(),
            KeyCode::BackTab | KeyCode::Up => form.prev_field(),
            KeyCode::Left => form.move_cursor_left(),
            KeyCode::Right => form.move_cursor_right(),
            KeyCode::Enter => {
                if form.field() == AddField::Tags {
                    submit_edit_session(app, store)?;
                } else {
                    form.next_field();
                }
            }
            KeyCode::Backspace => {
                let active_field = form.field();
                form.backspace_char();
                if active_field == AddField::Identity {
                    update_identity_state(form);
                }
            }
            KeyCode::Char(ch)
                if !key.modifiers.contains(event::KeyModifiers::CONTROL)
                    && !key.modifiers.contains(event::KeyModifiers::ALT) =>
            {
                let active_field = form.field();
                form.insert_char(ch);
                if active_field == AddField::Identity {
                    update_identity_state(form);
                }
            }
            _ => {}
        },
    }

    Ok(None)
}

fn handle_scp_key(
    app: &mut AppState,
    store: &dyn SessionStore,
    key: KeyEvent,
) -> Result<Option<Option<Session>>> {
    let mut submit_password = false;
    let mut refresh_autocomplete = false;
    {
        let Some(form) = app.scp_form_mut() else {
            app.cancel_scp();
            return Ok(None);
        };

        if let Some(prompt) = form.password_prompt_mut() {
            match handle_text_entry_key(prompt, key) {
                TextEntryAction::Cancel => {
                    form.close_password_prompt();
                    app.set_status("SCP password entry cancelled");
                }
                TextEntryAction::Submit => submit_password = true,
                TextEntryAction::Continue => {}
            }
            if !submit_password {
                return Ok(None);
            }
        } else {
            let field = form.field();

            match key.code {
                KeyCode::Esc => {
                    app.cancel_scp();
                    app.clear_status();
                }
                KeyCode::Tab => {
                    if matches!(field, ScpField::Local | ScpField::Remote)
                        && form.apply_selected_suggestion()
                    {
                        refresh_autocomplete = true;
                    } else {
                        form.next_field();
                        refresh_autocomplete =
                            matches!(form.field(), ScpField::Local | ScpField::Remote);
                    }
                }
                KeyCode::BackTab => {
                    form.prev_field();
                    refresh_autocomplete =
                        matches!(form.field(), ScpField::Local | ScpField::Remote);
                }
                KeyCode::Enter => {
                    if field == ScpField::Recursive {
                        submit_scp(app, store)?;
                    } else {
                        form.next_field();
                        refresh_autocomplete =
                            matches!(form.field(), ScpField::Local | ScpField::Remote);
                    }
                }
                KeyCode::Up if matches!(field, ScpField::Local | ScpField::Remote) => {
                    form.select_prev_suggestion();
                }
                KeyCode::Down if matches!(field, ScpField::Local | ScpField::Remote) => {
                    form.select_next_suggestion();
                }
                KeyCode::Left | KeyCode::Right | KeyCode::Char('t') | KeyCode::Char(' ')
                    if field == ScpField::Direction =>
                {
                    form.toggle_direction();
                }
                KeyCode::Left | KeyCode::Right | KeyCode::Char('r') | KeyCode::Char(' ')
                    if field == ScpField::Recursive =>
                {
                    form.toggle_recursive();
                }
                KeyCode::Backspace => {
                    if let Some(value) = form.active_editable_mut() {
                        value.pop();
                        refresh_autocomplete = true;
                    }
                }
                KeyCode::Char(ch)
                    if !key.modifiers.contains(event::KeyModifiers::CONTROL)
                        && !key.modifiers.contains(event::KeyModifiers::ALT) =>
                {
                    if let Some(value) = form.active_editable_mut() {
                        value.push(ch);
                        refresh_autocomplete = true;
                    }
                }
                _ => {}
            }
        }
    }

    if refresh_autocomplete {
        update_scp_autocomplete(app, store)?;
    }

    if submit_password {
        submit_scp(app, store)?;
    }

    Ok(None)
}

fn draw_ui(frame: &mut ratatui::Frame, app: &mut AppState, config: &UiConfig, theme: &Theme) {
    let show_inline_caret = inline_caret_visible();
    let size = frame.area();
    let mut constraints = Vec::new();
    let mut logo_index = None;
    let mut search_index = None;
    let mut cheat_index = None;

    if config.layout.show_logo && config.logo.enabled {
        logo_index = Some(constraints.len());
        constraints.push(Constraint::Length(config.layout.logo_height));
    }
    if config.layout.show_search {
        search_index = Some(constraints.len());
        constraints.push(Constraint::Length(config.layout.search_height));
    }
    let sessions_index = constraints.len();
    constraints.push(Constraint::Min(3));
    if config.layout.show_status || config.layout.show_help {
        cheat_index = Some(constraints.len());
        let mut bar_height = config.layout.help_height;
        if config.layout.show_status {
            bar_height = bar_height.saturating_add(config.layout.status_height);
        }
        constraints.push(Constraint::Length(bar_height));
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(size);

    if let Some(index) = logo_index {
        let logo_text = config.logo.lines.join("\n");
        let logo = Paragraph::new(logo_text).style(Style::default().fg(theme.logo));
        frame.render_widget(logo, chunks[index]);
    }

    if let Some(index) = search_index {
        let search_label = format!("/{}", app.filter);
        let filter = Paragraph::new(search_label)
            .style(Style::default().fg(theme.text))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.border))
                    .title("Search"),
            );
        frame.render_widget(filter, chunks[index]);

        if app.mode() == InputMode::Search {
            let cursor_x = chunks[index].x + 2 + app.filter.len() as u16;
            let cursor_y = chunks[index].y + 1;
            frame.set_cursor_position((cursor_x, cursor_y));
        }
    }

    let sessions_area = chunks[sessions_index];
    let (table_area, monitor_area) = if app.monitor_enabled() {
        let split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(3),
                Constraint::Length(config.layout.monitor_height),
            ])
            .split(sessions_area);
        (split[0], Some(split[1]))
    } else {
        (sessions_area, None)
    };

    let header = Row::new(vec![
        Cell::from("Name"),
        Cell::from("Target"),
        Cell::from("Port"),
        Cell::from("Identity"),
        Cell::from("Tags"),
        Cell::from("Pwd"),
    ])
    .style(
        Style::default()
            .fg(theme.header)
            .add_modifier(Modifier::BOLD),
    );

    let rows = app.filtered_sessions().into_iter().map(|session| {
        let identity = session
            .identity_file
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "-".to_string());
        let tags = if session.tags.is_empty() {
            "-".to_string()
        } else {
            session.tags.join(",")
        };
        let password_indicator = if session.has_stored_password {
            "★"
        } else {
            "-"
        };
        let highlight_style = get_session_highlight(session, config, theme);
        Row::new(vec![
            Cell::from(session.name.clone()),
            Cell::from(session.target()),
            Cell::from(session.port.to_string()),
            Cell::from(identity),
            Cell::from(tags),
            Cell::from(password_indicator),
        ])
        .style(highlight_style)
    });

    let table = Table::new(
        rows,
        [
            Constraint::Length(20),
            Constraint::Length(30),
            Constraint::Length(6),
            Constraint::Length(18),
            Constraint::Min(10),
            Constraint::Length(5),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border))
            .title("Sessions"),
    )
    .highlight_style(
        Style::default()
            .fg(theme.text)
            .bg(theme.highlight)
            .add_modifier(Modifier::BOLD),
    );

    let mut state = TableState::default();
    if let Some(selected) = app.selected_index() {
        state.select(Some(selected));
    }
    frame.render_stateful_widget(table, table_area, &mut state);

    if let Some(area) = monitor_area {
        let monitor_text = if let Some(session) = app.selected_session().cloned() {
            refresh_monitor(app, &session);
            let entries = app.monitor_entries();
            let connection_text = if entries.is_empty() {
                "Connections: -".to_string()
            } else {
                let connections = entries
                    .iter()
                    .map(|entry| {
                        let tty = entry.tty.as_deref().unwrap_or("?");
                        format!("{}@{}", entry.pid, tty)
                    })
                    .collect::<Vec<String>>()
                    .join(", ");
                format!("Connections: {}", connections)
            };
            let last_text = format!(
                "Last connected: {}",
                format_last_connected(session.last_connected_at)
            );
            format!("Host: {}\n{}\n{}", session.host, connection_text, last_text)
        } else {
            "No session selected.".to_string()
        };

        let monitor = Paragraph::new(monitor_text).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border))
                .title("Monitor"),
        );
        frame.render_widget(monitor, area);
    }

    if let Some(index) = cheat_index {
        let mut lines = Vec::new();
        if config.layout.show_status {
            let total = app.filtered_sessions().len();
            let selected_session = app.selected_session().cloned();
            if let Some(session) = selected_session.as_ref() {
                refresh_monitor(app, session);
            }
            let connections_text = match selected_session {
                Some(session) => {
                    let count = app.monitor_entries().len();
                    if count > 0 {
                        format!("{} connections on {}", count, session.name)
                    } else {
                        "not connected".to_string()
                    }
                }
                None => "not connected".to_string(),
            };
            let mut status_line =
                format!("Focus: Session | {} sessions | {}", total, connections_text);
            if !app.status().is_empty() {
                status_line.push_str(" | ");
                status_line.push_str(app.status());
            }
            lines.push(Line::styled(status_line, Style::default().fg(theme.status)));
        }
        if config.layout.show_help {
            lines.push(Line::styled(
                mode_help_text(app.mode()),
                Style::default().fg(theme.help),
            ));
        }
        let nav_line = format!("Navigation: {}", NAVIGATION_NOTES);
        lines.push(Line::styled(nav_line, Style::default().fg(theme.help)));

        let info = Paragraph::new(Text::from(lines)).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border))
                .title("Info"),
        );
        frame.render_widget(info, chunks[index]);
    }

    if app.mode() == InputMode::ConfirmDelete {
        if let Some(dialog) = app.delete_dialog() {
            render_popup_panel(
                frame,
                size,
                theme,
                build_text_entry_popup(
                    dialog.entry(),
                    &[format!("Delete session: {}", dialog.target())],
                    60,
                    30,
                ),
            );
        }
    }

    if app.mode() == InputMode::AddSession
        && let Some(form) = app.add_form()
    {
        render_popup_panel(
            frame,
            size,
            theme,
            PopupPanel {
                title: form_panel_title("Add", form.edit_mode(), theme),
                body_lines: build_add_form_lines(form, show_inline_caret),
                accent_lines: Vec::new(),
                width_percent: 70,
                height_percent: 50,
                cursor: None,
                wrap: false,
            },
        );
    }

    if app.mode() == InputMode::EditSession
        && let Some(form) = app.add_form()
    {
        render_popup_panel(
            frame,
            size,
            theme,
            PopupPanel {
                title: form_panel_title("Edit", form.edit_mode(), theme),
                body_lines: build_add_form_lines(form, show_inline_caret),
                accent_lines: Vec::new(),
                width_percent: 70,
                height_percent: 50,
                cursor: None,
                wrap: false,
            },
        );
    }

    if app.mode() == InputMode::Scp
        && let Some(form) = app.scp_form()
    {
        if let Some(prompt) = form.password_prompt() {
            render_popup_panel(
                frame,
                size,
                theme,
                build_text_entry_popup(
                    prompt,
                    &[
                        format!("Session: {} ({})", form.session.name, form.session.target()),
                        "Leave blank to try key-based auth".to_string(),
                    ],
                    60,
                    30,
                ),
            );
        } else {
            render_popup_panel(
                frame,
                size,
                theme,
                PopupPanel {
                    title: Line::from("SCP"),
                    body_lines: build_scp_form_lines(form, show_inline_caret),
                    accent_lines: Vec::new(),
                    width_percent: 70,
                    height_percent: 45,
                    cursor: None,
                    wrap: false,
                },
            );
        }
    }

    if app.mode() == InputMode::Help {
        render_popup_panel(
            frame,
            size,
            theme,
            PopupPanel {
                title: Line::from("Help"),
                body_lines: HELP_PANEL_LINES
                    .iter()
                    .map(|line| (*line).to_string())
                    .collect(),
                accent_lines: (0..HELP_PANEL_LINES.len()).collect(),
                width_percent: 70,
                height_percent: 60,
                cursor: None,
                wrap: false,
            },
        );
    }

    if let Some(error_details) = app.error_popup() {
        render_popup_panel(
            frame,
            size,
            theme,
            PopupPanel {
                title: Line::from("Error"),
                body_lines: vec![
                    "An error occurred.".to_string(),
                    String::new(),
                    error_details.to_string(),
                    String::new(),
                    "[Esc] Close this panel".to_string(),
                ],
                accent_lines: vec![0, 4],
                width_percent: 80,
                height_percent: 70,
                cursor: None,
                wrap: true,
            },
        );
    }
}

fn get_session_highlight(session: &Session, config: &UiConfig, _theme: &Theme) -> Style {
    let highlight =
        SessionHighlight::classify(session, config.ordering.lifetime.dying_threshold_days);

    let color = match highlight {
        SessionHighlight::Hot => parse_color(&config.highlights.hot),
        SessionHighlight::Normal => parse_color(&config.highlights.normal),
        SessionHighlight::Dying => parse_color(&config.highlights.dying),
    };

    Style::default().fg(color)
}

fn parse_color(name: &str) -> Color {
    match name.to_lowercase().as_str() {
        "black" => Color::Black,
        "red" => Color::Red,
        "green" => Color::Green,
        "yellow" => Color::Yellow,
        "blue" => Color::Blue,
        "magenta" => Color::Magenta,
        "cyan" => Color::Cyan,
        "gray" => Color::Gray,
        "darkgray" | "dark_gray" => Color::DarkGray,
        "lightred" | "light_red" => Color::LightRed,
        "lightgreen" | "light_green" => Color::LightGreen,
        "lightyellow" | "light_yellow" => Color::LightYellow,
        "lightblue" | "light_blue" => Color::LightBlue,
        "lightmagenta" | "light_magenta" => Color::LightMagenta,
        "lightcyan" | "light_cyan" => Color::LightCyan,
        "white" => Color::White,
        _ => Color::White,
    }
}

fn mode_help_text(mode: InputMode) -> &'static str {
    match mode {
        InputMode::Normal => {
            "j/k move | gg top | G bottom | Ctrl-d/u page | / search | o/O add | e edit | s scp | m monitor | yy yank | p paste | dd delete | Enter connect | q quit"
        }
        InputMode::Search => "Type to filter | Enter/Esc to exit | j/k move",
        InputMode::ConfirmDelete => "Type name | Enter confirm | Esc cancel",
        InputMode::AddSession => {
            "NORMAL: i/a insert, h/l cursor, j/k field, Enter next/save, Esc cancel | INSERT: type, Backspace, Esc normal"
        }
        InputMode::EditSession => {
            "NORMAL: i/a insert, h/l cursor, j/k field, Enter next/save, Esc cancel | INSERT: type, Backspace, Esc normal"
        }
        InputMode::Scp => "Tab/Enter next | Space toggle | Esc cancel",
        InputMode::Help => "? or Esc close | Ctrl-c exit",
    }
}

fn form_panel_title(panel: &str, mode: FormEditMode, theme: &Theme) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("{panel} Session | "),
            Style::default()
                .fg(theme.header)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            FormEditMode::Normal.label(),
            form_mode_badge_style(
                matches!(mode, FormEditMode::Normal),
                theme.header,
                theme.text,
            ),
        ),
        Span::raw(" "),
        Span::styled(
            FormEditMode::Insert.label(),
            form_mode_badge_style(
                matches!(mode, FormEditMode::Insert),
                theme.highlight,
                theme.text,
            ),
        ),
    ])
}

fn form_mode_badge_style(active: bool, active_color: Color, inactive_color: Color) -> Style {
    if active {
        Style::default()
            .fg(active_color)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
    } else {
        Style::default()
            .fg(inactive_color)
            .add_modifier(Modifier::DIM)
    }
}

fn set_form_mode_status(app: &mut AppState, panel: &str) {
    let Some(form) = app.add_form() else {
        return;
    };
    match form.edit_mode() {
        FormEditMode::Normal => app.set_status(format!(
            "{panel} session [NORMAL]: i/a insert, h/l move cursor, j/k change field, Esc cancel"
        )),
        FormEditMode::Insert => app.set_status(format!(
            "{panel} session [INSERT]: type text, Esc normal, Enter next/save"
        )),
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, rect: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(rect);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn render_popup_panel(
    frame: &mut ratatui::Frame,
    size: Rect,
    theme: &Theme,
    panel: PopupPanel<'_>,
) {
    let modal_area = centered_rect(panel.width_percent, panel.height_percent, size);
    frame.render_widget(Clear, modal_area);

    let text = panel
        .body_lines
        .into_iter()
        .enumerate()
        .map(|(index, line)| {
            let style = if panel.accent_lines.contains(&index) {
                Style::default().fg(theme.help)
            } else {
                Style::default().fg(theme.text)
            };
            Line::styled(line, style)
        })
        .collect::<Vec<_>>();

    let mut paragraph = Paragraph::new(Text::from(text)).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border))
            .title(panel.title),
    );
    if panel.wrap {
        paragraph = paragraph.wrap(Wrap { trim: false });
    }
    frame.render_widget(paragraph, modal_area);

    if let Some(cursor) = panel.cursor {
        frame.set_cursor_position((
            modal_area.x + 1 + cursor.column,
            modal_area.y + 1 + cursor.line,
        ));
    }
}

fn build_text_entry_popup(
    entry: &crate::ui::state::TextEntryPanel,
    context_lines: &[String],
    width_percent: u16,
    height_percent: u16,
) -> PopupPanel<'static> {
    let mut lines = context_lines.to_vec();
    let mut accent_lines = Vec::new();
    for index in 0..context_lines.len() {
        accent_lines.push(index);
    }
    if !lines.is_empty() {
        lines.push(String::new());
    }
    let prompt_index = lines.len();
    lines.push(entry.prompt().to_string());
    accent_lines.push(prompt_index);
    lines.push(String::new());
    let input_line = lines.len() as u16;
    lines.push(format!("> {}", entry.display_value()));
    lines.push(String::new());
    let footer_index = lines.len();
    lines.push(entry.footer_hint());
    accent_lines.push(footer_index);

    let input_column = 2 + entry.display_value().chars().count() as u16;

    PopupPanel {
        title: Line::from(entry.title().to_string()),
        body_lines: lines,
        accent_lines,
        width_percent,
        height_percent,
        cursor: Some(PopupCursor {
            line: input_line,
            column: input_column,
        }),
        wrap: false,
    }
}

fn build_add_form_lines(form: &AddSessionForm, show_inline_caret: bool) -> Vec<String> {
    // Mask password display
    let password_display = if form.password.is_empty() {
        String::new()
    } else {
        "*".repeat(form.password.len())
    };
    let active_caret = Caret::new(form.edit_mode(), form.cursor(), show_inline_caret);

    // Password mode label
    let passwd_mode_label = match form.passwd_mode {
        PasswdUnsafeMode::Normal => "normal (keyring)",
        PasswdUnsafeMode::Bare => "bare (plaintext)",
        PasswdUnsafeMode::Simple => "simple (XOR)",
    };

    let mut lines = vec![
        field_line(
            "Name",
            &form.name,
            caret_for(form, AddField::Name, active_caret),
        ),
        field_line(
            "Host",
            &form.host,
            caret_for(form, AddField::Host, active_caret),
        ),
        field_line(
            "User",
            &form.user,
            caret_for(form, AddField::User, active_caret),
        ),
        field_line(
            "Port",
            &form.port,
            caret_for(form, AddField::Port, active_caret),
        ),
        field_line(
            "Identity",
            &form.identity_file,
            caret_for(form, AddField::Identity, active_caret),
        ),
        field_line(
            "Password",
            &password_display,
            caret_for(form, AddField::Password, active_caret),
        ),
    ];

    // Only show password mode selector when password is entered
    if form.show_passwd_mode() {
        lines.push(field_line(
            "Pwd Mode",
            passwd_mode_label,
            if form.field() == AddField::PasswdMode {
                Some(Caret::new(
                    FormEditMode::Insert,
                    passwd_mode_label.chars().count(),
                    show_inline_caret,
                ))
            } else {
                None
            },
        ));
    }

    lines.push(field_line(
        "Tags",
        &form.tags,
        caret_for(form, AddField::Tags, active_caret),
    ));

    let identity_status = match form.identity_exists() {
        Some(true) => "yes",
        Some(false) => "missing",
        None => "-",
    };
    lines.push(format!("  Identity exists: {}", identity_status));

    if !form.identity_suggestions().is_empty() {
        lines.push("  Suggestions:".to_string());
        for suggestion in form.identity_suggestions().iter().take(3) {
            lines.push(format!("    {}", suggestion));
        }
    }

    if form.show_passwd_mode() {
        lines.push("  Space/Left/Right cycles password mode".to_string());
    }

    lines
}

fn build_scp_form_lines(form: &ScpForm, show_inline_caret: bool) -> Vec<String> {
    let mut lines = Vec::new();
    lines.push(format!(
        "Session: {} ({})",
        form.session.name,
        form.session.target()
    ));
    lines.push(field_line(
        "Direction",
        form.direction.label(),
        if form.field() == ScpField::Direction {
            Some(Caret::new(
                FormEditMode::Insert,
                form.direction.label().chars().count(),
                show_inline_caret,
            ))
        } else {
            None
        },
    ));
    lines.push(field_line(
        "Local",
        &form.local_path,
        if form.field() == ScpField::Local {
            Some(Caret::new(
                FormEditMode::Insert,
                form.local_path.chars().count(),
                show_inline_caret,
            ))
        } else {
            None
        },
    ));
    lines.push(field_line(
        "Remote",
        &form.remote_path,
        if form.field() == ScpField::Remote {
            Some(Caret::new(
                FormEditMode::Insert,
                form.remote_path.chars().count(),
                show_inline_caret,
            ))
        } else {
            None
        },
    ));
    let recursive_value = if form.recursive { "yes" } else { "no" };
    lines.push(field_line(
        "Recursive",
        recursive_value,
        if form.field() == ScpField::Recursive {
            Some(Caret::new(
                FormEditMode::Insert,
                recursive_value.chars().count(),
                show_inline_caret,
            ))
        } else {
            None
        },
    ));
    if !form.active_suggestions().is_empty() {
        lines.push("  Suggestions:".to_string());
        for (index, suggestion) in form.active_suggestions().iter().take(5).enumerate() {
            let marker = if form.selected_suggestion_index() == Some(index) {
                "->"
            } else {
                "  "
            };
            lines.push(format!("  {marker} {suggestion}"));
        }
    }
    lines.push("  Tab applies the selected suggestion, then moves on".to_string());
    lines.push("  Up/Down cycle path suggestions".to_string());
    lines.push("  Space toggles Direction/Recursive".to_string());
    lines.push("  Enter starts a password panel before transfer".to_string());
    lines
}

fn field_line(label: &str, value: &str, caret: Option<Caret>) -> String {
    let marker = if caret.is_some() { ">" } else { " " };
    let value = render_with_caret(value, caret);
    format!(
        "{} {:<width$} {}",
        marker,
        label,
        value,
        width = FIELD_LABEL_WIDTH
    )
}

#[derive(Debug, Clone, Copy)]
struct Caret {
    mode: FormEditMode,
    char_index: usize,
    blink_visible: bool,
}

impl Caret {
    fn new(mode: FormEditMode, char_index: usize, blink_visible: bool) -> Self {
        Self {
            mode,
            char_index,
            blink_visible,
        }
    }
}

fn caret_for(form: &AddSessionForm, field: AddField, active_caret: Caret) -> Option<Caret> {
    if form.field() == field {
        Some(active_caret)
    } else {
        None
    }
}

fn render_with_caret(value: &str, caret: Option<Caret>) -> String {
    let Some(caret) = caret else {
        return value.to_string();
    };

    let show = match caret.mode {
        FormEditMode::Normal => true,
        FormEditMode::Insert => caret.blink_visible,
    };
    if !show {
        return value.to_string();
    }

    let cursor = caret.char_index.min(value.chars().count());
    let byte_index = char_to_byte_index(value, cursor);
    let mut rendered = String::with_capacity(value.len() + 1);
    rendered.push_str(&value[..byte_index]);
    rendered.push('|');
    rendered.push_str(&value[byte_index..]);
    rendered
}

fn char_to_byte_index(value: &str, char_index: usize) -> usize {
    if char_index == 0 {
        return 0;
    }

    value
        .char_indices()
        .nth(char_index)
        .map(|(idx, _)| idx)
        .unwrap_or(value.len())
}

fn inline_caret_visible() -> bool {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| (duration.as_millis() / CARET_BLINK_MS).is_multiple_of(2))
        .unwrap_or(true)
}

fn submit_add_session(app: &mut AppState, store: &dyn SessionStore) -> Result<()> {
    let Some(form) = app.add_form() else {
        return Ok(());
    };

    let name = form.name.trim().to_string();
    let host = form.host.trim().to_string();
    let user = form.user.trim().to_string();
    let port_input = form.port.trim().to_string();
    let identity_input = form.identity_file.trim().to_string();
    let password = form.password.clone();
    let passwd_mode = form.passwd_mode.clone();
    let tags_input = form.tags.clone();

    if name.is_empty() || host.is_empty() || user.is_empty() {
        app.set_status("Name, host, and user are required");
        return Ok(());
    }

    let port = if port_input.is_empty() {
        22
    } else {
        match port_input.parse::<u16>() {
            Ok(port) => port,
            Err(_) => {
                app.set_status("Port must be a valid number");
                return Ok(());
            }
        }
    };

    let identity_file = if identity_input.is_empty() {
        None
    } else {
        Some(std::path::PathBuf::from(identity_input))
    };

    let tags = split_tags(&tags_input);

    // Handle password storage based on selected mode
    let mut password_warning: Option<String> = None;
    let mut stored_password: Option<String> = None;
    let has_stored_password = if !password.is_empty() {
        match passwd_mode {
            PasswdUnsafeMode::Normal => match try_store_password_for_ui(&name, &password)? {
                Some(warning) => {
                    password_warning = Some(warning);
                    false
                }
                None => true,
            },
            PasswdUnsafeMode::Bare => {
                stored_password = Some(password.clone());
                true
            }
            PasswdUnsafeMode::Simple => {
                let config = store.get_config()?;
                match password::store_unsafe_password(
                    &password,
                    &PasswdUnsafeMode::Simple,
                    config.passwd_unsafe_key.as_deref(),
                ) {
                    Ok(encoded) => {
                        stored_password = Some(encoded);
                        true
                    }
                    Err(err) => {
                        password_warning = Some(format!("password not saved: {}", err));
                        false
                    }
                }
            }
        }
    } else {
        false
    };

    // Determine passwd_unsafe_mode field - only set if password is provided
    let session_passwd_mode = if password.is_empty() {
        None
    } else {
        Some(passwd_mode)
    };

    let session = Session {
        name: name.clone(),
        host,
        user,
        port,
        identity_file,
        tags,
        last_connected_at: None,
        has_stored_password,
        passwd_unsafe_mode: session_passwd_mode,
        stored_password,
    };

    if let Err(err) = store.add(session.clone()) {
        show_error_popup(
            app,
            "Failed to add session (Esc closes error details)",
            format!("Failed to add session: {err:#}"),
        );
        return Ok(());
    }

    app.add_session(session);
    app.cancel_add_session();
    if let Some(warning) = password_warning {
        app.set_status(format!("Added session: {} ({})", name, warning));
    } else {
        app.set_status(format!("Added session: {}", name));
    }
    Ok(())
}

fn submit_edit_session(app: &mut AppState, store: &dyn SessionStore) -> Result<()> {
    let Some(form) = app.add_form() else {
        return Ok(());
    };

    let original_name = form.name.trim().to_string();
    let name = original_name.clone();
    let host = form.host.trim().to_string();
    let user = form.user.trim().to_string();
    let port_input = form.port.trim().to_string();
    let identity_input = form.identity_file.trim().to_string();
    let password = form.password.clone();
    let passwd_mode = form.passwd_mode.clone();
    let tags_input = form.tags.clone();

    if name.is_empty() || host.is_empty() || user.is_empty() {
        app.set_status("Name, host, and user are required");
        return Ok(());
    }

    let port = if port_input.is_empty() {
        22
    } else {
        match port_input.parse::<u16>() {
            Ok(port) => port,
            Err(_) => {
                app.set_status("Port must be a valid number");
                return Ok(());
            }
        }
    };

    let identity_file = if identity_input.is_empty() {
        None
    } else {
        Some(std::path::PathBuf::from(identity_input))
    };

    let tags = split_tags(&tags_input);

    // Get existing session for password preservation
    let existing_session = store.list()?.into_iter().find(|s| s.name == original_name);

    // Handle password update - preserve existing if no new password provided
    let mut password_warning: Option<String> = None;
    let mut stored_password: Option<String> = None;
    let (has_stored_password, session_passwd_mode) = if !password.is_empty() {
        // New password provided - store according to selected mode
        match passwd_mode {
            PasswdUnsafeMode::Normal => {
                let result = match try_store_password_for_ui(&name, &password)? {
                    Some(warning) => {
                        password_warning = Some(warning);
                        false
                    }
                    None => true,
                };
                (result, None)
            }
            PasswdUnsafeMode::Bare => {
                stored_password = Some(password.clone());
                (true, Some(PasswdUnsafeMode::Bare))
            }
            PasswdUnsafeMode::Simple => {
                let config = store.get_config()?;
                match password::store_unsafe_password(
                    &password,
                    &PasswdUnsafeMode::Simple,
                    config.passwd_unsafe_key.as_deref(),
                ) {
                    Ok(encoded) => {
                        stored_password = Some(encoded);
                        (true, Some(PasswdUnsafeMode::Simple))
                    }
                    Err(err) => {
                        password_warning = Some(format!("password not saved: {}", err));
                        (false, None)
                    }
                }
            }
        }
    } else {
        // No new password - preserve existing
        if let Some(ref existing) = existing_session {
            (
                existing.has_stored_password,
                existing.passwd_unsafe_mode.clone(),
            )
        } else {
            (false, None)
        }
    };

    // Preserve stored_password from existing session if not updating
    if password.is_empty() {
        stored_password = existing_session.and_then(|s| s.stored_password);
    }

    let session = Session {
        name: name.clone(),
        host,
        user,
        port,
        identity_file,
        tags,
        last_connected_at: None,
        has_stored_password,
        passwd_unsafe_mode: session_passwd_mode,
        stored_password,
    };

    if let Err(err) = store.update(session.clone()) {
        show_error_popup(
            app,
            "Failed to update session (Esc closes error details)",
            format!("Failed to update session: {err:#}"),
        );
        return Ok(());
    }

    app.update_session(&original_name, session);
    app.cancel_add_session();
    if let Some(warning) = password_warning {
        app.set_status(format!("Updated session: {} ({})", name, warning));
    } else {
        app.set_status(format!("Updated session: {}", name));
    }
    Ok(())
}

fn try_store_password_for_ui(session_name: &str, value: &str) -> Result<Option<String>> {
    use crate::password;

    match password::store_password(session_name, value) {
        Ok(()) => Ok(None),
        Err(err) if password::is_backend_unavailable_error(&err) => {
            Ok(Some("password not saved: keyring unavailable".to_string()))
        }
        Err(err) => Err(err),
    }
}

fn submit_scp(app: &mut AppState, store: &dyn SessionStore) -> Result<()> {
    let Some(form) = app.scp_form() else {
        return Ok(());
    };

    let local_path = form.local_path.trim().to_string();
    let remote_path = form.remote_path.trim().to_string();
    if local_path.is_empty() || remote_path.is_empty() {
        app.set_status("Local and remote paths are required");
        return Ok(());
    }

    let session = form.session.clone();
    let direction = form.direction;
    let recursive = form.recursive;
    let prompt_open = form.password_prompt().is_some();
    let prompted_password = form
        .password_prompt()
        .map(|prompt| prompt.value().to_string());

    let password = if prompt_open {
        prompted_password.filter(|value| !value.is_empty())
    } else {
        match resolve_session_password(store, &session) {
            Ok(password) => password,
            Err(err) if password::is_backend_unavailable_error(&err) => None,
            Err(err) => return Err(err),
        }
    };

    if password.is_none() && !prompt_open {
        if let Some(form) = app.scp_form_mut() {
            form.open_password_prompt();
        }
        app.set_status("SCP password: enter a password, or leave it blank to try key-based auth");
        return Ok(());
    }

    let output = run_scp_transfer(
        &session,
        direction,
        recursive,
        &local_path,
        &remote_path,
        password.as_deref(),
    )?;
    if !output.status.success() {
        if !prompt_open && let Some(form) = app.scp_form_mut() {
            form.open_password_prompt();
        }

        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let details = if stderr.is_empty() {
            format!("scp exited with status {}", output.status)
        } else {
            stderr
        };
        app.set_error("SCP transfer failed (Esc closes error details)", details);
        return Ok(());
    }

    store.touch_last_connected(&session.name, now_epoch_seconds())?;
    if let Some(form) = app.scp_form_mut() {
        form.close_password_prompt();
    }
    app.cancel_scp();
    app.set_status(format!("SCP complete: {}", session.name));
    Ok(())
}

fn run_scp_transfer(
    session: &Session,
    direction: ScpDirection,
    recursive: bool,
    local_path: &str,
    remote_path: &str,
    password: Option<&str>,
) -> Result<std::process::Output> {
    let mut command = std::process::Command::new("scp");
    if recursive {
        command.arg("-r");
    }
    if let Some(identity) = &session.identity_file {
        command.arg("-i").arg(identity);
    }
    command.arg("-P").arg(session.port.to_string());

    let remote_target = format!("{}@{}:{}", session.user, session.host, remote_path);
    match direction {
        ScpDirection::To => {
            command.arg(local_path).arg(remote_target);
        }
        ScpDirection::From => {
            command.arg(remote_target).arg(local_path);
        }
    }

    if let Some(password) = password {
        let script_path = write_askpass_script()?;
        command
            .env("SE_ASKPASS_PASSWORD", password)
            .env("SSH_ASKPASS", &script_path)
            .env("SSH_ASKPASS_REQUIRE", "force")
            .env(
                "DISPLAY",
                env::var("DISPLAY").unwrap_or_else(|_| ":0".to_string()),
            )
            .stdin(Stdio::null());
        let output = command.output()?;
        let _ = fs::remove_file(script_path);
        Ok(output)
    } else {
        Ok(command.output()?)
    }
}

fn write_askpass_script() -> Result<PathBuf> {
    let unique_nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let unique = format!("sshegg-scp-askpass-{}-{}", std::process::id(), unique_nanos);
    let path = env::temp_dir().join(unique);
    fs::write(
        &path,
        "#!/bin/sh\nprintf '%s\\n' \"$SE_ASKPASS_PASSWORD\"\n",
    )?;
    let mut permissions = fs::metadata(&path)?.permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&path, permissions)?;
    Ok(path)
}

fn auth_config_for_tui_session(store: &dyn SessionStore, session: &Session) -> Result<AuthConfig> {
    let password = match resolve_session_password(store, session) {
        Ok(password) => password,
        Err(err) if password::is_backend_unavailable_error(&err) => None,
        Err(err) => return Err(err),
    };

    Ok(AuthConfig {
        identity_file: session
            .identity_file
            .as_ref()
            .map(|path| path.display().to_string()),
        password_from_keyring: false,
        password,
        no_password: false,
        allow_manual_password_prompt: false,
        session_name: Some(session.name.clone()),
    })
}

fn local_path_suggestions(input: &str) -> Vec<String> {
    if input.trim().is_empty() {
        return Vec::new();
    }

    let expanded = expand_tilde(input);
    let path = PathBuf::from(&expanded);
    let (dir, prefix) = if expanded.ends_with('/') {
        (path, String::new())
    } else if let Some(parent) = path.parent() {
        let prefix = path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_default();
        (parent.to_path_buf(), prefix)
    } else {
        (PathBuf::from("."), expanded.clone())
    };

    let mut suggestions = Vec::new();
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.starts_with(&prefix) {
                continue;
            }

            let mut suggestion = dir.join(&name).display().to_string();
            if entry.path().is_dir() {
                suggestion.push('/');
            }
            suggestions.push(suggestion);
        }
    }
    suggestions.sort();
    suggestions
}

fn remote_path_suggestions(
    store: &dyn SessionStore,
    session: &Session,
    input: &str,
) -> Result<Vec<String>> {
    if input.trim().is_empty() {
        return Ok(Vec::new());
    }

    let auth_config = auth_config_for_tui_session(store, session)?;
    let mut connection =
        SshConnection::connect(&session.host, session.port, &session.user, &auth_config)?;
    connection.list_remote_path_suggestions(input)
}

fn update_scp_autocomplete(app: &mut AppState, store: &dyn SessionStore) -> Result<()> {
    let Some(form) = app.scp_form() else {
        return Ok(());
    };

    let field = form.field();
    let session = form.session.clone();
    let local_input = form.local_path.clone();
    let remote_input = form.remote_path.clone();

    match field {
        ScpField::Local => {
            let suggestions = local_path_suggestions(&local_input);
            if let Some(form) = app.scp_form_mut() {
                form.set_local_suggestions(suggestions);
            }
        }
        ScpField::Remote => match remote_path_suggestions(store, &session, &remote_input) {
            Ok(suggestions) => {
                if let Some(form) = app.scp_form_mut() {
                    form.set_remote_suggestions(suggestions);
                }
            }
            Err(err) => {
                if let Some(form) = app.scp_form_mut() {
                    form.set_remote_suggestions(Vec::new());
                }
                app.set_status(format!("Remote autocomplete unavailable: {err}"));
            }
        },
        _ => {
            if let Some(form) = app.scp_form_mut() {
                form.clear_active_suggestions();
            }
        }
    }

    Ok(())
}

fn update_identity_state(form: &mut AddSessionForm) {
    let input = form.identity_file.trim();
    if input.is_empty() {
        form.set_identity_state(None, Vec::new());
        return;
    }

    let expanded = expand_tilde(input);
    let path = std::path::PathBuf::from(&expanded);
    let exists = path.exists();

    let (dir, prefix) = if expanded.ends_with('/') {
        (path, String::new())
    } else if let Some(parent) = path.parent() {
        let prefix = path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_default();
        (parent.to_path_buf(), prefix)
    } else {
        (std::path::PathBuf::from("."), expanded.clone())
    };

    let mut suggestions = Vec::new();
    if dir.exists()
        && let Ok(entries) = std::fs::read_dir(&dir)
    {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with(&prefix) {
                let suggestion = dir.join(&name).display().to_string();
                suggestions.push(suggestion);
            }
        }
    }
    suggestions.sort();

    form.set_identity_state(Some(exists), suggestions);
}

fn expand_tilde(input: &str) -> String {
    if let Some(stripped) = input.strip_prefix("~/")
        && let Ok(home) = env::var("HOME")
    {
        return format!("{}/{}", home, stripped);
    }
    input.to_string()
}

fn split_tags(input: &str) -> Vec<String> {
    input
        .split(',')
        .map(|tag| tag.trim())
        .filter(|tag| !tag.is_empty())
        .map(|tag| tag.to_string())
        .collect()
}

fn refresh_monitor(app: &mut AppState, session: &Session) {
    let now = Instant::now();
    if !app.monitor_should_refresh(now, Duration::from_secs(1)) {
        return;
    }
    let entries = fetch_ssh_connections(&session.host);
    app.update_monitor(entries, now);
}

fn fetch_ssh_connections(host: &str) -> Vec<MonitorEntry> {
    let output = std::process::Command::new("ps")
        .args(["-eo", "pid=,tty=,command="])
        .output();

    let Ok(output) = output else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }

    let mut entries = Vec::new();
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        let mut parts = line.split_whitespace();
        let pid_str = parts.next().unwrap_or("");
        let tty_str = parts.next().unwrap_or("");
        let command = parts.collect::<Vec<&str>>().join(" ");
        if command.is_empty() {
            continue;
        }
        if !command.contains(host) {
            continue;
        }
        if !command.contains("ssh") && !command.contains("scp") && !command.contains("sftp") {
            continue;
        }
        if let Ok(pid) = pid_str.parse::<u32>() {
            let tty = match tty_str {
                "" | "?" => None,
                _ => Some(tty_str.to_string()),
            };
            entries.push(MonitorEntry { pid, tty });
        }
    }

    entries
}

fn format_last_connected(timestamp: Option<i64>) -> String {
    let Some(timestamp) = timestamp else {
        return "-".to_string();
    };
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0);
    let delta = now.saturating_sub(timestamp);

    if delta < 60 {
        format!("{}s ago", delta)
    } else if delta < 3600 {
        format!("{}m ago", delta / 60)
    } else if delta < 86_400 {
        format!("{}h ago", delta / 3600)
    } else {
        format!("{}d ago", delta / 86_400)
    }
}

fn now_epoch_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

fn default_user() -> Option<String> {
    env::var("USER").or_else(|_| env::var("USERNAME")).ok()
}

#[cfg(test)]
mod tests {
    use super::{build_scp_form_lines, build_text_entry_popup};
    use crate::model::Session;
    use crate::ui::state::{ScpForm, TextEntryPanel};

    fn sample_session() -> Session {
        Session {
            name: "office".to_string(),
            host: "example.com".to_string(),
            user: "alice".to_string(),
            port: 22,
            identity_file: None,
            tags: vec!["prod".to_string()],
            last_connected_at: None,
            has_stored_password: false,
            passwd_unsafe_mode: None,
            stored_password: None,
        }
    }

    #[test]
    fn text_entry_popup_separates_hints_from_input_area() {
        let entry = TextEntryPanel::new(
            "SCP Password",
            "Password for alice@example.com",
            "Transfer",
            true,
        );

        let popup = build_text_entry_popup(
            &entry,
            &[
                "Session: office (alice@example.com)".to_string(),
                "Leave blank to try key-based auth".to_string(),
            ],
            60,
            30,
        );

        assert_eq!(
            popup.body_lines,
            vec![
                "Session: office (alice@example.com)".to_string(),
                "Leave blank to try key-based auth".to_string(),
                String::new(),
                "Password for alice@example.com".to_string(),
                String::new(),
                "> ".to_string(),
                String::new(),
                "[Enter] Transfer | [Esc] Cancel".to_string(),
            ]
        );
        assert_eq!(popup.accent_lines, vec![0, 1, 3, 7]);
        assert_eq!(popup.cursor.expect("cursor").line, 5);
    }

    #[test]
    fn scp_form_lines_show_autocomplete_candidates_for_active_field() {
        let mut form = ScpForm::new(sample_session());
        form.local_path = "./Doc".to_string();
        form.set_local_suggestions(vec!["./Docs/".to_string(), "./Dockerfile".to_string()]);

        let lines = build_scp_form_lines(&form, true);

        assert!(lines.iter().any(|line| line == "  Suggestions:"));
        assert!(lines.iter().any(|line| line.contains("-> ./Docs/")));
        assert!(lines.iter().any(|line| line.contains("./Dockerfile")));
    }
}
