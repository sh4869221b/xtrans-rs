use std::path::PathBuf;

pub const DEFAULT_DICT_SOURCE_LANG: &str = "english";
pub const DEFAULT_DICT_TARGET_LANG: &str = "japanese";
pub const DEFAULT_DICT_ROOT: &str = "./Data/Strings/Translations";
const DICT_PREFS_FILE: &str = "dict_prefs.v1";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DictionaryPrefs {
    pub source_lang: String,
    pub target_lang: String,
    pub root: String,
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

pub fn dictionary_prefs_path() -> Option<PathBuf> {
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

pub fn load_dictionary_prefs() -> Result<DictionaryPrefs, String> {
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

pub fn save_dictionary_prefs(prefs: &DictionaryPrefs) -> Result<(), String> {
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

pub fn serialize_dictionary_prefs(prefs: &DictionaryPrefs) -> String {
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

pub fn parse_dictionary_prefs(content: &str) -> Result<DictionaryPrefs, String> {
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
