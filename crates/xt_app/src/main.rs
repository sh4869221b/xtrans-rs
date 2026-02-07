use xt_core::diff::{update_source, DiffEntry, EntryStatus};
use xt_core::encoding::{decode, encode, Encoding, EncodingError};
use xt_core::formats::plugin::{read_plugin, PluginFile};
use xt_core::formats::esp::extract_strings as extract_esp_strings;
use xt_core::formats::plugin_binary::extract_null_terminated_utf8;
use xt_core::formats::strings::{read_dlstrings, read_ilstrings, read_strings, StringsFile};
use xt_core::hybrid::{build_hybrid_entries, HybridEntry};
use xt_core::import_export::{export_entries, import_entries};
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
use std::path::PathBuf;

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

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
    let mut file_status = use_signal(|| String::new());
    let mut loaded_esp_strings = use_signal(|| Option::<Vec<ExtractedString>>::None);
    let item_height = 64.0f32;
    let overscan = 8usize;
    let list_padding = 12.0f32;

    let (window, entries, selected_key, selected_entry, query) = {
        let state = state.read();
        let filtered = state.filtered_entries();
        let total = filtered.len();
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
        (window, entries, selected_key, selected_entry, query)
    };

    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }

        div { id: "app",
            div { id: "pane-left",
                h2 { "Entries" }
                input {
                    id: "search",
                    placeholder: "Search source/target...",
                    value: "{query}",
                    oninput: move |event| {
                        state.write().set_query(&event.value());
                        scroll_offset.set(0.0);
                    },
                }
                div { id: "entry-list",
                    onscroll: move |event| {
                        let data = &event.data;
                        let offset = (data.scroll_top() as f32 - list_padding).max(0.0);
                        scroll_offset.set(offset);
                        viewport_height.set(data.client_height() as f32);
                    },
                    Spacer { window: window, position: SpacerPosition::Top }
                    for EntryView { key, source_text, target_text, is_selected } in entries {
                        button {
                            class: if is_selected { "entry-button selected" } else { "entry-button" },
                            onclick: move |_| {
                                state.write().select(&key);
                                edit_source.set(source_text.clone());
                                edit_target.set(target_text.clone());
                            },
                            span { class: "entry-source", "{source_text}" }
                            span { class: "entry-target", "{target_text}" }
                        }
                    }
                    Spacer { window: window, position: SpacerPosition::Bottom }
                }
            }
            div { id: "pane-right",
                h2 { "Detail" }
                if let Some(entry) = selected_entry {
                    div { id: "detail",
                        p { class: "detail-key", "Key: {entry.key}" }
                        label { class: "detail-label", "Source" }
                        textarea {
                            class: "detail-textarea",
                            value: "{edit_source}",
                            oninput: move |event| edit_source.set(event.value()),
                        }
                        label { class: "detail-label", "Target" }
                        textarea {
                            class: "detail-textarea",
                            value: "{edit_target}",
                            oninput: move |event| edit_target.set(event.value()),
                        }
                        div { class: "detail-actions",
                            button {
                                class: "action-button",
                                disabled: selected_key.is_none(),
                                onclick: move |_| {
                                    let Some(key) = selected_key.clone() else { return; };
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
                                class: "action-button",
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
                                class: "action-button",
                                onclick: move |_| {
                                    let Some(entry) = state.read().selected_entry().cloned() else {
                                        diff_status.set(None);
                                        return;
                                    };
                                    let mut diff_entry = DiffEntry::new(&entry.key, &entry.source_text, &entry.target_text);
                                    update_source(&mut diff_entry, &edit_source());
                                    diff_status.set(Some(diff_entry.status));
                                },
                                "Diff Check"
                            }
                            button {
                                class: "action-button",
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
                        }
                        if !validation_issues().is_empty() {
                            div { class: "detail-issues",
                                h3 { "Validation Issues" }
                                for issue in validation_issues() {
                                    p { "{issue.rule_id}: {issue.message}" }
                                }
                            }
                        }
                        if let Some(status) = diff_status() {
                            p { class: "detail-status", "Diff status: {status:?}" }
                        }
                        if !encoding_status().is_empty() {
                            p { class: "detail-status", "{encoding_status}" }
                        }
                    }
                } else {
                    p { class: "detail-empty", "Select an entry from the list." }
                }
                div { id: "tools",
                    h2 { "Tools" }
                    div { class: "tool-actions",
                        button {
                            class: "action-button",
                            onclick: move |_| {
                                let xml = export_entries(state.read().entries());
                                xml_text.set(xml);
                                xml_error.set(None);
                            },
                            "Export XML"
                        }
                        button {
                            class: "action-button",
                            onclick: move |_| {
                                match import_entries(&xml_text()) {
                                    Ok(entries) => {
                                        history.write().apply(entries.clone());
                                        state.write().set_entries(entries);
                                        xml_error.set(None);
                                    }
                                    Err(err) => {
                                        xml_error.set(Some(format!("Import error: {err:?}")));
                                    }
                                }
                            },
                            "Import XML"
                        }
                        button {
                            class: "action-button",
                            onclick: move |_| {
                                if history.write().undo() {
                                    let entries = history.read().present().clone();
                                    state.write().set_entries(entries);
                                }
                            },
                            "Undo"
                        }
                        button {
                            class: "action-button",
                            onclick: move |_| {
                                if history.write().redo() {
                                    let entries = history.read().present().clone();
                                    state.write().set_entries(entries);
                                }
                            },
                            "Redo"
                        }
                    }
                    div { class: "tool-loaders",
                        div { class: "tool-loader",
                            label { "Load XML" }
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
                                        Err(err) => {
                                            file_status.set(format!("XML read error: {err}"));
                                        }
                                    }
                                },
                            }
                        }
                        div { class: "tool-loader",
                            label { "Load Strings" }
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
                                                    file_status.set("Strings loaded.".to_string());
                                                }
                                                Err(err) => {
                                                    file_status.set(format!("Strings parse error: {err:?}"));
                                                }
                                            }
                                        }
                                        Err(err) => {
                                            file_status.set(format!("Strings read error: {err}"));
                                        }
                                    }
                                },
                            }
                        }
                        div { class: "tool-loader",
                            label { "Load Plugin (ESP/ESM/ESL)" }
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
                                                    file_status.set("Plugin loaded (xtplugin).".to_string());
                                                }
                                                Err(err) => {
                                                    file_status.set(format!("Plugin parse error: {err:?}"));
                                                }
                                            },
                                            Err(err) => {
                                                file_status.set(format!("Plugin read error: {err}"));
                                            }
                                        }
                                    } else {
                                        match file.read_bytes().await {
                                            Ok(bytes) => {
                                                let workspace_root = workspace_root_from_plugin(&path);
                                                let entries = match extract_esp_strings(&path, &workspace_root, Some("english")) {
                                                    Ok(strings) => {
                                                        loaded_esp_strings.set(Some(strings.clone()));
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
                                            Err(err) => {
                                                file_status.set(format!("Plugin read error: {err}"));
                                            }
                                        }
                                    }
                                },
                            }
                        }
                        button {
                            class: "action-button",
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
                    textarea {
                        class: "xml-textarea",
                        value: "{xml_text}",
                        oninput: move |event| xml_text.set(event.value()),
                    }
                    if !file_status().is_empty() {
                        p { class: "tool-status", "{file_status}" }
                    }
                    if let Some(err) = xml_error() {
                        p { class: "tool-error", "{err}" }
                    }
                    if !hybrid_preview().is_empty() {
                        div { class: "tool-hybrid",
                            h3 { "Hybrid Preview" }
                            for entry in hybrid_preview() {
                                p { "#{entry.id} {entry.context} -> {entry.target_text}" }
                            }
                        }
                    }
                    if let Some(err) = hybrid_error() {
                        p { class: "tool-error", "{err}" }
                    }
                }
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
