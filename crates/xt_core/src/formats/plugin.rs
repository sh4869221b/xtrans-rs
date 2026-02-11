#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginEntry {
    pub id: u32,
    pub context: String,
    pub source_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PluginFile {
    pub entries: Vec<PluginEntry>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum PluginError {
    InvalidHeader,
    InvalidLine,
    InvalidId,
    DuplicateId(u32),
    InvalidField,
}

pub fn read_plugin(input: &str) -> Result<PluginFile, PluginError> {
    let mut lines = input.lines();
    let header = lines.next().ok_or(PluginError::InvalidHeader)?;
    if header.trim() != "XTPLUGIN1" {
        return Err(PluginError::InvalidHeader);
    }

    let mut entries = Vec::new();
    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        let mut parts = line.splitn(3, '|');
        let id_str = parts.next().ok_or(PluginError::InvalidLine)?;
        let context = parts.next().ok_or(PluginError::InvalidLine)?;
        let source_text = parts.next().ok_or(PluginError::InvalidLine)?;
        let id = id_str.parse::<u32>().map_err(|_| PluginError::InvalidId)?;
        entries.push(PluginEntry {
            id,
            context: context.to_string(),
            source_text: source_text.to_string(),
        });
    }

    Ok(PluginFile { entries })
}

pub fn write_plugin(file: &PluginFile) -> Result<String, PluginError> {
    let mut entries = file.entries.clone();
    entries.sort_by_key(|entry| entry.id);
    for window in entries.windows(2) {
        if window[0].id == window[1].id {
            return Err(PluginError::DuplicateId(window[0].id));
        }
    }

    let mut out = String::new();
    out.push_str("XTPLUGIN1\n");
    for entry in entries {
        if entry.context.contains('|') || entry.source_text.contains('|') {
            return Err(PluginError::InvalidField);
        }
        out.push_str(&format!(
            "{}|{}|{}\n",
            entry.id, entry.context, entry.source_text
        ));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    const FIXTURE: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/plugin/simple.xtplugin"
    ));

    fn test_path(name: &str) -> std::path::PathBuf {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let mut path = std::env::temp_dir();
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        path.push(format!("xtrans-rs-{name}-{id}.xtplugin"));
        path
    }

    #[test]
    fn t_esp_ex_001_plugin_round_trip_edit() {
        let mut plugin = read_plugin(FIXTURE).expect("read plugin fixture");
        plugin.entries[0].source_text = "Edited line".to_string();

        let path = test_path("plugin-edit");
        let _ = std::fs::remove_file(&path);
        let encoded = write_plugin(&plugin).expect("write plugin");
        std::fs::write(&path, encoded).expect("write temp plugin");

        let reloaded = read_plugin(&std::fs::read_to_string(&path).expect("read temp plugin"))
            .expect("read plugin");
        assert_eq!(reloaded, plugin);
    }
}
