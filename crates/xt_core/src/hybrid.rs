use crate::formats::plugin::PluginFile;
use crate::formats::strings::StringsFile;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HybridEntry {
    pub id: u32,
    pub context: String,
    pub target_text: String,
}

pub fn build_hybrid_entries(plugin: &PluginFile, strings: &StringsFile) -> Vec<HybridEntry> {
    let targets: HashMap<u32, String> = strings
        .entries
        .iter()
        .map(|entry| (entry.id, entry.text.clone()))
        .collect();

    let mut entries = Vec::new();
    for entry in &plugin.entries {
        if let Some(target_text) = targets.get(&entry.id) {
            entries.push(HybridEntry {
                id: entry.id,
                context: entry.context.clone(),
                target_text: target_text.clone(),
            });
        }
    }
    entries
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::plugin::{PluginEntry, PluginFile};
    use crate::formats::strings::{StringsEntry, StringsFile};

    #[test]
    fn t_hyb_ctx_001_context_lookup() {
        let plugin = PluginFile {
            entries: vec![PluginEntry {
                id: 100,
                context: "Greeting".to_string(),
                source_text: "Hello".to_string(),
            }],
        };
        let strings = StringsFile {
            entries: vec![StringsEntry {
                id: 100,
                text: "こんにちは".to_string(),
            }],
        };
        let hybrid = build_hybrid_entries(&plugin, &strings);
        assert_eq!(hybrid.len(), 1);
        assert_eq!(hybrid[0].context, "Greeting");
        assert_eq!(hybrid[0].target_text, "こんにちは");
    }
}
