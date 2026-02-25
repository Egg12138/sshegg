mod filter;
mod state;

use crate::model::Session;
use crate::ui::state::AppState;
use anyhow::Result;
use crossterm::event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState};
use std::io;
use std::time::Duration;

pub fn run_tui(sessions: &[Session]) -> Result<Option<Session>> {
    let mut app = AppState::new(sessions);
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal, &mut app);

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
) -> Result<Option<Session>> {
    loop {
        terminal.draw(|frame| draw_ui(frame, app))?;

        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                if let Some(selection) = handle_key(app, key)? {
                    return Ok(selection);
                }
            }
        }
    }
}

fn handle_key(app: &mut AppState, key: KeyEvent) -> Result<Option<Option<Session>>> {
    match key.code {
        KeyCode::Char('c') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
            return Ok(Some(None));
        }
        KeyCode::Esc => return Ok(Some(None)),
        KeyCode::Enter => {
            return Ok(Some(app.selected_session().cloned()));
        }
        KeyCode::Up => app.move_prev(),
        KeyCode::Down => app.move_next(),
        KeyCode::Home => app.select_first(),
        KeyCode::End => app.select_last(),
        KeyCode::Backspace => app.backspace(),
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

fn draw_ui(frame: &mut ratatui::Frame, app: &AppState) {
    let size = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(3),
            Constraint::Length(2),
        ])
        .split(size);

    let filter = Paragraph::new(app.filter.clone())
        .block(Block::default().borders(Borders::ALL).title("Search"));
    frame.render_widget(filter, chunks[0]);

    let header = Row::new(vec![
        Cell::from("Name"),
        Cell::from("Target"),
        Cell::from("Port"),
        Cell::from("Identity"),
    ])
    .style(Style::default().add_modifier(Modifier::BOLD));

    let rows = app.filtered_sessions().into_iter().map(|session| {
        let identity = session
            .identity_file
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "-".to_string());
        Row::new(vec![
            Cell::from(session.name.clone()),
            Cell::from(session.target()),
            Cell::from(session.port.to_string()),
            Cell::from(identity),
        ])
    });

    let table = Table::new(
        rows,
        [
            Constraint::Length(20),
            Constraint::Length(30),
            Constraint::Length(6),
            Constraint::Min(10),
        ],
    )
    .header(header)
    .block(Block::default().borders(Borders::ALL).title("Sessions"))
    .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    let mut state = TableState::default();
    if let Some(selected) = app.selected_index() {
        state.select(Some(selected));
    }
    frame.render_stateful_widget(table, chunks[1], &mut state);

    let help = Paragraph::new("Arrows move | Type to filter | Enter connect | Esc quit")
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(help, chunks[2]);
}
