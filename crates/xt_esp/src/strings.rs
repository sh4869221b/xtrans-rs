#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StringsEntry {
    pub id: u32,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StringsFile {
    pub entries: Vec<StringsEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StringsError {
    UnexpectedEof,
    InvalidHeader,
    InvalidOffset,
    InvalidLength,
    MissingTerminator,
    Utf8,
    DuplicateId(u32),
}

pub fn read_strings(input: &[u8]) -> Result<StringsFile, StringsError> {
    if input.len() < 8 {
        return Err(StringsError::InvalidHeader);
    }
    let count = read_u32(input, 0)?;
    let data_size = read_u32(input, 4)? as usize;
    let directory_size = count
        .checked_mul(8)
        .ok_or(StringsError::InvalidHeader)? as usize;
    let data_start = 8usize
        .checked_add(directory_size)
        .ok_or(StringsError::InvalidHeader)?;
    let data_end = data_start
        .checked_add(data_size)
        .ok_or(StringsError::InvalidHeader)?;
    if data_end > input.len() {
        return Err(StringsError::UnexpectedEof);
    }

    let mut entries = Vec::with_capacity(count as usize);
    for i in 0..count as usize {
        let base = 8usize + i * 8;
        let id = read_u32(input, base)?;
        let offset = read_u32(input, base + 4)? as usize;
        if offset >= data_size {
            return Err(StringsError::InvalidOffset);
        }
        let start = data_start + offset;
        let mut end = start;
        while end < data_end && input[end] != 0 {
            end += 1;
        }
        if end >= data_end {
            return Err(StringsError::MissingTerminator);
        }
        let text = std::str::from_utf8(&input[start..end])
            .map_err(|_| StringsError::Utf8)?
            .to_string();
        entries.push(StringsEntry { id, text });
    }

    Ok(StringsFile { entries })
}

pub fn read_dlstrings(input: &[u8]) -> Result<StringsFile, StringsError> {
    read_length_prefixed_strings(input)
}

pub fn read_ilstrings(input: &[u8]) -> Result<StringsFile, StringsError> {
    read_length_prefixed_strings(input)
}

pub fn write_strings(file: &StringsFile) -> Result<Vec<u8>, StringsError> {
    let mut entries = file.entries.clone();
    entries.sort_by_key(|entry| entry.id);
    for window in entries.windows(2) {
        if window[0].id == window[1].id {
            return Err(StringsError::DuplicateId(window[0].id));
        }
    }

    let mut directory = Vec::with_capacity(entries.len());
    let mut data_block: Vec<u8> = Vec::new();
    for entry in &entries {
        let offset = data_block.len() as u32;
        data_block.extend_from_slice(entry.text.as_bytes());
        data_block.push(0);
        directory.push((entry.id, offset));
    }

    let count = entries.len() as u32;
    let data_size = data_block.len() as u32;
    let mut output = Vec::with_capacity(8 + directory.len() * 8 + data_block.len());
    output.extend_from_slice(&count.to_le_bytes());
    output.extend_from_slice(&data_size.to_le_bytes());
    for (id, offset) in directory {
        output.extend_from_slice(&id.to_le_bytes());
        output.extend_from_slice(&offset.to_le_bytes());
    }
    output.extend_from_slice(&data_block);

    Ok(output)
}

pub fn write_dlstrings(file: &StringsFile) -> Result<Vec<u8>, StringsError> {
    write_length_prefixed_strings(file)
}

pub fn write_ilstrings(file: &StringsFile) -> Result<Vec<u8>, StringsError> {
    write_length_prefixed_strings(file)
}

fn read_u32(input: &[u8], offset: usize) -> Result<u32, StringsError> {
    if offset + 4 > input.len() {
        return Err(StringsError::UnexpectedEof);
    }
    let mut bytes = [0u8; 4];
    bytes.copy_from_slice(&input[offset..offset + 4]);
    Ok(u32::from_le_bytes(bytes))
}

fn read_length_prefixed_strings(input: &[u8]) -> Result<StringsFile, StringsError> {
    if input.len() < 8 {
        return Err(StringsError::InvalidHeader);
    }
    let count = read_u32(input, 0)?;
    let data_size = read_u32(input, 4)? as usize;
    let directory_size = count
        .checked_mul(8)
        .ok_or(StringsError::InvalidHeader)? as usize;
    let data_start = 8usize
        .checked_add(directory_size)
        .ok_or(StringsError::InvalidHeader)?;
    let data_end = data_start
        .checked_add(data_size)
        .ok_or(StringsError::InvalidHeader)?;
    if data_end > input.len() {
        return Err(StringsError::UnexpectedEof);
    }

    let mut entries = Vec::with_capacity(count as usize);
    for i in 0..count as usize {
        let base = 8usize + i * 8;
        let id = read_u32(input, base)?;
        let offset = read_u32(input, base + 4)? as usize;
        if offset >= data_size {
            return Err(StringsError::InvalidOffset);
        }
        let len_offset = data_start + offset;
        let len = read_u32(input, len_offset)? as usize;
        if len == 0 {
            return Err(StringsError::InvalidLength);
        }
        let text_start = len_offset + 4;
        let text_end = text_start
            .checked_add(len)
            .ok_or(StringsError::UnexpectedEof)?;
        if text_end > data_end {
            return Err(StringsError::UnexpectedEof);
        }
        let slice = &input[text_start..text_end];
        if *slice.last().unwrap_or(&0) != 0 {
            return Err(StringsError::MissingTerminator);
        }
        let text = std::str::from_utf8(&slice[..slice.len() - 1])
            .map_err(|_| StringsError::Utf8)?
            .to_string();
        entries.push(StringsEntry { id, text });
    }

    Ok(StringsFile { entries })
}

fn write_length_prefixed_strings(file: &StringsFile) -> Result<Vec<u8>, StringsError> {
    let mut entries = file.entries.clone();
    entries.sort_by_key(|entry| entry.id);
    for window in entries.windows(2) {
        if window[0].id == window[1].id {
            return Err(StringsError::DuplicateId(window[0].id));
        }
    }

    let mut directory = Vec::with_capacity(entries.len());
    let mut data_block: Vec<u8> = Vec::new();
    for entry in &entries {
        let offset = data_block.len() as u32;
        let bytes = entry.text.as_bytes();
        let len = bytes
            .len()
            .checked_add(1)
            .ok_or(StringsError::UnexpectedEof)? as u32;
        data_block.extend_from_slice(&len.to_le_bytes());
        data_block.extend_from_slice(bytes);
        data_block.push(0);
        directory.push((entry.id, offset));
    }

    let count = entries.len() as u32;
    let data_size = data_block.len() as u32;
    let mut output = Vec::with_capacity(8 + directory.len() * 8 + data_block.len());
    output.extend_from_slice(&count.to_le_bytes());
    output.extend_from_slice(&data_size.to_le_bytes());
    for (id, offset) in directory {
        output.extend_from_slice(&id.to_le_bytes());
        output.extend_from_slice(&offset.to_le_bytes());
    }
    output.extend_from_slice(&data_block);

    Ok(output)
}
