use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use xt_core::dictionary::TranslationDictionary;
use xt_core::diff::EntryStatus;
use xt_core::formats::esp::ExtractedString;
use xt_core::formats::plugin::PluginFile;
use xt_core::formats::strings::StringsFile;
use xt_core::hybrid::HybridEntry;
use xt_core::import_export::XmlApplyStats;
use xt_core::model::Entry;
use xt_core::ui_state::TwoPaneState;
use xt_core::validation::ValidationIssue;

use crate::history::{BatchTargetChange, EntryHistory, SingleEditOp, DEFAULT_HISTORY_LIMIT};
use crate::prefs::{
    load_dictionary_prefs, save_dictionary_prefs, DictionaryPrefs, DEFAULT_DICT_ROOT,
    DEFAULT_DICT_SOURCE_LANG, DEFAULT_DICT_TARGET_LANG,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Home,
    Heuristic,
    Lang,
    Esp,
    Pex,
    Quest,
    Npc,
    Log,
}

impl Tab {
    pub fn all() -> [(Tab, &'static str); 8] {
        [
            (Tab::Home, "ホーム"),
            (Tab::Heuristic, "ヒューリスティック候補"),
            (Tab::Lang, "言語"),
            (Tab::Esp, "Espツリー"),
            (Tab::Pex, "Pex解析"),
            (Tab::Quest, "クエスト一覧"),
            (Tab::Npc, "NPC/音声リンク"),
            (Tab::Log, "ログ"),
        ]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StringsKind {
    Strings,
    DlStrings,
    IlStrings,
}

impl StringsKind {
    pub fn from_extension(ext: &str) -> Option<Self> {
        if ext.eq_ignore_ascii_case("strings") {
            Some(Self::Strings)
        } else if ext.eq_ignore_ascii_case("dlstrings") {
            Some(Self::DlStrings)
        } else if ext.eq_ignore_ascii_case("ilstrings") {
            Some(Self::IlStrings)
        } else {
            None
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DictionaryBuildSummary {
    pub built_at_unix: u64,
    pub pairs: usize,
    pub files_seen: usize,
    pub file_pairs: usize,
}

#[derive(Clone, Default, PartialEq, Eq)]
pub struct ChannelCounts {
    pub total: usize,
    pub translated: usize,
    pub strings: usize,
    pub dlstrings: usize,
    pub ilstrings: usize,
}

pub struct AppState {
    pub history: EntryHistory,
    pub pane: TwoPaneState,

    pub edit_source: String,
    pub edit_target: String,

    pub xml_text: String,
    pub xml_error: Option<String>,
    pub file_status: String,

    pub validation_issues: Vec<ValidationIssue>,
    pub diff_status: Option<EntryStatus>,
    pub encoding_status: String,

    pub hybrid_preview: Vec<HybridEntry>,
    pub hybrid_error: Option<String>,

    pub loaded_strings: Option<StringsFile>,
    pub loaded_strings_kind: Option<StringsKind>,
    pub loaded_strings_path: Option<PathBuf>,

    pub loaded_plugin: Option<PluginFile>,
    pub loaded_plugin_path: Option<PathBuf>,
    pub loaded_esp_strings: Option<Vec<ExtractedString>>,

    pub dict: Option<TranslationDictionary>,
    pub dict_source_lang: String,
    pub dict_target_lang: String,
    pub dict_root: String,
    pub dict_status: String,
    pub dict_prefs_error: String,
    pub dict_build_summary: Option<DictionaryBuildSummary>,

    pub active_tab: Tab,
    pub last_xml_stats: Option<XmlApplyStats>,

    filtered_index_cache: Vec<usize>,
    filtered_counts_cache: ChannelCounts,
    filtered_cache_dirty: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        let history = EntryHistory::with_limit(DEFAULT_HISTORY_LIMIT);
        let pane = TwoPaneState::new(Vec::new());
        let initial_prefs = load_dictionary_prefs().unwrap_or_default();

        Self {
            history,
            pane,
            edit_source: String::new(),
            edit_target: String::new(),
            xml_text: String::new(),
            xml_error: None,
            file_status: String::new(),
            validation_issues: Vec::new(),
            diff_status: None,
            encoding_status: String::new(),
            hybrid_preview: Vec::new(),
            hybrid_error: None,
            loaded_strings: None,
            loaded_strings_kind: None,
            loaded_strings_path: None,
            loaded_plugin: None,
            loaded_plugin_path: None,
            loaded_esp_strings: None,
            dict: None,
            dict_source_lang: initial_prefs.source_lang,
            dict_target_lang: initial_prefs.target_lang,
            dict_root: initial_prefs.root,
            dict_status: String::new(),
            dict_prefs_error: String::new(),
            dict_build_summary: None,
            active_tab: Tab::Home,
            last_xml_stats: None,
            filtered_index_cache: Vec::new(),
            filtered_counts_cache: ChannelCounts::default(),
            filtered_cache_dirty: true,
        }
    }

    pub fn selected_key(&self) -> Option<String> {
        self.pane.selected_key().map(ToString::to_string)
    }

    pub fn selected_entry(&self) -> Option<&Entry> {
        self.pane.selected_entry()
    }

    pub fn filtered_len(&mut self) -> usize {
        self.ensure_filtered_cache();
        self.filtered_index_cache.len()
    }

    pub fn filtered_entry(&mut self, idx: usize) -> Option<&Entry> {
        self.ensure_filtered_cache();
        let entry_idx = *self.filtered_index_cache.get(idx)?;
        self.pane.entries().get(entry_idx)
    }

    pub fn entries(&self) -> &[Entry] {
        self.pane.entries()
    }

    pub fn set_query(&mut self, query: &str) {
        self.pane.set_query(query);
        self.invalidate_filtered_cache();
    }

    pub fn select(&mut self, key: &str) {
        self.pane.select(key);
        if let Some(entry) = self.pane.selected_entry().cloned() {
            self.edit_source = entry.source_text;
            self.edit_target = entry.target_text;
        }
    }

    pub fn set_entries_with_history(&mut self, entries: Vec<Entry>) {
        self.history.clear();
        self.pane.set_entries(entries);
        self.invalidate_filtered_cache();
    }

    pub fn set_entries_without_history(&mut self, entries: Vec<Entry>) {
        self.pane.set_entries(entries);
        self.invalidate_filtered_cache();
    }

    pub fn update_entry(&mut self, key: &str, source: &str, target: &str) -> bool {
        let Some(index) = self
            .pane
            .entries()
            .iter()
            .position(|entry| entry.key == key)
        else {
            return false;
        };
        let entry = &self.pane.entries()[index];
        if entry.source_text == source && entry.target_text == target {
            return false;
        }

        let op = SingleEditOp {
            index,
            before_source: entry.source_text.clone(),
            before_target: entry.target_text.clone(),
            after_source: source.to_string(),
            after_target: target.to_string(),
        };

        if let Some(entry) = self.pane.entries_mut().get_mut(index) {
            entry.source_text.clear();
            entry.source_text.push_str(source);
            entry.target_text.clear();
            entry.target_text.push_str(target);
            self.history.record_single_edit(op);
            self.invalidate_filtered_cache();
            return true;
        }
        false
    }

    pub fn apply_target_updates_with_history(&mut self, next: Vec<Entry>) -> usize {
        let current = self.pane.entries();
        if current.len() != next.len()
            || current
                .iter()
                .zip(next.iter())
                .any(|(a, b)| a.key != b.key || a.source_text != b.source_text)
        {
            self.history.clear();
            self.set_entries_without_history(next);
            return 0;
        }

        let mut changes = Vec::new();
        for (index, (before, after)) in current.iter().zip(next.iter()).enumerate() {
            if before.target_text != after.target_text {
                changes.push(BatchTargetChange {
                    index,
                    before_target: before.target_text.clone(),
                    after_target: after.target_text.clone(),
                });
            }
        }

        if changes.is_empty() {
            return 0;
        }
        let updated = changes.len();
        self.history.record_batch_target_edit(changes);
        self.set_entries_without_history(next);
        updated
    }

    pub fn undo(&mut self) {
        if self.history.undo(self.pane.entries_mut()) {
            self.invalidate_filtered_cache();
        }
    }

    pub fn redo(&mut self) {
        if self.history.redo(self.pane.entries_mut()) {
            self.invalidate_filtered_cache();
        }
    }

    pub fn channel_counts(&mut self) -> ChannelCounts {
        self.ensure_filtered_cache();
        self.filtered_counts_cache.clone()
    }

    pub fn translation_ratio(&mut self) -> f32 {
        let counts = self.channel_counts();
        if counts.total == 0 {
            0.0
        } else {
            (counts.translated as f32 / counts.total as f32) * 100.0
        }
    }

    pub fn persist_dictionary_prefs(&mut self) {
        let prefs = DictionaryPrefs {
            source_lang: self.dict_source_lang.clone(),
            target_lang: self.dict_target_lang.clone(),
            root: self.dict_root.clone(),
        };
        match save_dictionary_prefs(&prefs) {
            Ok(()) => self.dict_prefs_error.clear(),
            Err(err) => self.dict_prefs_error = format!("辞書設定保存失敗: {err}"),
        }
    }

    pub fn reset_dictionary_lang_pair(&mut self) {
        self.dict_source_lang = DEFAULT_DICT_SOURCE_LANG.to_string();
        self.dict_target_lang = DEFAULT_DICT_TARGET_LANG.to_string();
        self.dict_root = DEFAULT_DICT_ROOT.to_string();
        self.dict_status = format!(
            "言語ペアを {} -> {} に設定",
            DEFAULT_DICT_SOURCE_LANG, DEFAULT_DICT_TARGET_LANG
        );
        self.persist_dictionary_prefs();
    }

    pub fn mark_dictionary_built(&mut self, pairs: usize, files_seen: usize, file_pairs: usize) {
        self.dict_build_summary = Some(DictionaryBuildSummary {
            built_at_unix: now_unix_seconds(),
            pairs,
            files_seen,
            file_pairs,
        });
    }

    fn invalidate_filtered_cache(&mut self) {
        self.filtered_cache_dirty = true;
    }

    fn ensure_filtered_cache(&mut self) {
        if !self.filtered_cache_dirty {
            return;
        }
        let query = self.pane.query().to_string();
        let entries = self.pane.entries();

        let mut indices = Vec::with_capacity(entries.len());
        let mut counts = ChannelCounts::default();
        for (idx, entry) in entries.iter().enumerate() {
            if query.is_empty()
                || entry.source_text.contains(&query)
                || entry.target_text.contains(&query)
            {
                indices.push(idx);
                counts.total += 1;
                if !entry.target_text.is_empty() {
                    counts.translated += 1;
                }
                let key = entry.key.to_ascii_lowercase();
                if key.contains("dlstrings") {
                    counts.dlstrings += 1;
                } else if key.contains("ilstrings") {
                    counts.ilstrings += 1;
                } else {
                    counts.strings += 1;
                }
            }
        }

        self.filtered_index_cache = indices;
        self.filtered_counts_cache = counts;
        self.filtered_cache_dirty = false;
    }
}

pub fn row_fields<'a>(key: &'a str, target_text: &str) -> (&'a str, &'static str, &'static str) {
    let edid = key.split(':').next_back().unwrap_or(key);
    let record_id = if key
        .split(':')
        .next()
        .map(|prefix| prefix.eq_ignore_ascii_case("plugin"))
        .unwrap_or(false)
    {
        "REC FULL"
    } else {
        "WEAP FULL"
    };
    let ld = if target_text.is_empty() { "-" } else { "T" };
    (edid, record_id, ld)
}

fn now_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use xt_core::model::Entry;

    #[test]
    fn t_perf_001_list_hot_path_baseline() {
        let mut state = AppState::new();
        let entries = (0..100_000)
            .map(|i| Entry {
                key: format!("plugin:{i:08x}"),
                source_text: format!("Source text {i} lorem ipsum dolor sit amet"),
                target_text: if i % 5 == 0 {
                    format!("訳文 {i}")
                } else {
                    String::new()
                },
            })
            .collect::<Vec<_>>();
        state.set_entries_with_history(entries);
        state.set_query("");

        let start = std::time::Instant::now();
        let mut checksum = 0usize;
        for frame in 0..120usize {
            let len = state.filtered_len();
            for row in 0..80usize {
                let idx = (frame + row) % len;
                let entry = state.filtered_entry(idx).expect("entry");
                checksum ^= entry.key.len();
            }
        }
        let elapsed = start.elapsed();
        println!(
            "t_perf_001_list_hot_path_baseline: {:?}, checksum={}",
            elapsed, checksum
        );
    }

    #[test]
    fn t_perf_002_row_render_compare_concat_vs_cells() {
        let entries = (0..80_000usize)
            .map(|i| Entry {
                key: format!("plugin:{i:08x}"),
                source_text: format!("Source text {i} lorem ipsum dolor sit amet"),
                target_text: if i % 3 == 0 {
                    format!("訳文 {i}")
                } else {
                    String::new()
                },
            })
            .collect::<Vec<_>>();

        let mut concat_checksum = 0usize;
        let concat_start = std::time::Instant::now();
        for entry in &entries {
            let (edid, record_id, ld) = row_fields(&entry.key, &entry.target_text);
            let row = format!(
                "{} | {} | {} | {} | {}",
                edid, record_id, entry.source_text, entry.target_text, ld
            );
            concat_checksum ^= std::hint::black_box(row.len());
        }
        let concat_elapsed = concat_start.elapsed();

        let mut cells_checksum = 0usize;
        let cells_start = std::time::Instant::now();
        for entry in &entries {
            let (edid, record_id, ld) = row_fields(&entry.key, &entry.target_text);
            cells_checksum ^= std::hint::black_box(edid.len());
            cells_checksum ^= std::hint::black_box(record_id.len());
            cells_checksum ^= std::hint::black_box(entry.source_text.len());
            cells_checksum ^= std::hint::black_box(entry.target_text.len());
            cells_checksum ^= std::hint::black_box(ld.len());
        }
        let cells_elapsed = cells_start.elapsed();

        println!(
            "t_perf_002_row_render_compare_concat_vs_cells: concat={:?} cells={:?} checksum={} {}",
            concat_elapsed, cells_elapsed, concat_checksum, cells_checksum
        );
    }
}
