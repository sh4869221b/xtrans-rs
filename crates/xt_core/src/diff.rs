#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub enum EntryStatus {
    Untranslated,
    Draft,
    Reviewed,
    NeedsReview,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffEntry {
    pub key: String,
    pub source_text: String,
    pub target_text: String,
    pub status: EntryStatus,
    pub source_hash: u64,
}

impl DiffEntry {
    pub fn new(key: &str, source_text: &str, target_text: &str) -> Self {
        let hash = hash_source(source_text);
        Self {
            key: key.to_string(),
            source_text: source_text.to_string(),
            target_text: target_text.to_string(),
            status: EntryStatus::Untranslated,
            source_hash: hash,
        }
    }
}

pub fn update_source(entry: &mut DiffEntry, new_source: &str) {
    let new_hash = hash_source(new_source);
    if new_hash != entry.source_hash {
        entry.status = EntryStatus::NeedsReview;
    }
    entry.source_text.clear();
    entry.source_text.push_str(new_source);
    entry.source_hash = new_hash;
}

pub fn hash_source(text: &str) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;
    let mut hash = FNV_OFFSET;
    for byte in text.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t_diff_001_source_change_marks_needs_review() {
        let mut entry = DiffEntry::new("k1", "Hello", "こんにちは");
        assert_eq!(entry.status, EntryStatus::Untranslated);
        update_source(&mut entry, "Hello world");
        assert_eq!(entry.status, EntryStatus::NeedsReview);
    }
}
