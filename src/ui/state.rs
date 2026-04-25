use crate::model::{PasswdUnsafeMode, Session};
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
    error_popup: Option<String>,
    delete_dialog: Option<DeleteDialog>,
    add_form: Option<AddSessionForm>,
    form_default_mode: FormEditMode,
    yank_buffer: Option<Session>,
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
            error_popup: None,
            delete_dialog: None,
            add_form: None,
            form_default_mode: FormEditMode::Normal,
            yank_buffer: None,
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

    pub fn set_error(&mut self, reminder: impl Into<String>, details: impl Into<String>) {
        self.status = reminder.into();
        self.error_popup = Some(details.into());
    }

    pub fn clear_error_popup(&mut self) {
        self.error_popup = None;
    }

    pub fn error_popup(&self) -> Option<&str> {
        self.error_popup.as_deref()
    }

    pub fn has_error_popup(&self) -> bool {
        self.error_popup.is_some()
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
            self.delete_dialog = Some(DeleteDialog::new(session.name.clone()));
            self.mode = InputMode::ConfirmDelete;
            true
        } else {
            false
        }
    }

    pub fn cancel_delete(&mut self) {
        self.delete_dialog = None;
        self.mode = InputMode::Normal;
    }

    pub fn delete_target(&self) -> Option<&str> {
        self.delete_dialog.as_ref().map(DeleteDialog::target)
    }

    pub fn confirm_delete_matches(&self) -> bool {
        self.delete_dialog
            .as_ref()
            .is_some_and(DeleteDialog::matches_target)
    }

    pub fn delete_dialog(&self) -> Option<&DeleteDialog> {
        self.delete_dialog.as_ref()
    }

    pub fn delete_dialog_mut(&mut self) -> Option<&mut DeleteDialog> {
        self.delete_dialog.as_mut()
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
        self.add_form = Some(AddSessionForm::new(default_user, self.form_default_mode));
        self.mode = InputMode::AddSession;
    }

    pub fn yank_selected(&mut self) -> Option<String> {
        let session = self.selected_session()?.clone();
        let name = session.name.clone();
        self.yank_buffer = Some(session);
        Some(name)
    }

    pub fn start_paste_session(&mut self, name: String) -> bool {
        let Some(session) = self.yank_buffer.as_ref() else {
            return false;
        };

        let mut form = AddSessionForm::from_session(session, self.form_default_mode);
        form.name = name;
        form.set_cursor_to_end();
        self.add_form = Some(form);
        self.mode = InputMode::AddSession;
        true
    }

    pub fn next_copy_name_for_yank(&self) -> Option<String> {
        let base = self
            .yank_buffer
            .as_ref()
            .map(|session| session.name.as_str())?;
        Some(self.next_copy_name(base))
    }

    pub fn next_copy_name(&self, base: &str) -> String {
        let base = if base.trim().is_empty() {
            "session"
        } else {
            base.trim()
        };
        let initial = format!("{base}-copy");
        if !self.sessions.iter().any(|session| session.name == initial) {
            return initial;
        }

        let mut index = 2;
        loop {
            let candidate = format!("{base}-copy-{index}");
            if !self
                .sessions
                .iter()
                .any(|session| session.name == candidate)
            {
                return candidate;
            }
            index += 1;
        }
    }

    pub fn start_edit_session(&mut self, session: &Session) {
        self.add_form = Some(AddSessionForm::from_session(
            session,
            self.form_default_mode,
        ));
        self.mode = InputMode::EditSession;
    }

    pub fn set_form_default_mode(&mut self, mode: FormEditMode) {
        self.form_default_mode = mode;
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
    Password,
    PasswdMode,
    Tags,
}

impl AddField {
    pub fn next(self) -> Self {
        match self {
            AddField::Name => AddField::Host,
            AddField::Host => AddField::User,
            AddField::User => AddField::Port,
            AddField::Port => AddField::Identity,
            AddField::Identity => AddField::Password,
            AddField::Password => AddField::PasswdMode,
            AddField::PasswdMode => AddField::Tags,
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
            AddField::Password => AddField::Identity,
            AddField::PasswdMode => AddField::Password,
            AddField::Tags => AddField::PasswdMode,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormEditMode {
    Normal,
    Insert,
}

impl FormEditMode {
    pub fn label(self) -> &'static str {
        match self {
            FormEditMode::Normal => "NORMAL",
            FormEditMode::Insert => "INSERT",
        }
    }
}

pub struct AddSessionForm {
    pub name: String,
    pub host: String,
    pub user: String,
    pub port: String,
    pub identity_file: String,
    pub password: String,
    pub passwd_mode: PasswdUnsafeMode,
    pub tags: String,
    field: AddField,
    edit_mode: FormEditMode,
    field_cursor: usize,
    identity_exists: Option<bool>,
    identity_suggestions: Vec<String>,
}

impl AddSessionForm {
    fn new(default_user: Option<String>, edit_mode: FormEditMode) -> Self {
        let mut form = Self {
            name: String::new(),
            host: String::new(),
            user: default_user.unwrap_or_default(),
            port: "22".to_string(),
            identity_file: String::new(),
            password: String::new(),
            passwd_mode: PasswdUnsafeMode::Normal,
            tags: String::new(),
            field: AddField::Name,
            edit_mode,
            field_cursor: 0,
            identity_exists: None,
            identity_suggestions: Vec::new(),
        };
        form.set_cursor_to_end();
        form
    }

    fn from_session(session: &Session, edit_mode: FormEditMode) -> Self {
        let mut form = Self {
            name: session.name.clone(),
            host: session.host.clone(),
            user: session.user.clone(),
            port: session.port.to_string(),
            identity_file: session
                .identity_file
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
            password: String::new(), // Don't load existing password
            passwd_mode: session
                .passwd_unsafe_mode
                .clone()
                .unwrap_or(PasswdUnsafeMode::Normal),
            tags: session.tags.join(","),
            field: AddField::Name,
            edit_mode,
            field_cursor: 0,
            identity_exists: None,
            identity_suggestions: Vec::new(),
        };
        form.set_cursor_to_end();
        form
    }

    pub fn field(&self) -> AddField {
        self.field
    }

    pub fn next_field(&mut self) {
        self.field = self.field.next();
        self.set_cursor_to_end();
    }

    pub fn prev_field(&mut self) {
        self.field = self.field.prev();
        self.set_cursor_to_end();
    }

    pub fn edit_mode(&self) -> FormEditMode {
        self.edit_mode
    }

    pub fn set_edit_mode(&mut self, edit_mode: FormEditMode) {
        self.edit_mode = edit_mode;
        self.clamp_cursor();
    }

    pub fn cursor(&self) -> usize {
        self.field_cursor
    }

    pub fn move_cursor_left(&mut self) {
        if self.field_cursor > 0 {
            self.field_cursor -= 1;
        }
    }

    pub fn move_cursor_right(&mut self) {
        let len = self.active_value().chars().count();
        if self.field_cursor < len {
            self.field_cursor += 1;
        }
    }

    pub fn set_cursor_to_end(&mut self) {
        self.field_cursor = self.active_value().chars().count();
    }

    pub fn enter_insert_before(&mut self) {
        self.edit_mode = FormEditMode::Insert;
        self.clamp_cursor();
    }

    pub fn enter_insert_after(&mut self) {
        let len = self.active_value().chars().count();
        if self.field_cursor < len {
            self.field_cursor += 1;
        }
        self.edit_mode = FormEditMode::Insert;
    }

    pub fn insert_char(&mut self, ch: char) {
        let cursor = self.field_cursor;
        let byte_index = char_to_byte_index(self.active_value(), cursor);
        self.active_value_mut().insert(byte_index, ch);
        self.field_cursor += 1;
    }

    pub fn backspace_char(&mut self) {
        if self.field_cursor == 0 {
            return;
        }

        let cursor = self.field_cursor;
        let value = self.active_value_mut();
        let start = char_to_byte_index(value, cursor - 1);
        let end = char_to_byte_index(value, cursor);
        value.replace_range(start..end, "");
        self.field_cursor -= 1;
    }

    pub fn active_value_mut(&mut self) -> &mut String {
        match self.field {
            AddField::Name => &mut self.name,
            AddField::Host => &mut self.host,
            AddField::User => &mut self.user,
            AddField::Port => &mut self.port,
            AddField::Identity => &mut self.identity_file,
            AddField::Password => &mut self.password,
            AddField::PasswdMode => &mut self.password, // Not editable directly
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

    pub fn active_value(&self) -> &str {
        match self.field {
            AddField::Name => &self.name,
            AddField::Host => &self.host,
            AddField::User => &self.user,
            AddField::Port => &self.port,
            AddField::Identity => &self.identity_file,
            AddField::Password => &self.password,
            AddField::PasswdMode => "", // Not a text field
            AddField::Tags => &self.tags,
        }
    }

    /// Cycle to the next password mode
    pub fn cycle_passwd_mode(&mut self) {
        self.passwd_mode = match self.passwd_mode {
            PasswdUnsafeMode::Normal => PasswdUnsafeMode::Bare,
            PasswdUnsafeMode::Bare => PasswdUnsafeMode::Simple,
            PasswdUnsafeMode::Simple => PasswdUnsafeMode::Normal,
        };
    }

    /// Check if password mode field should be shown (only when password is entered)
    pub fn show_passwd_mode(&self) -> bool {
        !self.password.is_empty()
    }

    fn clamp_cursor(&mut self) {
        let len = self.active_value().chars().count();
        if self.field_cursor > len {
            self.field_cursor = len;
        }
    }
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEntryPanel {
    title: String,
    prompt: String,
    submit_label: String,
    value: String,
    masked: bool,
}

impl TextEntryPanel {
    pub fn new(
        title: impl Into<String>,
        prompt: impl Into<String>,
        submit_label: impl Into<String>,
        masked: bool,
    ) -> Self {
        Self {
            title: title.into(),
            prompt: prompt.into(),
            submit_label: submit_label.into(),
            value: String::new(),
            masked,
        }
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn prompt(&self) -> &str {
        &self.prompt
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    pub fn push(&mut self, ch: char) {
        self.value.push(ch);
    }

    pub fn pop(&mut self) {
        self.value.pop();
    }

    pub fn display_value(&self) -> String {
        if self.masked {
            "*".repeat(self.value.chars().count())
        } else {
            self.value.clone()
        }
    }

    pub fn footer_hint(&self) -> String {
        format!("[Enter] {} | [Esc] Cancel", self.submit_label)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteDialog {
    target: String,
    entry: TextEntryPanel,
}

impl DeleteDialog {
    fn new(target: String) -> Self {
        Self {
            target,
            entry: TextEntryPanel::new(
                "Confirm Delete",
                "Type the session name to confirm deletion",
                "Delete",
                false,
            ),
        }
    }

    pub fn target(&self) -> &str {
        &self.target
    }

    pub fn entry(&self) -> &TextEntryPanel {
        &self.entry
    }

    pub fn entry_mut(&mut self) -> &mut TextEntryPanel {
        &mut self.entry
    }

    fn matches_target(&self) -> bool {
        self.target == self.entry.value()
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
    password_prompt: Option<TextEntryPanel>,
    local_suggestions: Vec<String>,
    remote_suggestions: Vec<String>,
    local_selected_suggestion: Option<usize>,
    remote_selected_suggestion: Option<usize>,
    remote_suggestion_cache_directory: Option<String>,
    remote_suggestion_cache_candidates: Vec<String>,
}

impl ScpForm {
    pub(crate) fn new(session: Session) -> Self {
        Self {
            session,
            local_path: String::new(),
            remote_path: String::new(),
            direction: ScpDirection::To,
            recursive: false,
            field: ScpField::Local,
            password_prompt: None,
            local_suggestions: Vec::new(),
            remote_suggestions: Vec::new(),
            local_selected_suggestion: None,
            remote_selected_suggestion: None,
            remote_suggestion_cache_directory: None,
            remote_suggestion_cache_candidates: Vec::new(),
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

    pub fn open_password_prompt(&mut self) {
        self.password_prompt = Some(TextEntryPanel::new(
            "SCP Password",
            format!("Password for {}", self.session.target()),
            "Transfer",
            true,
        ));
    }

    pub fn close_password_prompt(&mut self) {
        self.password_prompt = None;
    }

    pub fn password_prompt(&self) -> Option<&TextEntryPanel> {
        self.password_prompt.as_ref()
    }

    pub fn password_prompt_mut(&mut self) -> Option<&mut TextEntryPanel> {
        self.password_prompt.as_mut()
    }

    pub fn set_local_suggestions(&mut self, suggestions: Vec<String>) {
        self.local_suggestions = suggestions;
        self.local_selected_suggestion = (!self.local_suggestions.is_empty()).then_some(0);
    }

    pub fn set_remote_suggestions(&mut self, suggestions: Vec<String>) {
        self.remote_suggestions = suggestions;
        self.remote_selected_suggestion = (!self.remote_suggestions.is_empty()).then_some(0);
    }

    pub fn set_remote_suggestion_cache(&mut self, directory: String, suggestions: Vec<String>) {
        self.remote_suggestion_cache_directory = Some(directory);
        self.remote_suggestion_cache_candidates = suggestions;
    }

    pub fn remote_suggestion_cache_directory(&self) -> Option<&str> {
        self.remote_suggestion_cache_directory.as_deref()
    }

    pub fn remote_suggestion_cache_suggestions(&self) -> &[String] {
        &self.remote_suggestion_cache_candidates
    }

    pub fn clear_remote_suggestion_cache(&mut self) {
        self.remote_suggestion_cache_directory = None;
        self.remote_suggestion_cache_candidates.clear();
    }

    pub fn clear_active_suggestions(&mut self) {
        match self.field {
            ScpField::Local => self.set_local_suggestions(Vec::new()),
            ScpField::Remote => self.set_remote_suggestions(Vec::new()),
            _ => {}
        }
    }

    pub fn active_suggestions(&self) -> &[String] {
        match self.field {
            ScpField::Local => &self.local_suggestions,
            ScpField::Remote => &self.remote_suggestions,
            _ => &[],
        }
    }

    pub fn selected_suggestion_index(&self) -> Option<usize> {
        match self.field {
            ScpField::Local => self.local_selected_suggestion,
            ScpField::Remote => self.remote_selected_suggestion,
            _ => None,
        }
    }

    pub fn selected_suggestion(&self) -> Option<&str> {
        let index = self.selected_suggestion_index()?;
        self.active_suggestions().get(index).map(String::as_str)
    }

    pub fn select_next_suggestion(&mut self) {
        let len = self.active_suggestions().len();
        if len == 0 {
            return;
        }

        let next = match self.selected_suggestion_index() {
            Some(index) => (index + 1) % len,
            None => 0,
        };

        match self.field {
            ScpField::Local => self.local_selected_suggestion = Some(next),
            ScpField::Remote => self.remote_selected_suggestion = Some(next),
            _ => {}
        }
    }

    pub fn select_prev_suggestion(&mut self) {
        let len = self.active_suggestions().len();
        if len == 0 {
            return;
        }

        let prev = match self.selected_suggestion_index() {
            Some(0) | None => len - 1,
            Some(index) => index - 1,
        };

        match self.field {
            ScpField::Local => self.local_selected_suggestion = Some(prev),
            ScpField::Remote => self.remote_selected_suggestion = Some(prev),
            _ => {}
        }
    }

    pub fn apply_selected_suggestion(&mut self) -> bool {
        let Some(suggestion) = self.selected_suggestion().map(str::to_string) else {
            return false;
        };

        match self.field {
            ScpField::Local => {
                if self.local_path == suggestion {
                    return false;
                }
                self.local_path = suggestion;
            }
            ScpField::Remote => {
                if self.remote_path == suggestion {
                    return false;
                }
                self.remote_path = suggestion;
            }
            _ => return false,
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_session(name: &str) -> Session {
        Session {
            name: name.to_string(),
            host: "example.com".to_string(),
            user: "alice".to_string(),
            port: 22,
            identity_file: None,
            tags: vec!["prod".to_string()],
            last_connected_at: None,
            has_stored_password: true,
            passwd_unsafe_mode: None,
            stored_password: None,
        }
    }

    #[test]
    fn next_copy_name_uses_copy_suffix() {
        let sessions = vec![
            sample_session("office"),
            sample_session("office-copy"),
            sample_session("office-copy-2"),
        ];
        let app = AppState::new(&sessions);
        assert_eq!(app.next_copy_name("office"), "office-copy-3");
    }

    #[test]
    fn next_copy_name_uses_first_copy_when_available() {
        let sessions = vec![sample_session("office")];
        let app = AppState::new(&sessions);
        assert_eq!(app.next_copy_name("office"), "office-copy");
    }

    #[test]
    fn yank_and_paste_prefills_add_form() {
        let sessions = vec![sample_session("office")];
        let mut app = AppState::new(&sessions);

        assert_eq!(app.yank_selected(), Some("office".to_string()));
        assert!(app.start_paste_session("office-copy".to_string()));
        assert_eq!(app.mode(), InputMode::AddSession);

        let form = app.add_form().expect("paste should open add form");
        assert_eq!(form.name, "office-copy");
        assert_eq!(form.host, "example.com");
        assert_eq!(form.user, "alice");
        assert_eq!(form.port, "22");
        assert_eq!(form.tags, "prod");
        assert!(form.password.is_empty());
    }

    #[test]
    fn paste_without_yank_fails() {
        let sessions = vec![sample_session("office")];
        let mut app = AppState::new(&sessions);
        assert!(!app.start_paste_session("office-copy".to_string()));
    }

    #[test]
    fn form_default_mode_applies_to_edit_session() {
        let sessions = vec![sample_session("office")];
        let mut app = AppState::new(&sessions);
        app.set_form_default_mode(FormEditMode::Insert);
        let session = app.selected_session().unwrap().clone();
        app.start_edit_session(&session);
        let form = app.add_form().unwrap();
        assert_eq!(form.edit_mode(), FormEditMode::Insert);
    }

    #[test]
    fn insert_char_respects_cursor_position() {
        let sessions = vec![sample_session("office")];
        let mut app = AppState::new(&sessions);
        let session = app.selected_session().unwrap().clone();
        app.start_edit_session(&session);

        let form = app.add_form_mut().unwrap();
        form.move_cursor_left();
        form.move_cursor_left();
        form.enter_insert_before();
        form.insert_char('X');

        assert_eq!(form.name, "offiXce");
    }

    #[test]
    fn set_error_stores_popup_and_status_reminder() {
        let sessions = vec![sample_session("office")];
        let mut app = AppState::new(&sessions);

        app.set_error("Failure reminder", "full stack-like error details");

        assert_eq!(app.status(), "Failure reminder");
        assert_eq!(app.error_popup(), Some("full stack-like error details"));
        assert!(app.has_error_popup());
    }

    #[test]
    fn clear_error_popup_hides_details_and_keeps_status() {
        let sessions = vec![sample_session("office")];
        let mut app = AppState::new(&sessions);

        app.set_error("Failure reminder", "full error");
        app.clear_error_popup();

        assert_eq!(app.status(), "Failure reminder");
        assert_eq!(app.error_popup(), None);
        assert!(!app.has_error_popup());
    }

    #[test]
    fn text_entry_panel_masks_secret_values_and_exposes_explicit_actions() {
        let mut panel = TextEntryPanel::new(
            "SCP Password",
            "Password for alice@example.com",
            "Transfer",
            true,
        );

        panel.push('s');
        panel.push('e');
        panel.push('c');
        panel.push('r');
        panel.push('e');
        panel.push('t');

        assert_eq!(panel.value(), "secret");
        assert_eq!(panel.display_value(), "******");
        assert_eq!(panel.footer_hint(), "[Enter] Transfer | [Esc] Cancel");
    }

    #[test]
    fn start_delete_uses_shared_text_entry_panel() {
        let sessions = vec![sample_session("office")];
        let mut app = AppState::new(&sessions);

        assert!(app.start_delete());

        let dialog = app
            .delete_dialog()
            .expect("delete should use a shared text entry dialog");
        assert_eq!(dialog.target(), "office");
        assert_eq!(dialog.entry().title(), "Confirm Delete");
        assert_eq!(
            dialog.entry().prompt(),
            "Type the session name to confirm deletion"
        );
        assert_eq!(
            dialog.entry().footer_hint(),
            "[Enter] Delete | [Esc] Cancel"
        );
    }

    #[test]
    fn scp_form_can_open_a_dedicated_password_prompt() {
        let session = sample_session("office");
        let mut form = ScpForm::new(session);

        form.open_password_prompt();
        let prompt = form
            .password_prompt()
            .expect("scp password prompt should be present");
        assert_eq!(prompt.title(), "SCP Password");
        assert_eq!(prompt.prompt(), "Password for alice@example.com");
        assert_eq!(prompt.footer_hint(), "[Enter] Transfer | [Esc] Cancel");
        assert_eq!(prompt.display_value(), "");
    }

    #[test]
    fn scp_form_applies_local_autocomplete_suggestion() {
        let session = sample_session("office");
        let mut form = ScpForm::new(session);
        form.local_path = "./Doc".to_string();
        form.set_local_suggestions(vec!["./Docs/".to_string(), "./Dockerfile".to_string()]);

        assert_eq!(form.active_suggestions(), &["./Docs/", "./Dockerfile"]);
        assert_eq!(form.selected_suggestion_index(), Some(0));

        assert!(form.apply_selected_suggestion());
        assert_eq!(form.local_path, "./Docs/");
    }

    #[test]
    fn scp_form_keeps_remote_suggestions_separate_from_local_state() {
        let session = sample_session("office");
        let mut form = ScpForm::new(session);
        form.local_path = "./Doc".to_string();
        form.set_local_suggestions(vec!["./Docs/".to_string()]);

        form.next_field();
        form.remote_path = "/var/lo".to_string();
        form.set_remote_suggestions(vec!["/var/log/".to_string(), "/var/local/".to_string()]);

        assert_eq!(form.active_suggestions(), &["/var/log/", "/var/local/"]);
        assert_eq!(form.selected_suggestion_index(), Some(0));

        form.select_next_suggestion();
        assert_eq!(form.selected_suggestion_index(), Some(1));
        assert!(form.apply_selected_suggestion());
        assert_eq!(form.remote_path, "/var/local/");

        form.prev_field();
        assert_eq!(form.active_suggestions(), &["./Docs/"]);
        assert_eq!(form.selected_suggestion_index(), Some(0));
    }

    #[test]
    fn scp_form_stores_remote_suggestion_cache_by_directory() {
        let session = sample_session("office");
        let mut form = ScpForm::new(session);

        form.set_remote_suggestion_cache(
            "/var/log".to_string(),
            vec![
                "/var/log/auth.log".to_string(),
                "/var/log/nginx/".to_string(),
            ],
        );

        assert_eq!(form.remote_suggestion_cache_directory(), Some("/var/log"));
        assert_eq!(
            form.remote_suggestion_cache_suggestions(),
            &["/var/log/auth.log", "/var/log/nginx/"]
        );
    }

    #[test]
    fn scp_form_clears_remote_suggestion_cache() {
        let session = sample_session("office");
        let mut form = ScpForm::new(session);
        form.set_remote_suggestion_cache(
            ".".to_string(),
            vec!["./Downloads/".to_string(), "./Documents/".to_string()],
        );

        form.clear_remote_suggestion_cache();

        assert_eq!(form.remote_suggestion_cache_directory(), None);
        assert!(form.remote_suggestion_cache_suggestions().is_empty());
    }
}
