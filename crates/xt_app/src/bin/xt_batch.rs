use std::collections::HashMap;
use std::path::{Path, PathBuf};
use xt_core::dictionary::TranslationDictionary;
use xt_core::formats::esp::{apply_translations, extract_strings, ExtractedString};
use xt_core::formats::strings::{
    read_dlstrings, read_ilstrings, read_strings, write_dlstrings, write_ilstrings, write_strings,
    StringsEntry, StringsFile,
};
use xt_core::import_export::{apply_xml_default, export_entries, import_entries};
use xt_core::model::Entry;

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let opts = parse_args(&args)?;

    if let Some(dir) = opts.generate_dictionary.clone() {
        let source = opts.source.clone().unwrap_or_else(|| "english".to_string());
        let target = opts
            .target
            .clone()
            .unwrap_or_else(|| "japanese".to_string());
        let out = opts
            .dict_out
            .clone()
            .ok_or_else(|| "--dict-out is required with --generate-dictionary".to_string())?;
        let (dict, stats) = TranslationDictionary::build_from_strings_dir(&dir, &source, &target)
            .map_err(|e| e.to_string())?;
        dict.save_to_path(&out).map_err(|e| e.to_string())?;
        println!(
            "generated dictionary: pairs={} files_seen={} file_pairs={} out={}",
            dict.len(),
            stats.files_seen,
            stats.file_pairs,
            out.display()
        );
        return Ok(());
    }

    let import_xml = opts
        .importxml
        .clone()
        .ok_or_else(|| "--importxml <translation.xml> is required".to_string())?;
    let finalize = opts
        .finalize
        .clone()
        .ok_or_else(|| "--finalize <output> is required".to_string())?;

    let (base_entries, base_kind) = load_base(&opts)?;
    let trans_xml = std::fs::read_to_string(&import_xml)
        .map_err(|e| format!("read {}: {e}", import_xml.display()))?;
    let imported = import_entries(&trans_xml).map_err(|e| format!("parse import xml: {e:?}"))?;
    let (mut merged, stats) = apply_xml_default(&base_entries, &imported);
    println!(
        "xml apply: updated={} unchanged={} missing={}",
        stats.updated, stats.unchanged, stats.missing
    );

    let mut dict_updated = 0usize;
    if let Some(dict_path) = opts.dict_in.clone() {
        let dict = TranslationDictionary::load_from_path(&dict_path).map_err(|e| e.to_string())?;
        let all_keys = merged.iter().map(|e| e.key.clone()).collect::<Vec<_>>();
        let (next, updated) = dict.apply_quick(&merged, &all_keys, true);
        merged = next;
        dict_updated = updated;
        println!("quick auto-translate applied: updated={dict_updated}");
    }

    if let Some(dict_out) = opts.dict_out.clone() {
        let dict = TranslationDictionary::build_from_entries(&merged);
        dict.save_to_path(&dict_out).map_err(|e| e.to_string())?;
        println!(
            "saved dictionary: pairs={} out={}",
            dict.len(),
            dict_out.display()
        );
    }

    finalize_output(&base_kind, &merged, &finalize, &opts)?;
    println!(
        "finalized: xml_updated={} xml_unchanged={} xml_missing={} dict_updated={} out={}",
        stats.updated,
        stats.unchanged,
        stats.missing,
        dict_updated,
        finalize.display()
    );
    Ok(())
}

#[derive(Clone)]
enum BaseKind {
    Xml,
    Strings {
        base: StringsFile,
        kind: StringsKindCli,
    },
    Esp {
        input_path: PathBuf,
        extracted: Vec<ExtractedString>,
        workspace_root: PathBuf,
    },
}

fn load_base(opts: &BatchOptions) -> Result<(Vec<Entry>, BaseKind), String> {
    let mut count = 0usize;
    if opts.load.is_some() {
        count += 1;
    }
    if opts.load_strings.is_some() {
        count += 1;
    }
    if opts.load_plugin.is_some() {
        count += 1;
    }
    if count != 1 {
        return Err("exactly one of --load, --load-strings, --load-plugin is required".to_string());
    }

    if let Some(path) = opts.load.clone() {
        let xml =
            std::fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
        let entries = import_entries(&xml).map_err(|e| format!("parse base xml: {e:?}"))?;
        return Ok((entries, BaseKind::Xml));
    }

    if let Some(path) = opts.load_strings.clone() {
        let kind = StringsKindCli::from_path(&path)?;
        let bytes = std::fs::read(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
        let base = match kind {
            StringsKindCli::Strings => read_strings(&bytes),
            StringsKindCli::DlStrings => read_dlstrings(&bytes),
            StringsKindCli::IlStrings => read_ilstrings(&bytes),
        }
        .map_err(|e| format!("parse strings {}: {e:?}", path.display()))?;
        let entries = base
            .entries
            .iter()
            .map(|entry| Entry {
                key: format!("strings:{}", entry.id),
                source_text: entry.text.clone(),
                target_text: String::new(),
            })
            .collect::<Vec<_>>();
        return Ok((entries, BaseKind::Strings { base, kind }));
    }

    let path = opts
        .load_plugin
        .clone()
        .ok_or_else(|| "--load-plugin required".to_string())?;
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !matches!(ext.as_str(), "esp" | "esm" | "esl") {
        return Err("load-plugin supports only .esp/.esm/.esl".to_string());
    }
    let workspace_root = opts
        .workspace_root
        .clone()
        .unwrap_or_else(|| workspace_root_from_plugin(&path));
    let extracted = extract_strings(&path, &workspace_root, Some("english"))
        .map_err(|e| format!("extract strings {}: {e}", path.display()))?;
    let entries = extracted
        .iter()
        .map(|entry| Entry {
            key: entry.get_unique_key(),
            source_text: entry.text.clone(),
            target_text: String::new(),
        })
        .collect::<Vec<_>>();
    Ok((
        entries,
        BaseKind::Esp {
            input_path: path,
            extracted,
            workspace_root,
        },
    ))
}

fn finalize_output(
    base: &BaseKind,
    entries: &[Entry],
    finalize: &Path,
    _opts: &BatchOptions,
) -> Result<(), String> {
    if let Some(parent) = finalize.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("create {}: {e}", parent.display()))?;
    }
    match base {
        BaseKind::Xml => {
            let out_xml = export_entries(entries);
            std::fs::write(finalize, out_xml)
                .map_err(|e| format!("write {}: {e}", finalize.display()))?;
            Ok(())
        }
        BaseKind::Strings { base, kind } => {
            let updated = apply_entries_to_strings(base, entries);
            let bytes = match kind {
                StringsKindCli::Strings => write_strings(&updated),
                StringsKindCli::DlStrings => write_dlstrings(&updated),
                StringsKindCli::IlStrings => write_ilstrings(&updated),
            }
            .map_err(|e| format!("{e:?}"))?;
            std::fs::write(finalize, bytes)
                .map_err(|e| format!("write {}: {e}", finalize.display()))?;
            Ok(())
        }
        BaseKind::Esp {
            input_path,
            extracted,
            workspace_root,
        } => {
            let mut map: HashMap<&str, &str> = HashMap::new();
            for entry in entries {
                if !entry.target_text.is_empty() {
                    map.insert(entry.key.as_str(), entry.target_text.as_str());
                }
            }
            let mut translated = extracted.clone();
            for item in &mut translated {
                let key = item.get_unique_key();
                if let Some(target) = map.get(key.as_str()) {
                    item.text = (*target).to_string();
                }
            }
            let output_dir = finalize.parent().unwrap_or_else(|| Path::new("."));
            let written = apply_translations(
                input_path,
                workspace_root,
                output_dir,
                translated,
                Some("english"),
            )
            .map_err(|e| format!("apply translations: {e}"))?;
            if written != finalize {
                std::fs::copy(&written, finalize).map_err(|e| {
                    format!(
                        "copy {} -> {} failed: {e}",
                        written.display(),
                        finalize.display()
                    )
                })?;
            }
            Ok(())
        }
    }
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

#[derive(Clone, Copy)]
enum StringsKindCli {
    Strings,
    DlStrings,
    IlStrings,
}

impl StringsKindCli {
    fn from_path(path: &Path) -> Result<Self, String> {
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        match ext.as_str() {
            "strings" => Ok(Self::Strings),
            "dlstrings" => Ok(Self::DlStrings),
            "ilstrings" => Ok(Self::IlStrings),
            _ => Err(format!("unsupported strings extension: {ext}")),
        }
    }
}

#[derive(Default, Clone)]
struct BatchOptions {
    load: Option<PathBuf>,
    load_strings: Option<PathBuf>,
    load_plugin: Option<PathBuf>,
    importxml: Option<PathBuf>,
    finalize: Option<PathBuf>,
    workspace_root: Option<PathBuf>,
    dict_in: Option<PathBuf>,
    dict_out: Option<PathBuf>,
    source: Option<String>,
    target: Option<String>,
    generate_dictionary: Option<PathBuf>,
}

fn parse_args(args: &[String]) -> Result<BatchOptions, String> {
    let mut opts = BatchOptions::default();
    let mut map: HashMap<String, String> = HashMap::new();
    let mut i = 0usize;
    while i < args.len() {
        let key = args[i].as_str();
        if !key.starts_with("--") {
            return Err(format!("invalid argument: {}", args[i]));
        }
        let Some(value) = args.get(i + 1) else {
            return Err(format!("missing value for {key}"));
        };
        map.insert(key.to_string(), value.to_string());
        i += 2;
    }
    opts.load = map.get("--load").map(PathBuf::from);
    opts.load_strings = map.get("--load-strings").map(PathBuf::from);
    opts.load_plugin = map.get("--load-plugin").map(PathBuf::from);
    opts.importxml = map.get("--importxml").map(PathBuf::from);
    opts.finalize = map.get("--finalize").map(PathBuf::from);
    opts.workspace_root = map.get("--workspace-root").map(PathBuf::from);
    opts.dict_in = map.get("--dict-in").map(PathBuf::from);
    opts.dict_out = map.get("--dict-out").map(PathBuf::from);
    opts.source = map.get("--source").cloned();
    opts.target = map.get("--target").cloned();
    opts.generate_dictionary = map.get("--generate-dictionary").map(PathBuf::from);
    Ok(opts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn t_batch_001_parse_pipeline_args() {
        let args = vec![
            "--load".to_string(),
            "base.xml".to_string(),
            "--importxml".to_string(),
            "tr.xml".to_string(),
            "--finalize".to_string(),
            "out.xml".to_string(),
        ];
        let opts = parse_args(&args).expect("parse");
        assert_eq!(opts.load.as_deref(), Some(Path::new("base.xml")));
        assert_eq!(opts.importxml.as_deref(), Some(Path::new("tr.xml")));
        assert_eq!(opts.finalize.as_deref(), Some(Path::new("out.xml")));
    }

    #[test]
    fn t_batch_002_parse_strings_plugin_args() {
        let args = vec![
            "--load-strings".to_string(),
            "a.strings".to_string(),
            "--importxml".to_string(),
            "x.xml".to_string(),
            "--finalize".to_string(),
            "out.strings".to_string(),
            "--workspace-root".to_string(),
            "/game".to_string(),
        ];
        let opts = parse_args(&args).expect("parse");
        assert_eq!(opts.load_strings.as_deref(), Some(Path::new("a.strings")));
        assert_eq!(opts.workspace_root.as_deref(), Some(Path::new("/game")));
    }
}
