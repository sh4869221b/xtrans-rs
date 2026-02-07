#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchEntry {
    pub key: String,
    pub source_text: String,
    pub target_text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum SearchField {
    Source,
    Target,
    Either,
}

pub fn search_entries(entries: &[SearchEntry], query: &str, field: SearchField) -> Vec<String> {
    if query.is_empty() {
        return entries.iter().map(|entry| entry.key.clone()).collect();
    }

    entries
        .iter()
        .filter(|entry| match field {
            SearchField::Source => entry.source_text.contains(query),
            SearchField::Target => entry.target_text.contains(query),
            SearchField::Either => {
                entry.source_text.contains(query) || entry.target_text.contains(query)
            }
        })
        .map(|entry| entry.key.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t_srch_001_search_source_target() {
        let entries = vec![
            SearchEntry {
                key: "key-1".to_string(),
                source_text: "Hello World".to_string(),
                target_text: "こんにちは世界".to_string(),
            },
            SearchEntry {
                key: "key-2".to_string(),
                source_text: "Goodbye".to_string(),
                target_text: "さようなら".to_string(),
            },
            SearchEntry {
                key: "key-3".to_string(),
                source_text: "Hello again".to_string(),
                target_text: "もう一度こんにちは".to_string(),
            },
        ];

        let source_hits = search_entries(&entries, "Hello", SearchField::Source);
        assert_eq!(source_hits, vec!["key-1".to_string(), "key-3".to_string()]);

        let target_hits = search_entries(&entries, "さよう", SearchField::Target);
        assert_eq!(target_hits, vec!["key-2".to_string()]);
    }
}
