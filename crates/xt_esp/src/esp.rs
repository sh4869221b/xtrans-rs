use crate::strings::{
    read_dlstrings, read_ilstrings, read_strings, write_dlstrings, write_ilstrings, write_strings,
    StringsFile,
};
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use std::collections::HashMap;
use std::fmt;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

pub type EspResult<T> = Result<T, EspError>;

const RECORD_HEADER_SIZE: usize = 24;
const GROUP_HEADER_SIZE: usize = 24;
const RECORD_COMPRESSED: u32 = 0x0004_0000;

#[derive(Debug)]
pub enum EspError {
    Io(std::io::Error),
    InvalidHeader,
    InvalidRecord,
    InvalidGroup,
    InvalidSubrecord,
    InvalidUtf8,
    MissingStringsFile(StringsKind),
    MissingStringId(u32),
    InvalidStringsPath,
}

impl From<std::io::Error> for EspError {
    fn from(err: std::io::Error) -> Self {
        EspError::Io(err)
    }
}

impl fmt::Display for EspError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EspError::Io(err) => write!(f, "io error: {err}"),
            EspError::InvalidHeader => write!(f, "invalid header"),
            EspError::InvalidRecord => write!(f, "invalid record"),
            EspError::InvalidGroup => write!(f, "invalid group"),
            EspError::InvalidSubrecord => write!(f, "invalid subrecord"),
            EspError::InvalidUtf8 => write!(f, "invalid utf-8"),
            EspError::MissingStringsFile(kind) => write!(f, "missing strings file: {kind}"),
            EspError::MissingStringId(id) => write!(f, "missing string id: {id}"),
            EspError::InvalidStringsPath => write!(f, "invalid strings path"),
        }
    }
}

impl std::error::Error for EspError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            EspError::Io(err) => Some(err),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StringsKind {
    Strings,
    DlStrings,
    IlStrings,
}

impl StringsKind {
    fn extension(self) -> &'static str {
        match self {
            StringsKind::Strings => "strings",
            StringsKind::DlStrings => "dlstrings",
            StringsKind::IlStrings => "ilstrings",
        }
    }
}

impl fmt::Display for StringsKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.extension())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StringStorage {
    Inline,
    Localized { kind: StringsKind, id: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedString {
    key: String,
    pub record_type: [u8; 4],
    pub subrecord_type: [u8; 4],
    pub form_id: u32,
    pub index: usize,
    pub text: String,
    pub storage: StringStorage,
}

impl ExtractedString {
    pub fn get_unique_key(&self) -> String {
        self.key.clone()
    }
}

#[derive(Debug, Clone)]
struct RecordHeader {
    record_type: [u8; 4],
    flags: u32,
    form_id: u32,
    stamp: u16,
    version_control: u16,
    version: u16,
    unknown: u16,
}

#[derive(Debug, Clone)]
struct Record {
    header: RecordHeader,
    subrecords: Vec<Subrecord>,
    compressed: bool,
}

#[derive(Debug, Clone)]
struct Group {
    label: [u8; 4],
    group_type: u32,
    stamp: u32,
    unknown: u32,
    children: Vec<Block>,
}

#[derive(Debug, Clone)]
enum Block {
    Record(Record),
    Group(Group),
}

#[derive(Debug, Clone)]
struct Subrecord {
    sub_type: [u8; 4],
    data: Vec<u8>,
}

#[derive(Debug, Clone)]
struct StringsBundle {
    strings: Option<StringsFile>,
    dlstrings: Option<StringsFile>,
    ilstrings: Option<StringsFile>,
    base_name: String,
    language: String,
}

pub fn extract_strings(
    path: &Path,
    workspace_root: &Path,
    language: Option<&str>,
) -> EspResult<Vec<ExtractedString>> {
    let bytes = std::fs::read(path)?;
    let bundle = load_strings_bundle(path, workspace_root, language)?;
    let strings_map = build_strings_map(&bundle);
    let blocks = parse_plugin(&bytes)?;

    let mut results = Vec::new();
    let mut stack = Vec::new();
    stack.extend(blocks.iter());
    while let Some(block) = stack.pop() {
        match block {
            Block::Record(record) => collect_strings(record, &strings_map, &mut results),
            Block::Group(group) => stack.extend(group.children.iter()),
        }
    }
    Ok(results)
}

pub fn apply_translations(
    input_path: &Path,
    workspace_root: &Path,
    output_dir: &Path,
    translations: Vec<ExtractedString>,
    language: Option<&str>,
) -> EspResult<PathBuf> {
    let bytes = std::fs::read(input_path)?;
    let mut bundle = load_strings_bundle(input_path, workspace_root, language)?;
    let mut blocks = parse_plugin(&bytes)?;
    let mut translation_map: HashMap<String, ExtractedString> = translations
        .into_iter()
        .map(|entry| (entry.get_unique_key(), entry))
        .collect();

    let mut stack: Vec<&mut Block> = blocks.iter_mut().collect();
    while let Some(block) = stack.pop() {
        match block {
            Block::Record(record) => apply_to_record(record, &mut bundle, &mut translation_map)?,
            Block::Group(group) => stack.extend(group.children.iter_mut()),
        }
    }

    let output_path = output_dir.join(input_path.file_name().ok_or(EspError::InvalidStringsPath)?);
    let output_bytes = serialize_blocks(&blocks)?;
    std::fs::create_dir_all(output_dir)?;
    std::fs::write(&output_path, output_bytes)?;
    write_strings_bundle(&bundle, workspace_root)?;
    Ok(output_path)
}

fn collect_strings(record: &Record, strings_map: &StringsMap, results: &mut Vec<ExtractedString>) {
    let mut index = 0usize;
    for subrecord in &record.subrecords {
        if !is_string_subrecord(&subrecord.sub_type) {
            continue;
        }
        if let Some((text, storage)) = decode_subrecord_string(&subrecord.data, strings_map) {
            let record_type = record.header.record_type;
            let subrecord_type = subrecord.sub_type;
            let key = format!(
                "{}:{:08X}:{}:{}",
                tag_to_string(record_type),
                record.header.form_id,
                tag_to_string(subrecord_type),
                index
            );
            results.push(ExtractedString {
                key,
                record_type,
                subrecord_type,
                form_id: record.header.form_id,
                index,
                text,
                storage,
            });
            index = index.saturating_add(1);
        }
    }
}

fn apply_to_record(
    record: &mut Record,
    bundle: &mut StringsBundle,
    translations: &mut HashMap<String, ExtractedString>,
) -> EspResult<()> {
    let mut index = 0usize;
    for subrecord in &mut record.subrecords {
        if !is_string_subrecord(&subrecord.sub_type) {
            continue;
        }
        let key = format!(
            "{}:{:08X}:{}:{}",
            tag_to_string(record.header.record_type),
            record.header.form_id,
            tag_to_string(subrecord.sub_type),
            index
        );
        if let Some(updated) = translations.remove(&key) {
            match updated.storage {
                StringStorage::Inline => {
                    let null_terminated = subrecord.data.last().copied() == Some(0);
                    subrecord.data = encode_string(&updated.text, null_terminated);
                }
                StringStorage::Localized { kind, id } => {
                    update_strings_bundle(bundle, kind, id, &updated.text)?;
                }
            }
        }
        index = index.saturating_add(1);
    }
    Ok(())
}

fn parse_plugin(bytes: &[u8]) -> EspResult<Vec<Block>> {
    let mut blocks = Vec::new();
    let mut offset = 0usize;
    while offset < bytes.len() {
        let tag = read_tag(bytes, offset)?;
        if &tag == b"GRUP" {
            let (group, next) = parse_group(bytes, offset)?;
            blocks.push(Block::Group(group));
            offset = next;
        } else {
            let (record, next) = parse_record(bytes, offset)?;
            blocks.push(Block::Record(record));
            offset = next;
        }
    }
    Ok(blocks)
}

fn parse_group(bytes: &[u8], offset: usize) -> EspResult<(Group, usize)> {
    if offset + GROUP_HEADER_SIZE > bytes.len() {
        return Err(EspError::InvalidGroup);
    }
    let size = read_u32(bytes, offset + 4)? as usize;
    if size < GROUP_HEADER_SIZE || offset + size > bytes.len() {
        return Err(EspError::InvalidGroup);
    }
    let label = read_tag(bytes, offset + 8)?;
    let group_type = read_u32(bytes, offset + 12)?;
    let stamp = read_u32(bytes, offset + 16)?;
    let unknown = read_u32(bytes, offset + 20)?;
    let mut children = Vec::new();
    let mut cursor = offset + GROUP_HEADER_SIZE;
    let end = offset + size;
    while cursor < end {
        let tag = read_tag(bytes, cursor)?;
        if &tag == b"GRUP" {
            let (group, next) = parse_group(bytes, cursor)?;
            children.push(Block::Group(group));
            cursor = next;
        } else {
            let (record, next) = parse_record(bytes, cursor)?;
            children.push(Block::Record(record));
            cursor = next;
        }
    }
    Ok((
        Group {
            label,
            group_type,
            stamp,
            unknown,
            children,
        },
        end,
    ))
}

fn parse_record(bytes: &[u8], offset: usize) -> EspResult<(Record, usize)> {
    if offset + RECORD_HEADER_SIZE > bytes.len() {
        return Err(EspError::InvalidRecord);
    }
    let record_type = read_tag(bytes, offset)?;
    let data_size = read_u32(bytes, offset + 4)? as usize;
    let flags = read_u32(bytes, offset + 8)?;
    let form_id = read_u32(bytes, offset + 12)?;
    let stamp = read_u16(bytes, offset + 16)?;
    let version_control = read_u16(bytes, offset + 18)?;
    let version = read_u16(bytes, offset + 20)?;
    let unknown = read_u16(bytes, offset + 22)?;
    let data_start = offset + RECORD_HEADER_SIZE;
    let data_end = data_start
        .checked_add(data_size)
        .ok_or(EspError::InvalidRecord)?;
    if data_end > bytes.len() {
        return Err(EspError::InvalidRecord);
    }
    let stored_data = &bytes[data_start..data_end];
    let compressed = (flags & RECORD_COMPRESSED) != 0;
    let data = if compressed {
        decompress_record_data(stored_data)?
    } else {
        stored_data.to_vec()
    };
    let subrecords = parse_subrecords(&data)?;
    Ok((
        Record {
            header: RecordHeader {
                record_type,
                flags,
                form_id,
                stamp,
                version_control,
                version,
                unknown,
            },
            subrecords,
            compressed,
        },
        data_end,
    ))
}

fn parse_subrecords(data: &[u8]) -> EspResult<Vec<Subrecord>> {
    let mut subrecords = Vec::new();
    let mut cursor = 0usize;
    let mut extended_len: Option<u32> = None;
    while cursor + 6 <= data.len() {
        let sub_type = read_tag(data, cursor)?;
        let len = read_u16(data, cursor + 4)? as usize;
        let payload_start = cursor + 6;
        if &sub_type == b"XXXX" {
            if len != 4 || payload_start + 4 > data.len() {
                return Err(EspError::InvalidSubrecord);
            }
            extended_len = Some(read_u32(data, payload_start)?);
            cursor = payload_start + len;
            continue;
        }
        let actual_len = extended_len
            .take()
            .map(|value| value as usize)
            .unwrap_or(len);
        let payload_end = payload_start
            .checked_add(actual_len)
            .ok_or(EspError::InvalidSubrecord)?;
        if payload_end > data.len() {
            return Err(EspError::InvalidSubrecord);
        }
        let payload = data[payload_start..payload_end].to_vec();
        subrecords.push(Subrecord {
            sub_type,
            data: payload,
        });
        cursor = payload_end;
    }
    Ok(subrecords)
}

fn serialize_blocks(blocks: &[Block]) -> EspResult<Vec<u8>> {
    let mut out = Vec::new();
    for block in blocks {
        match block {
            Block::Record(record) => out.extend_from_slice(&serialize_record(record)?),
            Block::Group(group) => out.extend_from_slice(&serialize_group(group)?),
        }
    }
    Ok(out)
}

fn serialize_group(group: &Group) -> EspResult<Vec<u8>> {
    let mut data = Vec::new();
    for child in &group.children {
        match child {
            Block::Record(record) => data.extend_from_slice(&serialize_record(record)?),
            Block::Group(nested) => data.extend_from_slice(&serialize_group(nested)?),
        }
    }
    let size = (GROUP_HEADER_SIZE + data.len()) as u32;
    let mut out = Vec::with_capacity(GROUP_HEADER_SIZE + data.len());
    out.extend_from_slice(b"GRUP");
    out.extend_from_slice(&size.to_le_bytes());
    out.extend_from_slice(&group.label);
    out.extend_from_slice(&group.group_type.to_le_bytes());
    out.extend_from_slice(&group.stamp.to_le_bytes());
    out.extend_from_slice(&group.unknown.to_le_bytes());
    out.extend_from_slice(&data);
    Ok(out)
}

fn serialize_record(record: &Record) -> EspResult<Vec<u8>> {
    let mut data = serialize_subrecords(&record.subrecords);
    if record.compressed {
        let compressed = compress_record_data(&data)?;
        data = compressed;
    }
    let data_size = data.len() as u32;
    let mut out = Vec::with_capacity(RECORD_HEADER_SIZE + data.len());
    out.extend_from_slice(&record.header.record_type);
    out.extend_from_slice(&data_size.to_le_bytes());
    out.extend_from_slice(&record.header.flags.to_le_bytes());
    out.extend_from_slice(&record.header.form_id.to_le_bytes());
    out.extend_from_slice(&record.header.stamp.to_le_bytes());
    out.extend_from_slice(&record.header.version_control.to_le_bytes());
    out.extend_from_slice(&record.header.version.to_le_bytes());
    out.extend_from_slice(&record.header.unknown.to_le_bytes());
    out.extend_from_slice(&data);
    Ok(out)
}

fn serialize_subrecords(subrecords: &[Subrecord]) -> Vec<u8> {
    let mut out = Vec::new();
    for subrecord in subrecords {
        let len = subrecord.data.len();
        if len > u16::MAX as usize {
            out.extend_from_slice(b"XXXX");
            out.extend_from_slice(&(4u16).to_le_bytes());
            out.extend_from_slice(&(len as u32).to_le_bytes());
            out.extend_from_slice(&subrecord.sub_type);
            out.extend_from_slice(&0u16.to_le_bytes());
        } else {
            out.extend_from_slice(&subrecord.sub_type);
            out.extend_from_slice(&(len as u16).to_le_bytes());
        }
        out.extend_from_slice(&subrecord.data);
    }
    out
}

fn decompress_record_data(data: &[u8]) -> EspResult<Vec<u8>> {
    if data.len() < 4 {
        return Err(EspError::InvalidRecord);
    }
    let mut decoder = ZlibDecoder::new(&data[4..]);
    let mut out = Vec::new();
    decoder.read_to_end(&mut out)?;
    Ok(out)
}

fn compress_record_data(data: &[u8]) -> EspResult<Vec<u8>> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data)?;
    let compressed = encoder.finish()?;
    let mut out = Vec::with_capacity(4 + compressed.len());
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out.extend_from_slice(&compressed);
    Ok(out)
}

fn is_string_subrecord(tag: &[u8; 4]) -> bool {
    tag == b"FULL" || tag == b"DESC"
}

fn decode_subrecord_string(
    data: &[u8],
    strings_map: &StringsMap,
) -> Option<(String, StringStorage)> {
    if data.len() == 4 {
        let id = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        if let Some((kind, text)) = strings_map.lookup(id) {
            return Some((text.to_string(), StringStorage::Localized { kind, id }));
        }
    }
    let slice = match data.iter().position(|b| *b == 0) {
        Some(end) => &data[..end],
        None => data,
    };
    if slice.is_empty() {
        return None;
    }
    let text = std::str::from_utf8(slice).ok()?;
    if !looks_like_text(text) {
        return None;
    }
    Some((text.to_string(), StringStorage::Inline))
}

fn encode_string(text: &str, null_terminated: bool) -> Vec<u8> {
    let mut out = text.as_bytes().to_vec();
    if null_terminated {
        out.push(0);
    }
    out
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

fn read_tag(bytes: &[u8], offset: usize) -> EspResult<[u8; 4]> {
    if offset + 4 > bytes.len() {
        return Err(EspError::InvalidHeader);
    }
    let mut tag = [0u8; 4];
    tag.copy_from_slice(&bytes[offset..offset + 4]);
    Ok(tag)
}

fn read_u16(bytes: &[u8], offset: usize) -> EspResult<u16> {
    if offset + 2 > bytes.len() {
        return Err(EspError::InvalidHeader);
    }
    Ok(u16::from_le_bytes([bytes[offset], bytes[offset + 1]]))
}

fn read_u32(bytes: &[u8], offset: usize) -> EspResult<u32> {
    if offset + 4 > bytes.len() {
        return Err(EspError::InvalidHeader);
    }
    Ok(u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ]))
}

fn tag_to_string(tag: [u8; 4]) -> String {
    tag.iter().map(|b| *b as char).collect()
}

fn load_strings_bundle(
    path: &Path,
    workspace_root: &Path,
    language: Option<&str>,
) -> EspResult<StringsBundle> {
    let base_name = path
        .file_stem()
        .and_then(|name| name.to_str())
        .ok_or(EspError::InvalidStringsPath)?
        .to_string();
    let language = language.unwrap_or("english").to_lowercase();
    let strings_dir = workspace_root.join("Data").join("Strings");

    let strings_path =
        resolve_strings_path(&strings_dir, &base_name, &language, StringsKind::Strings);
    let dlstrings_path =
        resolve_strings_path(&strings_dir, &base_name, &language, StringsKind::DlStrings);
    let ilstrings_path =
        resolve_strings_path(&strings_dir, &base_name, &language, StringsKind::IlStrings);

    let strings = load_strings_file(strings_path.as_deref(), StringsKind::Strings)?;
    let dlstrings = load_strings_file(dlstrings_path.as_deref(), StringsKind::DlStrings)?;
    let ilstrings = load_strings_file(ilstrings_path.as_deref(), StringsKind::IlStrings)?;

    Ok(StringsBundle {
        strings,
        dlstrings,
        ilstrings,
        base_name,
        language,
    })
}

fn resolve_strings_path(
    strings_dir: &Path,
    base_name: &str,
    language: &str,
    kind: StringsKind,
) -> Option<PathBuf> {
    let file_name = format!("{base_name}_{language}.{}", kind.extension());
    let candidate = strings_dir.join(&file_name);
    if candidate.exists() {
        Some(candidate)
    } else {
        None
    }
}

fn load_strings_file(path: Option<&Path>, kind: StringsKind) -> EspResult<Option<StringsFile>> {
    let Some(path) = path else {
        return Ok(None);
    };
    let bytes = std::fs::read(path)?;
    let file = match kind {
        StringsKind::Strings => read_strings(&bytes),
        StringsKind::DlStrings => read_dlstrings(&bytes),
        StringsKind::IlStrings => read_ilstrings(&bytes),
    }
    .map_err(|_| EspError::InvalidHeader)?;
    Ok(Some(file))
}

fn build_strings_map(bundle: &StringsBundle) -> StringsMap {
    StringsMap::new(bundle)
}

fn update_strings_bundle(
    bundle: &mut StringsBundle,
    kind: StringsKind,
    id: u32,
    text: &str,
) -> EspResult<()> {
    let target = match kind {
        StringsKind::Strings => bundle.strings.as_mut(),
        StringsKind::DlStrings => bundle.dlstrings.as_mut(),
        StringsKind::IlStrings => bundle.ilstrings.as_mut(),
    };
    let Some(file) = target else {
        return Err(EspError::MissingStringsFile(kind));
    };
    if let Some(entry) = file.entries.iter_mut().find(|entry| entry.id == id) {
        entry.text = text.to_string();
        Ok(())
    } else {
        Err(EspError::MissingStringId(id))
    }
}

fn write_strings_bundle(bundle: &StringsBundle, workspace_root: &Path) -> EspResult<()> {
    let output_strings = workspace_root.join("Data").join("Strings");
    std::fs::create_dir_all(&output_strings)?;

    if let Some(file) = &bundle.strings {
        let bytes = write_strings(file).map_err(|_| EspError::InvalidHeader)?;
        let path = output_strings.join(format!(
            "{}_{}.{}",
            bundle.base_name,
            bundle.language,
            StringsKind::Strings.extension()
        ));
        std::fs::write(path, bytes)?;
    }
    if let Some(file) = &bundle.dlstrings {
        let bytes = write_dlstrings(file).map_err(|_| EspError::InvalidHeader)?;
        let path = output_strings.join(format!(
            "{}_{}.{}",
            bundle.base_name,
            bundle.language,
            StringsKind::DlStrings.extension()
        ));
        std::fs::write(path, bytes)?;
    }
    if let Some(file) = &bundle.ilstrings {
        let bytes = write_ilstrings(file).map_err(|_| EspError::InvalidHeader)?;
        let path = output_strings.join(format!(
            "{}_{}.{}",
            bundle.base_name,
            bundle.language,
            StringsKind::IlStrings.extension()
        ));
        std::fs::write(path, bytes)?;
    }
    Ok(())
}

#[derive(Debug)]
struct StringsMap {
    strings: HashMap<u32, String>,
    dlstrings: HashMap<u32, String>,
    ilstrings: HashMap<u32, String>,
}

impl StringsMap {
    fn new(bundle: &StringsBundle) -> Self {
        Self {
            strings: bundle
                .strings
                .as_ref()
                .map(|file| build_string_index(file))
                .unwrap_or_default(),
            dlstrings: bundle
                .dlstrings
                .as_ref()
                .map(|file| build_string_index(file))
                .unwrap_or_default(),
            ilstrings: bundle
                .ilstrings
                .as_ref()
                .map(|file| build_string_index(file))
                .unwrap_or_default(),
        }
    }

    fn lookup(&self, id: u32) -> Option<(StringsKind, &str)> {
        if let Some(text) = self.strings.get(&id) {
            return Some((StringsKind::Strings, text.as_str()));
        }
        if let Some(text) = self.dlstrings.get(&id) {
            return Some((StringsKind::DlStrings, text.as_str()));
        }
        if let Some(text) = self.ilstrings.get(&id) {
            return Some((StringsKind::IlStrings, text.as_str()));
        }
        None
    }
}

fn build_string_index(file: &StringsFile) -> HashMap<u32, String> {
    file.entries
        .iter()
        .map(|entry| (entry.id, entry.text.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strings::StringsEntry;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn temp_path(name: &str, ext: &str) -> PathBuf {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let mut path = std::env::temp_dir();
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        path.push(format!("xtrans-esp-{name}-{id}.{ext}"));
        path
    }

    fn temp_dir(name: &str) -> PathBuf {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let mut path = std::env::temp_dir();
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        path.push(format!("xtrans-esp-{name}-{id}"));
        path
    }

    fn make_subrecord(tag: &[u8; 4], data: &[u8]) -> Vec<u8> {
        let mut out = Vec::with_capacity(6 + data.len());
        out.extend_from_slice(tag);
        out.extend_from_slice(&(data.len() as u16).to_le_bytes());
        out.extend_from_slice(data);
        out
    }

    fn make_record(
        tag: &[u8; 4],
        form_id: u32,
        flags: u32,
        subrecords: Vec<Vec<u8>>,
        compress: bool,
    ) -> Vec<u8> {
        let mut data = Vec::new();
        for sub in subrecords {
            data.extend_from_slice(&sub);
        }
        let data = if compress {
            compress_record_data(&data).expect("compress")
        } else {
            data
        };
        let data_size = data.len() as u32;
        let mut out = Vec::with_capacity(RECORD_HEADER_SIZE + data.len());
        out.extend_from_slice(tag);
        out.extend_from_slice(&data_size.to_le_bytes());
        out.extend_from_slice(&flags.to_le_bytes());
        out.extend_from_slice(&form_id.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&data);
        out
    }

    fn write_strings_fixture(
        dir: &Path,
        base_name: &str,
        language: &str,
        kind: StringsKind,
        file: &StringsFile,
    ) -> PathBuf {
        let output_dir = dir.join("Data").join("Strings");
        std::fs::create_dir_all(&output_dir).expect("create strings dir");
        let path = output_dir.join(format!("{base_name}_{language}.{}", kind.extension()));
        let bytes = match kind {
            StringsKind::Strings => write_strings(file).expect("write strings"),
            StringsKind::DlStrings => write_dlstrings(file).expect("write dlstrings"),
            StringsKind::IlStrings => write_ilstrings(file).expect("write ilstrings"),
        };
        std::fs::write(&path, bytes).expect("write strings file");
        path
    }

    #[test]
    fn t_esp_ex_001_inline_round_trip_edit() {
        let record = make_record(
            b"NPC_",
            0x01020304,
            0,
            vec![make_subrecord(b"FULL", b"Hello\0")],
            false,
        );
        let path = temp_path("inline", "esm");
        std::fs::write(&path, &record).expect("write plugin");
        let workspace_root = temp_dir("inline-root");

        let extracted =
            extract_strings(&path, &workspace_root, Some("english")).expect("extract strings");
        assert_eq!(extracted.len(), 1);
        assert_eq!(extracted[0].text, "Hello");

        let mut updated = extracted[0].clone();
        updated.text = "Hi".to_string();
        let out_dir = temp_dir("inline-out");
        let out_path = apply_translations(
            &path,
            &workspace_root,
            &out_dir,
            vec![updated],
            Some("english"),
        )
        .expect("apply");
        let refreshed =
            extract_strings(&out_path, &workspace_root, Some("english")).expect("extract updated");
        assert_eq!(refreshed[0].text, "Hi");
    }

    #[test]
    fn t_esp_ex_001_localized_round_trip_edit() {
        let base_name = "TestPlugin";
        let language = "english";
        let workspace_root = temp_dir("localized-root");
        let data_dir = workspace_root.join("Data");
        std::fs::create_dir_all(&data_dir).expect("create data dir");
        let plugin_path = data_dir.join(format!("{base_name}.esm"));

        let string_id = 100u32;
        let record = make_record(
            b"NPC_",
            0x0A0B0C0D,
            0,
            vec![make_subrecord(b"FULL", &string_id.to_le_bytes())],
            false,
        );
        std::fs::write(&plugin_path, &record).expect("write plugin");

        let strings_file = StringsFile {
            entries: vec![StringsEntry {
                id: string_id,
                text: "Hello".to_string(),
            }],
        };
        write_strings_fixture(
            &workspace_root,
            base_name,
            language,
            StringsKind::Strings,
            &strings_file,
        );

        let extracted = extract_strings(&plugin_path, &workspace_root, Some(language))
            .expect("extract localized");
        assert_eq!(extracted.len(), 1);
        assert_eq!(extracted[0].text, "Hello");
        match extracted[0].storage {
            StringStorage::Localized { kind, id } => {
                assert_eq!(kind, StringsKind::Strings);
                assert_eq!(id, string_id);
            }
            _ => panic!("expected localized storage"),
        }

        let mut updated = extracted[0].clone();
        updated.text = "こんにちは".to_string();
        let out_dir = data_dir.clone();
        let out_path = apply_translations(
            &plugin_path,
            &workspace_root,
            &out_dir,
            vec![updated],
            Some(language),
        )
        .expect("apply");
        let refreshed =
            extract_strings(&out_path, &workspace_root, Some(language)).expect("extract updated");
        assert_eq!(refreshed[0].text, "こんにちは");
    }

    #[test]
    fn t_esp_ex_001_compressed_round_trip_edit() {
        let flags = RECORD_COMPRESSED;
        let record = make_record(
            b"NPC_",
            0x01020305,
            flags,
            vec![make_subrecord(b"DESC", b"Compressed\0")],
            true,
        );
        let path = temp_path("compressed", "esm");
        std::fs::write(&path, &record).expect("write plugin");
        let workspace_root = temp_dir("compressed-root");

        let extracted =
            extract_strings(&path, &workspace_root, Some("english")).expect("extract strings");
        assert_eq!(extracted.len(), 1);
        assert_eq!(extracted[0].text, "Compressed");

        let mut updated = extracted[0].clone();
        updated.text = "Updated".to_string();
        let out_dir = temp_dir("compressed-out");
        let out_path = apply_translations(
            &path,
            &workspace_root,
            &out_dir,
            vec![updated],
            Some("english"),
        )
        .expect("apply");
        let refreshed =
            extract_strings(&out_path, &workspace_root, Some("english")).expect("extract updated");
        assert_eq!(refreshed[0].text, "Updated");
    }
}
