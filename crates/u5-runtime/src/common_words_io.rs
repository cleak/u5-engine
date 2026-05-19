//! Optional clean common-word dictionary sidecar for TLK and SHOPPE text.
//!
//! The public specs define the shared 128-entry dictionary mechanics, but the
//! resident vocabulary is data content. This loader accepts a clean published
//! table without embedding the words in the engine.

use std::collections::HashSet;
use std::path::Path;
use std::{array, fs, io};

use crate::COMMON_WORD_DICTIONARY_ENTRIES;
use crate::tlk_control_codes::{shoppe_dictionary_index, tlk_dictionary_index};

pub const COMMON_WORD_DICTIONARY_FILE: &str = "common_words.tsv";

pub type CommonWordDictionary = [String; COMMON_WORD_DICTIONARY_ENTRIES];

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommonWordDictionaryError {
    MissingTab { line: usize },
    InvalidIndex { line: usize, value: String },
    IndexOutOfRange { line: usize, index: usize },
    DuplicateIndex { line: usize, index: usize },
    MissingIndex { index: usize },
    ContainsNul { line: usize, index: usize },
}

impl std::fmt::Display for CommonWordDictionaryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingTab { line } => write!(
                f,
                "{COMMON_WORD_DICTIONARY_FILE} line {line} must be: INDEX<TAB>WORD"
            ),
            Self::InvalidIndex { line, value } => write!(
                f,
                "{COMMON_WORD_DICTIONARY_FILE} line {line} has invalid index `{value}`"
            ),
            Self::IndexOutOfRange { line, index } => write!(
                f,
                "{COMMON_WORD_DICTIONARY_FILE} line {line} index {index} is outside 0..127"
            ),
            Self::DuplicateIndex { line, index } => write!(
                f,
                "{COMMON_WORD_DICTIONARY_FILE} line {line} duplicates index {index}"
            ),
            Self::MissingIndex { index } => {
                write!(f, "{COMMON_WORD_DICTIONARY_FILE} is missing index {index}")
            }
            Self::ContainsNul { line, index } => write!(
                f,
                "{COMMON_WORD_DICTIONARY_FILE} line {line} index {index} contains a NUL byte"
            ),
        }
    }
}

impl std::error::Error for CommonWordDictionaryError {}

pub fn parse_common_word_dictionary(
    text: &str,
) -> Result<CommonWordDictionary, CommonWordDictionaryError> {
    let mut dictionary: CommonWordDictionary = array::from_fn(|_| String::new());
    let mut seen = HashSet::new();

    for (line_number, raw_line) in text.lines().enumerate() {
        let line_number = line_number + 1;
        let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some((index_text, word)) = line.split_once('\t') else {
            return Err(CommonWordDictionaryError::MissingTab { line: line_number });
        };
        let index =
            index_text
                .parse::<usize>()
                .map_err(|_| CommonWordDictionaryError::InvalidIndex {
                    line: line_number,
                    value: index_text.to_string(),
                })?;
        if index >= COMMON_WORD_DICTIONARY_ENTRIES {
            return Err(CommonWordDictionaryError::IndexOutOfRange {
                line: line_number,
                index,
            });
        }
        if !seen.insert(index) {
            return Err(CommonWordDictionaryError::DuplicateIndex {
                line: line_number,
                index,
            });
        }
        if word.as_bytes().contains(&0) {
            return Err(CommonWordDictionaryError::ContainsNul {
                line: line_number,
                index,
            });
        }
        dictionary[index] = word.to_string();
    }

    for index in 0..COMMON_WORD_DICTIONARY_ENTRIES {
        if !seen.contains(&index) {
            return Err(CommonWordDictionaryError::MissingIndex { index });
        }
    }

    Ok(dictionary)
}

pub fn common_word_dictionary_refs(
    dictionary: &CommonWordDictionary,
) -> [&str; COMMON_WORD_DICTIONARY_ENTRIES] {
    array::from_fn(|index| dictionary[index].as_str())
}

pub fn load_common_word_dictionary_optional(
    game_dir: &Path,
) -> io::Result<Option<CommonWordDictionary>> {
    let path = game_dir.join(COMMON_WORD_DICTIONARY_FILE);
    match fs::read_to_string(&path) {
        Ok(text) => parse_common_word_dictionary(&text)
            .map(Some)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err)),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err),
    }
}

pub fn missing_common_word_dictionary_error(context: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("{COMMON_WORD_DICTIONARY_FILE} is required to render tokenized {context} text"),
    )
}

pub fn tlk_stream_uses_common_word_dictionary(bytes: &[u8]) -> bool {
    bytes
        .iter()
        .copied()
        .any(|byte| tlk_dictionary_index(byte).is_some())
}

pub fn tlk_fields_use_common_word_dictionary(fields: &[Vec<u8>]) -> bool {
    fields
        .iter()
        .any(|field| tlk_stream_uses_common_word_dictionary(field))
}

pub fn shoppe_bark_uses_common_word_dictionary(bytes: &[u8]) -> bool {
    bytes
        .iter()
        .copied()
        .take_while(|byte| *byte != 0)
        .any(|byte| shoppe_dictionary_index(byte).is_some())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn full_dictionary_text() -> String {
        (0..COMMON_WORD_DICTIONARY_ENTRIES)
            .map(|index| {
                let word = if index == 11 {
                    String::new()
                } else {
                    format!("word{index}")
                };
                format!("{index}\t{word}\n")
            })
            .collect()
    }

    #[test]
    fn parses_complete_dictionary_and_preserves_empty_sentinels() {
        let dictionary = parse_common_word_dictionary(&full_dictionary_text()).unwrap();
        assert_eq!(dictionary[0], "word0");
        assert_eq!(dictionary[11], "");
        assert_eq!(dictionary[127], "word127");
        let refs = common_word_dictionary_refs(&dictionary);
        assert_eq!(refs[127], "word127");
    }

    #[test]
    fn rejects_missing_duplicate_and_out_of_range_rows() {
        let mut missing = full_dictionary_text();
        missing = missing.replace("127\tword127\n", "");
        assert_eq!(
            parse_common_word_dictionary(&missing).unwrap_err(),
            CommonWordDictionaryError::MissingIndex { index: 127 }
        );

        let duplicate = format!("{}0\tagain\n", full_dictionary_text());
        assert!(matches!(
            parse_common_word_dictionary(&duplicate).unwrap_err(),
            CommonWordDictionaryError::DuplicateIndex { index: 0, .. }
        ));

        let out_of_range = format!("{}128\ttoo-far\n", full_dictionary_text());
        assert!(matches!(
            parse_common_word_dictionary(&out_of_range).unwrap_err(),
            CommonWordDictionaryError::IndexOutOfRange { index: 128, .. }
        ));
    }

    #[test]
    fn detects_dictionary_tokens_in_tlk_and_shoppe_streams() {
        assert!(tlk_stream_uses_common_word_dictionary(&[0x01]));
        assert!(!tlk_stream_uses_common_word_dictionary(&[
            0x00,
            0x80,
            b'A' | 0x80
        ]));
        assert!(tlk_fields_use_common_word_dictionary(&[
            vec![b'A' | 0x80],
            vec![0x7f]
        ]));

        assert!(shoppe_bark_uses_common_word_dictionary(&[0x80]));
        assert!(shoppe_bark_uses_common_word_dictionary(&[b'A', 0xff, 0]));
        assert!(!shoppe_bark_uses_common_word_dictionary(&[b'A', 0, 0x80]));
    }
}
