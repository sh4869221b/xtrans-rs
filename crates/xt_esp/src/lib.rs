mod strings;
pub mod esp;

pub use esp::{
    apply_translations, extract_strings, EspError, ExtractedString, StringStorage, StringsKind,
};
