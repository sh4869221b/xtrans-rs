use crate::model::Entry;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TwoPaneState {
    entries: Vec<Entry>,
    selected_key: Option<String>,
    query: String,
}

impl TwoPaneState {
    pub fn new(entries: Vec<Entry>) -> Self {
        Self {
            entries,
            selected_key: None,
            query: String::new(),
        }
    }

    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn set_query(&mut self, query: &str) {
        self.query.clear();
        self.query.push_str(query);
    }

    pub fn set_entries(&mut self, entries: Vec<Entry>) {
        self.entries = entries;
        if let Some(selected) = self.selected_key.clone() {
            if !self.entries.iter().any(|entry| entry.key == selected) {
                self.selected_key = None;
            }
        }
    }

    pub fn update_entry(&mut self, key: &str, source: &str, target: &str) -> bool {
        if let Some(entry) = self.entries.iter_mut().find(|entry| entry.key == key) {
            entry.source_text.clear();
            entry.source_text.push_str(source);
            entry.target_text.clear();
            entry.target_text.push_str(target);
            return true;
        }
        false
    }

    pub fn filtered_entries(&self) -> Vec<Entry> {
        if self.query.is_empty() {
            return self.entries.clone();
        }
        self.entries
            .iter()
            .filter(|entry| {
                entry.source_text.contains(&self.query)
                    || entry.target_text.contains(&self.query)
            })
            .cloned()
            .collect()
    }

    pub fn selected_key(&self) -> Option<&str> {
        self.selected_key.as_deref()
    }

    pub fn select(&mut self, key: &str) -> bool {
        if self.entries.iter().any(|entry| entry.key == key) {
            self.selected_key = Some(key.to_string());
            true
        } else {
            false
        }
    }

    pub fn selected_entry(&self) -> Option<&Entry> {
        let key = self.selected_key.as_ref()?;
        self.entries.iter().find(|entry| &entry.key == key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t_ui_001_select_updates_detail() {
        let entries = vec![
            Entry {
                key: "k1".to_string(),
                source_text: "Hello".to_string(),
                target_text: "こんにちは".to_string(),
            },
            Entry {
                key: "k2".to_string(),
                source_text: "World".to_string(),
                target_text: "世界".to_string(),
            },
        ];
        let mut state = TwoPaneState::new(entries);
        assert!(state.entries().len() == 2);
        assert!(state.selected_entry().is_none());
        assert!(state.select("k2"));
        let selected = state.selected_entry().expect("selected entry");
        assert_eq!(selected.key, "k2");
    }

    #[test]
    fn t_ui_001_search_filters_entries() {
        let entries = vec![
            Entry {
                key: "k1".to_string(),
                source_text: "Hello".to_string(),
                target_text: "こんにちは".to_string(),
            },
            Entry {
                key: "k2".to_string(),
                source_text: "World".to_string(),
                target_text: "世界".to_string(),
            },
        ];
        let mut state = TwoPaneState::new(entries);
        state.set_query("Hello");
        let filtered = state.filtered_entries();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].key, "k1");
    }

    #[test]
    fn t_ui_001_update_entry_changes_data() {
        let entries = vec![Entry {
            key: "k1".to_string(),
            source_text: "Hello".to_string(),
            target_text: "こんにちは".to_string(),
        }];
        let mut state = TwoPaneState::new(entries);
        assert!(state.update_entry("k1", "Hi", "やあ"));
        let updated = state.entries().first().expect("entry");
        assert_eq!(updated.source_text, "Hi");
        assert_eq!(updated.target_text, "やあ");
    }

    #[test]
    fn t_ui_001_set_entries_resets_selection_when_missing() {
        let entries = vec![Entry {
            key: "k1".to_string(),
            source_text: "Hello".to_string(),
            target_text: "こんにちは".to_string(),
        }];
        let mut state = TwoPaneState::new(entries);
        assert!(state.select("k1"));
        state.set_entries(vec![Entry {
            key: "k2".to_string(),
            source_text: "World".to_string(),
            target_text: "世界".to_string(),
        }]);
        assert!(state.selected_entry().is_none());
    }
}
