use crate::actions::{dispatch, AppAction};
use crate::state::{AppState, Tab};

pub struct AppDriver {
    state: AppState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppSnapshot {
    pub total_entries: usize,
    pub translated_entries: usize,
    pub selected_key: Option<String>,
    pub file_status: String,
    pub dict_status: String,
    pub validation_issue_count: usize,
    pub diff_status: Option<String>,
    pub encoding_status: String,
    pub active_tab: Tab,
}

impl Default for AppDriver {
    fn default() -> Self {
        Self::new()
    }
}

impl AppDriver {
    pub fn new() -> Self {
        Self {
            state: AppState::new(),
        }
    }

    pub fn state(&self) -> &AppState {
        &self.state
    }

    pub fn state_mut(&mut self) -> &mut AppState {
        &mut self.state
    }

    pub fn dispatch(&mut self, action: AppAction) -> Result<(), String> {
        dispatch(&mut self.state, action)
    }

    pub fn snapshot(&self) -> AppSnapshot {
        let entries = self.state.entries();
        let translated_entries = entries
            .iter()
            .filter(|entry| !entry.target_text.is_empty())
            .count();

        AppSnapshot {
            total_entries: entries.len(),
            translated_entries,
            selected_key: self.state.selected_key(),
            file_status: self.state.file_status.clone(),
            dict_status: self.state.dict_status.clone(),
            validation_issue_count: self.state.validation_issues.len(),
            diff_status: self
                .state
                .diff_status
                .as_ref()
                .map(|status| format!("{status:?}")),
            encoding_status: self.state.encoding_status.clone(),
            active_tab: self.state.active_tab,
        }
    }
}
