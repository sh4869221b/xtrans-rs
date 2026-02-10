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
use xt_core::undo::UndoStack;
use xt_core::validation::ValidationIssue;

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
    pub history: UndoStack<Vec<Entry>>,
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
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        let history = UndoStack::new(Vec::new());
        let pane = TwoPaneState::new(history.present().clone());
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
        }
    }

    pub fn selected_key(&self) -> Option<String> {
        self.pane.selected_key().map(ToString::to_string)
    }

    pub fn selected_entry(&self) -> Option<Entry> {
        self.pane.selected_entry().cloned()
    }

    pub fn filtered_entries(&self) -> Vec<Entry> {
        self.pane.filtered_entries().to_vec()
    }

    pub fn entries(&self) -> &[Entry] {
        self.pane.entries()
    }

    pub fn set_query(&mut self, query: &str) {
        self.pane.set_query(query);
    }

    pub fn select(&mut self, key: &str) {
        self.pane.select(key);
        if let Some(entry) = self.pane.selected_entry().cloned() {
            self.edit_source = entry.source_text;
            self.edit_target = entry.target_text;
        }
    }

    pub fn set_entries_with_history(&mut self, entries: Vec<Entry>) {
        self.history.apply(entries.clone());
        self.pane.set_entries(entries);
    }

    pub fn set_entries_without_history(&mut self, entries: Vec<Entry>) {
        self.pane.set_entries(entries);
    }

    pub fn undo(&mut self) {
        if self.history.undo() {
            self.pane.set_entries(self.history.present().clone());
        }
    }

    pub fn redo(&mut self) {
        if self.history.redo() {
            self.pane.set_entries(self.history.present().clone());
        }
    }

    pub fn channel_counts(&self) -> ChannelCounts {
        count_channels(&self.pane.filtered_entries())
    }

    pub fn translation_ratio(&self) -> f32 {
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
}

pub fn row_fields(key: &str, target_text: &str) -> (String, String, String) {
    let edid = key.split(':').next_back().unwrap_or(key).to_string();
    let record_id = if key.to_ascii_lowercase().contains("plugin") {
        "REC FULL".to_string()
    } else {
        "WEAP FULL".to_string()
    };
    let ld = if target_text.is_empty() { "-" } else { "T" }.to_string();
    (edid, record_id, ld)
}

fn count_channels(entries: &[Entry]) -> ChannelCounts {
    let mut c = ChannelCounts::default();
    for entry in entries {
        c.total += 1;
        if !entry.target_text.is_empty() {
            c.translated += 1;
        }
        let key = entry.key.to_ascii_lowercase();
        if key.contains("dlstrings") {
            c.dlstrings += 1;
        } else if key.contains("ilstrings") {
            c.ilstrings += 1;
        } else {
            c.strings += 1;
        }
    }
    c
}

fn now_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
