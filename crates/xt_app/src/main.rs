use dioxus::prelude::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use xt_core::dictionary::TranslationDictionary;
use xt_core::diff::{update_source, DiffEntry, EntryStatus};
use xt_core::encoding::{decode, encode, Encoding, EncodingError};
use xt_core::formats::esp::{apply_translations, extract_strings as extract_esp_strings, ExtractedString};
use xt_core::formats::plugin::{read_plugin, write_plugin, PluginFile};
use xt_core::formats::plugin_binary::extract_null_terminated_utf8;
use xt_core::formats::strings::{
    read_dlstrings, read_ilstrings, read_strings, write_dlstrings, write_ilstrings, write_strings,
    StringsEntry, StringsFile,
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

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

const MENU_XML_EXPORT: &str = "file.xml_export";
const MENU_XML_APPLY: &str = "file.xml_apply";
const MENU_SAVE_OVERWRITE: &str = "file.save_overwrite";
const MENU_SAVE_AS: &str = "file.save_as";
const MENU_DICT_BUILD: &str = "translate.dict_build";
const MENU_QUICK_AUTO: &str = "translate.quick_auto";
const MENU_LANG_PANEL: &str = "options.lang_panel";
const MENU_LANG_RESET: &str = "options.lang_reset";
const MENU_UNDO: &str = "tools.undo";
const MENU_REDO: &str = "tools.redo";
const MENU_LOG_TAB: &str = "tools.log_tab";

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

#[derive(Clone, Copy, PartialEq)]
enum SpacerPosition {
    Top,
    Bottom,
}

fn main() {
    #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
    {
        use dioxus::desktop::Config;
        dioxus::LaunchBuilder::new()
            .with_cfg(Config::new().with_menu(build_native_menu()))
            .launch(App);
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let mut history = use_signal(|| UndoStack::new(sample_entries()));
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

    let mut dict = use_signal(|| Option::<TranslationDictionary>::None);
    let mut dict_source_lang = use_signal(|| "english".to_string());
    let mut dict_target_lang = use_signal(|| "japanese".to_string());
    let mut dict_root = use_signal(|| "./Data/Strings/Translations".to_string());
    let mut dict_status = use_signal(String::new);

    let mut active_tab = use_signal(|| Tab::Home);

    #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
    {
        use dioxus::desktop::use_muda_event_handler;
        use_muda_event_handler(move |event| match event.id.as_ref() {
            MENU_XML_EXPORT => {
                xml_text.set(export_entries(state.read().entries()));
                xml_error.set(None);
                file_status.set("XMLを書き出しました（エディタ）".to_string());
            }
            MENU_XML_APPLY => match import_entries(&xml_text()) {
                Ok(imported) => {
                    let (merged, stats) = apply_xml_default(state.read().entries(), &imported);
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
                Err(err) => xml_error.set(Some(format!("XML import error: {err:?}"))),
            },
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
                        dict_status.set(format!(
                            "辞書構築: pairs={} files={} pair_files={}",
                            pairs, stats.files_seen, stats.file_pairs
                        ));
                    }
                    Err(err) => dict_status.set(format!("辞書構築失敗: {err}")),
                }
            }
            MENU_QUICK_AUTO => {
                let Some(d) = dict() else {
                    dict_status.set("辞書未構築".to_string());
                    return;
                };
                let selected = state
                    .read()
                    .selected_key()
                    .map(|s| s.to_string())
                    .into_iter()
                    .collect::<Vec<_>>();
                let (next, updated) = d.apply_quick(state.read().entries(), &selected, true);
                if updated > 0 {
                    history.write().apply(next.clone());
                    state.write().set_entries(next);
                }
                dict_status.set(format!("Quick自動翻訳: updated={updated}"));
            }
            MENU_LANG_PANEL => active_tab.set(Tab::Lang),
            MENU_LANG_RESET => {
                dict_source_lang.set("english".to_string());
                dict_target_lang.set("japanese".to_string());
                dict_status.set("言語ペアを english -> japanese に設定".to_string());
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

    let selected_key = state.read().selected_key().map(|s| s.to_string());
    let selected_entry = state.read().selected_entry().cloned();
    let query = state.read().query().to_string();

    let filtered = state.read().filtered_entries().to_vec();
    let window = virtual_window(
        filtered.len(),
        item_height,
        *viewport_height.read(),
        *scroll_offset.read(),
        overscan,
    );
    let rows = filtered
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
        .collect::<Vec<_>>();

    let counts = count_channels(&filtered);
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
                        scroll_offset.set(data.scroll_top() as f32);
                        viewport_height.set(data.client_height() as f32);
                    },
                    Spacer { window: window, position: SpacerPosition::Top }
                    for row in rows {
                        button {
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
                    label { class: "io",
                        "Load XML"
                        input {
                            r#type: "file",
                            accept: ".xml",
                            onchange: move |event| async move {
                                let Some(file) = event.files().into_iter().next() else { return; };
                                match file.read_string().await {
                                    Ok(contents) => {
                                        xml_text.set(contents);
                                        xml_error.set(None);
                                        file_status.set("XMLを読み込みました".to_string());
                                    }
                                    Err(err) => file_status.set(format!("XML read error: {err}")),
                                }
                            },
                        }
                    }
                    label { class: "io",
                        "Load Strings"
                        input {
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
                    }
                    label { class: "io",
                        "Load Plugin"
                        input {
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
                }

                textarea {
                    class: "xml",
                    value: "{xml_text}",
                    oninput: move |e| xml_text.set(e.value()),
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

#[derive(Default)]
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

#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
fn build_native_menu() -> dioxus::desktop::muda::Menu {
    use dioxus::desktop::muda::{Menu, MenuItem, PredefinedMenuItem, Submenu};

    let menu = Menu::new();

    let file_menu = Submenu::new("ファイル(F)", true);
    let xml_export = MenuItem::with_id(MENU_XML_EXPORT, "翻訳XMLを書き出し", true, None);
    let xml_apply = MenuItem::with_id(MENU_XML_APPLY, "翻訳XMLを一括適用", true, None);
    let save = MenuItem::with_id(MENU_SAVE_OVERWRITE, "上書き保存", true, None);
    let save_as = MenuItem::with_id(MENU_SAVE_AS, "別名保存", true, None);
    let sep_file_1 = PredefinedMenuItem::separator();
    let sep_file_2 = PredefinedMenuItem::separator();
    let quit = PredefinedMenuItem::quit(None);
    let _ = file_menu.append_items(&[
        &xml_export,
        &xml_apply,
        &sep_file_1,
        &save,
        &save_as,
        &sep_file_2,
        &quit,
    ]);

    let translate_menu = Submenu::new("翻訳(T)", true);
    let dict_build = MenuItem::with_id(MENU_DICT_BUILD, "辞書を構築", true, None);
    let quick_auto = MenuItem::with_id(MENU_QUICK_AUTO, "Quick自動翻訳", true, None);
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
            std::fs::write(&out, encoded).map_err(|e| format!("plugin save {}: {e}", out.display()))?;
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

fn save_strings(entries: &[Entry], base: &StringsFile, kind: StringsKind, path: &Path) -> Result<PathBuf, String> {
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

fn save_esp(entries: &[Entry], input_path: &Path, output_path: &Path, extracted: Vec<ExtractedString>) -> Result<PathBuf, String> {
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
    let written = apply_translations(input_path, &workspace_root, out_dir, translated, Some("english"))
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
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("output");
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
}
