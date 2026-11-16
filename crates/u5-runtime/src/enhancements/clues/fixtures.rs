//! Synthetic `.TLK` blobs for the clue log's tests (design 8.6): invented
//! words laid out as `formats/tlk.md §7` describes, bit 7 set on text, no
//! game text, keyword or name.

/// Text as it is stored: each byte with bit 7 set.
pub(crate) fn text(s: &str) -> Vec<u8> {
    s.bytes().map(|b| b | 0x80).collect()
}

/// `parts` joined into one record.
pub(crate) fn record(parts: &[&[u8]]) -> Vec<u8> {
    parts.concat()
}

/// A blob: the five leading records, then `pairs` as keyword/response
/// records, then `blocks` (each a list of records, the first beginning
/// with its `0x90 <label>` declaration).
pub(crate) fn blob(pairs: &[(&str, Vec<u8>)], blocks: &[Vec<Vec<u8>>]) -> Vec<Vec<u8>> {
    let mut fields = vec![
        text("Vorpelin"),
        text("a dusty quillwright"),
        text("Hail, wanderer."),
        text("I mend quills."),
        text("Fare well."),
    ];
    for (keyword, response) in pairs {
        fields.push(text(keyword));
        fields.push(response.clone());
    }
    for block in blocks {
        fields.extend(block.iter().cloned());
    }
    fields
}

/// A block declared with `label`: head bytes, then the other records.
pub(crate) fn block(label: u8, head: &[u8], rest: &[Vec<u8>]) -> Vec<Vec<u8>> {
    let mut first = vec![0x90, label];
    first.extend_from_slice(head);
    std::iter::once(first).chain(rest.iter().cloned()).collect()
}

/// The decoded form of raw fields, as the session's `decoded_fields`
/// would hold for plain text: bit 7 stripped.
pub(crate) fn decoded(fields: &[Vec<u8>]) -> Vec<String> {
    fields
        .iter()
        .map(|field| field.iter().map(|b| char::from(b & 0x7F)).collect())
        .collect()
}
