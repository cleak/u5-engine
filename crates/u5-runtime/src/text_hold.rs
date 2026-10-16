//! Holds embedded in an already-composed message: a timed pause that a key
//! cuts short, or a blocking one-key wait.
//!
//! `conversation.md §7.3` gives the conversation byte runner two such codes:
//! `0x83` PAUSE, "a **timed** pause of up to 28 passes. Each pass redraws the
//! world view and polls the keyboard **without blocking**: a waiting key is
//! consumed and ends the pause at once; otherwise the pass waits one BIOS
//! tick ... after the 28th pass the pause ends by itself, key or no key. The
//! keyboard type-ahead buffer is emptied on either exit", and `0x8F`
//! WAIT-KEY, "Block for exactly one command keystroke (any key; its value is
//! discarded) ... The stream then continues." `conversation.md §6`'s reserved
//! rebuke runs the same bounded, key-abortable pause loop, and
//! `formats/signs-dat.md §4` has sign byte `0x0D` pause for a keypress.
//!
//! The producers here compose their whole output in one call, so a hold is
//! carried *inside* the composed text as a one-byte marker, and the message
//! layer splits the text at the markers when it prints it: the first page
//! prints at once and each later page waits for its hold to end. Markers are
//! ASCII control bytes that no producer emits as text and no font draws, so
//! they keep the one-byte-per-glyph alignment of
//! [`crate::tlk_runner::TlkRenderedText`] and survive quote framing and
//! whitespace trimming untouched.

use std::collections::VecDeque;

use crate::tlk_runner::{TlkRenderedGlyph, TlkRenderedText};

/// Marker for a timed, key-abortable pause (`conversation.md §7.3` `0x83`).
pub const TEXT_HOLD_TIMED_PAUSE_MARKER: u8 = 0x01;
/// Marker for a blocking one-key wait (`conversation.md §7.3` `0x8F`,
/// `formats/signs-dat.md §4` `0x0D`).
pub const TEXT_HOLD_WAIT_KEY_MARKER: u8 = 0x02;

/// `conversation.md §7.3`: "A **timed** pause of up to 28 passes", each pass
/// waiting one BIOS tick. `conversation.md §6`'s rebuke loop "attempts at most
/// twenty-eight" passes.
pub const TEXT_HOLD_TIMED_PAUSE_PASSES: u16 = 28;

/// What ends a hold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextHoldKind {
    /// Ends after [`TEXT_HOLD_TIMED_PAUSE_PASSES`] BIOS ticks, or at once on
    /// a key, which it consumes.
    TimedPause,
    /// Ends only on a key, which it consumes.
    WaitKey,
}

impl TextHoldKind {
    pub const fn marker(self) -> u8 {
        match self {
            TextHoldKind::TimedPause => TEXT_HOLD_TIMED_PAUSE_MARKER,
            TextHoldKind::WaitKey => TEXT_HOLD_WAIT_KEY_MARKER,
        }
    }

    pub const fn from_marker(byte: u8) -> Option<Self> {
        match byte {
            TEXT_HOLD_TIMED_PAUSE_MARKER => Some(TextHoldKind::TimedPause),
            TEXT_HOLD_WAIT_KEY_MARKER => Some(TextHoldKind::WaitKey),
            _ => None,
        }
    }

    pub fn marker_glyph(self) -> TlkRenderedGlyph {
        TlkRenderedGlyph::ordinary(self.marker())
    }

    pub fn marker_str(self) -> &'static str {
        match self {
            TextHoldKind::TimedPause => "\u{1}",
            TextHoldKind::WaitKey => "\u{2}",
        }
    }
}

/// Whether `text` carries any hold marker.
pub fn text_has_hold_markers(text: &str) -> bool {
    text.bytes().any(|byte| TextHoldKind::from_marker(byte).is_some())
}

/// `text` with every hold marker removed: what the composed output reads as
/// once all its holds have ended.
pub fn strip_hold_markers(text: &str) -> String {
    text.chars()
        .filter(|ch| !ch.is_ascii() || TextHoldKind::from_marker(*ch as u8).is_none())
        .collect()
}

/// Split rendered text at its hold markers. The first page has no hold in
/// front of it; every later page carries the hold that precedes it.
pub fn split_rendered_at_holds(
    rendered: &TlkRenderedText,
) -> (TlkRenderedText, Vec<(TextHoldKind, TlkRenderedText)>) {
    let mut first = TlkRenderedText::default();
    let mut rest: Vec<(TextHoldKind, TlkRenderedText)> = Vec::new();
    for glyph in &rendered.glyphs {
        if let Some(kind) = TextHoldKind::from_marker(glyph.byte) {
            rest.push((kind, TlkRenderedText::default()));
            continue;
        }
        let page = rest.last_mut().map_or(&mut first, |(_, page)| page);
        page.text.push(char::from(glyph.byte));
        page.glyphs.push(*glyph);
    }
    (first, rest)
}

/// One page still to print and the hold in front of it.
#[derive(Clone, Debug, PartialEq)]
pub struct HeldPage {
    pub hold: TextHoldKind,
    pub text: TlkRenderedText,
}

/// A message whose remaining pages are waiting behind holds.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PendingTextHold {
    pub pages: VecDeque<HeldPage>,
    /// Clock reading at which the head timed pause ends. `None` until the
    /// first advance after the head became current, so the wait starts from
    /// the frontend's own clock (see [`crate::narration`]).
    pub due_at_secs: Option<f64>,
    /// The previous page left its row open (it printed text and did not end
    /// with a line feed), so the next page's first segment continues it.
    /// Known exactly: it is the page this hold itself printed.
    pub row_open: bool,
    /// Prompt to print once the last page is out, for a caller whose prompt
    /// is not part of the composed text.
    pub then_message: Option<String>,
}

impl PendingTextHold {
    /// The hold that is currently running.
    pub fn current(&self) -> Option<TextHoldKind> {
        self.pages.front().map(|page| page.hold)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_keeps_pages_and_holds_in_order() {
        let rendered = TlkRenderedText::plain("A\u{1}B\nC\u{2}\u{1}D");
        let (first, rest) = split_rendered_at_holds(&rendered);
        assert_eq!(first.text, "A");
        let rest: Vec<_> = rest
            .into_iter()
            .map(|(kind, page)| (kind, page.text))
            .collect();
        assert_eq!(
            rest,
            vec![
                (TextHoldKind::TimedPause, "B\nC".to_string()),
                (TextHoldKind::WaitKey, String::new()),
                (TextHoldKind::TimedPause, "D".to_string()),
            ]
        );
    }

    #[test]
    fn markers_are_not_whitespace_and_strip_cleanly() {
        let text = "\"Hi\u{1}\"";
        assert_eq!(text.trim_end_matches('\n'), text);
        assert_eq!(strip_hold_markers(text), "\"Hi\"");
        assert!(text_has_hold_markers(text));
        assert!(!text_has_hold_markers("plain"));
    }
}
