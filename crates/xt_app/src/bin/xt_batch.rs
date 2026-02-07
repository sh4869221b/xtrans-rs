use std::collections::HashMap;
use std::path::PathBuf;
use xt_core::dictionary::TranslationDictionary;
use xt_core::import_export::{apply_xml_default, export_entries, import_entries};

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let opts = parse_args(&args)?;

    if let Some(dir) = opts.generate_dictionary {
        let source = opts.source.unwrap_or_else(|| "english".to_string());
        let target = opts.target.unwrap_or_else(|| "japanese".to_string());
        let out = opts
            .dict_out
            .ok_or_else(|| "--dict-out is required with --generate-dictionary".to_string())?;
        let (dict, stats) =
            TranslationDictionary::build_from_strings_dir(&dir, &source, &target)
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

    let load = opts
        .load
        .ok_or_else(|| "--load <base.xml> is required".to_string())?;
    let import_xml = opts
        .importxml
        .ok_or_else(|| "--importxml <translation.xml> is required".to_string())?;
    let finalize = opts
        .finalize
        .ok_or_else(|| "--finalize <out.xml> is required".to_string())?;

    let base_xml =
        std::fs::read_to_string(&load).map_err(|e| format!("read {}: {e}", load.display()))?;
    let trans_xml = std::fs::read_to_string(&import_xml)
        .map_err(|e| format!("read {}: {e}", import_xml.display()))?;
    let base = import_entries(&base_xml).map_err(|e| format!("parse base xml: {e:?}"))?;
    let imported = import_entries(&trans_xml).map_err(|e| format!("parse import xml: {e:?}"))?;
    let (mut merged, stats) = apply_xml_default(&base, &imported);

    if let Some(dict_path) = opts.dict_in {
        let dict = TranslationDictionary::load_from_path(&dict_path).map_err(|e| e.to_string())?;
        let all_keys = merged.iter().map(|e| e.key.clone()).collect::<Vec<_>>();
        let (next, dict_updated) = dict.apply_quick(&merged, &all_keys, true);
        merged = next;
        println!("quick auto-translate applied: {dict_updated}");
    }

    if let Some(dict_out) = opts.dict_out {
        let dict = TranslationDictionary::build_from_entries(&merged);
        dict.save_to_path(&dict_out).map_err(|e| e.to_string())?;
        println!("saved dictionary: pairs={} out={}", dict.len(), dict_out.display());
    }

    let out_xml = export_entries(&merged);
    std::fs::write(&finalize, out_xml)
        .map_err(|e| format!("write {}: {e}", finalize.display()))?;
    println!(
        "finalized: updated={} unchanged={} missing={} out={}",
        stats.updated,
        stats.unchanged,
        stats.missing,
        finalize.display()
    );
    Ok(())
}

#[derive(Default)]
struct BatchOptions {
    load: Option<PathBuf>,
    importxml: Option<PathBuf>,
    finalize: Option<PathBuf>,
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
    opts.importxml = map.get("--importxml").map(PathBuf::from);
    opts.finalize = map.get("--finalize").map(PathBuf::from);
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

    #[test]
    fn t_batch_001_parse_pipeline_args() {
        use std::path::Path;
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
}
