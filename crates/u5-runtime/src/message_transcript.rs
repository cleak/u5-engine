//! The scrolling message-window transcript and the resident verb echo.
//!
//! `commands.md §5` requires each command block to print its resident verb
//! prefix before it invokes the handler or the refusal path, and the
//! original's message window is a scrolling transcript rather than a
//! single repainted line (`text-output.md §2`). This module owns both:
//! the dispatchers open an echo entry before dispatch, the handler's own
//! output is folded into or appended after that entry, and renderers read
//! the result through [`PlayState::message_entries`].

use crate::play_state_impl::chunk_02::high_byte_direction_from_key;
use crate::play_state_struct::PendingCommandEcho;
use crate::*;

impl PlayState {
    /// The message-window transcript, oldest entry first. Entries with
    /// `is_command_echo` set are the lines the original draws with the
    /// leading `>` command glyph; the rest are handler continuation
    /// lines drawn without it.
    pub fn message_entries(&self) -> &[MessageEntry] {
        &self.message_transcript
    }

    /// Bumped on every transcript push. Callers use it to tell whether a
    /// dispatch already recorded its own output.
    pub fn message_transcript_revision(&self) -> u64 {
        self.message_transcript_revision
    }

    /// Whether the compatibility message slot still contains output that has
    /// not been represented in the transcript.
    pub fn message_slot_needs_flush(&self) -> bool {
        !self.message.is_empty() && self.message != self.message_flushed
    }

    /// Append one entry and honour the transcript's capacity. Shared by
    /// the revision-bumping public push and the epilogue push below.
    fn append_transcript_entry(
        &mut self,
        text: String,
        glyphs: Vec<TlkRenderedGlyph>,
        is_command_echo: bool,
        centered: bool,
        explicit_blank: bool,
    ) {
        self.message_transcript.push(MessageEntry {
            text,
            glyphs,
            is_command_echo,
            centered,
            explicit_blank,
        });
        if self.message_transcript.len() > MESSAGE_TRANSCRIPT_CAPACITY {
            let excess = self.message_transcript.len() - MESSAGE_TRANSCRIPT_CAPACITY;
            self.message_transcript.drain(0..excess);
        }
    }

    /// Append one already-classified transcript line.
    pub fn push_message_entry(&mut self, text: impl Into<String>, is_command_echo: bool) {
        let text = text.into();
        let glyphs = ordinary_glyphs_from_engine_text(&text);
        self.append_transcript_entry(text, glyphs, is_command_echo, false, false);
        self.message_transcript_revision = self.message_transcript_revision.wrapping_add(1);
    }

    fn push_tlk_message_entry(
        &mut self,
        text: String,
        glyphs: Vec<TlkRenderedGlyph>,
        is_command_echo: bool,
    ) {
        self.append_transcript_entry(text, glyphs, is_command_echo, false, false);
        self.message_transcript_revision = self.message_transcript_revision.wrapping_add(1);
    }

    /// Append one output line with the text cursor's centre mode enabled.
    pub fn push_centered_message_entry(&mut self, text: impl Into<String>) {
        let text = text.into();
        let glyphs = ordinary_glyphs_from_engine_text(&text);
        self.append_transcript_entry(text, glyphs, false, true, false);
        self.message_transcript_revision = self.message_transcript_revision.wrapping_add(1);
    }

    pub fn push_explicit_blank_message_entry(&mut self) {
        self.append_transcript_entry(String::new(), Vec::new(), false, false, true);
        self.message_transcript_revision = self.message_transcript_revision.wrapping_add(1);
    }

    /// Append a handler message as continuation lines. Embedded newlines
    /// become separate transcript entries, matching the per-cell
    /// emitter's treatment of line-feed bytes (`text-output.md §5`).
    ///
    /// The published transcripts of `overworld.md §8.1`,
    /// `doors-and-z-transitions.md §12.1` and `dungeon-mode.md §8.1` all
    /// state that "`\n` is one line feed and is part of the string, so a
    /// leading `\n` produces a **blank row**" and a trailing `\n\n`
    /// "produces a blank row after the text". That makes the split's
    /// segments mean two different things: every segment *before* the
    /// last is a completed row - an empty one is a blank row the player
    /// sees - while the last segment is only where the cursor is left, so
    /// an empty tail is the ordinary "line terminated" case and draws
    /// nothing. Emitting an ordinary empty entry for a leading `\n` lost
    /// the blank row, because the window's log drops empty output lines.
    pub fn push_message_transcript_lines(&mut self, text: &str) {
        let mut segments = text.split('\n').peekable();
        while let Some(line) = segments.next() {
            let is_last = segments.peek().is_none();
            if line.is_empty() {
                if !is_last {
                    self.push_explicit_blank_message_entry();
                }
                continue;
            }
            self.push_message_entry(line, false);
        }
    }

    /// Append a TLK response without flattening its ordinary/runic font mask.
    pub fn push_tlk_message_transcript_lines(&mut self, rendered: &TlkRenderedText) {
        let mut text_lines = rendered.text.split('\n');
        let mut glyph_lines = rendered.rendered_lines();
        loop {
            let text = text_lines.next();
            let glyphs = glyph_lines.next();
            match (text, glyphs) {
                (Some(text), Some(glyphs)) => {
                    self.push_tlk_message_entry(text.to_string(), glyphs.to_vec(), false);
                }
                (None, None) => break,
                _ => panic!("TLK rendered text and glyph line counts diverged"),
            }
        }
    }

    /// `text-output.md §11`: emit one line into the transcript *now*,
    /// rather than leaving it in the slot for a later writer to
    /// overwrite. The original "has no message slot to overwrite": a
    /// line reaches the window when it is produced, and a second line
    /// produced in the same turn prints beneath it. Every producer that
    /// can run alongside another in one turn — the per-turn epilogue
    /// above all — must use this rather than assigning `message`.
    ///
    /// The slot is still written, because ~300 readers and the terminal
    /// harness take the newest line from it.
    pub fn emit_message_line(&mut self, text: impl Into<String>) {
        let text = text.into();
        // A handler that wrote the slot directly earlier in the same turn has
        // not reached a composition boundary yet. Flush it first, or taking
        // the slot here silently drops it - which is exactly what the
        // per-turn epilogue's consequence lines did to the command's own
        // line. `text-output.md §11`: "a second line produced in the same
        // turn prints beneath" the first, never instead of it.
        self.flush_message_slot();
        self.push_message_transcript_lines(&text);
        self.message = text.clone();
        self.message_flushed = text;
    }

    /// Emit one line that the turn loop's own prompt marker belongs to.
    ///
    /// `text-output.md §10.2`: the mode loop emits the newline and the
    /// end-cap marker **before** it reads the key, so "echoed command
    /// lines carry it and pure output lines do not". A refusal the
    /// dispatcher prints in place of a verb echo - the `What?` of
    /// `commands.md §5.2` - is written onto that already-marked line.
    pub fn emit_command_echo_line(&mut self, text: impl Into<String>) {
        let text = text.into();
        self.push_message_entry(text.clone(), true);
        self.message = text.clone();
        self.message_flushed = text;
    }

    /// [`Self::emit_message_line`] for a line the active window's centre
    /// flag is set for. `text-output.md §3`: "`0xFC` sets centre-output",
    /// and §3's window table has "the next call to the wrap-aware string
    /// printer centres its line within the window's width before
    /// emitting". The transcript keeps the centring as a per-entry flag
    /// so the message window can place the row; the compatibility slot
    /// still receives the raw text.
    pub fn emit_centered_message_line(&mut self, text: impl Into<String>) {
        let text = text.into();
        self.push_centered_message_entry(text.clone());
        self.message = text.clone();
        self.message_flushed = text;
    }

    /// Emit one line that *continues* the row the cursor was left on.
    ///
    /// A prompt whose stored string ends without a line feed - the town-family
    /// exit's `\nDost thou wish to leave? ` of
    /// `doors-and-z-transitions.md §12.1` is the published case - leaves the
    /// cursor mid-row, and the handler's answer word therefore lands on that
    /// same row. Emitting the answer as its own line renders it one row lower
    /// than the original. Only the first segment continues; every line feed
    /// inside `text` behaves exactly as it does in
    /// [`Self::push_message_transcript_lines`].
    pub fn emit_message_line_continuing_row(&mut self, text: impl Into<String>) {
        let text = text.into();
        self.flush_message_slot();
        let mut segments = text.split('\n').peekable();
        if let Some(first) = segments.next() {
            let more_follow = segments.peek().is_some();
            if first.is_empty() {
                if more_follow {
                    self.push_explicit_blank_message_entry();
                }
            } else if let Some(last) = self
                .message_transcript
                .last_mut()
                .filter(|entry| !entry.explicit_blank)
            {
                last.text.push_str(first);
                last.glyphs.extend(ordinary_glyphs_from_engine_text(first));
                self.message_transcript_revision = self.message_transcript_revision.wrapping_add(1);
            } else {
                self.push_message_entry(first, false);
            }
        }
        while let Some(line) = segments.next() {
            let is_last = segments.peek().is_none();
            if line.is_empty() {
                if !is_last {
                    self.push_explicit_blank_message_entry();
                }
                continue;
            }
            self.push_message_entry(line, false);
        }
        self.message = text.clone();
        self.message_flushed = text;
    }

    /// Emit one font-preserving TLK response immediately.
    pub fn emit_tlk_message(&mut self, rendered: TlkRenderedText) {
        self.push_tlk_message_transcript_lines(&rendered);
        self.message = rendered.text.clone();
        self.message_flushed = rendered.text;
    }

    /// `text-output.md §11`: append whatever the slot is holding, if the
    /// transcript has not already recorded it.
    ///
    /// This is the safety net under [`Self::emit_message_line`], for the
    /// many handlers that still assign `message` directly. It is called
    /// at the turn-composition boundaries — at the end of the per-turn
    /// epilogue, and at the end of the key dispatch — so a line written
    /// before one of those boundaries is on the transcript before the
    /// next writer can replace the slot.
    ///
    /// It infers "a write happened" from the value changing, because a
    /// plain `String` field cannot report its own writes. That inference
    /// is exact except for two byte-identical lines emitted back to back
    /// through direct assignment on a path that opens no verb echo,
    /// which collapse into one entry. Promote such a producer to
    /// [`Self::emit_message_line`] to make it exact.
    ///
    /// Returns whether anything was appended.
    pub fn flush_message_slot(&mut self) -> bool {
        if self.message.is_empty() || self.message == self.message_flushed {
            return false;
        }
        let text = self.message.clone();
        self.push_message_transcript_lines(&text);
        self.message_flushed = text;
        true
    }

    /// `commands.md §5`: open a transcript entry with the command's
    /// resident verb echo before the handler prompts or refuses.
    pub fn begin_command_echo(&mut self, echo: CommandEcho) {
        self.abort_command_echo();
        self.push_message_entry(echo.text, true);
        self.pending_command_echo = Some(PendingCommandEcho {
            echo,
            message_at_entry: std::mem::take(&mut self.message),
        });
        // The slot is empty again, so the next value in it is a new
        // emission whatever it says.
        self.message_flushed.clear();
    }

    /// Drop an echo that was opened for a key the active mode turned out
    /// not to handle, restoring the message the caller saw before.
    pub fn abort_command_echo(&mut self) {
        let Some(pending) = self.pending_command_echo.take() else {
            return;
        };
        if self
            .message_transcript
            .last()
            .is_some_and(|entry| entry.is_command_echo && entry.text == pending.echo.text)
        {
            self.message_transcript.pop();
            self.message_transcript_revision = self.message_transcript_revision.wrapping_sub(1);
        }
        if self.message.is_empty() {
            self.message = pending.message_at_entry;
            self.message_flushed = self.message.clone();
        }
    }

    /// Fold the handler's own output into the entry the verb echo opened.
    ///
    /// A handler that printed nothing leaves the echo standing alone. A
    /// handler whose own text already leads with the verb — the direction
    /// prompts render `Look-` themselves — replaces the echo rather than
    /// doubling it. Otherwise the text either continues the echoed line
    /// (`Look-` + `Pass`) or starts the next one (`Use item` then
    /// `Item:`), per the echo's join mode.
    ///
    /// Returns whether anything was committed.
    pub fn commit_command_echo(&mut self) -> bool {
        let Some(pending) = self.pending_command_echo.take() else {
            return false;
        };
        let message = std::mem::take(&mut self.message);
        if message.is_empty() {
            self.message = pending.message_at_entry;
            self.message_flushed = self.message.clone();
            return true;
        }
        if message == self.message_flushed {
            // The handler emitted through `emit_message_line`, so the
            // transcript already carries it; only the slot needs
            // restoring.
            self.message = message;
            return true;
        }
        self.message = message.clone();
        self.message_flushed = message.clone();

        let verb = pending.echo.text;
        let echo_is_last = self
            .message_transcript
            .last()
            .is_some_and(|entry| entry.is_command_echo && entry.text == verb);
        let mut lines = message.split('\n');
        let first = lines.next().unwrap_or_default();

        if echo_is_last && first == verb {
            // The handler re-emitted exactly the verb; keep one copy.
            // `#81`: only an exact repeat folds — a refusal that merely
            // starts with the verb is still a separate line, because the
            // echo is printed *before* the precondition check.
        } else if echo_is_last && first.starts_with(verb) && verb.ends_with('-') {
            // A direction-prompting handler renders its own `Verb-` and
            // then appends the direction, so keep its fuller line.
            if let Some(last) = self.message_transcript.last_mut() {
                last.text = first.to_string();
                last.glyphs = ordinary_glyphs_from_engine_text(first);
            }
        } else if echo_is_last && pending.echo.join.continues_line() {
            if let Some(last) = self.message_transcript.last_mut() {
                last.text.push_str(first);
                last.glyphs.extend(ordinary_glyphs_from_engine_text(first));
            }
        } else {
            self.push_message_entry(first, false);
        }
        for line in lines {
            self.push_message_entry(line, false);
        }
        true
    }

    /// Complete a direction echo that was published when a prompt opened on
    /// an earlier input event. The original keeps the hyphenated verb line
    /// open while it waits, so the accepted direction or `Pass` belongs on
    /// that existing line rather than in a new transcript entry.
    pub(crate) fn complete_open_direction_echo(&mut self, verb: &str, continuation: &str) -> bool {
        let Some(last) = self.message_transcript.last_mut() else {
            if self.message == verb {
                self.message.push_str(continuation);
                self.message_flushed = self.message.clone();
                return true;
            }
            return false;
        };
        if last.text != verb {
            return false;
        }
        last.text.push_str(continuation);
        last.glyphs
            .extend(ordinary_glyphs_from_engine_text(continuation));
        self.message_transcript_revision = self.message_transcript_revision.wrapping_add(1);
        if self.message == verb {
            self.message.push_str(continuation);
            self.message_flushed = self.message.clone();
        }
        true
    }
}

/// `commands.md §5`: resolve the verb echo an overworld/town-family key
/// dispatches with. The lookup mirrors the dispatcher's own key order —
/// `Space`, then the uppercase command letters, then the movement keys,
/// then the lowercase command aliases — so a legacy movement key such as
/// lowercase `w` echoes its direction rather than a command name.
pub fn top_down_command_echo(key: char) -> Option<CommandEcho> {
    let surface = |command| command_echo(command, CommandEchoMode::Surface);
    if key == ' ' {
        return surface(Command::Pass);
    }
    if key.is_ascii_uppercase() {
        return command_for_letter(key as u8).and_then(unassigned_or(key, surface));
    }
    if let Some(direction) = Direction::from_play_key(key) {
        return movement_echo(direction);
    }
    if key.is_ascii_alphabetic() {
        return command_for_letter(key as u8).and_then(unassigned_or(key, surface));
    }
    match key {
        '<' | '>' => surface(Command::Klimb),
        _ => None,
    }
}

/// `commands.md §5.2`: `D` and `W` print a disambiguating refusal rather
/// than the bare `What?`.
fn unassigned_or(
    key: char,
    resolve: impl Fn(Command) -> Option<CommandEcho>,
) -> impl Fn(Command) -> Option<CommandEcho> {
    move |command| {
        if matches!(command, Command::UnassignedRefusal) {
            return Some(CommandEcho {
                text: unassigned_refusal_echo(key as u8),
                join: CommandEchoJoin::Complete,
            });
        }
        resolve(command)
    }
}

/// `commands.md §5` + `dungeon-mode.md §10`: resolve the verb echo a
/// dungeon key dispatches with. Dungeon movement uses its own literals —
/// a forward step echoes `Advance` — and the back-step and the two turn
/// keys have no observed literal, so they emit no echo at all rather than
/// an invented one (`cleak/u5-spec#81`).
pub fn dungeon_command_echo(key: char) -> Option<CommandEcho> {
    let dungeon = |command| command_echo(command, CommandEchoMode::Dungeon);
    if key == ' ' {
        return dungeon(Command::Pass);
    }
    if matches!(key, 'S' | 'A' | 'D' | 'W') {
        return command_for_letter(key as u8).and_then(unassigned_or(key, dungeon));
    }
    // `commands.md §5.2`: dungeon movement carries its own verb set.
    if let Some(direction) = high_byte_direction_from_key(key) {
        return match direction {
            Direction::North => Some(DungeonMovementEcho::Advance.echo()),
            Direction::South => Some(DungeonMovementEcho::BackUp.echo()),
            Direction::West => Some(DungeonMovementEcho::TurnLeft.echo()),
            Direction::East => Some(DungeonMovementEcho::TurnRight.echo()),
            _ => None,
        };
    }
    match key.to_ascii_lowercase() {
        '8' | 'w' | '.' | '\r' | '\n' => Some(DungeonMovementEcho::Advance.echo()),
        '2' | 's' => Some(DungeonMovementEcho::BackUp.echo()),
        '4' | 'a' => Some(DungeonMovementEcho::TurnLeft.echo()),
        '6' | 'd' => Some(DungeonMovementEcho::TurnRight.echo()),
        // The rejected diagonals print the movement-family refusal.
        '7' | '9' | '1' | '3' => None,
        '<' | '>' => dungeon(Command::Klimb),
        other => command_for_letter(other as u8).and_then(unassigned_or(key, dungeon)),
    }
}

impl PlayState {
    /// Convenience wrapper for the pre-dispatch branches that resolve a
    /// command before the world/dungeon dispatchers see the key.
    pub fn begin_command_echo_for(&mut self, command: Command) {
        let mode = if matches!(self.area, Area::Dungeon { .. }) {
            CommandEchoMode::Dungeon
        } else {
            CommandEchoMode::Surface
        };
        if let Some(echo) = command_echo(command, mode) {
            self.begin_command_echo(echo);
        }
    }
}
