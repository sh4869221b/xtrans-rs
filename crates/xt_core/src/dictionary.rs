use crate::formats::strings::{
    read_dlstrings, read_ilstrings, read_strings, StringsEntry, StringsFile,
};
use crate::model::Entry;
use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Default)]
pub struct TranslationDictionary {
    pairs: HashMap<String, String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DictionaryBuildStats {
    pub files_seen: usize,
    pub file_pairs: usize,
    pub entries_added: usize,
}

#[derive(Debug)]
pub enum DictionaryError {
    Io(std::io::Error),
    InvalidFileName,
    InvalidUtf8Name,
    InvalidFormat,
}

impl fmt::Display for DictionaryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DictionaryError::Io(err) => write!(f, "io error: {err}"),
            DictionaryError::InvalidFileName => write!(f, "invalid file name"),
            DictionaryError::InvalidUtf8Name => write!(f, "invalid utf-8 in file name"),
            DictionaryError::InvalidFormat => write!(f, "invalid dictionary format"),
        }
    }
}

impl std::error::Error for DictionaryError {}

impl From<std::io::Error> for DictionaryError {
    fn from(err: std::io::Error) -> Self {
        DictionaryError::Io(err)
    }
}

impl TranslationDictionary {
    pub fn is_empty(&self) -> bool {
        self.pairs.is_empty()
    }

    pub fn len(&self) -> usize {
        self.pairs.len()
    }

    pub fn build_from_entries(entries: &[Entry]) -> Self {
        let mut pairs = HashMap::new();
        for entry in entries {
            if !entry.source_text.is_empty() && !entry.target_text.is_empty() {
                pairs.insert(entry.source_text.clone(), entry.target_text.clone());
            }
        }
        Self { pairs }
    }

    pub fn apply_quick(
        &self,
        entries: &[Entry],
        selected_keys: &[String],
        only_untranslated: bool,
    ) -> (Vec<Entry>, usize) {
        let mut selected: HashMap<&str, ()> = HashMap::new();
        for key in selected_keys {
            selected.insert(key.as_str(), ());
        }
        let use_selection = !selected.is_empty();
        let mut updated = 0usize;
        let next = entries
            .iter()
            .map(|entry| {
                if use_selection && !selected.contains_key(entry.key.as_str()) {
                    return entry.clone();
                }
                if only_untranslated && !entry.target_text.is_empty() {
                    return entry.clone();
                }
                if let Some(target) = self.pairs.get(entry.source_text.as_str()) {
                    if target != &entry.target_text {
                        let mut out = entry.clone();
                        out.target_text = target.clone();
                        updated += 1;
                        return out;
                    }
                }
                entry.clone()
            })
            .collect::<Vec<_>>();
        (next, updated)
    }

    pub fn save_to_path(&self, path: &Path) -> Result<(), DictionaryError> {
        let mut rows = Vec::new();
        for (source, target) in &self.pairs {
            rows.push(format!("{}\t{}", escape_line(source), escape_line(target)));
        }
        rows.sort();
        fs::write(path, rows.join("\n"))?;
        Ok(())
    }

    pub fn load_from_path(path: &Path) -> Result<Self, DictionaryError> {
        let mut pairs = HashMap::new();
        let data = fs::read_to_string(path)?;
        for line in data.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let Some((source, target)) = line.split_once('\t') else {
                return Err(DictionaryError::InvalidFormat);
            };
            pairs.insert(unescape_line(source)?, unescape_line(target)?);
        }
        Ok(Self { pairs })
    }

    pub fn build_from_strings_dir(
        dir: &Path,
        source_lang: &str,
        target_lang: &str,
    ) -> Result<(Self, DictionaryBuildStats), DictionaryError> {
        let mut pairs = HashMap::new();
        let mut stats = DictionaryBuildStats::default();
        let source_lower = source_lang.to_ascii_lowercase();
        let target_lower = target_lang.to_ascii_lowercase();
        let entries = fs::read_dir(dir)?;
        for entry in entries {
            let path = entry?.path();
            if !path.is_file() {
                continue;
            }
            let name_os = path.file_name().ok_or(DictionaryError::InvalidFileName)?;
            let name = name_os.to_str().ok_or(DictionaryError::InvalidUtf8Name)?;
            let Some((stem, lang, ext)) = parse_lang_file_name(name) else {
                continue;
            };
            if lang != source_lower {
                continue;
            }
            stats.files_seen += 1;
            let target_name = format!("{stem}_{target_lower}.{ext}");
            let target_path = dir.join(target_name);
            if !target_path.exists() {
                continue;
            }
            let source_file = read_strings_file(&path, ext)?;
            let target_file = read_strings_file(&target_path, ext)?;
            let mut by_id = HashMap::new();
            for StringsEntry { id, text } in &target_file.entries {
                by_id.insert(*id, text.as_str());
            }
            let before = pairs.len();
            for StringsEntry { id, text } in &source_file.entries {
                if let Some(target) = by_id.get(id) {
                    if !text.is_empty() && !target.is_empty() {
                        pairs.insert(text.clone(), (*target).to_string());
                    }
                }
            }
            if pairs.len() > before {
                stats.file_pairs += 1;
            }
        }
        stats.entries_added = pairs.len();
        Ok((Self { pairs }, stats))
    }
}

fn parse_lang_file_name(name: &str) -> Option<(String, String, &'static str)> {
    let lower = name.to_ascii_lowercase();
    let (stem_ext, ext) = if lower.ends_with(".strings") {
        (&lower[..lower.len() - 8], "strings")
    } else if lower.ends_with(".dlstrings") {
        (&lower[..lower.len() - 10], "dlstrings")
    } else if lower.ends_with(".ilstrings") {
        (&lower[..lower.len() - 10], "ilstrings")
    } else {
        return None;
    };
    let (stem, lang) = stem_ext.rsplit_once('_')?;
    Some((stem.to_string(), lang.to_string(), ext))
}

fn read_strings_file(path: &Path, ext: &str) -> Result<StringsFile, DictionaryError> {
    let bytes = fs::read(path)?;
    let file = match ext {
        "strings" => read_strings(&bytes),
        "dlstrings" => read_dlstrings(&bytes),
        "ilstrings" => read_ilstrings(&bytes),
        _ => return Err(DictionaryError::InvalidFormat),
    }
    .map_err(|_| DictionaryError::InvalidFormat)?;
    Ok(file)
}

fn escape_line(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\n', "\\n")
}

fn unescape_line(s: &str) -> Result<String, DictionaryError> {
    let mut out = String::new();
    let mut chars = s.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        let Some(next) = chars.next() else {
            return Err(DictionaryError::InvalidFormat);
        };
        match next {
            '\\' => out.push('\\'),
            't' => out.push('\t'),
            'n' => out.push('\n'),
            _ => return Err(DictionaryError::InvalidFormat),
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::strings::{write_strings, StringsEntry};

    #[test]
    fn t_dict_001_apply_quick_selection_only() {
        let dict = TranslationDictionary {
            pairs: HashMap::from([("Hello".to_string(), "こんにちは".to_string())]),
        };
        let entries = vec![
            Entry {
                key: "k1".to_string(),
                source_text: "Hello".to_string(),
                target_text: String::new(),
            },
            Entry {
                key: "k2".to_string(),
                source_text: "Hello".to_string(),
                target_text: String::new(),
            },
        ];
        let (updated, count) = dict.apply_quick(&entries, &[String::from("k2")], true);
        assert_eq!(count, 1);
        assert_eq!(updated[0].target_text, "");
        assert_eq!(updated[1].target_text, "こんにちは");
    }

    #[test]
    fn t_dict_002_build_from_strings_dir() {
        let dir = std::env::temp_dir().join(format!(
            "xt_dict_test_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create");
        let en = StringsFile {
            entries: vec![StringsEntry {
                id: 1,
                text: "Iron Sword".to_string(),
            }],
        };
        let ja = StringsFile {
            entries: vec![StringsEntry {
                id: 1,
                text: "鉄の剣".to_string(),
            }],
        };
        fs::write(
            dir.join("skyrim_english.strings"),
            write_strings(&en).expect("write en"),
        )
        .expect("save en");
        fs::write(
            dir.join("skyrim_japanese.strings"),
            write_strings(&ja).expect("write ja"),
        )
        .expect("save ja");
        let (dict, stats) = TranslationDictionary::build_from_strings_dir(
            &dir,
            "english",
            "japanese",
        )
        .expect("build");
        assert_eq!(stats.entries_added, 1);
        assert_eq!(stats.file_pairs, 1);
        assert_eq!(dict.len(), 1);
        let _ = fs::remove_dir_all(&dir);
    }
}
