use crate::model::Entry;

#[derive(Debug, PartialEq, Eq)]
pub enum XmlError {
    InvalidFormat,
    MissingAttr(&'static str),
    InvalidEscape,
}

pub fn export_entries(entries: &[Entry]) -> String {
    let mut out = String::new();
    out.push_str(r#"<?xml version="1.0" encoding="utf-8"?>"#);
    out.push('\n');
    out.push_str(r#"<xtrans version="1">"#);
    out.push('\n');
    for entry in entries {
        out.push_str("  <entry");
        out.push_str(r#" key=""#);
        out.push_str(&escape_xml(&entry.key));
        out.push('"');
        out.push_str(r#" source=""#);
        out.push_str(&escape_xml(&entry.source_text));
        out.push('"');
        out.push_str(r#" target=""#);
        out.push_str(&escape_xml(&entry.target_text));
        out.push('"');
        out.push_str(" />\n");
    }
    out.push_str("</xtrans>\n");
    out
}

pub fn import_entries(xml: &str) -> Result<Vec<Entry>, XmlError> {
    let mut entries = Vec::new();
    let mut rest = xml;
    while let Some(start) = rest.find("<entry") {
        rest = &rest[start + 6..];
        let end = rest.find("/>").ok_or(XmlError::InvalidFormat)?;
        let tag = &rest[..end];
        let key = parse_attr(tag, "key")?;
        let source_text = parse_attr(tag, "source")?;
        let target_text = parse_attr(tag, "target")?;
        entries.push(Entry {
            key,
            source_text,
            target_text,
        });
        rest = &rest[end + 2..];
    }
    Ok(entries)
}

fn parse_attr(tag: &str, name: &'static str) -> Result<String, XmlError> {
    let needle = format!(r#"{name}=""#);
    let start = tag.find(&needle).ok_or(XmlError::MissingAttr(name))?;
    let after = &tag[start + needle.len()..];
    let end = after.find('"').ok_or(XmlError::InvalidFormat)?;
    unescape_xml(&after[..end])
}

fn escape_xml(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            '\n' => out.push_str("&#10;"),
            '\r' => out.push_str("&#13;"),
            '\t' => out.push_str("&#9;"),
            _ => out.push(ch),
        }
    }
    out
}

fn unescape_xml(input: &str) -> Result<String, XmlError> {
    let mut out = String::with_capacity(input.len());
    let mut i = 0;
    while i < input.len() {
        if input.as_bytes()[i] == b'&' {
            let rest = &input[i..];
            let end = rest.find(';').ok_or(XmlError::InvalidEscape)?;
            let entity = &rest[1..end];
            match entity {
                "amp" => out.push('&'),
                "lt" => out.push('<'),
                "gt" => out.push('>'),
                "quot" => out.push('"'),
                "apos" => out.push('\''),
                _ => {
                    if let Some(num) = entity.strip_prefix('#') {
                        let value = num.parse::<u32>().map_err(|_| XmlError::InvalidEscape)?;
                        let ch = char::from_u32(value).ok_or(XmlError::InvalidEscape)?;
                        out.push(ch);
                    } else {
                        return Err(XmlError::InvalidEscape);
                    }
                }
            }
            i += end + 1;
        } else {
            let ch = input[i..].chars().next().ok_or(XmlError::InvalidEscape)?;
            out.push(ch);
            i += ch.len_utf8();
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t_xml_rt_001_export_import_round_trip() {
        let entries = vec![
            Entry {
                key: "strings:1".to_string(),
                source_text: "Hello & <world>".to_string(),
                target_text: "こんにちは".to_string(),
            },
            Entry {
                key: "strings:2".to_string(),
                source_text: "Line1\nLine2".to_string(),
                target_text: "A\"B'".to_string(),
            },
        ];
        let xml = export_entries(&entries);
        let parsed = import_entries(&xml).expect("import xml");
        assert_eq!(parsed, entries);
    }
}
