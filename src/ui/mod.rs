mod config;
mod filter;
mod state;

use crate::model::Session;
use crate::store::SessionStore;
use crate::ui::state::{
    AddField, AddSessionForm, AppState, InputMode, ScpDirection, ScpField, ScpForm,
};
use anyhow::Result;
use crossterm::event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, TableState};
use std::env;
use std::io;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub use config::{UiConfig, load_ui_config};

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
    let sessions = store.list()?;
    let mut app = AppState::new(&sessions);
    app.set_monitor_enabled(config.layout.show_monitor);
    let theme = Theme::from_config(config);
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal, &mut app, store, config, &theme);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
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

        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                if let Some(selection) = handle_key(app, store, key)? {
                    return Ok(selection);
                }
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

    match app.mode() {
        InputMode::Normal => handle_normal_key(app, key),
        InputMode::Search => handle_search_key(app, key),
        InputMode::ConfirmDelete => handle_confirm_delete_key(app, store, key),
        InputMode::AddSession => handle_add_session_key(app, store, key),
        InputMode::Scp => handle_scp_key(app, store, key),
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
        KeyCode::Char('a') => {
            app.start_add_session(default_user());
            app.set_status("Add session: Enter/Tab/Up/Down move fields, Esc cancel");
        }
        KeyCode::Char('o') | KeyCode::Char('O') => {
            app.start_add_session(default_user());
            app.set_status("Add session: Enter/Tab/Up/Down move fields, Esc cancel");
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
            KeyCode::Char('g') | KeyCode::Char('d') => {}
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

fn handle_confirm_delete_key(
    app: &mut AppState,
    store: &dyn SessionStore,
    key: KeyEvent,
) -> Result<Option<Option<Session>>> {
    match key.code {
        KeyCode::Esc => {
            app.cancel_delete();
            app.clear_status();
        }
        KeyCode::Enter => {
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
        KeyCode::Backspace => app.pop_delete_input(),
        KeyCode::Char(ch)
            if !key.modifiers.contains(event::KeyModifiers::CONTROL)
                && !key.modifiers.contains(event::KeyModifiers::ALT) =>
        {
            app.push_delete_input(ch)
        }
        _ => {}
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

    match key.code {
        KeyCode::Esc => {
            app.cancel_add_session();
            app.clear_status();
        }
        KeyCode::Tab => form.next_field(),
        KeyCode::BackTab => form.prev_field(),
        KeyCode::Up => form.prev_field(),
        KeyCode::Down => form.next_field(),
        KeyCode::Enter => {
            if form.field() == AddField::Tags {
                submit_add_session(app, store)?;
            } else {
                form.next_field();
            }
        }
        KeyCode::Backspace => {
            form.active_value_mut().pop();
            if form.field() == AddField::Identity {
                update_identity_state(form);
            }
        }
        KeyCode::Char(ch)
            if !key.modifiers.contains(event::KeyModifiers::CONTROL)
                && !key.modifiers.contains(event::KeyModifiers::ALT) =>
        {
            form.active_value_mut().push(ch);
            if form.field() == AddField::Identity {
                update_identity_state(form);
            }
        }
        _ => {}
    }

    Ok(None)
}

fn handle_scp_key(
    app: &mut AppState,
    store: &dyn SessionStore,
    key: KeyEvent,
) -> Result<Option<Option<Session>>> {
    let Some(form) = app.scp_form_mut() else {
        app.cancel_scp();
        return Ok(None);
    };

    let field = form.field();

    match key.code {
        KeyCode::Esc => {
            app.cancel_scp();
            app.clear_status();
        }
        KeyCode::Tab => form.next_field(),
        KeyCode::BackTab => form.prev_field(),
        KeyCode::Enter => {
            if field == ScpField::Recursive {
                submit_scp(app, store)?;
            } else {
                form.next_field();
            }
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
            }
        }
        KeyCode::Char(ch)
            if !key.modifiers.contains(event::KeyModifiers::CONTROL)
                && !key.modifiers.contains(event::KeyModifiers::ALT) =>
        {
            if let Some(value) = form.active_editable_mut() {
                value.push(ch);
            }
        }
        _ => {}
    }

    Ok(None)
}

fn draw_ui(frame: &mut ratatui::Frame, app: &mut AppState, config: &UiConfig, theme: &Theme) {
    let size = frame.area();
    let mut constraints = Vec::new();
    let mut logo_index = None;
    let mut search_index = None;
    let table_index;
    let mut monitor_index = None;
    let mut status_index = None;

    if config.layout.show_logo && config.logo.enabled {
        logo_index = Some(constraints.len());
        constraints.push(Constraint::Length(config.layout.logo_height));
    }
    if config.layout.show_search {
        search_index = Some(constraints.len());
        constraints.push(Constraint::Length(config.layout.search_height));
    }
    table_index = constraints.len();
    constraints.push(Constraint::Min(3));
    if app.monitor_enabled() {
        monitor_index = Some(constraints.len());
        constraints.push(Constraint::Length(config.layout.monitor_height));
    }
    if config.layout.show_status {
        status_index = Some(constraints.len());
        constraints.push(Constraint::Length(config.layout.status_height));
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

    let header = Row::new(vec![
        Cell::from("Name"),
        Cell::from("Target"),
        Cell::from("Port"),
        Cell::from("Identity"),
        Cell::from("Tags"),
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
        Row::new(vec![
            Cell::from(session.name.clone()),
            Cell::from(session.target()),
            Cell::from(session.port.to_string()),
            Cell::from(identity),
            Cell::from(tags),
        ])
    });

    let table = Table::new(
        rows,
        [
            Constraint::Length(20),
            Constraint::Length(30),
            Constraint::Length(6),
            Constraint::Length(18),
            Constraint::Min(10),
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
    frame.render_stateful_widget(table, chunks[table_index], &mut state);

    if let Some(index) = monitor_index {
        let monitor_text = if let Some(session) = app.selected_session().cloned() {
            refresh_monitor(app, &session);
            let pids = app.monitor_pids();
            let pid_text = if pids.is_empty() {
                "Active PIDs: -".to_string()
            } else {
                format!(
                    "Active PIDs: {}",
                    pids.iter()
                        .map(|pid| pid.to_string())
                        .collect::<Vec<String>>()
                        .join(", ")
                )
            };
            let last_text = format!(
                "Last connected: {}",
                format_last_connected(session.last_connected_at)
            );
            format!("Host: {}\n{}\n{}", session.host, pid_text, last_text)
        } else {
            "No session selected.".to_string()
        };

        let monitor = Paragraph::new(monitor_text).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border))
                .title("Monitor"),
        );
        frame.render_widget(monitor, chunks[index]);
    }

    if let Some(index) = status_index {
        let total = app.filtered_sessions().len();
        let selected = app.selected_index().map(|idx| idx + 1).unwrap_or(0);
        let info_line = if app.status().is_empty() {
            format!(
                "Mode: {:?} | {} sessions | {} selected",
                app.mode(),
                total,
                selected
            )
        } else {
            format!("{} | {} sessions", app.status(), total)
        };

        let mut lines = Vec::new();
        lines.push(Line::styled(info_line, Style::default().fg(theme.status)));
        if config.layout.show_help && config.layout.status_height > 1 {
            let help_line = format!("Help: {}", mode_help_text(app.mode()));
            lines.push(Line::styled(help_line, Style::default().fg(theme.help)));
        }

        let info = Paragraph::new(Text::from(lines)).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border))
                .title("Info"),
        );
        frame.render_widget(info, chunks[index]);
    }

    if app.mode() == InputMode::ConfirmDelete {
        let target = app.delete_target().unwrap_or("-");
        let modal_area = centered_rect(60, 30, size);
        frame.render_widget(Clear, modal_area);
        let text = format!(
            "Delete session: {}\nType session name to confirm:\n> {}",
            target,
            app.delete_input()
        );
        let modal = Paragraph::new(text).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border))
                .title("Confirm Delete"),
        );
        frame.render_widget(modal, modal_area);

        let cursor_x = modal_area.x + 3 + app.delete_input().len() as u16;
        let cursor_y = modal_area.y + 3;
        frame.set_cursor_position((cursor_x, cursor_y));
    }

    if app.mode() == InputMode::AddSession {
        if let Some(form) = app.add_form() {
            let modal_area = centered_rect(70, 50, size);
            frame.render_widget(Clear, modal_area);
            let lines = build_add_form_lines(form);
            let modal = Paragraph::new(lines.join("\n")).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.border))
                    .title("Add Session"),
            );
            frame.render_widget(modal, modal_area);

            let field_index = add_field_index(form.field()) as u16;
            let cursor_x = modal_area.x
                + 1
                + (FIELD_LABEL_WIDTH + 4) as u16
                + form.active_value().len() as u16;
            let cursor_y = modal_area.y + 1 + field_index;
            frame.set_cursor_position((cursor_x, cursor_y));
        }
    }

    if app.mode() == InputMode::Scp {
        if let Some(form) = app.scp_form() {
            let modal_area = centered_rect(70, 45, size);
            frame.render_widget(Clear, modal_area);
            let lines = build_scp_form_lines(form);
            let modal = Paragraph::new(lines.join("\n")).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.border))
                    .title("SCP"),
            );
            frame.render_widget(modal, modal_area);

            if matches!(form.field(), ScpField::Local | ScpField::Remote) {
                let field_index = scp_field_index(form.field()) as u16;
                let value_len = match form.field() {
                    ScpField::Local => form.local_path.len(),
                    ScpField::Remote => form.remote_path.len(),
                    _ => 0,
                } as u16;
                let cursor_x = modal_area.x + 1 + (FIELD_LABEL_WIDTH + 4) as u16 + value_len;
                let cursor_y = modal_area.y + 1 + field_index;
                frame.set_cursor_position((cursor_x, cursor_y));
            }
        }
    }
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
            "j/k move | gg top | G bottom | Ctrl-d/u page | / search | o/O add | s scp | m monitor | dd delete | Enter connect | q quit"
        }
        InputMode::Search => "Type to filter | Enter/Esc to exit | j/k move",
        InputMode::ConfirmDelete => "Type name | Enter confirm | Esc cancel",
        InputMode::AddSession => "Up/Down move | Tab/Enter next | Shift-Tab prev | Esc cancel",
        InputMode::Scp => "Tab/Enter next | Space toggle | Esc cancel",
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

fn build_add_form_lines(form: &AddSessionForm) -> Vec<String> {
    let mut lines = Vec::new();
    lines.push(field_line(
        "Name",
        &form.name,
        form.field() == AddField::Name,
    ));
    lines.push(field_line(
        "Host",
        &form.host,
        form.field() == AddField::Host,
    ));
    lines.push(field_line(
        "User",
        &form.user,
        form.field() == AddField::User,
    ));
    lines.push(field_line(
        "Port",
        &form.port,
        form.field() == AddField::Port,
    ));
    lines.push(field_line(
        "Identity",
        &form.identity_file,
        form.field() == AddField::Identity,
    ));
    lines.push(field_line(
        "Tags",
        &form.tags,
        form.field() == AddField::Tags,
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

    lines
}

fn build_scp_form_lines(form: &ScpForm) -> Vec<String> {
    let mut lines = Vec::new();
    lines.push(format!(
        "Session: {} ({})",
        form.session.name,
        form.session.target()
    ));
    lines.push(field_line(
        "Direction",
        form.direction.label(),
        form.field() == ScpField::Direction,
    ));
    lines.push(field_line(
        "Local",
        &form.local_path,
        form.field() == ScpField::Local,
    ));
    lines.push(field_line(
        "Remote",
        &form.remote_path,
        form.field() == ScpField::Remote,
    ));
    let recursive_value = if form.recursive { "yes" } else { "no" };
    lines.push(field_line(
        "Recursive",
        recursive_value,
        form.field() == ScpField::Recursive,
    ));
    lines.push("  Space toggles Direction/Recursive".to_string());
    lines
}

fn field_line(label: &str, value: &str, active: bool) -> String {
    let marker = if active { ">" } else { " " };
    format!(
        "{} {:<width$} {}",
        marker,
        label,
        value,
        width = FIELD_LABEL_WIDTH
    )
}

fn add_field_index(field: AddField) -> usize {
    match field {
        AddField::Name => 0,
        AddField::Host => 1,
        AddField::User => 2,
        AddField::Port => 3,
        AddField::Identity => 4,
        AddField::Tags => 5,
    }
}

fn scp_field_index(field: ScpField) -> usize {
    match field {
        ScpField::Direction => 1,
        ScpField::Local => 2,
        ScpField::Remote => 3,
        ScpField::Recursive => 4,
    }
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
    let session = Session {
        name: name.clone(),
        host,
        user,
        port,
        identity_file,
        tags,
        last_connected_at: None,
    };

    if let Err(err) = store.add(session.clone()) {
        app.set_status(format!("Failed to add session: {}", err));
        return Ok(());
    }

    app.add_session(session);
    app.cancel_add_session();
    app.set_status(format!("Added session: {}", name));
    Ok(())
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
            command.arg(&local_path).arg(remote_target);
        }
        ScpDirection::From => {
            command.arg(remote_target).arg(&local_path);
        }
    }

    let status = command.status()?;
    if !status.success() {
        app.set_status(format!("scp exited with status {}", status));
        return Ok(());
    }

    store.touch_last_connected(&session.name, now_epoch_seconds())?;
    app.cancel_scp();
    app.set_status(format!("SCP complete: {}", session.name));
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
    if dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with(&prefix) {
                    let suggestion = dir.join(&name).display().to_string();
                    suggestions.push(suggestion);
                }
            }
        }
    }
    suggestions.sort();

    form.set_identity_state(Some(exists), suggestions);
}

fn expand_tilde(input: &str) -> String {
    if let Some(stripped) = input.strip_prefix("~/") {
        if let Ok(home) = env::var("HOME") {
            return format!("{}/{}", home, stripped);
        }
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
    let pids = fetch_ssh_pids(&session.host);
    app.update_monitor(pids, now);
}

fn fetch_ssh_pids(host: &str) -> Vec<u32> {
    let output = std::process::Command::new("ps")
        .args(["-eo", "pid=,command="])
        .output();

    let Ok(output) = output else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }

    let mut pids = Vec::new();
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        let mut parts = line.trim().splitn(2, ' ');
        let pid_str = parts.next().unwrap_or("");
        let command = parts.next().unwrap_or("");
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
            pids.push(pid);
        }
    }

    pids
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
