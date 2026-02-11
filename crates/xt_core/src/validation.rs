#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub enum Severity {
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationIssue {
    pub entry_key: String,
    pub severity: Severity,
    pub rule_id: String,
    pub message: String,
}

impl ValidationIssue {
    fn placeholder_mismatch(entry_key: &str) -> Self {
        Self {
            entry_key: entry_key.to_string(),
            severity: Severity::Error,
            rule_id: "placeholder.braced.mismatch".to_string(),
            message: "Braced placeholders do not match between source and target.".to_string(),
        }
    }

    fn printf_placeholder_mismatch(entry_key: &str) -> Self {
        Self {
            entry_key: entry_key.to_string(),
            severity: Severity::Error,
            rule_id: "placeholder.printf.mismatch".to_string(),
            message: "Printf-style placeholders do not match between source and target."
                .to_string(),
        }
    }

    fn alias_tag_mismatch(entry_key: &str) -> Self {
        Self {
            entry_key: entry_key.to_string(),
            severity: Severity::Error,
            rule_id: "alias.tag.mismatch".to_string(),
            message: "Alias tags do not match between source and target.".to_string(),
        }
    }
}

pub fn validate_braced_placeholders(
    entry_key: &str,
    source_text: &str,
    target_text: &str,
) -> Vec<ValidationIssue> {
    let mut source = extract_braced_placeholders(source_text);
    let mut target = extract_braced_placeholders(target_text);
    source.sort();
    target.sort();

    if source == target {
        Vec::new()
    } else {
        vec![ValidationIssue::placeholder_mismatch(entry_key)]
    }
}

pub fn validate_printf_placeholders(
    entry_key: &str,
    source_text: &str,
    target_text: &str,
) -> Vec<ValidationIssue> {
    let mut source = extract_printf_placeholders(source_text);
    let mut target = extract_printf_placeholders(target_text);
    source.sort();
    target.sort();

    if source == target {
        Vec::new()
    } else {
        vec![ValidationIssue::printf_placeholder_mismatch(entry_key)]
    }
}

pub fn validate_alias_tags(
    entry_key: &str,
    source_text: &str,
    target_text: &str,
) -> Vec<ValidationIssue> {
    let mut source = extract_alias_tags(source_text);
    let mut target = extract_alias_tags(target_text);
    source.sort();
    target.sort();

    if source == target {
        Vec::new()
    } else {
        vec![ValidationIssue::alias_tag_mismatch(entry_key)]
    }
}

fn extract_braced_placeholders(text: &str) -> Vec<String> {
    let bytes = text.as_bytes();
    let mut placeholders = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'{' {
            let start = i + 1;
            let mut j = start;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
            if j > start && j < bytes.len() && bytes[j] == b'}' {
                if let Ok(token) = std::str::from_utf8(&bytes[i..=j]) {
                    placeholders.push(token.to_string());
                }
                i = j + 1;
                continue;
            }
        }
        i += 1;
    }
    placeholders
}

fn extract_printf_placeholders(text: &str) -> Vec<String> {
    let bytes = text.as_bytes();
    let mut placeholders = Vec::new();
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] == b'%' {
            let next = bytes[i + 1];
            if next == b'%' {
                i += 2;
                continue;
            }
            if next == b's' || next == b'd' {
                if let Ok(token) = std::str::from_utf8(&bytes[i..=i + 1]) {
                    placeholders.push(token.to_string());
                }
                i += 2;
                continue;
            }
        }
        i += 1;
    }
    placeholders
}

fn extract_alias_tags(text: &str) -> Vec<String> {
    let mut tags = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("<Alias=") {
        rest = &rest[start + 7..];
        let end = match rest.find('>') {
            Some(end) => end,
            None => break,
        };
        tags.push(rest[..end].to_string());
        rest = &rest[end + 1..];
    }
    tags
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t_val_ph_001_mismatch_returns_error() {
        let issues = validate_braced_placeholders("entry:1", "Hello {0}", "こんにちは");
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].severity, Severity::Error);
    }

    #[test]
    fn t_val_ph_001_match_returns_no_issues() {
        let issues = validate_braced_placeholders("entry:2", "A {0} B {1}", "B {1} A {0}");
        assert!(issues.is_empty());
    }

    #[test]
    fn t_val_ph_002_mismatch_returns_error() {
        let issues = validate_printf_placeholders("entry:3", "Hello %s %d", "こんにちは %s");
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].severity, Severity::Error);
    }

    #[test]
    fn t_val_ph_002_match_returns_no_issues() {
        let issues = validate_printf_placeholders("entry:4", "Rate 100%% %s", "Rate 100%% %s");
        assert!(issues.is_empty());
    }

    #[test]
    fn t_val_alias_001_mismatch_returns_error() {
        let issues =
            validate_alias_tags("entry:5", "Hello <Alias=John>", "こんにちは <Alias=Jane>");
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].severity, Severity::Error);
    }

    #[test]
    fn t_val_alias_001_match_returns_no_issues() {
        let issues = validate_alias_tags(
            "entry:6",
            "Hello <Alias=John> <Alias=Jane>",
            "こんにちは <Alias=Jane> <Alias=John>",
        );
        assert!(issues.is_empty());
    }
}
