//! GIF-style LZW bit reader and codec used by tile/graphic asset envelopes.

use std::io;

use crate::*;

pub struct LzwBitReader<'a> {
    pub bytes: &'a [u8],
    pub bit_pos: usize,
}

pub fn reset_lzw_dictionary(dictionary: &mut Vec<Vec<u8>>) {
    dictionary.clear();
    for byte in 0..=255u16 {
        dictionary.push(vec![byte as u8]);
    }
    dictionary.push(Vec::new());
    dictionary.push(Vec::new());
}

pub fn decode_lzw_envelope(bytes: &[u8], resource_name: &str) -> io::Result<Vec<u8>> {
    if bytes.len() < LZW_ENVELOPE_LENGTH_HEADER_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{resource_name} LZW envelope is shorter than its length header"),
        ));
    }
    let expected_len = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
    decode_gif_lzw_payload(
        &bytes[LZW_ENVELOPE_LENGTH_HEADER_BYTES..],
        expected_len,
        resource_name,
    )
}

pub fn decode_gif_lzw_payload(
    payload: &[u8],
    expected_len: usize,
    resource_name: &str,
) -> io::Result<Vec<u8>> {
    let mut reader = LzwBitReader::new(payload);
    let mut dictionary = Vec::with_capacity(LZW_MAX_CODES as usize);
    reset_lzw_dictionary(&mut dictionary);

    let mut code_size = LZW_INITIAL_CODE_SIZE;
    let mut next_code = LZW_FIRST_USER_CODE;
    let mut previous: Option<Vec<u8>> = None;
    let mut output = Vec::with_capacity(expected_len);
    let mut saw_end = false;

    loop {
        let Some(code) = reader.read_code(code_size) else {
            break;
        };

        match code {
            LZW_CLEAR_CODE => {
                reset_lzw_dictionary(&mut dictionary);
                code_size = LZW_INITIAL_CODE_SIZE;
                next_code = LZW_FIRST_USER_CODE;
                previous = None;
                continue;
            }
            LZW_END_CODE => {
                saw_end = true;
                break;
            }
            _ => {}
        }

        let entry = if code == next_code {
            let Some(previous_entry) = previous.as_ref() else {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("{resource_name} LZW stream used KwKwK before a previous entry"),
                ));
            };
            let mut entry = previous_entry.clone();
            entry.push(previous_entry[0]);
            entry
        } else if code < next_code
            && (code as usize) < dictionary.len()
            && !dictionary[code as usize].is_empty()
        {
            dictionary[code as usize].clone()
        } else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{resource_name} LZW stream referenced invalid code {code}"),
            ));
        };

        output.extend_from_slice(&entry);
        if output.len() > expected_len {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{resource_name} LZW output exceeded declared length {expected_len}"),
            ));
        }

        if let Some(previous_entry) = previous.as_ref() {
            if next_code < LZW_MAX_CODES {
                let mut dictionary_entry = previous_entry.clone();
                dictionary_entry.push(entry[0]);
                dictionary.push(dictionary_entry);
                next_code += 1;
                if next_code == (1u16 << code_size) && code_size < LZW_MAX_CODE_SIZE {
                    code_size += 1;
                }
            }
        }

        previous = Some(entry);
    }

    if !saw_end {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{resource_name} LZW payload ended before the end code"),
        ));
    }
    if output.len() != expected_len {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{resource_name} LZW declared length {expected_len}, decoded {} bytes",
                output.len()
            ),
        ));
    }

    Ok(output)
}

impl<'a> LzwBitReader<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, bit_pos: 0 }
    }

    pub fn read_code(&mut self, code_size: u8) -> Option<u16> {
        let bit_count = code_size as usize;
        if self.bit_pos + bit_count > self.bytes.len() * 8 {
            return None;
        }

        let mut code = 0u16;
        for bit_offset in 0..bit_count {
            let source_bit = self.bit_pos + bit_offset;
            let bit = (self.bytes[source_bit / 8] >> (source_bit % 8)) & 1;
            code |= (bit as u16) << bit_offset;
        }
        self.bit_pos += bit_count;
        Some(code)
    }
}
