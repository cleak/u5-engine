//! The section codec the `AMAP` (design 4.3.8) and `GEMK` (10.3.7)
//! payloads share. Integers are little-endian.
//!
//! ```text
//! offset size field
//! 0      1    tag
//! 1      3    key (meaning set by the tag)
//! 4      4    payload length n (u32)
//! 8      n    payload
//! ```
//!
//! A payload is a section count followed by that many sections. The codec
//! knows no tags: a decoder matches the tags it understands and skips the
//! rest, which the length makes possible, so a later version can add a
//! tag and stay readable by version-1 code. There is no file I/O here;
//! the foundation's `SidecarStore` owns the container.

use std::fmt;

/// Bytes before each section's payload: tag, key and length.
pub const SECTION_HEADER_LEN: usize = 8;

/// One section borrowed from a payload.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SectionRef<'a> {
    pub tag: u8,
    pub key: [u8; 3],
    pub payload: &'a [u8],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SectionError {
    /// The bytes end inside a section header or payload. `at` is the
    /// offset where the incomplete section starts.
    Truncated { at: usize },
    /// A payload longer than a `u32` can describe.
    TooLong { len: usize },
    /// More sections than the `u16` count can describe.
    TooManySections,
}

impl fmt::Display for SectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Truncated { at } => write!(f, "section truncated at byte {at}"),
            Self::TooLong { len } => write!(f, "section payload of {len} bytes is too long"),
            Self::TooManySections => write!(f, "more than 65535 sections"),
        }
    }
}

impl std::error::Error for SectionError {}

/// Appends one section to `out`.
pub fn write_section(
    out: &mut Vec<u8>,
    tag: u8,
    key: [u8; 3],
    payload: &[u8],
) -> Result<(), SectionError> {
    let len =
        u32::try_from(payload.len()).map_err(|_| SectionError::TooLong { len: payload.len() })?;
    out.reserve(SECTION_HEADER_LEN + payload.len());
    out.push(tag);
    out.extend_from_slice(&key);
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(payload);
    Ok(())
}

/// Reads `count` sections from the start of `bytes`. Returns them and the
/// number of bytes they used; bytes after the last section are left to
/// the caller.
pub fn read_sections(
    bytes: &[u8],
    count: u16,
) -> Result<(Vec<SectionRef<'_>>, usize), SectionError> {
    let mut sections = Vec::with_capacity(count as usize);
    let mut at = 0;
    for _ in 0..count {
        let (section, next) = read_one(bytes, at)?;
        sections.push(section);
        at = next;
    }
    Ok((sections, at))
}

fn read_one(bytes: &[u8], at: usize) -> Result<(SectionRef<'_>, usize), SectionError> {
    let truncated = SectionError::Truncated { at };
    let header = bytes
        .get(at..at.checked_add(SECTION_HEADER_LEN).ok_or(truncated)?)
        .ok_or(truncated)?;
    let len = u32::from_le_bytes([header[4], header[5], header[6], header[7]]) as usize;
    let start = at + SECTION_HEADER_LEN;
    let end = start.checked_add(len).ok_or(truncated)?;
    let payload = bytes.get(start..end).ok_or(truncated)?;
    Ok((
        SectionRef {
            tag: header[0],
            key: [header[1], header[2], header[3]],
            payload,
        },
        end,
    ))
}

/// Builds the sections of a payload and counts them, so the `u16` count
/// written before them always matches.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SectionList {
    count: u16,
    bytes: Vec<u8>,
}

impl SectionList {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds one section.
    pub fn push(&mut self, tag: u8, key: [u8; 3], payload: &[u8]) -> Result<(), SectionError> {
        let count = self
            .count
            .checked_add(1)
            .ok_or(SectionError::TooManySections)?;
        write_section(&mut self.bytes, tag, key, payload)?;
        self.count = count;
        Ok(())
    }

    pub fn count(&self) -> u16 {
        self.count
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Appends the `u16` count and then the sections to `out`.
    pub fn write_counted(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.count.to_le_bytes());
        out.extend_from_slice(&self.bytes);
    }

    /// The sections alone, without the count.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_section_has_the_documented_layout() {
        let mut out = Vec::new();
        write_section(&mut out, 3, [2, 5, 0], &[0xAA, 0xBB]).unwrap();
        assert_eq!(out, [3, 2, 5, 0, 2, 0, 0, 0, 0xAA, 0xBB]);
    }

    #[test]
    fn sections_round_trip() {
        let rows = [1u8, 2, 4, 8, 16, 32, 64, 128];
        let mut list = SectionList::new();
        list.push(3, [4, 6, 0], &rows).unwrap();
        list.push(5, [0, 0, 0], &[9, 17, 25]).unwrap();
        list.push(2, [7, 0xFF, 0], &[]).unwrap();
        list.push(1, [1, 0, 0], &vec![0x5A; 8192]).unwrap();
        assert_eq!(list.count(), 4);

        let mut payload = vec![0x01];
        list.write_counted(&mut payload);
        let count = u16::from_le_bytes([payload[1], payload[2]]);
        assert_eq!(count, 4);
        let (sections, used) = read_sections(&payload[3..], count).unwrap();
        assert_eq!(used, payload.len() - 3);
        assert_eq!(
            sections[0],
            SectionRef {
                tag: 3,
                key: [4, 6, 0],
                payload: &rows
            }
        );
        assert_eq!(sections[1].payload, [9, 17, 25]);
        assert_eq!(sections[2].key, [7, 0xFF, 0]);
        assert!(sections[2].payload.is_empty());
        assert_eq!(sections[3].payload.len(), 8192);
    }

    #[test]
    fn an_unknown_tag_is_skipped_by_its_length() {
        // A version-1 reader that knows tags 3 and 5 meets a later tag 6
        // between them, carrying bytes that look like a section header.
        let mut list = SectionList::new();
        list.push(3, [0, 1, 0], &[0xFF; 8]).unwrap();
        list.push(6, [0, 0, 0], &[3, 0, 0, 0, 8, 0, 0, 0, 1])
            .unwrap();
        list.push(5, [0, 0, 0], &[12]).unwrap();
        let (sections, _) = read_sections(list.as_bytes(), list.count()).unwrap();
        let known: Vec<(u8, &[u8])> = sections
            .iter()
            .filter(|section| matches!(section.tag, 3 | 5))
            .map(|section| (section.tag, section.payload))
            .collect();
        assert_eq!(known, [(3, &[0xFF; 8][..]), (5, &[12][..])]);
    }

    #[test]
    fn truncation_is_an_error_at_every_cut() {
        let mut list = SectionList::new();
        list.push(3, [0, 0, 0], &[1, 2, 3, 4, 5, 6, 7, 8]).unwrap();
        list.push(5, [0, 0, 0], &[9, 10]).unwrap();
        let bytes = list.as_bytes();
        let second = SECTION_HEADER_LEN + 8;
        for cut in 0..bytes.len() {
            let expect_at = if cut < second { 0 } else { second };
            assert_eq!(
                read_sections(&bytes[..cut], 2),
                Err(SectionError::Truncated { at: expect_at }),
                "cut at {cut}"
            );
        }
        assert!(read_sections(bytes, 2).is_ok());
    }

    #[test]
    fn a_huge_declared_length_is_truncation_not_a_panic() {
        let bytes = [1, 0, 0, 0, 0xFF, 0xFF, 0xFF, 0xFF, 0];
        assert_eq!(
            read_sections(&bytes, 1),
            Err(SectionError::Truncated { at: 0 })
        );
    }

    #[test]
    fn trailing_bytes_are_left_to_the_caller() {
        let mut out = Vec::new();
        write_section(&mut out, 1, [0; 3], &[7]).unwrap();
        out.extend_from_slice(&[0xEE, 0xEE]);
        let (sections, used) = read_sections(&out, 1).unwrap();
        assert_eq!(sections.len(), 1);
        assert_eq!(used, SECTION_HEADER_LEN + 1);
        assert_eq!(read_sections(&out, 0).unwrap(), (Vec::new(), 0));
    }
}
