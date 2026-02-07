#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum Encoding {
    Utf8,
    Latin1,
}

#[derive(Debug, PartialEq, Eq)]
pub enum EncodingError {
    InvalidUtf8,
    UnrepresentableChar,
}

pub fn decode(bytes: &[u8], encoding: Encoding) -> Result<String, EncodingError> {
    match encoding {
        Encoding::Utf8 => std::str::from_utf8(bytes)
            .map(|s| s.to_string())
            .map_err(|_| EncodingError::InvalidUtf8),
        Encoding::Latin1 => Ok(bytes.iter().map(|b| *b as char).collect()),
    }
}

pub fn encode(text: &str, encoding: Encoding) -> Result<Vec<u8>, EncodingError> {
    match encoding {
        Encoding::Utf8 => Ok(text.as_bytes().to_vec()),
        Encoding::Latin1 => {
            let mut out = Vec::with_capacity(text.len());
            for ch in text.chars() {
                if (ch as u32) <= 0xFF {
                    out.push(ch as u8);
                } else {
                    return Err(EncodingError::UnrepresentableChar);
                }
            }
            Ok(out)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t_enc_001_latin1_round_trip() {
        let bytes = [0x48, 0x65, 0x6C, 0x6C, 0x6F, 0xE9];
        let decoded = decode(&bytes, Encoding::Latin1).expect("decode latin1");
        let encoded = encode(&decoded, Encoding::Latin1).expect("encode latin1");
        assert_eq!(encoded, bytes);
    }
}
