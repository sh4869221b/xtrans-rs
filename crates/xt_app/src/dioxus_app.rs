use dioxus::html::HasFileData;
use dioxus::prelude::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use xt_core::dictionary::TranslationDictionary;
use xt_core::diff::{update_source, DiffEntry, EntryStatus};
use xt_core::encoding::{decode, encode, Encoding, EncodingError};
use xt_core::formats::esp::{
    apply_translations, extract_strings as extract_esp_strings, ExtractedString,
};
use xt_core::formats::plugin::{read_plugin, write_plugin, PluginFile};
use xt_core::formats::plugin_binary::extract_null_terminated_utf8;
use xt_core::formats::strings::{
    read_dlstrings, read_ilstrings, read_strings, write_dlstrings, write_ilstrings, write_strings,
    StringsEntry, StringsFile,
};
use xt_core::hybrid::{build_hybrid_entries, HybridEntry};
use xt_core::import_export::{apply_xml_default, export_entries, import_entries, XmlApplyStats};
use xt_core::model::Entry;
use xt_core::ui_state::TwoPaneState;
use xt_core::undo::UndoStack;
use xt_core::validation::{
    validate_alias_tags, validate_braced_placeholders, validate_printf_placeholders,
    ValidationIssue,
};
use xt_core::virtual_list::{virtual_window, VirtualWindow};

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

const MENU_XML_EXPORT: &str = "file.xml_export";
const MENU_XML_APPLY: &str = "file.xml_apply";
const MENU_OPEN_PLUGIN: &str = "file.open_plugin";
const MENU_OPEN_STRINGS: &str = "file.open_strings";
const MENU_SAVE_OVERWRITE: &str = "file.save_overwrite";
const MENU_SAVE_AS: &str = "file.save_as";
const MENU_DICT_BUILD: &str = "translate.dict_build";
const MENU_QUICK_AUTO: &str = "translate.quick_auto";
const MENU_LANG_PANEL: &str = "options.lang_panel";
const MENU_LANG_RESET: &str = "options.lang_reset";
const MENU_UNDO: &str = "tools.undo";
const MENU_REDO: &str = "tools.redo";
const MENU_LOG_TAB: &str = "tools.log_tab";

const DEFAULT_DICT_SOURCE_LANG: &str = "english";
const DEFAULT_DICT_TARGET_LANG: &str = "japanese";
const DEFAULT_DICT_ROOT: &str = "./Data/Strings/Translations";
const DICT_PREFS_FILE: &str = "dict_prefs.v1";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
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
    fn all() -> [(Tab, &'static str); 8] {
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

#[derive(Clone, Copy)]
enum StringsKind {
    Strings,
    DlStrings,
    IlStrings,
}

impl StringsKind {
    fn from_extension(ext: &str) -> Option<Self> {
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
struct DictionaryPrefs {
    source_lang: String,
    target_lang: String,
    root: String,
}

impl Default for DictionaryPrefs {
    fn default() -> Self {
        Self {
            source_lang: DEFAULT_DICT_SOURCE_LANG.to_string(),
            target_lang: DEFAULT_DICT_TARGET_LANG.to_string(),
            root: DEFAULT_DICT_ROOT.to_string(),
        }
    }
}

#[derive(Clone)]
struct DictionaryBuildSummary {
    built_at_unix: u64,
    pairs: usize,
    files_seen: usize,
    file_pairs: usize,
}

#[derive(Clone)]
struct RowView {
    key: String,
    source_text: String,
    target_text: String,
    edid: String,
    record_id: String,
    ld: String,
    selected: bool,
}

#[derive(Clone, PartialEq)]
struct FilterSnapshot {
    entries: Vec<Entry>,
    counts: ChannelCounts,
}

#[derive(Clone, Copy, PartialEq)]
enum SpacerPosition {
    Top,
    Bottom,
}

pub fn run() {
    #[cfg(all(
        feature = "dioxus-desktop",
        any(target_os = "windows", target_os = "linux", target_os = "macos")
    ))]
    {
        use dioxus::desktop::Config;
        dioxus::LaunchBuilder::new()
            .with_cfg(Config::new().with_menu(build_native_menu()))
            .launch(App);
    }
    #[cfg(not(all(
        feature = "dioxus-desktop",
        any(target_os = "windows", target_os = "linux", target_os = "macos")
    )))]
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let mut history = use_signal(|| UndoStack::new(Vec::new()));
    let mut state = use_signal(|| TwoPaneState::new(history.read().present().clone()));

    let mut scroll_offset = use_signal(|| 0.0f32);
    let mut viewport_height = use_signal(|| 520.0f32);
    let item_height = 64.0f32;
    let overscan = 8usize;

    let mut edit_source = use_signal(String::new);
    let mut edit_target = use_signal(String::new);

    let mut xml_text = use_signal(String::new);
    let mut xml_error = use_signal(|| Option::<String>::None);
    let mut file_status = use_signal(String::new);

    let mut validation_issues = use_signal(Vec::<ValidationIssue>::new);
    let mut diff_status = use_signal(|| Option::<EntryStatus>::None);
    let mut encoding_status = use_signal(String::new);

    let mut hybrid_preview = use_signal(Vec::<HybridEntry>::new);
    let mut hybrid_error = use_signal(|| Option::<String>::None);

    let mut loaded_strings = use_signal(|| Option::<StringsFile>::None);
    let mut loaded_strings_kind = use_signal(|| Option::<StringsKind>::None);
    let mut loaded_strings_path = use_signal(|| Option::<PathBuf>::None);

    let mut loaded_plugin = use_signal(|| Option::<PluginFile>::None);
    let mut loaded_plugin_path = use_signal(|| Option::<PathBuf>::None);
    let mut loaded_esp_strings = use_signal(|| Option::<Vec<ExtractedString>>::None);

    let initial_prefs = load_dictionary_prefs().unwrap_or_default();
    let mut dict = use_signal(|| Option::<TranslationDictionary>::None);
    let mut dict_source_lang = use_signal({
        let value = initial_prefs.source_lang.clone();
        move || value.clone()
    });
    let mut dict_target_lang = use_signal({
        let value = initial_prefs.target_lang.clone();
        move || value.clone()
    });
    let mut dict_root = use_signal({
        let value = initial_prefs.root.clone();
        move || value.clone()
    });
    let mut dict_status = use_signal(String::new);
    let mut dict_prefs_error = use_signal(String::new);
    let mut dict_build_summary = use_signal(|| Option::<DictionaryBuildSummary>::None);

    let mut active_tab = use_signal(|| Tab::Home);

    use_effect(move || {
        let prefs = DictionaryPrefs {
            source_lang: dict_source_lang(),
            target_lang: dict_target_lang(),
            root: dict_root(),
        };
        match save_dictionary_prefs(&prefs) {
            Ok(()) => dict_prefs_error.set(String::new()),
            Err(err) => dict_prefs_error.set(format!("辞書設定保存失敗: {err}")),
        }
    });

    #[cfg(all(
        feature = "dioxus-desktop",
        any(target_os = "windows", target_os = "linux", target_os = "macos")
    ))]
    {
        use dioxus::desktop::use_muda_event_handler;
        use_muda_event_handler(move |event| match event.id.as_ref() {
            MENU_OPEN_PLUGIN => {
                document::eval(
                    "const el = document.getElementById('plugin-picker-native'); if (el) { el.click(); }",
                );
            }
            MENU_OPEN_STRINGS => {
                document::eval(
                    "const el = document.getElementById('strings-picker-native'); if (el) { el.click(); }",
                );
            }
            MENU_XML_EXPORT => {
                xml_text.set(export_entries(state.read().entries()));
                xml_error.set(None);
                file_status.set("XMLを書き出しました（エディタ）".to_string());
            }
            MENU_XML_APPLY => {
                document::eval(
                    "const el = document.getElementById('xml-picker-native'); if (el) { el.click(); }",
                );
            }
            MENU_SAVE_OVERWRITE => match save_overwrite(
                state.read().entries(),
                loaded_strings(),
                loaded_strings_kind(),
                loaded_strings_path(),
                loaded_plugin(),
                loaded_plugin_path(),
                loaded_esp_strings(),
            ) {
                Ok(path) => file_status.set(format!("保存: {}", path.display())),
                Err(err) => file_status.set(format!("保存失敗: {err}")),
            },
            MENU_SAVE_AS => match save_as(
                state.read().entries(),
                loaded_strings(),
                loaded_strings_kind(),
                loaded_strings_path(),
                loaded_plugin(),
                loaded_plugin_path(),
                loaded_esp_strings(),
            ) {
                Ok(path) => file_status.set(format!("別名保存: {}", path.display())),
                Err(err) => file_status.set(format!("別名保存失敗: {err}")),
            },
            MENU_DICT_BUILD => {
                let root = PathBuf::from(dict_root());
                match TranslationDictionary::build_from_strings_dir(
                    &root,
                    &dict_source_lang(),
                    &dict_target_lang(),
                ) {
                    Ok((built, stats)) => {
                        let pairs = built.len();
                        dict.set(Some(built));
                        dict_build_summary.set(Some(DictionaryBuildSummary {
                            built_at_unix: now_unix_seconds(),
                            pairs,
                            files_seen: stats.files_seen,
                            file_pairs: stats.file_pairs,
                        }));
                        dict_status.set(format!(
                            "辞書構築: pairs={} files={} pair_files={}",
                            pairs, stats.files_seen, stats.file_pairs
                        ));
                    }
                    Err(err) => dict_status.set(format!("辞書構築失敗: {err}")),
                }
            }
            MENU_QUICK_AUTO => {
                let selected = state.read().selected_key().map(|s| s.to_string());
                let entries = state.read().entries().to_vec();
                let result = {
                    let current = dict.read();
                    apply_quick_auto_selection(current.as_ref(), &entries, selected)
                };
                match result {
                    Ok((next, updated)) => {
                        if updated > 0 {
                            history.write().apply(next.clone());
                            state.write().set_entries(next);
                        }
                        dict_status.set(format!("Quick自動翻訳: updated={updated}"));
                    }
                    Err(err) => dict_status.set(err.to_string()),
                }
            }
            MENU_LANG_PANEL => active_tab.set(Tab::Lang),
            MENU_LANG_RESET => {
                dict_source_lang.set(DEFAULT_DICT_SOURCE_LANG.to_string());
                dict_target_lang.set(DEFAULT_DICT_TARGET_LANG.to_string());
                dict_root.set(DEFAULT_DICT_ROOT.to_string());
                dict_status.set(format!(
                    "言語ペアを {} -> {} に設定",
                    DEFAULT_DICT_SOURCE_LANG, DEFAULT_DICT_TARGET_LANG
                ));
            }
            MENU_UNDO => {
                if history.write().undo() {
                    let entries = history.read().present().clone();
                    state.write().set_entries(entries);
                }
            }
            MENU_REDO => {
                if history.write().redo() {
                    let entries = history.read().present().clone();
                    state.write().set_entries(entries);
                }
            }
            MENU_LOG_TAB => active_tab.set(Tab::Log),
            _ => {}
        });
    }

    #[cfg(all(
        feature = "dioxus-desktop",
        any(target_os = "windows", target_os = "linux", target_os = "macos")
    ))]
    {
        use dioxus::desktop::{use_global_shortcut, HotKeyState};
        if let Err(err) = use_global_shortcut("Ctrl+R", move |hotkey_state| {
            if hotkey_state != HotKeyState::Pressed {
                return;
            }
            let selected = state.read().selected_key().map(|s| s.to_string());
            let entries = state.read().entries().to_vec();
            let result = {
                let current = dict.read();
                apply_quick_auto_selection(current.as_ref(), &entries, selected)
            };
            match result {
                Ok((next, updated)) => {
                    if updated > 0 {
                        history.write().apply(next.clone());
                        state.write().set_entries(next);
                    }
                    dict_status.set(format!("Quick自動翻訳(Ctrl+R): updated={updated}"));
                }
                Err(err) => dict_status.set(err.to_string()),
            }
        }) {
            dict_status.set(format!("ショートカット登録失敗: {err:?}"));
        }
    }

    let selected_key = state.read().selected_key().map(|s| s.to_string());
    let selected_entry = state.read().selected_entry().cloned();
    let query = state.read().query().to_string();

    // Rebuild heavy derived data only when search/entries change, not on scroll updates.
    let filtered_snapshot = use_memo(move || {
        let entries = state.read().filtered_entries().to_vec();
        let counts = count_channels(&entries);
        FilterSnapshot { entries, counts }
    });

    let filtered_len = {
        let snapshot = filtered_snapshot.read();
        snapshot.entries.len()
    };
    let window = virtual_window(
        filtered_len,
        item_height,
        *viewport_height.read(),
        *scroll_offset.read(),
        overscan,
    );
    let rows = {
        let snapshot = filtered_snapshot.read();
        snapshot
            .entries
            .iter()
            .skip(window.start)
            .take(window.len())
            .map(|entry| {
                let (edid, record_id, ld) = row_fields(&entry.key, &entry.target_text);
                RowView {
                    key: entry.key.clone(),
                    source_text: entry.source_text.clone(),
                    target_text: entry.target_text.clone(),
                    edid,
                    record_id,
                    ld,
                    selected: selected_key.as_deref() == Some(entry.key.as_str()),
                }
            })
            .collect::<Vec<_>>()
    };

    let counts = filtered_snapshot.read().counts.clone();
    let ratio = if counts.total == 0 {
        0.0f32
    } else {
        (counts.translated as f32 / counts.total as f32) * 100.0
    };

    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }

        div { id: "app-shell",
            ondragover: move |event| {
                event.prevent_default();
            },
            ondrop: move |event| async move {
                event.prevent_default();
                let Some(file) = event.files().into_iter().next() else {
                    return;
                };
                let path = file.path();
                let ext = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_ascii_lowercase();
                if ext != "xml" {
                    file_status.set(format!("drop ignored (not xml): {}", path.display()));
                    return;
                }
                match file.read_string().await {
                    Ok(contents) => {
                        xml_text.set(contents.clone());
                        let current_entries = state.read().entries().to_vec();
                        match apply_xml_payload(&current_entries, &contents) {
                            Ok((merged, stats)) => {
                                if stats.updated > 0 {
                                    history.write().apply(merged.clone());
                                    state.write().set_entries(merged);
                                }
                                file_status.set(format!(
                                    "XML適用(drop): updated={} unchanged={} missing={}",
                                    stats.updated, stats.unchanged, stats.missing
                                ));
                                xml_error.set(None);
                            }
                            Err(err) => {
                                xml_error.set(Some(err.clone()));
                                file_status.set(format!("XML適用失敗(drop): {err}"));
                            }
                        }
                    }
                    Err(err) => file_status.set(format!("XML read error (drop): {err}")),
                }
            },
            div { class: "toolbar",
                button { class: "tool-ic", "F" }
                button { class: "tool-ic", "T" }
                button { class: "tool-ic", "O" }
                button { class: "tool-ic", "Y" }
                input {
                    id: "search",
                    value: "{query}",
                    placeholder: "原文/訳文/ID検索...",
                    oninput: move |e| {
                        state.write().set_query(&e.value());
                        scroll_offset.set(0.0);
                    },
                }
                button {
                    class: "tool-btn",
                    onclick: move |_| {
                        let Some(entry) = state.read().selected_entry().cloned() else {
                            validation_issues.set(Vec::new());
                            return;
                        };
                        let mut issues = Vec::new();
                        issues.extend(validate_braced_placeholders(&entry.key, &edit_source(), &edit_target()));
                        issues.extend(validate_printf_placeholders(&entry.key, &edit_source(), &edit_target()));
                        issues.extend(validate_alias_tags(&entry.key, &edit_source(), &edit_target()));
                        validation_issues.set(issues);
                    },
                    "Validate"
                }
                button {
                    class: "tool-btn",
                    onclick: move |_| {
                        let Some(entry) = state.read().selected_entry().cloned() else {
                            diff_status.set(None);
                            return;
                        };
                        let mut d = DiffEntry::new(&entry.key, &entry.source_text, &entry.target_text);
                        update_source(&mut d, &edit_source());
                        diff_status.set(Some(d.status));
                    },
                    "Diff"
                }
                button {
                    class: "tool-btn",
                    onclick: move |_| {
                        let msg = match encode(&edit_target(), Encoding::Latin1)
                            .and_then(|bytes| decode(&bytes, Encoding::Latin1)) {
                            Ok(_) => "Latin1 OK".to_string(),
                            Err(EncodingError::UnrepresentableChar) => "Latin1 error: unrepresentable".to_string(),
                            Err(EncodingError::InvalidUtf8) => "Latin1 error: invalid utf8".to_string(),
                        };
                        encoding_status.set(msg);
                    },
                    "Encoding"
                }
            }

            div { class: "channels",
                ChannelBox { label: format!("STRINGS [{}/{}]", counts.translated, counts.strings), ratio: ratio, color: "#dc4a4a" }
                ChannelBox { label: format!("DLSTRINGS [0/{}]", counts.dlstrings), ratio: 0.0, color: "#557fd9" }
                ChannelBox { label: format!("ILSTRINGS [0/{}]", counts.ilstrings), ratio: 0.0, color: "#76a65d" }
            }

            div { class: "grid-wrap",
                div { class: "grid-head",
                    span { class: "c-edid", "EDID" }
                    span { class: "c-id", "ID" }
                    span { class: "c-src", "原文" }
                    span { class: "c-dst", "訳文" }
                    span { class: "c-ld", "LD" }
                }
                div {
                    class: "grid-body",
                    onscroll: move |ev| {
                        let data = &ev.data;
                        let next_scroll = data.scroll_top() as f32;
                        let next_viewport = data.client_height() as f32;
                        // Virtual window only needs to advance when crossing the next row boundary.
                        let snapped_scroll = (next_scroll / item_height).floor() * item_height;
                        if (snapped_scroll - *scroll_offset.read()).abs() >= 0.5 {
                            scroll_offset.set(snapped_scroll);
                        }
                        if (next_viewport - *viewport_height.read()).abs() >= 0.5 {
                            viewport_height.set(next_viewport);
                        }
                    },
                    Spacer { window: window, position: SpacerPosition::Top }
                    for row in rows {
                        button {
                            key: "{row.key}",
                            class: if row.selected { "grid-row sel" } else { "grid-row" },
                            onclick: move |_| {
                                state.write().select(&row.key);
                                edit_source.set(row.source_text.clone());
                                edit_target.set(row.target_text.clone());
                            },
                            span { class: "c-edid", "{row.edid}" }
                            span { class: "c-id", "{row.record_id}" }
                            span { class: "c-src", "{row.source_text}" }
                            span { class: "c-dst", "{row.target_text}" }
                            span { class: "c-ld", "{row.ld}" }
                        }
                    }
                    Spacer { window: window, position: SpacerPosition::Bottom }
                }
            }

            div { class: "tabs",
                for (tab, label) in Tab::all() {
                    button {
                        class: if active_tab() == tab { "tab active" } else { "tab" },
                        onclick: move |_| active_tab.set(tab),
                        "{label}"
                    }
                }
            }

            div { class: "panel",
                if active_tab() == Tab::Home {
                    if let Some(entry) = selected_entry {
                        div { class: "editor",
                            p { class: "k", "Key: {entry.key}" }
                            label { "原文" }
                            textarea {
                                class: "txt",
                                value: "{edit_source}",
                                oninput: move |e| edit_source.set(e.value()),
                            }
                            label { "訳文" }
                            textarea {
                                class: "txt",
                                value: "{edit_target}",
                                oninput: move |e| edit_target.set(e.value()),
                            }
                            div { class: "actions",
                                button {
                                    class: "tool-btn",
                                    disabled: selected_key.is_none(),
                                    onclick: move |_| {
                                        let Some(key) = state.read().selected_key().map(|s| s.to_string()) else { return; };
                                        let next = {
                                            let mut s = state.write();
                                            if s.update_entry(&key, &edit_source(), &edit_target()) {
                                                Some(s.entries().to_vec())
                                            } else {
                                                None
                                            }
                                        };
                                        if let Some(entries) = next {
                                            history.write().apply(entries);
                                        }
                                    },
                                    "Apply Edit"
                                }
                                button {
                                    class: "tool-btn",
                                    onclick: move |_| {
                                        let p = loaded_plugin().clone();
                                        let s = loaded_strings().clone();
                                        match (p, s) {
                                            (Some(plugin), Some(strings)) => {
                                                hybrid_preview.set(build_hybrid_entries(&plugin, &strings));
                                                hybrid_error.set(None);
                                            }
                                            _ => hybrid_error.set(Some("Plugin/Stringsを先に読み込んでください".to_string())),
                                        }
                                    },
                                    "Build Hybrid"
                                }
                            }
                        }
                    } else {
                        p { "行を選択してください。" }
                    }
                } else if active_tab() == Tab::Log {
                    div { class: "log",
                        if !file_status().is_empty() { p { "{file_status}" } }
                        if !dict_status().is_empty() { p { "{dict_status}" } }
                        if !dict_prefs_error().is_empty() { p { class: "err", "{dict_prefs_error}" } }
                        if let Some(summary) = dict_build_summary() {
                            p {
                                "辞書情報: built_at(unix)={summary.built_at_unix} pairs={summary.pairs} files={summary.files_seen} pair_files={summary.file_pairs}"
                            }
                        }
                        if let Some(err) = xml_error() { p { class: "err", "{err}" } }
                        if let Some(err) = hybrid_error() { p { class: "err", "{err}" } }
                        if let Some(status) = diff_status() { p { "Diff status: {status:?}" } }
                        if !encoding_status().is_empty() { p { "{encoding_status}" } }
                        if !validation_issues().is_empty() {
                            for issue in validation_issues() {
                                p { "{issue.rule_id}: {issue.message}" }
                            }
                        }
                    }
                } else {
                    p { "このタブは次フェーズで実装します。" }
                }

                div { class: "io-row",
                    p { class: "inline", "XML適用: メニュー「ファイル > 翻訳XMLを一括適用」またはXMLファイルのドラッグ&ドロップを使用" }
                }

                div { class: "io-row",
                    label { class: "io",
                        "Dict Src"
                        input {
                            value: "{dict_source_lang}",
                            oninput: move |e| dict_source_lang.set(e.value()),
                        }
                    }
                    label { class: "io",
                        "Dict Dst"
                        input {
                            value: "{dict_target_lang}",
                            oninput: move |e| dict_target_lang.set(e.value()),
                        }
                    }
                    label { class: "io io-wide",
                        "Dict Root"
                        input {
                            value: "{dict_root}",
                            oninput: move |e| dict_root.set(e.value()),
                        }
                    }
                    if !dict_status().is_empty() {
                        p { class: "inline", "{dict_status}" }
                    }
                    if !dict_prefs_error().is_empty() {
                        p { class: "inline err", "{dict_prefs_error}" }
                    }
                }

                textarea {
                    class: "xml",
                    value: "{xml_text}",
                    oninput: move |e| xml_text.set(e.value()),
                }

                input {
                    id: "xml-picker-native",
                    style: "display:none;",
                    r#type: "file",
                    accept: ".xml",
                    onchange: move |event| async move {
                        let Some(file) = event.files().into_iter().next() else { return; };
                        match file.read_string().await {
                            Ok(contents) => {
                                xml_text.set(contents.clone());
                                let current_entries = state.read().entries().to_vec();
                                match apply_xml_payload(&current_entries, &contents) {
                                    Ok((merged, stats)) => {
                                        if stats.updated > 0 {
                                            history.write().apply(merged.clone());
                                            state.write().set_entries(merged);
                                        }
                                        file_status.set(format!(
                                            "XML適用: updated={} unchanged={} missing={}",
                                            stats.updated, stats.unchanged, stats.missing
                                        ));
                                        xml_error.set(None);
                                    }
                                    Err(err) => {
                                        xml_error.set(Some(err.clone()));
                                        file_status.set(format!("XML適用失敗: {err}"));
                                    }
                                }
                            }
                            Err(err) => file_status.set(format!("XML read error: {err}")),
                        }
                    },
                }

                input {
                    id: "strings-picker-native",
                    style: "display:none;",
                    r#type: "file",
                    accept: ".strings,.dlstrings,.ilstrings",
                    onchange: move |event| async move {
                        let Some(file) = event.files().into_iter().next() else { return; };
                        let path = file.path();
                        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                        let Some(kind) = StringsKind::from_extension(ext) else {
                            file_status.set(format!("unsupported strings extension: {ext}"));
                            return;
                        };
                        let bytes = match file.read_bytes().await {
                            Ok(v) => v,
                            Err(err) => {
                                file_status.set(format!("Strings read error: {err}"));
                                return;
                            }
                        };
                        let parsed = match kind {
                            StringsKind::Strings => read_strings(&bytes),
                            StringsKind::DlStrings => read_dlstrings(&bytes),
                            StringsKind::IlStrings => read_ilstrings(&bytes),
                        };
                        match parsed {
                            Ok(strings) => {
                                let entries = strings
                                    .entries
                                    .iter()
                                    .map(|e| Entry {
                                        key: format!("strings:{}", e.id),
                                        source_text: e.text.clone(),
                                        target_text: String::new(),
                                    })
                                    .collect::<Vec<_>>();
                                history.write().apply(entries.clone());
                                state.write().set_entries(entries);
                                loaded_strings.set(Some(strings));
                                loaded_strings_kind.set(Some(kind));
                                loaded_strings_path.set(Some(path));
                                loaded_plugin.set(None);
                                loaded_plugin_path.set(None);
                                loaded_esp_strings.set(None);
                                file_status.set("Stringsを読み込みました".to_string());
                            }
                            Err(err) => file_status.set(format!("Strings parse error: {err:?}")),
                        }
                    },
                }

                input {
                    id: "plugin-picker-native",
                    style: "display:none;",
                    r#type: "file",
                    accept: ".esp,.esm,.esl,.xtplugin",
                    onchange: move |event| async move {
                        let Some(file) = event.files().into_iter().next() else { return; };
                        let path = file.path();
                        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
                        if ext == "xtplugin" {
                            match file.read_string().await {
                                Ok(content) => match read_plugin(&content) {
                                    Ok(plugin) => {
                                        let entries = plugin
                                            .entries
                                            .iter()
                                            .map(|e| Entry {
                                                key: format!("plugin:{}", e.id),
                                                source_text: e.source_text.clone(),
                                                target_text: String::new(),
                                            })
                                            .collect::<Vec<_>>();
                                        history.write().apply(entries.clone());
                                        state.write().set_entries(entries);
                                        loaded_plugin.set(Some(plugin));
                                        loaded_plugin_path.set(Some(path));
                                        loaded_esp_strings.set(None);
                                        loaded_strings.set(None);
                                        loaded_strings_kind.set(None);
                                        loaded_strings_path.set(None);
                                        file_status.set("xtpluginを読み込みました".to_string());
                                    }
                                    Err(err) => file_status.set(format!("xtplugin parse error: {err:?}")),
                                },
                                Err(err) => file_status.set(format!("xtplugin read error: {err}")),
                            }
                        } else {
                            let bytes = match file.read_bytes().await {
                                Ok(v) => v,
                                Err(err) => {
                                    file_status.set(format!("plugin read error: {err}"));
                                    return;
                                }
                            };
                            let workspace_root = workspace_root_from_plugin(&path);
                            let entries = match extract_esp_strings(&path, &workspace_root, Some("english")) {
                                Ok(strings) => {
                                    loaded_esp_strings.set(Some(strings.clone()));
                                    strings
                                        .iter()
                                        .map(|s| Entry {
                                            key: s.get_unique_key(),
                                            source_text: s.text.clone(),
                                            target_text: String::new(),
                                        })
                                        .collect::<Vec<_>>()
                                }
                                Err(err) => {
                                    file_status.set(format!("ESP parse error (fallback): {err}"));
                                    extract_null_terminated_utf8(&bytes, 4)
                                        .into_iter()
                                        .map(|x| Entry {
                                            key: format!("plugin:{:08x}", x.offset),
                                            source_text: x.text,
                                            target_text: String::new(),
                                        })
                                        .collect::<Vec<_>>()
                                }
                            };
                            history.write().apply(entries.clone());
                            state.write().set_entries(entries);
                            loaded_plugin.set(None);
                            loaded_plugin_path.set(Some(path));
                            loaded_strings.set(None);
                            loaded_strings_kind.set(None);
                            loaded_strings_path.set(None);
                            file_status.set("Pluginを読み込みました".to_string());
                        }
                    },
                }
            }

            div { class: "status",
                div { class: "meter", div { class: "meter-fill", style: "width: {ratio}%" } }
                div { class: "s", "[{dict_source_lang}] -> [{dict_target_lang}]" }
                div { class: "s", "{file_status}" }
                div { class: "s", "{counts.translated}/{counts.total}" }
            }
        }
    }
}

#[component]
fn ChannelBox(label: String, ratio: f32, color: &'static str) -> Element {
    rsx! {
        div { class: "ch",
            div { class: "t", "{label}" }
            div { class: "bar", div { class: "fill", style: "width: {ratio}%; background: {color};" } }
        }
    }
}

#[component]
fn Spacer(window: VirtualWindow, position: SpacerPosition) -> Element {
    let height = match position {
        SpacerPosition::Top => window.top_pad,
        SpacerPosition::Bottom => window.bottom_pad,
    };
    rsx! { div { class: "spacer", style: "height: {height}px;" } }
}

#[derive(Clone, Default, PartialEq)]
struct ChannelCounts {
    total: usize,
    translated: usize,
    strings: usize,
    dlstrings: usize,
    ilstrings: usize,
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

fn row_fields(key: &str, target_text: &str) -> (String, String, String) {
    let edid = key.split(':').next_back().unwrap_or(key).to_string();
    let record_id = if key.to_ascii_lowercase().contains("plugin") {
        "REC FULL".to_string()
    } else {
        "WEAP FULL".to_string()
    };
    let ld = if target_text.is_empty() { "-" } else { "T" }.to_string();
    (edid, record_id, ld)
}

fn now_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn apply_quick_auto_selection(
    dict: Option<&TranslationDictionary>,
    entries: &[Entry],
    selected_key: Option<String>,
) -> Result<(Vec<Entry>, usize), &'static str> {
    let Some(dict) = dict else {
        return Err("辞書未構築");
    };
    let Some(selected_key) = selected_key else {
        return Err("Quick自動翻訳対象の行を選択してください");
    };
    let selected = vec![selected_key];
    Ok(dict.apply_quick(entries, &selected, true))
}

fn apply_xml_payload(
    current: &[Entry],
    xml_contents: &str,
) -> Result<(Vec<Entry>, XmlApplyStats), String> {
    let imported = import_entries(xml_contents).map_err(|err| format!("{err:?}"))?;
    Ok(apply_xml_default(current, &imported))
}

fn dictionary_prefs_path() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("XDG_CONFIG_HOME") {
        return Some(PathBuf::from(dir).join("xtrans-rs").join(DICT_PREFS_FILE));
    }
    if let Ok(home) = std::env::var("HOME") {
        return Some(
            PathBuf::from(home)
                .join(".config")
                .join("xtrans-rs")
                .join(DICT_PREFS_FILE),
        );
    }
    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            return Some(
                PathBuf::from(appdata)
                    .join("xtrans-rs")
                    .join(DICT_PREFS_FILE),
            );
        }
    }
    None
}

fn load_dictionary_prefs() -> Result<DictionaryPrefs, String> {
    let Some(path) = dictionary_prefs_path() else {
        return Ok(DictionaryPrefs::default());
    };
    if !path.exists() {
        return Ok(DictionaryPrefs::default());
    }
    let content =
        std::fs::read_to_string(&path).map_err(|err| format!("read {}: {err}", path.display()))?;
    parse_dictionary_prefs(&content)
}

fn save_dictionary_prefs(prefs: &DictionaryPrefs) -> Result<(), String> {
    let Some(path) = dictionary_prefs_path() else {
        return Err("設定保存先を解決できません".to_string());
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("create {}: {err}", parent.display()))?;
    }
    std::fs::write(&path, serialize_dictionary_prefs(prefs))
        .map_err(|err| format!("write {}: {err}", path.display()))
}

fn serialize_dictionary_prefs(prefs: &DictionaryPrefs) -> String {
    let mut lines = Vec::new();
    lines.push("version=1".to_string());
    lines.push(format!(
        "source_lang={}",
        escape_pref_value(&prefs.source_lang)
    ));
    lines.push(format!(
        "target_lang={}",
        escape_pref_value(&prefs.target_lang)
    ));
    lines.push(format!("root={}", escape_pref_value(&prefs.root)));
    lines.join("\n")
}

fn parse_dictionary_prefs(content: &str) -> Result<DictionaryPrefs, String> {
    let mut out = DictionaryPrefs::default();
    let mut version = None::<u32>;
    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return Err("辞書設定フォーマットが不正です".to_string());
        };
        match key {
            "version" => {
                let v = value
                    .parse::<u32>()
                    .map_err(|_| "辞書設定versionが不正です".to_string())?;
                version = Some(v);
            }
            "source_lang" => out.source_lang = unescape_pref_value(value)?,
            "target_lang" => out.target_lang = unescape_pref_value(value)?,
            "root" => out.root = unescape_pref_value(value)?,
            _ => {}
        }
    }
    match version {
        Some(1) => Ok(out),
        Some(v) => Err(format!("未対応の辞書設定version: {v}")),
        None => Err("辞書設定versionがありません".to_string()),
    }
}

fn escape_pref_value(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for b in input.bytes() {
        match b {
            b'%' => out.push_str("%25"),
            b'=' => out.push_str("%3D"),
            b'\n' => out.push_str("%0A"),
            b'\r' => out.push_str("%0D"),
            _ => out.push(b as char),
        }
    }
    out
}

fn unescape_pref_value(input: &str) -> Result<String, String> {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if i + 2 >= bytes.len() {
                return Err("辞書設定エスケープが不正です".to_string());
            }
            let hi = (bytes[i + 1] as char)
                .to_digit(16)
                .ok_or_else(|| "辞書設定エスケープが不正です".to_string())?;
            let lo = (bytes[i + 2] as char)
                .to_digit(16)
                .ok_or_else(|| "辞書設定エスケープが不正です".to_string())?;
            out.push((hi * 16 + lo) as u8);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).map_err(|_| "辞書設定文字列が不正です".to_string())
}

#[cfg(all(
    feature = "dioxus-desktop",
    any(target_os = "windows", target_os = "linux", target_os = "macos")
))]
fn build_native_menu() -> dioxus::desktop::muda::Menu {
    use dioxus::desktop::muda::{
        accelerator::{Accelerator, Code, Modifiers},
        Menu, MenuItem, PredefinedMenuItem, Submenu,
    };

    let menu = Menu::new();

    let file_menu = Submenu::new("ファイル(F)", true);
    let open_plugin = MenuItem::with_id(MENU_OPEN_PLUGIN, "Esp/Esmファイルを開く", true, None);
    let open_strings = MenuItem::with_id(MENU_OPEN_STRINGS, "Stringsファイルを開く", true, None);
    let xml_export = MenuItem::with_id(MENU_XML_EXPORT, "翻訳XMLを書き出し", true, None);
    let xml_apply = MenuItem::with_id(MENU_XML_APPLY, "翻訳XMLを一括適用", true, None);
    let save = MenuItem::with_id(MENU_SAVE_OVERWRITE, "上書き保存", true, None);
    let save_as = MenuItem::with_id(MENU_SAVE_AS, "別名保存", true, None);
    let sep_file_1 = PredefinedMenuItem::separator();
    let sep_file_2 = PredefinedMenuItem::separator();
    let quit = PredefinedMenuItem::quit(None);
    let _ = file_menu.append_items(&[
        &open_plugin,
        &open_strings,
        &sep_file_1,
        &xml_export,
        &xml_apply,
        &save,
        &save_as,
        &sep_file_2,
        &quit,
    ]);

    let translate_menu = Submenu::new("翻訳(T)", true);
    let dict_build = MenuItem::with_id(MENU_DICT_BUILD, "辞書を構築", true, None);
    let quick_auto = MenuItem::with_id(
        MENU_QUICK_AUTO,
        "Quick自動翻訳",
        true,
        Some(Accelerator::new(Some(Modifiers::CONTROL), Code::KeyR)),
    );
    let _ = translate_menu.append_items(&[&dict_build, &quick_auto]);

    let options_menu = Submenu::new("オプション(Z)", true);
    let lang_panel = MenuItem::with_id(MENU_LANG_PANEL, "言語と辞書を開く", true, None);
    let lang_reset = MenuItem::with_id(MENU_LANG_RESET, "言語ペアを既定に戻す", true, None);
    let _ = options_menu.append_items(&[&lang_panel, &lang_reset]);

    let tools_menu = Submenu::new("ツール(Y)", true);
    let undo = MenuItem::with_id(MENU_UNDO, "Undo", true, None);
    let redo = MenuItem::with_id(MENU_REDO, "Redo", true, None);
    let sep_tools = PredefinedMenuItem::separator();
    let log_tab = MenuItem::with_id(MENU_LOG_TAB, "ログタブを開く", true, None);
    let _ = tools_menu.append_items(&[&undo, &redo, &sep_tools, &log_tab]);

    let _ = menu.append_items(&[&file_menu, &translate_menu, &options_menu, &tools_menu]);
    menu
}

fn save_overwrite(
    entries: &[Entry],
    loaded_strings: Option<StringsFile>,
    loaded_strings_kind: Option<StringsKind>,
    loaded_strings_path: Option<PathBuf>,
    loaded_plugin: Option<PluginFile>,
    loaded_plugin_path: Option<PathBuf>,
    loaded_esp_strings: Option<Vec<ExtractedString>>,
) -> Result<PathBuf, String> {
    if let Some(plugin_path) = loaded_plugin_path {
        if let Some(extracted) = loaded_esp_strings {
            return save_esp(entries, &plugin_path, &plugin_path, extracted);
        }
        if let Some(plugin) = loaded_plugin {
            ensure_backup(&plugin_path)?;
            let encoded = write_plugin(&plugin).map_err(|e| format!("{e:?}"))?;
            std::fs::write(&plugin_path, encoded)
                .map_err(|e| format!("plugin save {}: {e}", plugin_path.display()))?;
            return Ok(plugin_path);
        }
    }

    if let (Some(strings), Some(kind), Some(path)) =
        (loaded_strings, loaded_strings_kind, loaded_strings_path)
    {
        return save_strings(entries, &strings, kind, &path);
    }

    Err("保存対象がありません".to_string())
}

fn save_as(
    entries: &[Entry],
    loaded_strings: Option<StringsFile>,
    loaded_strings_kind: Option<StringsKind>,
    loaded_strings_path: Option<PathBuf>,
    loaded_plugin: Option<PluginFile>,
    loaded_plugin_path: Option<PathBuf>,
    loaded_esp_strings: Option<Vec<ExtractedString>>,
) -> Result<PathBuf, String> {
    if let Some(plugin_path) = loaded_plugin_path {
        if let Some(extracted) = loaded_esp_strings {
            let out = with_suffix_path(&plugin_path, "_translated");
            return save_esp(entries, &plugin_path, &out, extracted);
        }
        if let Some(plugin) = loaded_plugin {
            let out = with_suffix_path(&plugin_path, "_translated");
            let encoded = write_plugin(&plugin).map_err(|e| format!("{e:?}"))?;
            std::fs::write(&out, encoded)
                .map_err(|e| format!("plugin save {}: {e}", out.display()))?;
            return Ok(out);
        }
    }

    if let (Some(strings), Some(kind), Some(path)) =
        (loaded_strings, loaded_strings_kind, loaded_strings_path)
    {
        let out = with_suffix_path(&path, "_translated");
        return save_strings(entries, &strings, kind, &out);
    }

    Err("保存対象がありません".to_string())
}

fn save_strings(
    entries: &[Entry],
    base: &StringsFile,
    kind: StringsKind,
    path: &Path,
) -> Result<PathBuf, String> {
    if path.exists() {
        ensure_backup(path)?;
    }
    let updated = apply_entries_to_strings(base, entries);
    let bytes = match kind {
        StringsKind::Strings => write_strings(&updated),
        StringsKind::DlStrings => write_dlstrings(&updated),
        StringsKind::IlStrings => write_ilstrings(&updated),
    }
    .map_err(|e| format!("{e:?}"))?;
    std::fs::write(path, bytes).map_err(|e| format!("write {}: {e}", path.display()))?;
    Ok(path.to_path_buf())
}

fn save_esp(
    entries: &[Entry],
    input_path: &Path,
    output_path: &Path,
    extracted: Vec<ExtractedString>,
) -> Result<PathBuf, String> {
    if input_path == output_path && input_path.exists() {
        ensure_backup(input_path)?;
    }

    let mut targets: HashMap<&str, &str> = HashMap::new();
    for entry in entries {
        if !entry.target_text.is_empty() {
            targets.insert(entry.key.as_str(), entry.target_text.as_str());
        }
    }

    let mut translated = extracted;
    for item in &mut translated {
        let key = item.get_unique_key();
        if let Some(target) = targets.get(key.as_str()) {
            item.text = (*target).to_string();
        }
    }

    let out_dir = output_path.parent().unwrap_or_else(|| Path::new("."));
    let workspace_root = workspace_root_from_plugin(input_path);
    let written = apply_translations(
        input_path,
        &workspace_root,
        out_dir,
        translated,
        Some("english"),
    )
    .map_err(|e| format!("esp apply failed {}: {e}", input_path.display()))?;

    if written == output_path {
        return Ok(written);
    }

    std::fs::copy(&written, output_path).map_err(|e| {
        format!(
            "copy {} -> {} failed: {e}",
            written.display(),
            output_path.display()
        )
    })?;
    Ok(output_path.to_path_buf())
}

fn apply_entries_to_strings(base: &StringsFile, entries: &[Entry]) -> StringsFile {
    let mut by_id: HashMap<u32, &str> = HashMap::new();
    for entry in entries {
        if let Some(id) = parse_strings_id(&entry.key) {
            if !entry.target_text.is_empty() {
                by_id.insert(id, entry.target_text.as_str());
            }
        }
    }
    let out = base
        .entries
        .iter()
        .map(|entry| {
            if let Some(target) = by_id.get(&entry.id) {
                StringsEntry {
                    id: entry.id,
                    text: (*target).to_string(),
                }
            } else {
                entry.clone()
            }
        })
        .collect::<Vec<_>>();
    StringsFile { entries: out }
}

fn parse_strings_id(key: &str) -> Option<u32> {
    let (_, id) = key.rsplit_once(':')?;
    id.parse::<u32>().ok()
}

fn ensure_backup(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    let backup = next_backup_path(path);
    std::fs::copy(path, &backup).map_err(|e| {
        format!(
            "backup failed {} -> {}: {e}",
            path.display(),
            backup.display()
        )
    })?;
    Ok(())
}

fn next_backup_path(path: &Path) -> PathBuf {
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
    let parent = path.parent().unwrap_or_else(|| Path::new("."));

    for i in 0usize..1000usize {
        let name = if i == 0 {
            if ext.is_empty() {
                format!("{stem}.bak")
            } else {
                format!("{stem}.bak.{ext}")
            }
        } else if ext.is_empty() {
            format!("{stem}.bak{i}")
        } else {
            format!("{stem}.bak{i}.{ext}")
        };
        let p = parent.join(name);
        if !p.exists() {
            return p;
        }
    }

    with_suffix_path(path, ".bak999")
}

fn with_suffix_path(path: &Path, suffix: &str) -> PathBuf {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
    let file = if ext.is_empty() {
        format!("{stem}{suffix}")
    } else {
        format!("{stem}{suffix}.{ext}")
    };
    path.parent().unwrap_or_else(|| Path::new(".")).join(file)
}

fn workspace_root_from_plugin(path: &Path) -> PathBuf {
    let Some(parent) = path.parent() else {
        return PathBuf::from(".");
    };
    let is_data_dir = parent
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.eq_ignore_ascii_case("Data"))
        .unwrap_or(false);
    if is_data_dir {
        if let Some(root) = parent.parent() {
            return root.to_path_buf();
        }
    }
    parent.to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t_app_001_apply_entries_to_strings_updates_target() {
        let base = StringsFile {
            entries: vec![
                StringsEntry {
                    id: 1,
                    text: "Iron Sword".to_string(),
                },
                StringsEntry {
                    id: 2,
                    text: "Steel Sword".to_string(),
                },
            ],
        };
        let entries = vec![Entry {
            key: "strings:1".to_string(),
            source_text: "Iron Sword".to_string(),
            target_text: "鉄の剣".to_string(),
        }];
        let updated = apply_entries_to_strings(&base, &entries);
        assert_eq!(updated.entries[0].text, "鉄の剣");
        assert_eq!(updated.entries[1].text, "Steel Sword");
    }

    #[test]
    fn t_app_002_parse_strings_id() {
        assert_eq!(parse_strings_id("strings:42"), Some(42));
        assert_eq!(parse_strings_id("plugin:abcd"), None);
    }

    #[test]
    fn t_app_003_next_backup_path_increments() {
        let root = std::env::temp_dir().join(format!("xt_app_backup_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("create");
        let base = root.join("file.strings");
        std::fs::write(&base, b"abc").expect("write");
        let b0 = next_backup_path(&base);
        std::fs::write(&b0, b"x").expect("write b0");
        let b1 = next_backup_path(&base);
        assert_ne!(b0, b1);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn t_app_004_dict_prefs_round_trip() {
        let prefs = DictionaryPrefs {
            source_lang: "english".to_string(),
            target_lang: "japanese".to_string(),
            root: "/tmp/with=equals".to_string(),
        };
        let encoded = serialize_dictionary_prefs(&prefs);
        let decoded = parse_dictionary_prefs(&encoded).expect("parse prefs");
        assert_eq!(decoded, prefs);
    }

    #[test]
    fn t_app_005_quick_auto_requires_selection() {
        let entries = vec![Entry {
            key: "k1".to_string(),
            source_text: "Iron Sword".to_string(),
            target_text: String::new(),
        }];
        let dict = TranslationDictionary::build_from_entries(&[Entry {
            key: "d".to_string(),
            source_text: "Iron Sword".to_string(),
            target_text: "鉄の剣".to_string(),
        }]);
        let err =
            apply_quick_auto_selection(Some(&dict), &entries, None).expect_err("selection error");
        assert_eq!(err, "Quick自動翻訳対象の行を選択してください");
    }

    #[test]
    fn t_app_006_apply_xml_payload_updates_entry() {
        let current = vec![Entry {
            key: "k1".to_string(),
            source_text: "Iron Sword".to_string(),
            target_text: String::new(),
        }];
        let xml = export_entries(&[Entry {
            key: "k1".to_string(),
            source_text: "Iron Sword".to_string(),
            target_text: "鉄の剣".to_string(),
        }]);
        let (merged, stats) = apply_xml_payload(&current, &xml).expect("apply xml");
        assert_eq!(stats.updated, 1);
        assert_eq!(stats.missing, 0);
        assert_eq!(merged[0].target_text, "鉄の剣");
    }
}
