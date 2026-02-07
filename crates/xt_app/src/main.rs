use xt_core::diff::{update_source, DiffEntry, EntryStatus};
use xt_core::encoding::{decode, encode, Encoding, EncodingError};
use xt_core::dictionary::TranslationDictionary;
use xt_core::formats::plugin::{read_plugin, write_plugin, PluginFile};
use xt_core::formats::esp::{apply_translations, extract_strings as extract_esp_strings};
use xt_core::formats::plugin_binary::extract_null_terminated_utf8;
use xt_core::formats::strings::{
    read_dlstrings, read_ilstrings, read_strings, write_dlstrings, write_ilstrings, write_strings,
    StringsFile,
};
use xt_core::hybrid::{build_hybrid_entries, HybridEntry};
use xt_core::import_export::{apply_xml_default, export_entries, import_entries};
use xt_core::model::Entry;
use xt_core::ui_state::TwoPaneState;
use xt_core::undo::UndoStack;
use xt_core::validation::{
    validate_alias_tags, validate_braced_placeholders, validate_printf_placeholders,
    ValidationIssue,
};
use xt_core::virtual_list::{virtual_window, VirtualWindow};
use dioxus::prelude::*;
use xt_core::formats::esp::ExtractedString;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

#[derive(Clone, Copy)]
enum StringsKind {
    Strings,
    DlStrings,
    IlStrings,
}

impl StringsKind {
    fn from_extension(ext: &str) -> Self {
        if ext.eq_ignore_ascii_case("dlstrings") {
            Self::DlStrings
        } else if ext.eq_ignore_ascii_case("ilstrings") {
            Self::IlStrings
        } else {
            Self::Strings
        }
    }
}

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let mut history = use_signal(|| UndoStack::new(sample_entries()));
    let mut state = use_signal(|| TwoPaneState::new(history.read().present().clone()));
    let mut scroll_offset = use_signal(|| 0.0f32);
    let mut viewport_height = use_signal(|| 520.0f32);
    let mut edit_source = use_signal(|| String::new());
    let mut edit_target = use_signal(|| String::new());
    let mut xml_text = use_signal(|| String::new());
    let mut xml_error = use_signal(|| Option::<String>::None);
    let mut validation_issues = use_signal(Vec::<ValidationIssue>::new);
    let mut diff_status = use_signal(|| Option::<EntryStatus>::None);
    let mut encoding_status = use_signal(|| String::new());
    let mut hybrid_preview = use_signal(Vec::<HybridEntry>::new);
    let mut hybrid_error = use_signal(|| Option::<String>::None);
    let mut loaded_plugin = use_signal(|| Option::<PluginFile>::None);
    let mut loaded_strings = use_signal(|| Option::<StringsFile>::None);
    let mut loaded_strings_path = use_signal(|| Option::<PathBuf>::None);
    let mut loaded_strings_kind = use_signal(|| Option::<StringsKind>::None);
    let mut loaded_plugin_path = use_signal(|| Option::<PathBuf>::None);
    let mut file_status = use_signal(|| String::new());
    let mut loaded_esp_strings = use_signal(|| Option::<Vec<ExtractedString>>::None);
    let mut active_tab = use_signal(|| "home".to_string());
    let mut dictionary = use_signal(|| Option::<TranslationDictionary>::None);
    let mut dictionary_status = use_signal(String::new);
    let mut dict_source_lang = use_signal(|| "english".to_string());
    let mut dict_target_lang = use_signal(|| "japanese".to_string());
    let mut dict_root = use_signal(|| "./Data/Strings/Translations".to_string());
    let item_height = 64.0f32;
    let overscan = 8usize;
    let list_padding = 0.0f32;

    let (
        window,
        entries,
        selected_key,
        selected_entry,
        query,
        total_count,
        translated_count,
        strings_count,
        dlstrings_count,
        ilstrings_count,
    ) = {
        let state = state.read();
        let filtered = state.filtered_entries();
        let total = filtered.len();
        let mut strings_count = 0usize;
        let mut dlstrings_count = 0usize;
        let mut ilstrings_count = 0usize;
        let mut translated_count = 0usize;
        for entry in filtered.iter() {
            if !entry.target_text.is_empty() {
                translated_count += 1;
            }
            let key = entry.key.to_ascii_lowercase();
            if key.contains("dlstrings") {
                dlstrings_count += 1;
            } else if key.contains("ilstrings") {
                ilstrings_count += 1;
            } else {
                strings_count += 1;
            }
        }
        let window = virtual_window(
            total,
            item_height,
            *viewport_height.read(),
            *scroll_offset.read(),
            overscan,
        );
        let selected_key = state.selected_key().map(|key| key.to_string());
        let entries = filtered
            .iter()
            .skip(window.start)
            .take(window.len())
            .map(|entry| EntryView {
                key: entry.key.clone(),
                source_text: entry.source_text.clone(),
                target_text: entry.target_text.clone(),
                is_selected: selected_key.as_deref() == Some(entry.key.as_str()),
            })
            .collect::<Vec<_>>();
        let selected_entry = state.selected_entry().cloned();
        let query = state.query().to_string();
        (
            window,
            entries,
            selected_key,
            selected_entry,
            query,
            total,
            translated_count,
            strings_count,
            dlstrings_count,
            ilstrings_count,
        )
    };

    let translation_ratio = if total_count == 0 {
        0.0f32
    } else {
        (translated_count as f32 / total_count as f32) * 100.0
    };

    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }

        div { id: "app-shell",
            div { class: "menu-bar",
                button {
                    class: "menu-item",
                    onclick: move |_| {
                        let xml = export_entries(state.read().entries());
                        xml_text.set(xml);
                        xml_error.set(None);
                        file_status.set("XML exported to editor.".to_string());
                    },
                    "XML書き出し"
                }
                button {
                    class: "menu-item",
                    onclick: move |_| {
                        match import_entries(&xml_text()) {
                            Ok(imported) => {
                                let (merged, stats) = apply_xml_default(state.read().entries(), &imported);
                                if stats.updated == 0 {
                                    file_status.set(format!(
                                        "XML import(default): updated=0 unchanged={} missing={}",
                                        stats.unchanged, stats.missing
                                    ));
                                    return;
                                }
                                history.write().apply(merged.clone());
                                state.write().set_entries(merged);
                                xml_error.set(None);
                                file_status.set(format!(
                                    "XML import(default): updated={} unchanged={} missing={}",
                                    stats.updated, stats.unchanged, stats.missing
                                ));
                            }
                            Err(err) => {
                                xml_error.set(Some(format!("Import error: {err:?}")));
                            }
                        }
                    },
                    "XML一括適用"
                }
                button {
                    class: "menu-item",
                    onclick: move |_| {
                        match save_overwrite(
                            state.read().entries(),
                            loaded_strings(),
                            loaded_strings_kind(),
                            loaded_strings_path(),
                            loaded_plugin(),
                            loaded_plugin_path(),
                            loaded_esp_strings(),
                        ) {
                            Ok(path) => file_status.set(format!("Saved: {}", path.display())),
                            Err(err) => file_status.set(format!("Save failed: {err}")),
                        }
                    },
                    "上書き保存"
                }
                button {
                    class: "menu-item",
                    onclick: move |_| {
                        match save_as_translated(
                            state.read().entries(),
                            loaded_strings(),
                            loaded_strings_kind(),
                            loaded_strings_path(),
                            loaded_plugin(),
                            loaded_plugin_path(),
                            loaded_esp_strings(),
                        ) {
                            Ok(path) => file_status.set(format!("Saved as: {}", path.display())),
                            Err(err) => file_status.set(format!("Save as failed: {err}")),
                        }
                    },
                    "別名保存"
                }
                button {
                    class: "menu-item",
                    onclick: move |_| {
                        let root = PathBuf::from(dict_root());
                        match TranslationDictionary::build_from_strings_dir(
                            &root,
                            &dict_source_lang(),
                            &dict_target_lang(),
                        ) {
                            Ok((built, stats)) => {
                                let size = built.len();
                                dictionary.set(Some(built));
                                dictionary_status.set(format!(
                                    "dict: pairs={} files_seen={} file_pairs={}",
                                    size, stats.files_seen, stats.file_pairs
                                ));
                            }
                            Err(err) => {
                                dictionary_status.set(format!("dict build failed: {err}"));
                            }
                        }
                    },
                    "辞書構築"
                }
                button {
                    class: "menu-item",
                    onclick: move |_| {
                        let Some(dict) = dictionary() else {
                            dictionary_status.set("dict not built".to_string());
                            return;
                        };
                        let selected = state
                            .read()
                            .selected_key()
                            .map(|s| s.to_string())
                            .into_iter()
                            .collect::<Vec<_>>();
                        let (next, updated) =
                            dict.apply_quick(state.read().entries(), &selected, true);
                        if updated == 0 {
                            dictionary_status.set("quick auto-translate: no updates".to_string());
                            return;
                        }
                        history.write().apply(next.clone());
                        state.write().set_entries(next);
                        dictionary_status
                            .set(format!("quick auto-translate: {updated} updated"));
                    },
                    "Quick自動翻訳"
                }
                button { class: "menu-item", "オプション(Z)" }
                button { class: "menu-item", "ツール(Y)" }
            }
            div { class: "tool-bar",
                input {
                    id: "search",
                    placeholder: "原文/訳文を検索...",
                    value: "{query}",
                    oninput: move |event| {
                        state.write().set_query(&event.value());
                        scroll_offset.set(0.0);
                    },
                }
                button {
                    class: "tool-button",
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
                    class: "tool-button",
                    onclick: move |_| {
                        let Some(entry) = state.read().selected_entry().cloned() else {
                            diff_status.set(None);
                            return;
                        };
                        let mut diff_entry = DiffEntry::new(&entry.key, &entry.source_text, &entry.target_text);
                        update_source(&mut diff_entry, &edit_source());
                        diff_status.set(Some(diff_entry.status));
                    },
                    "Diff"
                }
                button {
                    class: "tool-button",
                    onclick: move |_| {
                        let result = match encode(&edit_target(), Encoding::Latin1)
                            .and_then(|bytes| decode(&bytes, Encoding::Latin1)) {
                            Ok(text) => format!("Latin1 OK: {text}"),
                            Err(EncodingError::UnrepresentableChar) => "Latin1 error: unrepresentable".to_string(),
                            Err(EncodingError::InvalidUtf8) => "Latin1 error: invalid utf8".to_string(),
                        };
                        encoding_status.set(result);
                    },
                    "Encoding"
                }
                button {
                    class: "tool-button",
                    onclick: move |_| {
                        if history.write().undo() {
                            let entries = history.read().present().clone();
                            state.write().set_entries(entries);
                        }
                    },
                    "Undo"
                }
                button {
                    class: "tool-button",
                    onclick: move |_| {
                        if history.write().redo() {
                            let entries = history.read().present().clone();
                            state.write().set_entries(entries);
                        }
                    },
                    "Redo"
                }
            }
            div { class: "channel-bar",
                div { class: "channel-box",
                    div { class: "channel-title", "STRINGS [{translated_count}/{strings_count}]" }
                    div { class: "channel-meter",
                        div { class: "channel-fill", style: "width: {translation_ratio}%" }
                    }
                }
                div { class: "channel-box",
                    div { class: "channel-title", "DLSTRINGS [0/{dlstrings_count}]" }
                    div { class: "channel-meter",
                        div { class: "channel-fill dl", style: "width: 0%" }
                    }
                }
                div { class: "channel-box",
                    div { class: "channel-title", "ILSTRINGS [0/{ilstrings_count}]" }
                    div { class: "channel-meter",
                        div { class: "channel-fill il", style: "width: 0%" }
                    }
                }
            }
            div { class: "grid-root",
                div { class: "grid-header",
                    span { class: "col-edid", "EDID" }
                    span { class: "col-id", "ID" }
                    span { class: "col-src", "原文" }
                    span { class: "col-dst", "訳文" }
                    span { class: "col-ld", "LD" }
                }
                div { class: "grid-body",
                    onscroll: move |event| {
                        let data = &event.data;
                        let offset = (data.scroll_top() as f32 - list_padding).max(0.0);
                        scroll_offset.set(offset);
                        viewport_height.set(data.client_height() as f32);
                    },
                    Spacer { window: window, position: SpacerPosition::Top }
                    for EntryView { key, source_text, target_text, is_selected } in entries {
                        {
                            let (edid, rec_id, ld) = row_fields(&key, &target_text);
                            rsx! {
                                button {
                                    class: if is_selected { "grid-row selected" } else { "grid-row" },
                                    onclick: move |_| {
                                        state.write().select(&key);
                                        edit_source.set(source_text.clone());
                                        edit_target.set(target_text.clone());
                                    },
                                    span { class: "col-edid", "{edid}" }
                                    span { class: "col-id", "{rec_id}" }
                                    span { class: "col-src", "{source_text}" }
                                    span { class: "col-dst", "{target_text}" }
                                    span { class: "col-ld", "{ld}" }
                                }
                            }
                        }
                    }
                    Spacer { window: window, position: SpacerPosition::Bottom }
                }
            }
            div { class: "work-tabs",
                for (id, label) in [
                    ("home", "ホーム"),
                    ("heuristic", "ヒューリスティック候補"),
                    ("lang", "言語"),
                    ("esp", "Espツリー"),
                    ("pex", "Pex解析"),
                    ("quest", "クエスト一覧"),
                    ("npc", "NPC/音声リンク"),
                    ("log", "ログ"),
                ] {
                    button {
                        class: if active_tab() == id { "tab-item active" } else { "tab-item" },
                        onclick: move |_| active_tab.set(id.to_string()),
                        "{label}"
                    }
                }
            }
            div { class: "info-pane",
                if active_tab() == "home" {
                    if let Some(entry) = selected_entry {
                        div { class: "editor-box",
                            p { class: "editor-key", "Key: {entry.key}" }
                            label { "原文" }
                            textarea {
                                class: "detail-textarea",
                                value: "{edit_source}",
                                oninput: move |event| edit_source.set(event.value()),
                            }
                            label { "訳文" }
                            textarea {
                                class: "detail-textarea",
                                value: "{edit_target}",
                                oninput: move |event| edit_target.set(event.value()),
                            }
                            div { class: "editor-actions",
                                button {
                                    class: "tool-button",
                                    disabled: selected_key.is_none(),
                                    onclick: move |_| {
                                        let Some(key) = state.read().selected_key().map(|s| s.to_string()) else { return; };
                                        let updated = {
                                            let mut state = state.write();
                                            if state.update_entry(&key, &edit_source(), &edit_target()) {
                                                Some(state.entries().to_vec())
                                            } else {
                                                None
                                            }
                                        };
                                        if let Some(entries) = updated {
                                            history.write().apply(entries);
                                        }
                                    },
                                    "Apply Edit"
                                }
                                button {
                                    class: "tool-button",
                                    onclick: move |_| {
                                        let xml = export_entries(state.read().entries());
                                        xml_text.set(xml);
                                        xml_error.set(None);
                                    },
                                    "Export XML"
                                }
                                button {
                                    class: "tool-button",
                                    onclick: move |_| {
                                        match import_entries(&xml_text()) {
                                            Ok(entries) => {
                                                history.write().apply(entries.clone());
                                                state.write().set_entries(entries);
                                                xml_error.set(None);
                                                file_status.set("XML imported.".to_string());
                                            }
                                            Err(err) => {
                                                xml_error.set(Some(format!("Import error: {err:?}")));
                                            }
                                        }
                                    },
                                    "Import XML"
                                }
                                button {
                                    class: "tool-button",
                                    onclick: move |_| {
                                        let plugin = loaded_plugin().clone();
                                        let strings = loaded_strings().clone();
                                        match (plugin, strings) {
                                            (Some(plugin), Some(strings)) => {
                                                let hybrid = build_hybrid_entries(&plugin, &strings);
                                                hybrid_preview.set(hybrid);
                                                hybrid_error.set(None);
                                            }
                                            _ => {
                                                hybrid_error.set(Some("Load plugin and strings first.".to_string()));
                                            }
                                        }
                                    },
                                    "Build Hybrid"
                                }
                            }
                        }
                    } else {
                        p { "Select an entry from the grid." }
                    }
                } else if active_tab() == "log" {
                    div { class: "log-box",
                        if !file_status().is_empty() {
                            p { "{file_status}" }
                        }
                        if let Some(err) = xml_error() {
                            p { class: "tool-error", "{err}" }
                        }
                        if let Some(status) = diff_status() {
                            p { "Diff status: {status:?}" }
                        }
                        if !encoding_status().is_empty() {
                            p { "{encoding_status}" }
                        }
                        if !validation_issues().is_empty() {
                            for issue in validation_issues() {
                                p { "{issue.rule_id}: {issue.message}" }
                            }
                        }
                        if let Some(err) = hybrid_error() {
                            p { class: "tool-error", "{err}" }
                        }
                    }
                } else {
                    p { "このタブは次フェーズで実装します。" }
                }
                div { class: "loader-row",
                    label { class: "loader-item",
                        "Load XML"
                        input {
                            r#type: "file",
                            accept: ".xml",
                            onchange: move |event| async move {
                                let Some(file) = event.files().into_iter().next() else {
                                    return;
                                };
                                match file.read_string().await {
                                    Ok(contents) => {
                                        xml_text.set(contents);
                                        xml_error.set(None);
                                        file_status.set("XML loaded.".to_string());
                                    }
                                    Err(err) => file_status.set(format!("XML read error: {err}")),
                                }
                            },
                        }
                    }
                    label { class: "loader-item",
                        "Load Strings"
                        input {
                            r#type: "file",
                            accept: ".strings,.dlstrings,.ilstrings",
                            onchange: move |event| async move {
                                let Some(file) = event.files().into_iter().next() else {
                                    return;
                                };
                                let path = file.path();
                                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                                match file.read_bytes().await {
                                    Ok(bytes) => {
                                        let parsed = match ext.to_ascii_lowercase().as_str() {
                                            "strings" => read_strings(&bytes),
                                            "dlstrings" => read_dlstrings(&bytes),
                                            "ilstrings" => read_ilstrings(&bytes),
                                            _ => Err(xt_core::formats::strings::StringsError::InvalidHeader),
                                        };
                                        match parsed {
                                            Ok(strings) => {
                                                let entries = strings_to_entries(&strings);
                                                history.write().apply(entries.clone());
                                                state.write().set_entries(entries);
                                                loaded_strings.set(Some(strings));
                                                loaded_strings_path.set(Some(path.clone()));
                                                loaded_strings_kind.set(Some(StringsKind::from_extension(ext)));
                                                file_status.set("Strings loaded.".to_string());
                                            }
                                            Err(err) => file_status.set(format!("Strings parse error: {err:?}")),
                                        }
                                    }
                                    Err(err) => file_status.set(format!("Strings read error: {err}")),
                                }
                            },
                        }
                    }
                    label { class: "loader-item",
                        "Load Plugin"
                        input {
                            r#type: "file",
                            accept: ".esp,.esm,.esl,.xtplugin",
                            onchange: move |event| async move {
                                let Some(file) = event.files().into_iter().next() else {
                                    return;
                                };
                                let path = file.path();
                                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                                if ext.eq_ignore_ascii_case("xtplugin") {
                                    match file.read_string().await {
                                        Ok(contents) => match read_plugin(&contents) {
                                            Ok(plugin) => {
                                                loaded_plugin.set(Some(plugin));
                                                loaded_plugin_path.set(Some(path.clone()));
                                                loaded_esp_strings.set(None);
                                                file_status.set("Plugin loaded (xtplugin).".to_string());
                                            }
                                            Err(err) => file_status.set(format!("Plugin parse error: {err:?}")),
                                        },
                                        Err(err) => file_status.set(format!("Plugin read error: {err}")),
                                    }
                                } else {
                                    match file.read_bytes().await {
                                        Ok(bytes) => {
                                            let workspace_root = workspace_root_from_plugin(&path);
                                            let entries = match extract_esp_strings(&path, &workspace_root, Some("english")) {
                                                Ok(strings) => {
                                                    loaded_esp_strings.set(Some(strings.clone()));
                                                    loaded_plugin_path.set(Some(path.clone()));
                                                    strings_to_entries_from_extracted(&strings)
                                                }
                                                Err(err) => {
                                                    let fallback = extract_null_terminated_utf8(&bytes, 4)
                                                        .into_iter()
                                                        .map(|entry| Entry {
                                                            key: format!("plugin:{:08x}", entry.offset),
                                                            source_text: entry.text,
                                                            target_text: String::new(),
                                                        })
                                                        .collect::<Vec<_>>();
                                                    file_status.set(format!("ESP parse error (fallback to binary): {err}"));
                                                    fallback
                                                }
                                            };
                                            history.write().apply(entries.clone());
                                            state.write().set_entries(entries);
                                            loaded_plugin.set(None);
                                            file_status.set("Plugin loaded.".to_string());
                                        }
                                        Err(err) => file_status.set(format!("Plugin read error: {err}")),
                                    }
                                }
                            },
                        }
                    }
                }
                div { class: "loader-row",
                    label { class: "loader-item",
                        "Dict Src"
                        input {
                            value: "{dict_source_lang}",
                            oninput: move |e| dict_source_lang.set(e.value()),
                        }
                    }
                    label { class: "loader-item",
                        "Dict Dst"
                        input {
                            value: "{dict_target_lang}",
                            oninput: move |e| dict_target_lang.set(e.value()),
                        }
                    }
                    label { class: "loader-item dict-root",
                        "Dict Root"
                        input {
                            value: "{dict_root}",
                            oninput: move |e| dict_root.set(e.value()),
                        }
                    }
                    if !dictionary_status().is_empty() {
                        p { class: "status-inline", "{dictionary_status}" }
                    }
                }
                textarea {
                    class: "xml-textarea",
                    value: "{xml_text}",
                    oninput: move |event| xml_text.set(event.value()),
                }
            }
            div { class: "status-bar",
                div { class: "status-progress",
                    div { class: "status-progress-fill", style: "width: {translation_ratio}%" }
                }
                div { class: "status-text", "[{dict_source_lang}] -> [{dict_target_lang}]" }
                div { class: "status-text", "{file_status}" }
                div { class: "status-text", "{translated_count}/{total_count}" }
            }
        }
    }
}

#[derive(Clone)]
struct EntryView {
    key: String,
    source_text: String,
    target_text: String,
    is_selected: bool,
}

#[derive(Clone, Copy, PartialEq)]
enum SpacerPosition {
    Top,
    Bottom,
}

#[component]
fn Spacer(window: VirtualWindow, position: SpacerPosition) -> Element {
    let height = match position {
        SpacerPosition::Top => window.top_pad,
        SpacerPosition::Bottom => window.bottom_pad,
    };
    rsx! {
        div { class: "spacer", style: "height: {height}px;" }
    }
}

fn sample_entries() -> Vec<Entry> {
    let mut entries = Vec::with_capacity(10_000);
    for index in 0..10_000 {
        entries.push(Entry {
            key: format!("strings:skyrim:{index}"),
            source_text: format!("Source text {index}"),
            target_text: format!("訳文 {index}"),
        });
    }
    entries
}

fn strings_to_entries(strings: &StringsFile) -> Vec<Entry> {
    strings
        .entries
        .iter()
        .map(|entry| Entry {
            key: format!("strings:{}", entry.id),
            source_text: entry.text.clone(),
            target_text: String::new(),
        })
        .collect()
}

fn strings_to_entries_from_extracted(strings: &[ExtractedString]) -> Vec<Entry> {
    strings
        .iter()
        .map(|entry| Entry {
            key: entry.get_unique_key(),
            source_text: entry.text.clone(),
            target_text: String::new(),
        })
        .collect()
}

fn workspace_root_from_plugin(path: &std::path::Path) -> PathBuf {
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

fn row_fields(key: &str, target_text: &str) -> (String, String, String) {
    let edid = key
        .split(':')
        .next_back()
        .unwrap_or(key)
        .to_string();
    let rec_id = if key.to_ascii_lowercase().contains("plugin") {
        "REC FULL".to_string()
    } else {
        "WEAP FULL".to_string()
    };
    let ld = if target_text.is_empty() { "-" } else { "T" }.to_string();
    (edid, rec_id, ld)
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
            std::fs::write(&plugin_path, encoded).map_err(|e| e.to_string())?;
            return Ok(plugin_path);
        }
    }

    if let (Some(strings), Some(kind), Some(path)) =
        (loaded_strings, loaded_strings_kind, loaded_strings_path)
    {
        return save_strings(entries, &strings, kind, &path);
    }
    Err("no loaded file to save".to_string())
}

fn save_as_translated(
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
            let out_dir = plugin_path
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join("translated_output");
            return save_esp(entries, &plugin_path, &out_dir.join(file_name_or_default(&plugin_path)), extracted);
        }
        if let Some(plugin) = loaded_plugin {
            let out_path = with_suffix_path(&plugin_path, "_translated");
            let encoded = write_plugin(&plugin).map_err(|e| format!("{e:?}"))?;
            std::fs::write(&out_path, encoded).map_err(|e| e.to_string())?;
            return Ok(out_path);
        }
    }

    if let (Some(strings), Some(kind), Some(path)) =
        (loaded_strings, loaded_strings_kind, loaded_strings_path)
    {
        let out_path = with_suffix_path(&path, "_translated");
        return save_strings(entries, &strings, kind, &out_path);
    }
    Err("no loaded file to save".to_string())
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
    std::fs::write(path, bytes).map_err(|e| e.to_string())?;
    Ok(path.to_path_buf())
}

fn save_esp(
    entries: &[Entry],
    input_path: &Path,
    output_path: &Path,
    extracted: Vec<ExtractedString>,
) -> Result<PathBuf, String> {
    if input_path.exists() && input_path == output_path {
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
        if let Some(target) = targets.get(item.get_unique_key().as_str()) {
            item.text = (*target).to_string();
        }
    }
    let output_dir = output_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();
    let workspace_root = workspace_root_from_plugin(input_path);
    let written = apply_translations(
        input_path,
        &workspace_root,
        &output_dir,
        translated,
        Some("english"),
    )
    .map_err(|e| format!("esp write failed ({}): {e}", input_path.display()))?;
    if written == output_path {
        return Ok(written);
    }
    std::fs::copy(&written, output_path).map_err(|e| {
        format!(
            "esp rename failed ({} -> {}): {e}",
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
    let updated = base
        .entries
        .iter()
        .map(|entry| {
            if let Some(target) = by_id.get(&entry.id) {
                xt_core::formats::strings::StringsEntry {
                    id: entry.id,
                    text: (*target).to_string(),
                }
            } else {
                entry.clone()
            }
        })
        .collect::<Vec<_>>();
    StringsFile { entries: updated }
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
            "backup failed ({} -> {}): {e}",
            path.display(),
            backup.display()
        )
    })?;
    Ok(())
}

fn with_suffix_path(path: &Path, suffix: &str) -> PathBuf {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
    let name = if ext.is_empty() {
        format!("{stem}{suffix}")
    } else {
        format!("{stem}{suffix}.{ext}")
    };
    path.parent().unwrap_or_else(|| Path::new(".")).join(name)
}

fn next_backup_path(path: &Path) -> PathBuf {
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    for index in 0usize..1000usize {
        let name = if index == 0 {
            if ext.is_empty() {
                format!("{stem}.bak")
            } else {
                format!("{stem}.bak.{ext}")
            }
        } else if ext.is_empty() {
            format!("{stem}.bak{index}")
        } else {
            format!("{stem}.bak{index}.{ext}")
        };
        let candidate = parent.join(name);
        if !candidate.exists() {
            return candidate;
        }
    }
    with_suffix_path(path, ".bak999")
}

fn file_name_or_default(path: &Path) -> String {
    path.file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("plugin.esp")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use xt_core::formats::strings::StringsEntry;

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
}
