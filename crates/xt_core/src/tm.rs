use crate::formats::strings::StringsFile;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranslationMemory {
    exact: HashMap<String, Vec<String>>,
}

impl TranslationMemory {
    pub fn from_strings(file: &StringsFile) -> Self {
        let mut exact: HashMap<String, Vec<String>> = HashMap::new();
        for entry in &file.entries {
            exact
                .entry(entry.text.clone())
                .or_default()
                .push(entry.text.clone());
        }
        Self { exact }
    }

    pub fn exact_match(&self, source_text: &str) -> Option<&[String]> {
        self.exact.get(source_text).map(|list| list.as_slice())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::strings::StringsEntry;

    #[test]
    fn t_tm_001_strings_exact_match() {
        let file = StringsFile {
            entries: vec![
                StringsEntry {
                    id: 1,
                    text: "Hello".to_string(),
                },
                StringsEntry {
                    id: 2,
                    text: "Hello".to_string(),
                },
                StringsEntry {
                    id: 3,
                    text: "World".to_string(),
                },
            ],
        };
        let tm = TranslationMemory::from_strings(&file);
        let matches = tm.exact_match("Hello").expect("match");
        assert_eq!(matches.len(), 2);
        assert!(tm.exact_match("Missing").is_none());
    }
}
