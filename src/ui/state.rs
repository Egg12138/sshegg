use crate::model::Session;
use crate::ui::filter::filter_sessions;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Search,
    ConfirmDelete,
    AddSession,
    EditSession,
    Help,
    Scp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonitorEntry {
    pub pid: u32,
    pub tty: Option<String>,
}

pub struct AppState {
    sessions: Vec<Session>,
    pub filter: String,
    filtered_indices: Vec<usize>,
    selected: usize,
    mode: InputMode,
    pending: Option<char>,
    status: String,
    delete_target: Option<String>,
    delete_input: String,
    add_form: Option<AddSessionForm>,
    scp_form: Option<ScpForm>,
    monitor_enabled: bool,
    monitor_last_update: Option<Instant>,
    monitor_entries: Vec<MonitorEntry>,
}

impl AppState {
    pub fn new(sessions: &[Session]) -> Self {
        let mut state = Self {
            sessions: sessions.to_vec(),
            filter: String::new(),
            filtered_indices: Vec::new(),
            selected: 0,
            mode: InputMode::Normal,
            pending: None,
            status: String::new(),
            delete_target: None,
            delete_input: String::new(),
            add_form: None,
            scp_form: None,
            monitor_enabled: false,
            monitor_last_update: None,
            monitor_entries: Vec::new(),
        };
        state.refresh_filter();
        state
    }

    pub fn refresh_filter(&mut self) {
        self.filtered_indices = filter_sessions(&self.sessions, &self.filter);
        if self.selected >= self.filtered_indices.len() {
            self.selected = 0;
        }
    }

    pub fn mode(&self) -> InputMode {
        self.mode
    }

    pub fn set_mode(&mut self, mode: InputMode) {
        self.mode = mode;
    }

    pub fn set_pending(&mut self, pending: Option<char>) {
        self.pending = pending;
    }

    pub fn pending(&self) -> Option<char> {
        self.pending
    }

    pub fn set_status(&mut self, status: impl Into<String>) {
        self.status = status.into();
    }

    pub fn clear_status(&mut self) {
        self.status.clear();
    }

    pub fn move_next(&mut self) {
        if self.filtered_indices.is_empty() {
            return;
        }
        self.selected = (self.selected + 1) % self.filtered_indices.len();
    }

    pub fn move_prev(&mut self) {
        if self.filtered_indices.is_empty() {
            return;
        }
        if self.selected == 0 {
            self.selected = self.filtered_indices.len() - 1;
        } else {
            self.selected -= 1;
        }
    }

    pub fn page_down(&mut self, step: usize) {
        for _ in 0..step {
            self.move_next();
        }
    }

    pub fn page_up(&mut self, step: usize) {
        for _ in 0..step {
            self.move_prev();
        }
    }

    pub fn select_first(&mut self) {
        if !self.filtered_indices.is_empty() {
            self.selected = 0;
        }
    }

    pub fn select_last(&mut self) {
        if !self.filtered_indices.is_empty() {
            self.selected = self.filtered_indices.len() - 1;
        }
    }

    pub fn on_char(&mut self, ch: char) {
        self.filter.push(ch);
        self.refresh_filter();
    }

    pub fn backspace(&mut self) {
        self.filter.pop();
        self.refresh_filter();
    }

    pub fn start_delete(&mut self) -> bool {
        if let Some(session) = self.selected_session() {
            self.delete_target = Some(session.name.clone());
            self.delete_input.clear();
            self.mode = InputMode::ConfirmDelete;
            true
        } else {
            false
        }
    }

    pub fn cancel_delete(&mut self) {
        self.delete_target = None;
        self.delete_input.clear();
        self.mode = InputMode::Normal;
    }

    pub fn delete_target(&self) -> Option<&str> {
        self.delete_target.as_deref()
    }

    pub fn delete_input(&self) -> &str {
        &self.delete_input
    }

    pub fn push_delete_input(&mut self, ch: char) {
        self.delete_input.push(ch);
    }

    pub fn pop_delete_input(&mut self) {
        self.delete_input.pop();
    }

    pub fn confirm_delete_matches(&self) -> bool {
        match &self.delete_target {
            Some(target) => target == &self.delete_input,
            None => false,
        }
    }

    pub fn remove_by_name(&mut self, name: &str) -> bool {
        let before = self.sessions.len();
        self.sessions.retain(|session| session.name != name);
        let removed = self.sessions.len() != before;
        if removed {
            self.refresh_filter();
        }
        removed
    }

    pub fn add_session(&mut self, session: Session) {
        self.sessions.push(session);
        self.refresh_filter();
    }

    pub fn update_session(&mut self, original_name: &str, session: Session) {
        if let Some(existing) = self.sessions.iter_mut().find(|s| s.name == original_name) {
            *existing = session;
            self.refresh_filter();
        }
    }

    pub fn start_add_session(&mut self, default_user: Option<String>) {
        self.add_form = Some(AddSessionForm::new(default_user));
        self.mode = InputMode::AddSession;
    }

    pub fn start_edit_session(&mut self, session: &Session) {
        self.add_form = Some(AddSessionForm::from_session(session));
        self.mode = InputMode::EditSession;
    }

    pub fn cancel_add_session(&mut self) {
        self.add_form = None;
        self.mode = InputMode::Normal;
    }

    pub fn add_form(&self) -> Option<&AddSessionForm> {
        self.add_form.as_ref()
    }

    pub fn add_form_mut(&mut self) -> Option<&mut AddSessionForm> {
        self.add_form.as_mut()
    }

    pub fn start_scp(&mut self, session: Session) {
        self.scp_form = Some(ScpForm::new(session));
        self.mode = InputMode::Scp;
    }

    pub fn cancel_scp(&mut self) {
        self.scp_form = None;
        self.mode = InputMode::Normal;
    }

    pub fn scp_form(&self) -> Option<&ScpForm> {
        self.scp_form.as_ref()
    }

    pub fn scp_form_mut(&mut self) -> Option<&mut ScpForm> {
        self.scp_form.as_mut()
    }

    pub fn set_monitor_enabled(&mut self, enabled: bool) {
        self.monitor_enabled = enabled;
    }

    pub fn toggle_monitor(&mut self) {
        self.monitor_enabled = !self.monitor_enabled;
    }

    pub fn monitor_enabled(&self) -> bool {
        self.monitor_enabled
    }

    pub fn monitor_should_refresh(&self, now: Instant, interval: Duration) -> bool {
        match self.monitor_last_update {
            Some(last) => now.duration_since(last) >= interval,
            None => true,
        }
    }

    pub fn update_monitor(&mut self, entries: Vec<MonitorEntry>, now: Instant) {
        self.monitor_entries = entries;
        self.monitor_last_update = Some(now);
    }

    pub fn monitor_entries(&self) -> &[MonitorEntry] {
        &self.monitor_entries
    }

    pub fn status(&self) -> &str {
        &self.status
    }

    pub fn selected_session(&self) -> Option<&Session> {
        self.filtered_indices
            .get(self.selected)
            .map(|index| &self.sessions[*index])
    }

    pub fn filtered_sessions(&self) -> Vec<&Session> {
        self.filtered_indices
            .iter()
            .map(|index| &self.sessions[*index])
            .collect()
    }

    pub fn selected_index(&self) -> Option<usize> {
        if self.filtered_indices.is_empty() {
            None
        } else {
            Some(self.selected)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddField {
    Name,
    Host,
    User,
    Port,
    Identity,
    Tags,
}

impl AddField {
    pub fn next(self) -> Self {
        match self {
            AddField::Name => AddField::Host,
            AddField::Host => AddField::User,
            AddField::User => AddField::Port,
            AddField::Port => AddField::Identity,
            AddField::Identity => AddField::Tags,
            AddField::Tags => AddField::Name,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            AddField::Name => AddField::Tags,
            AddField::Host => AddField::Name,
            AddField::User => AddField::Host,
            AddField::Port => AddField::User,
            AddField::Identity => AddField::Port,
            AddField::Tags => AddField::Identity,
        }
    }
}

pub struct AddSessionForm {
    pub name: String,
    pub host: String,
    pub user: String,
    pub port: String,
    pub identity_file: String,
    pub tags: String,
    field: AddField,
    identity_exists: Option<bool>,
    identity_suggestions: Vec<String>,
}

impl AddSessionForm {
    fn new(default_user: Option<String>) -> Self {
        Self {
            name: String::new(),
            host: String::new(),
            user: default_user.unwrap_or_default(),
            port: "22".to_string(),
            identity_file: String::new(),
            tags: String::new(),
            field: AddField::Name,
            identity_exists: None,
            identity_suggestions: Vec::new(),
        }
    }

    fn from_session(session: &Session) -> Self {
        Self {
            name: session.name.clone(),
            host: session.host.clone(),
            user: session.user.clone(),
            port: session.port.to_string(),
            identity_file: session
                .identity_file
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
            tags: session.tags.join(","),
            field: AddField::Name,
            identity_exists: None,
            identity_suggestions: Vec::new(),
        }
    }

    pub fn field(&self) -> AddField {
        self.field
    }

    pub fn next_field(&mut self) {
        self.field = self.field.next();
    }

    pub fn prev_field(&mut self) {
        self.field = self.field.prev();
    }

    pub fn active_value(&self) -> &str {
        match self.field {
            AddField::Name => &self.name,
            AddField::Host => &self.host,
            AddField::User => &self.user,
            AddField::Port => &self.port,
            AddField::Identity => &self.identity_file,
            AddField::Tags => &self.tags,
        }
    }

    pub fn active_value_mut(&mut self) -> &mut String {
        match self.field {
            AddField::Name => &mut self.name,
            AddField::Host => &mut self.host,
            AddField::User => &mut self.user,
            AddField::Port => &mut self.port,
            AddField::Identity => &mut self.identity_file,
            AddField::Tags => &mut self.tags,
        }
    }

    pub fn identity_exists(&self) -> Option<bool> {
        self.identity_exists
    }

    pub fn identity_suggestions(&self) -> &[String] {
        &self.identity_suggestions
    }

    pub fn set_identity_state(&mut self, exists: Option<bool>, suggestions: Vec<String>) {
        self.identity_exists = exists;
        self.identity_suggestions = suggestions;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScpField {
    Direction,
    Local,
    Remote,
    Recursive,
}

impl ScpField {
    pub fn next(self) -> Self {
        match self {
            ScpField::Direction => ScpField::Local,
            ScpField::Local => ScpField::Remote,
            ScpField::Remote => ScpField::Recursive,
            ScpField::Recursive => ScpField::Direction,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            ScpField::Direction => ScpField::Recursive,
            ScpField::Local => ScpField::Direction,
            ScpField::Remote => ScpField::Local,
            ScpField::Recursive => ScpField::Remote,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScpDirection {
    To,
    From,
}

impl ScpDirection {
    pub fn toggle(self) -> Self {
        match self {
            ScpDirection::To => ScpDirection::From,
            ScpDirection::From => ScpDirection::To,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ScpDirection::To => "to (local -> remote)",
            ScpDirection::From => "from (remote -> local)",
        }
    }
}

pub struct ScpForm {
    pub session: Session,
    pub local_path: String,
    pub remote_path: String,
    pub direction: ScpDirection,
    pub recursive: bool,
    field: ScpField,
}

impl ScpForm {
    fn new(session: Session) -> Self {
        Self {
            session,
            local_path: String::new(),
            remote_path: String::new(),
            direction: ScpDirection::To,
            recursive: false,
            field: ScpField::Local,
        }
    }

    pub fn field(&self) -> ScpField {
        self.field
    }

    pub fn next_field(&mut self) {
        self.field = self.field.next();
    }

    pub fn prev_field(&mut self) {
        self.field = self.field.prev();
    }

    pub fn toggle_direction(&mut self) {
        self.direction = self.direction.toggle();
    }

    pub fn toggle_recursive(&mut self) {
        self.recursive = !self.recursive;
    }

    pub fn active_editable_mut(&mut self) -> Option<&mut String> {
        match self.field {
            ScpField::Local => Some(&mut self.local_path),
            ScpField::Remote => Some(&mut self.remote_path),
            _ => None,
        }
    }
}
