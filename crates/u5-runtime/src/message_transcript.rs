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

    /// Append one already-classified transcript line.
    pub fn push_message_entry(&mut self, text: impl Into<String>, is_command_echo: bool) {
        self.message_transcript.push(MessageEntry {
            text: text.into(),
            is_command_echo,
        });
        self.message_transcript_revision = self.message_transcript_revision.wrapping_add(1);
        if self.message_transcript.len() > MESSAGE_TRANSCRIPT_CAPACITY {
            let excess = self.message_transcript.len() - MESSAGE_TRANSCRIPT_CAPACITY;
            self.message_transcript.drain(0..excess);
        }
    }

    /// Append a handler message as continuation lines. Embedded newlines
    /// become separate transcript entries, matching the per-cell
    /// emitter's treatment of line-feed bytes (`text-output.md §5`).
    pub fn push_message_transcript_lines(&mut self, text: &str) {
        for line in text.split('\n') {
            self.push_message_entry(line, false);
        }
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
            return true;
        }
        self.message = message.clone();

        let verb = pending.echo.text;
        let echo_is_last = self
            .message_transcript
            .last()
            .is_some_and(|entry| entry.is_command_echo && entry.text == verb);
        let mut lines = message.split('\n');
        let first = lines.next().unwrap_or_default();

        if echo_is_last && first.starts_with(verb) {
            // The handler re-emitted the verb itself; keep one copy.
            if let Some(last) = self.message_transcript.last_mut() {
                last.text = first.to_string();
            }
        } else if echo_is_last && pending.echo.join == CommandEchoJoin::SameLine {
            if let Some(last) = self.message_transcript.last_mut() {
                last.text.push_str(first);
            }
        } else {
            self.push_message_entry(first, false);
        }
        for line in lines {
            self.push_message_entry(line, false);
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
    if key == ' ' {
        return Command::Pass.echo();
    }
    if key.is_ascii_uppercase() {
        return command_for_letter(key as u8).and_then(Command::echo);
    }
    if let Some(direction) = Direction::from_play_key(key) {
        return movement_echo(direction);
    }
    if key.is_ascii_alphabetic() {
        return command_for_letter(key as u8).and_then(Command::echo);
    }
    match key {
        '<' | '>' => Command::Klimb.echo(),
        _ => None,
    }
}

/// `commands.md §5` + `dungeon-mode.md §10`: resolve the verb echo a
/// dungeon key dispatches with. Dungeon movement uses its own literals —
/// a forward step echoes `Advance` — and the back-step and the two turn
/// keys have no observed literal, so they emit no echo at all rather than
/// an invented one (`cleak/u5-spec#81`).
pub fn dungeon_command_echo(key: char) -> Option<CommandEcho> {
    if key == ' ' {
        return Command::Pass.echo();
    }
    if matches!(key, 'S' | 'A' | 'D' | 'W') {
        return command_for_letter(key as u8).and_then(Command::echo);
    }
    if let Some(direction) = high_byte_direction_from_key(key) {
        return match direction {
            Direction::North => Some(DUNGEON_ADVANCE_ECHO),
            _ => None,
        };
    }
    match key.to_ascii_lowercase() {
        '8' | 'w' | '.' | '\r' | '\n' => Some(DUNGEON_ADVANCE_ECHO),
        // Back-step, the two turn keys and the rejected diagonals.
        '2' | 's' | '4' | 'a' | '6' | 'd' | '7' | '9' | '1' | '3' => None,
        '<' | '>' => Command::Klimb.echo(),
        other => command_for_letter(other as u8).and_then(Command::echo),
    }
}

impl PlayState {
    /// Convenience wrapper for the pre-dispatch branches that resolve a
    /// command before the world/dungeon dispatchers see the key.
    pub fn begin_command_echo_for(&mut self, command: Command) {
        if let Some(echo) = command.echo() {
            self.begin_command_echo(echo);
        }
    }
}
