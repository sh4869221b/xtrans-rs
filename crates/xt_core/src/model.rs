#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub key: String,
    pub source_text: String,
    pub target_text: String,
}
