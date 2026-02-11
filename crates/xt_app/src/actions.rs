use std::collections::HashMap;
use std::path::{Path, PathBuf};

use xt_core::dictionary::TranslationDictionary;
use xt_core::diff::{update_source, DiffEntry};
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
use xt_core::hybrid::build_hybrid_entries;
use xt_core::import_export::{apply_xml_default, export_entries, import_entries, XmlApplyStats};
use xt_core::model::Entry;
use xt_core::validation::{
    validate_alias_tags, validate_braced_placeholders, validate_printf_placeholders,
};

use crate::state::{AppState, StringsKind, Tab};

pub enum AppAction {
    SetQuery(String),
    SelectEntry(String),
    SetEditSource(String),
    SetEditTarget(String),
    SetXmlText(String),
    ExportXmlToEditor,
    ApplyXmlFromEditor,
    LoadXml(PathBuf),
    LoadStrings(PathBuf),
    LoadPlugin(PathBuf),
    ApplyEdit,
    BuildHybrid,
    BuildDictionary,
    QuickAuto,
    Validate,
    DiffCheck,
    EncodingCheck,
    SetDictSourceLang(String),
    SetDictTargetLang(String),
    SetDictRoot(String),
    ResetDictLanguagePair,
    Undo,
    Redo,
    SetActiveTab(Tab),
    SaveOverwrite,
    SaveAsAuto,
    SaveAsPath(PathBuf),
}

pub fn dispatch(state: &mut AppState, action: AppAction) -> Result<(), String> {
    match action {
        AppAction::SetQuery(query) => {
            state.set_query(&query);
        }
        AppAction::SelectEntry(key) => {
            state.select(&key);
        }
        AppAction::SetEditSource(value) => {
            state.edit_source = value;
        }
        AppAction::SetEditTarget(value) => {
            state.edit_target = value;
        }
        AppAction::SetXmlText(value) => {
            state.xml_text = value;
        }
        AppAction::ExportXmlToEditor => {
            state.xml_text = export_entries(state.entries());
            state.xml_error = None;
            state.file_status = "XMLを書き出しました（エディタ）".to_string();
        }
        AppAction::ApplyXmlFromEditor => {
            apply_xml_to_current(state, state.xml_text.clone())?;
        }
        AppAction::LoadXml(path) => {
            let contents = std::fs::read_to_string(&path)
                .map_err(|err| format!("read {}: {err}", path.display()))?;
            apply_xml_to_current(state, contents)?;
            state.file_status = format!("XML適用: {}", path.display());
        }
        AppAction::LoadStrings(path) => {
            load_strings_from_path(state, &path)?;
        }
        AppAction::LoadPlugin(path) => {
            load_plugin_from_path(state, &path)?;
        }
        AppAction::ApplyEdit => {
            let Some(key) = state.selected_key() else {
                return Ok(());
            };
            let source = state.edit_source.clone();
            let target = state.edit_target.clone();
            if state.update_entry(&key, &source, &target) {
                let entries = state.entries().to_vec();
                state.history.apply(entries);
                state.file_status = "編集を反映しました".to_string();
            }
        }
        AppAction::BuildHybrid => {
            let p = state.loaded_plugin.clone();
            let s = state.loaded_strings.clone();
            match (p, s) {
                (Some(plugin), Some(strings)) => {
                    state.hybrid_preview = build_hybrid_entries(&plugin, &strings);
                    state.hybrid_error = None;
                }
                _ => {
                    state.hybrid_error = Some("Plugin/Stringsを先に読み込んでください".to_string());
                }
            }
        }
        AppAction::BuildDictionary => {
            let root = PathBuf::from(&state.dict_root);
            match TranslationDictionary::build_from_strings_dir(
                &root,
                &state.dict_source_lang,
                &state.dict_target_lang,
            ) {
                Ok((built, stats)) => {
                    let pairs = built.len();
                    state.dict = Some(built);
                    state.mark_dictionary_built(pairs, stats.files_seen, stats.file_pairs);
                    state.dict_status = format!(
                        "辞書構築: pairs={} files={} pair_files={}",
                        pairs, stats.files_seen, stats.file_pairs
                    );
                }
                Err(err) => {
                    state.dict_status = format!("辞書構築失敗: {err}");
                    return Err(state.dict_status.clone());
                }
            }
        }
        AppAction::QuickAuto => {
            let selected = state.selected_key();
            let entries = state.entries().to_vec();
            let result = {
                let current = state.dict.as_ref();
                apply_quick_auto_selection(current, &entries, selected)
            };
            match result {
                Ok((next, updated)) => {
                    if updated > 0 {
                        state.history.apply(next.clone());
                        state.set_entries_without_history(next);
                    }
                    state.dict_status = format!("Quick自動翻訳: updated={updated}");
                }
                Err(err) => {
                    state.dict_status = err.to_string();
                    return Err(err.to_string());
                }
            }
        }
        AppAction::Validate => {
            let Some(entry) = state.selected_entry() else {
                state.validation_issues.clear();
                return Ok(());
            };
            let mut issues = Vec::new();
            issues.extend(validate_braced_placeholders(
                &entry.key,
                &state.edit_source,
                &state.edit_target,
            ));
            issues.extend(validate_printf_placeholders(
                &entry.key,
                &state.edit_source,
                &state.edit_target,
            ));
            issues.extend(validate_alias_tags(
                &entry.key,
                &state.edit_source,
                &state.edit_target,
            ));
            state.validation_issues = issues;
        }
        AppAction::DiffCheck => {
            let Some(entry) = state.selected_entry() else {
                state.diff_status = None;
                return Ok(());
            };
            let mut diff = DiffEntry::new(&entry.key, &entry.source_text, &entry.target_text);
            update_source(&mut diff, &state.edit_source);
            state.diff_status = Some(diff.status);
        }
        AppAction::EncodingCheck => {
            state.encoding_status = match encode(&state.edit_target, Encoding::Latin1)
                .and_then(|bytes| decode(&bytes, Encoding::Latin1))
            {
                Ok(_) => "Latin1 OK".to_string(),
                Err(EncodingError::UnrepresentableChar) => {
                    "Latin1 error: unrepresentable".to_string()
                }
                Err(EncodingError::InvalidUtf8) => "Latin1 error: invalid utf8".to_string(),
            };
        }
        AppAction::SetDictSourceLang(value) => {
            state.dict_source_lang = value;
            state.persist_dictionary_prefs();
        }
        AppAction::SetDictTargetLang(value) => {
            state.dict_target_lang = value;
            state.persist_dictionary_prefs();
        }
        AppAction::SetDictRoot(value) => {
            state.dict_root = value;
            state.persist_dictionary_prefs();
        }
        AppAction::ResetDictLanguagePair => {
            state.reset_dictionary_lang_pair();
        }
        AppAction::Undo => {
            state.undo();
        }
        AppAction::Redo => {
            state.redo();
        }
        AppAction::SetActiveTab(tab) => {
            state.active_tab = tab;
        }
        AppAction::SaveOverwrite => {
            let path = save_overwrite(
                state.entries(),
                state.loaded_strings.clone(),
                state.loaded_strings_kind,
                state.loaded_strings_path.clone(),
                state.loaded_plugin.clone(),
                state.loaded_plugin_path.clone(),
                state.loaded_esp_strings.clone(),
            )?;
            state.file_status = format!("保存: {}", path.display());
        }
        AppAction::SaveAsAuto => {
            let path = save_as(
                state.entries(),
                state.loaded_strings.clone(),
                state.loaded_strings_kind,
                state.loaded_strings_path.clone(),
                state.loaded_plugin.clone(),
                state.loaded_plugin_path.clone(),
                state.loaded_esp_strings.clone(),
                None,
            )?;
            state.file_status = format!("別名保存: {}", path.display());
        }
        AppAction::SaveAsPath(path) => {
            let path = save_as(
                state.entries(),
                state.loaded_strings.clone(),
                state.loaded_strings_kind,
                state.loaded_strings_path.clone(),
                state.loaded_plugin.clone(),
                state.loaded_plugin_path.clone(),
                state.loaded_esp_strings.clone(),
                Some(path),
            )?;
            state.file_status = format!("別名保存: {}", path.display());
        }
    }

    Ok(())
}

fn load_strings_from_path(state: &mut AppState, path: &Path) -> Result<(), String> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let Some(kind) = StringsKind::from_extension(ext) else {
        let msg = format!("unsupported strings extension: {ext}");
        state.file_status = msg.clone();
        return Err(msg);
    };

    let bytes = std::fs::read(path).map_err(|err| format!("Strings read error: {err}"))?;
    let parsed = match kind {
        StringsKind::Strings => read_strings(&bytes),
        StringsKind::DlStrings => read_dlstrings(&bytes),
        StringsKind::IlStrings => read_ilstrings(&bytes),
    }
    .map_err(|err| format!("Strings parse error: {err:?}"))?;

    let entries = parsed
        .entries
        .iter()
        .map(|e| Entry {
            key: format!("strings:{}", e.id),
            source_text: e.text.clone(),
            target_text: String::new(),
        })
        .collect::<Vec<_>>();

    state.set_entries_with_history(entries);
    state.loaded_strings = Some(parsed);
    state.loaded_strings_kind = Some(kind);
    state.loaded_strings_path = Some(path.to_path_buf());

    state.loaded_plugin = None;
    state.loaded_plugin_path = None;
    state.loaded_esp_strings = None;

    state.file_status = "Stringsを読み込みました".to_string();
    Ok(())
}

fn load_plugin_from_path(state: &mut AppState, path: &Path) -> Result<(), String> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    if ext == "xtplugin" {
        let content =
            std::fs::read_to_string(path).map_err(|err| format!("xtplugin read error: {err}"))?;
        let plugin =
            read_plugin(&content).map_err(|err| format!("xtplugin parse error: {err:?}"))?;

        let entries = plugin
            .entries
            .iter()
            .map(|e| Entry {
                key: format!("plugin:{}", e.id),
                source_text: e.source_text.clone(),
                target_text: String::new(),
            })
            .collect::<Vec<_>>();

        state.set_entries_with_history(entries);
        state.loaded_plugin = Some(plugin);
        state.loaded_plugin_path = Some(path.to_path_buf());
        state.loaded_esp_strings = None;
        state.loaded_strings = None;
        state.loaded_strings_kind = None;
        state.loaded_strings_path = None;
        state.file_status = "xtpluginを読み込みました".to_string();
        return Ok(());
    }

    let bytes = std::fs::read(path).map_err(|err| format!("plugin read error: {err}"))?;
    let workspace_root = workspace_root_from_plugin(path);
    let entries = match extract_esp_strings(path, &workspace_root, Some("english")) {
        Ok(strings) => {
            state.loaded_esp_strings = Some(strings.clone());
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
            state.file_status = format!("ESP parse error (fallback): {err}");
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

    state.set_entries_with_history(entries);
    state.loaded_plugin = None;
    state.loaded_plugin_path = Some(path.to_path_buf());
    state.loaded_strings = None;
    state.loaded_strings_kind = None;
    state.loaded_strings_path = None;
    state.file_status = "Pluginを読み込みました".to_string();
    Ok(())
}

fn apply_xml_to_current(state: &mut AppState, contents: String) -> Result<(), String> {
    state.xml_text = contents.clone();
    let current_entries = state.entries().to_vec();
    let (merged, stats) = apply_xml_payload(&current_entries, &contents)?;
    if stats.updated > 0 {
        state.history.apply(merged.clone());
        state.set_entries_without_history(merged);
    }
    state.file_status = format!(
        "XML適用: updated={} unchanged={} missing={}",
        stats.updated, stats.unchanged, stats.missing
    );
    state.last_xml_stats = Some(stats);
    state.xml_error = None;
    Ok(())
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
    output_override: Option<PathBuf>,
) -> Result<PathBuf, String> {
    if let Some(plugin_path) = loaded_plugin_path {
        if let Some(extracted) = loaded_esp_strings {
            let out =
                output_override.unwrap_or_else(|| with_suffix_path(&plugin_path, "_translated"));
            return save_esp(entries, &plugin_path, &out, extracted);
        }
        if let Some(plugin) = loaded_plugin {
            let out =
                output_override.unwrap_or_else(|| with_suffix_path(&plugin_path, "_translated"));
            let encoded = write_plugin(&plugin).map_err(|e| format!("{e:?}"))?;
            std::fs::write(&out, encoded)
                .map_err(|e| format!("plugin save {}: {e}", out.display()))?;
            return Ok(out);
        }
    }

    if let (Some(strings), Some(kind), Some(path)) =
        (loaded_strings, loaded_strings_kind, loaded_strings_path)
    {
        let out = output_override.unwrap_or_else(|| with_suffix_path(&path, "_translated"));
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
