use crate::model::Session;
use crate::ui::filter::filter_sessions;

pub struct AppState {
    sessions: Vec<Session>,
    pub filter: String,
    filtered_indices: Vec<usize>,
    selected: usize,
}

impl AppState {
    pub fn new(sessions: &[Session]) -> Self {
        let mut state = Self {
            sessions: sessions.to_vec(),
            filter: String::new(),
            filtered_indices: Vec::new(),
            selected: 0,
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
