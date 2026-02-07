#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedText {
    pub offset: usize,
    pub length: usize,
    pub text: String,
}

#[derive(Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub enum PluginBinaryError {
    InvalidUtf8,
    LengthMismatch,
}

pub fn extract_null_terminated_utf8(bytes: &[u8], min_len: usize) -> Vec<ExtractedText> {
    let mut results = Vec::new();
    let mut start = 0usize;
    while start < bytes.len() {
        if let Some(end) = bytes[start..].iter().position(|b| *b == 0) {
            let slice = &bytes[start..start + end];
            if slice.len() >= min_len {
                if let Ok(text) = std::str::from_utf8(slice) {
                    if looks_like_text(text) {
                        results.push(ExtractedText {
                            offset: start,
                            length: slice.len(),
                            text: text.to_string(),
                        });
                    }
                }
            }
            start += end + 1;
        } else {
            break;
        }
    }
    results
}

pub fn apply_inplace_replacements(
    bytes: &mut [u8],
    replacements: &[(usize, &str)],
) -> Result<(), PluginBinaryError> {
    for (offset, new_text) in replacements {
        let new_bytes = new_text.as_bytes();
        if *offset + new_bytes.len() > bytes.len() {
            return Err(PluginBinaryError::LengthMismatch);
        }
        let end = *offset + new_bytes.len();
        bytes[*offset..end].copy_from_slice(new_bytes);
    }
    Ok(())
}

fn looks_like_text(text: &str) -> bool {
    let mut has_letter = false;
    for ch in text.chars() {
        if ch.is_control() && ch != '\n' && ch != '\t' {
            return false;
        }
        if ch.is_alphanumeric() || ch.is_alphabetic() {
            has_letter = true;
        }
    }
    has_letter
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/plugin/binary_simple.esm"
    ));

    #[test]
    fn t_esp_ex_001_binary_extract_edit_round_trip() {
        let mut bytes = FIXTURE.to_vec();
        let entries = extract_null_terminated_utf8(&bytes, 3);
        let hello = entries.iter().find(|e| e.text == "HELLO").unwrap();
        apply_inplace_replacements(&mut bytes, &[(hello.offset, "CELLO")]).expect("apply");
        let updated = extract_null_terminated_utf8(&bytes, 3);
        assert!(updated.iter().any(|e| e.text == "CELLO"));
    }
}
