use crate::model::Entry;
use std::collections::HashMap;

#[derive(Debug, PartialEq, Eq)]
pub enum XmlError {
    InvalidFormat,
    MissingAttr(&'static str),
    InvalidEscape,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct XmlApplyStats {
    pub updated: usize,
    pub unchanged: usize,
    pub missing: usize,
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
    let xml = strip_bom(xml);
    if xml.contains("<SSTXMLRessources") {
        return import_entries_xtranslator(xml);
    }
    import_entries_xtrans(xml)
}

fn import_entries_xtrans(xml: &str) -> Result<Vec<Entry>, XmlError> {
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

fn import_entries_xtranslator(xml: &str) -> Result<Vec<Entry>, XmlError> {
    let mut entries = Vec::new();
    let mut rest = xml;
    let mut index = 0usize;

    while let Some(start) = find_string_tag(rest) {
        let block = &rest[start..];
        let open_end = block.find('>').ok_or(XmlError::InvalidFormat)?;
        let open_tag = &block[..=open_end];
        let body_with_tail = &block[open_end + 1..];
        let close = body_with_tail
            .find("</String>")
            .ok_or(XmlError::InvalidFormat)?;
        let body = &body_with_tail[..close];

        let source_text = parse_element_text(body, "Source")?;
        let target_text = parse_element_text(body, "Dest")?;

        // xTranslator XML has no stable key for our internal entries.
        // We keep a synthetic key and rely on source-text fallback matching.
        let list = parse_attr(open_tag, "List").ok();
        let sid = parse_attr(open_tag, "sID").ok();
        let key = format!(
            "xtr:{}:{}:{}",
            list.unwrap_or_else(|| "0".to_string()),
            sid.unwrap_or_else(|| "-".to_string()),
            index
        );

        entries.push(Entry {
            key,
            source_text,
            target_text,
        });
        index = index.saturating_add(1);
        rest = &body_with_tail[close + "</String>".len()..];
    }

    if entries.is_empty() {
        return Err(XmlError::InvalidFormat);
    }
    Ok(entries)
}

pub fn apply_xml_default(current: &[Entry], imported: &[Entry]) -> (Vec<Entry>, XmlApplyStats) {
    let mut import_map: HashMap<&str, &str> = HashMap::new();
    let mut source_map: HashMap<&str, Option<&str>> = HashMap::new();
    for entry in imported {
        if !entry.target_text.is_empty() {
            import_map.insert(entry.key.as_str(), entry.target_text.as_str());
            match source_map.get(entry.source_text.as_str()) {
                None => {
                    source_map.insert(entry.source_text.as_str(), Some(entry.target_text.as_str()));
                }
                Some(Some(prev)) if *prev != entry.target_text.as_str() => {
                    source_map.insert(entry.source_text.as_str(), None);
                }
                _ => {}
            }
        }
    }
    let mut stats = XmlApplyStats::default();
    let merged = current
        .iter()
        .map(|entry| {
            let mut next = entry.clone();
            let key_target = import_map.get(entry.key.as_str()).copied();
            let source_target = source_map
                .get(entry.source_text.as_str())
                .and_then(|v| v.as_ref().copied());
            match key_target.or(source_target) {
                Some(target) => {
                    if next.target_text != target {
                        next.target_text = target.to_string();
                        stats.updated += 1;
                    } else {
                        stats.unchanged += 1;
                    }
                }
                None => stats.missing += 1,
            }
            next
        })
        .collect::<Vec<_>>();
    (merged, stats)
}

fn parse_attr(tag: &str, name: &'static str) -> Result<String, XmlError> {
    let needle = format!(r#"{name}=""#);
    let start = tag.find(&needle).ok_or(XmlError::MissingAttr(name))?;
    let after = &tag[start + needle.len()..];
    let end = after.find('"').ok_or(XmlError::InvalidFormat)?;
    unescape_xml(&after[..end])
}

fn parse_element_text(input: &str, name: &'static str) -> Result<String, XmlError> {
    let mut from = 0usize;
    while let Some(rel_start) = input[from..].find(&format!("<{name}")) {
        let start = from + rel_start;
        let tail = &input[start + name.len() + 1..];
        let Some(next) = tail.as_bytes().first().copied() else {
            return Err(XmlError::InvalidFormat);
        };
        if !matches!(next, b'>' | b' ' | b'\t' | b'\r' | b'\n') {
            from = start + 1;
            continue;
        }
        let open_end = input[start..]
            .find('>')
            .ok_or(XmlError::InvalidFormat)?
            + start;
        let close_tag = format!("</{name}>");
        let close_start = input[open_end + 1..]
            .find(&close_tag)
            .ok_or(XmlError::InvalidFormat)?
            + open_end
            + 1;
        return unescape_xml(&input[open_end + 1..close_start]);
    }
    Err(XmlError::InvalidFormat)
}

fn find_string_tag(input: &str) -> Option<usize> {
    let mut from = 0usize;
    while let Some(rel_start) = input[from..].find("<String") {
        let start = from + rel_start;
        let tail = &input[start + "<String".len()..];
        let next = tail.as_bytes().first().copied();
        if matches!(next, Some(b'>') | Some(b' ') | Some(b'\t') | Some(b'\r') | Some(b'\n')) {
            return Some(start);
        }
        from = start + 1;
    }
    None
}

fn strip_bom(input: &str) -> &str {
    input.strip_prefix('\u{feff}').unwrap_or(input)
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

    #[test]
    fn t_xml_apply_001_default_profile_stats() {
        let current = vec![
            Entry {
                key: "k1".to_string(),
                source_text: "A".to_string(),
                target_text: String::new(),
            },
            Entry {
                key: "k2".to_string(),
                source_text: "B".to_string(),
                target_text: "X".to_string(),
            },
            Entry {
                key: "k3".to_string(),
                source_text: "C".to_string(),
                target_text: String::new(),
            },
        ];
        let imported = vec![
            Entry {
                key: "k1".to_string(),
                source_text: "A".to_string(),
                target_text: "AA".to_string(),
            },
            Entry {
                key: "k2".to_string(),
                source_text: "B".to_string(),
                target_text: "X".to_string(),
            },
        ];
        let (merged, stats) = apply_xml_default(&current, &imported);
        assert_eq!(stats.updated, 1);
        assert_eq!(stats.unchanged, 1);
        assert_eq!(stats.missing, 1);
        assert_eq!(merged[0].target_text, "AA");
    }

    #[test]
    fn t_xml_import_002_accept_xtranslator_schema() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<SSTXMLRessources>
  <Params>
    <Addon>isilNarsil</Addon>
    <Source>english</Source>
    <Dest>japanese</Dest>
    <Version>2</Version>
  </Params>
  <Content>
    <String List="0" sID="000001">
      <EDID>IronSword</EDID>
      <REC id="0" idMax="1">WEAP:FULL</REC>
      <Source>Iron Sword</Source>
      <Dest>鉄の剣</Dest>
    </String>
    <String List="0" sID="000002">
      <Source>Steel Sword</Source>
      <Dest>鋼鉄の剣</Dest>
    </String>
  </Content>
</SSTXMLRessources>"#;

        let parsed = import_entries(xml).expect("import xtranslator xml");
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].source_text, "Iron Sword");
        assert_eq!(parsed[0].target_text, "鉄の剣");
        assert_eq!(parsed[1].source_text, "Steel Sword");
        assert_eq!(parsed[1].target_text, "鋼鉄の剣");
    }

    #[test]
    fn t_xml_apply_002_source_fallback_for_xtranslator() {
        let current = vec![
            Entry {
                key: "WEAP:00012EB7:FULL:0".to_string(),
                source_text: "Iron Sword".to_string(),
                target_text: String::new(),
            },
            Entry {
                key: "WEAP:00013989:FULL:0".to_string(),
                source_text: "Steel Sword".to_string(),
                target_text: String::new(),
            },
        ];

        // imported keys intentionally do not match current keys.
        let imported = vec![
            Entry {
                key: "xtr:0:000001:0".to_string(),
                source_text: "Iron Sword".to_string(),
                target_text: "鉄の剣".to_string(),
            },
            Entry {
                key: "xtr:0:000002:1".to_string(),
                source_text: "Steel Sword".to_string(),
                target_text: "鋼鉄の剣".to_string(),
            },
        ];

        let (merged, stats) = apply_xml_default(&current, &imported);
        assert_eq!(stats.updated, 2);
        assert_eq!(stats.unchanged, 0);
        assert_eq!(stats.missing, 0);
        assert_eq!(merged[0].target_text, "鉄の剣");
        assert_eq!(merged[1].target_text, "鋼鉄の剣");
    }

    #[test]
    fn t_xml_apply_003_source_fallback_skips_ambiguous_targets() {
        let current = vec![Entry {
            key: "k1".to_string(),
            source_text: "Moonforge".to_string(),
            target_text: String::new(),
        }];
        let imported = vec![
            Entry {
                key: "xtr:a".to_string(),
                source_text: "Moonforge".to_string(),
                target_text: "ムーンフォージ".to_string(),
            },
            Entry {
                key: "xtr:b".to_string(),
                source_text: "Moonforge".to_string(),
                target_text: "月鍛冶".to_string(),
            },
        ];

        let (merged, stats) = apply_xml_default(&current, &imported);
        assert_eq!(stats.updated, 0);
        assert_eq!(stats.unchanged, 0);
        assert_eq!(stats.missing, 1);
        assert_eq!(merged[0].target_text, "");
    }
}
