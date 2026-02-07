use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Game {
    Skyrim,
    SkyrimSeAe,
    Fallout4,
    Starfield,
}

impl Game {
    fn as_str(&self) -> &'static str {
        match self {
            Game::Skyrim => "Skyrim",
            Game::SkyrimSeAe => "SkyrimSE/AE",
            Game::Fallout4 => "Fallout4",
            Game::Starfield => "Starfield",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Workspace {
    pub name: String,
    pub game: Game,
    pub root_dir: String,
    pub strings_files: Vec<String>,
    pub load_order: Vec<String>,
    pub cache_dir: Option<String>,
    pub cache_policy: CachePolicy,
}

impl Workspace {
    pub fn save_to_path(&self, path: &Path) -> Result<(), WorkspaceError> {
        let mut lines = Vec::new();
        lines.push("version=1".to_string());
        lines.push(format!("name={}", escape_value(&self.name)));
        lines.push(format!("game={}", self.game.as_str()));
        lines.push(format!("root_dir={}", escape_value(&self.root_dir)));
        for file in &self.strings_files {
            lines.push(format!("strings_file={}", escape_value(file)));
        }
        for plugin in &self.load_order {
            lines.push(format!("load_order={}", escape_value(plugin)));
        }
        if let Some(cache_dir) = &self.cache_dir {
            lines.push(format!("cache_dir={}", escape_value(cache_dir)));
        }
        lines.push(format!("cache_policy={}", self.cache_policy.as_str()));
        let content = lines.join("\n");
        std::fs::write(path, content).map_err(WorkspaceError::Io)
    }

    pub fn load_from_path(path: &Path) -> Result<Self, WorkspaceError> {
        let content = std::fs::read_to_string(path).map_err(WorkspaceError::Io)?;
        let mut version: Option<u32> = None;
        let mut name: Option<String> = None;
        let mut game: Option<Game> = None;
        let mut root_dir: Option<String> = None;
        let mut strings_files: Vec<String> = Vec::new();
        let mut load_order: Vec<String> = Vec::new();
        let mut cache_dir: Option<String> = None;
        let mut cache_policy: Option<CachePolicy> = None;

        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let (key, value) = split_key_value(line)?;
            match key {
                "version" => {
                    let parsed = value
                        .parse::<u32>()
                        .map_err(|_| WorkspaceError::InvalidFormat)?;
                    version = Some(parsed);
                }
                "name" => {
                    name = Some(unescape_value(value)?);
                }
                "game" => {
                    game = Some(parse_game(value)?);
                }
                "root_dir" => {
                    root_dir = Some(unescape_value(value)?);
                }
                "strings_file" => {
                    strings_files.push(unescape_value(value)?);
                }
                "load_order" => {
                    load_order.push(unescape_value(value)?);
                }
                "cache_dir" => {
                    cache_dir = Some(unescape_value(value)?);
                }
                "cache_policy" => {
                    cache_policy = Some(parse_cache_policy(value)?);
                }
                _ => {
                    // Ignore unknown keys for forward compatibility.
                }
            }
        }

        let version = version.ok_or(WorkspaceError::MissingField("version"))?;
        if version != 1 {
            return Err(WorkspaceError::UnsupportedVersion(version));
        }

        Ok(Workspace {
            name: name.ok_or(WorkspaceError::MissingField("name"))?,
            game: game.ok_or(WorkspaceError::MissingField("game"))?,
            root_dir: root_dir.ok_or(WorkspaceError::MissingField("root_dir"))?,
            strings_files,
            load_order,
            cache_dir,
            cache_policy: cache_policy.unwrap_or(CachePolicy::Auto),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CachePolicy {
    Auto,
    None,
}

impl CachePolicy {
    fn as_str(&self) -> &'static str {
        match self {
            CachePolicy::Auto => "auto",
            CachePolicy::None => "none",
        }
    }
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum WorkspaceError {
    Io(std::io::Error),
    InvalidFormat,
    MissingField(&'static str),
    UnsupportedVersion(u32),
    UnknownGame,
    UnknownCachePolicy,
    InvalidEscape,
}

fn split_key_value(line: &str) -> Result<(&str, &str), WorkspaceError> {
    let mut iter = line.splitn(2, '=');
    let key = iter.next().ok_or(WorkspaceError::InvalidFormat)?;
    let value = iter.next().ok_or(WorkspaceError::InvalidFormat)?;
    if key.is_empty() {
        return Err(WorkspaceError::InvalidFormat);
    }
    Ok((key, value))
}

fn parse_game(value: &str) -> Result<Game, WorkspaceError> {
    match value {
        "Skyrim" => Ok(Game::Skyrim),
        "SkyrimSE/AE" => Ok(Game::SkyrimSeAe),
        "Fallout4" => Ok(Game::Fallout4),
        "Starfield" => Ok(Game::Starfield),
        _ => Err(WorkspaceError::UnknownGame),
    }
}

fn parse_cache_policy(value: &str) -> Result<CachePolicy, WorkspaceError> {
    match value {
        "auto" => Ok(CachePolicy::Auto),
        "none" => Ok(CachePolicy::None),
        _ => Err(WorkspaceError::UnknownCachePolicy),
    }
}

fn escape_value(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.bytes() {
        match ch {
            b'%' => out.push_str("%25"),
            b'=' => out.push_str("%3D"),
            b'\n' => out.push_str("%0A"),
            b'\r' => out.push_str("%0D"),
            _ => out.push(ch as char),
        }
    }
    out
}

fn unescape_value(value: &str) -> Result<String, WorkspaceError> {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if i + 2 >= bytes.len() {
                return Err(WorkspaceError::InvalidEscape);
            }
            let hi = (bytes[i + 1] as char).to_digit(16);
            let lo = (bytes[i + 2] as char).to_digit(16);
            let value = match (hi, lo) {
                (Some(hi), Some(lo)) => (hi * 16 + lo) as u8,
                _ => return Err(WorkspaceError::InvalidEscape),
            };
            out.push(value);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).map_err(|_| WorkspaceError::InvalidEscape)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn test_path(name: &str) -> std::path::PathBuf {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let mut path = std::env::temp_dir();
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        path.push(format!("xtrans-rs-{name}-{id}.xtws"));
        path
    }

    #[test]
    fn t_ws_001_workspace_round_trip() {
        let workspace = Workspace {
            name: "Test Workspace".to_string(),
            game: Game::SkyrimSeAe,
            root_dir: "/games/skyrim".to_string(),
            strings_files: vec![
                "Data/Strings/Skyrim_en.strings".to_string(),
                "Data/Strings/Skyrim_ja.strings".to_string(),
            ],
            load_order: vec!["Skyrim.esm".to_string(), "Update.esm".to_string()],
            cache_dir: Some("/games/skyrim/cache".to_string()),
            cache_policy: CachePolicy::Auto,
        };

        let path = test_path("workspace");
        let _ = std::fs::remove_file(&path);
        workspace.save_to_path(&path).expect("save workspace");
        let loaded = Workspace::load_from_path(&path).expect("load workspace");
        assert_eq!(workspace, loaded);
    }

    #[test]
    fn t_ws_001_workspace_defaults_for_missing_fields() {
        let path = test_path("workspace-defaults");
        let content = "\
version=1
name=Defaults
game=Skyrim
root_dir=/games/skyrim
strings_file=Data/Strings/Skyrim_en.strings
";
        std::fs::write(&path, content).expect("write fixture");
        let loaded = Workspace::load_from_path(&path).expect("load workspace");
        assert_eq!(loaded.cache_policy, CachePolicy::Auto);
        assert!(loaded.cache_dir.is_none());
        assert!(loaded.load_order.is_empty());
    }
}
