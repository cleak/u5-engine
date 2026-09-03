//! Final input-layer direction codes per `input.md` §5. The keyboard
//! peek translates numpad cardinals/diagonals, extended arrow keys, and
//! shifted top-row digits into the eight direction codes used by the
//! upper layers; gameplay mode loops handle these inline before the
//! central command dispatcher sees ordinary letter keys.
//!
//! `input.md §5`: "The four cardinals land in a low code block and the
//! four diagonals in a high one, and neither collides with printable
//! ASCII." §14 lists the corrected cardinal assignments as a settled
//! revision, so the cardinals are `0x01..=0x04` and not the high tail
//! an earlier revision of this module recorded.

/// `input.md §10,§11` cardinal-direction prompt outcome. The shared
/// adjacent-tile and spell-direction prompts both block on one
/// keystroke and accept exactly the same vocabulary: a cardinal
/// direction key adjusts the cached target by one cell and the
/// caller reads the no-direction result on `Pass` (Space). Diagonal
/// direction codes, function keys, ordinary letters, and unshifted
/// top-row digits are ignored and the prompt reads again rather
/// than returning to the caller.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CardinalPromptAction {
    /// Cardinal direction — adjust the cached target by one cell.
    /// The caller still tests the returned direction; this enum
    /// only encodes the prompt-level discrimination.
    Cardinal(InputDirection),
    /// Space — `Pass`. Returns the no-direction result; callers
    /// typically treat that as silent cancellation.
    Pass,
    /// Any other byte — silently re-prompt; the prompt reads the
    /// next keystroke without echoing or returning.
    Ignored,
}

/// `input.md §10,§11`: classify one keystroke for the shared
/// adjacent-tile / spell-direction prompts. Only the four cardinal
/// direction codes and Space round-trip through this classifier;
/// every other byte (diagonals, letters, function keys, control
/// bytes) lands in [`CardinalPromptAction::Ignored`].
pub const fn cardinal_direction_prompt_action(byte: u8) -> CardinalPromptAction {
    match byte {
        INPUT_CODE_NORTH => CardinalPromptAction::Cardinal(InputDirection::North),
        INPUT_CODE_SOUTH => CardinalPromptAction::Cardinal(InputDirection::South),
        INPUT_CODE_EAST => CardinalPromptAction::Cardinal(InputDirection::East),
        INPUT_CODE_WEST => CardinalPromptAction::Cardinal(InputDirection::West),
        b' ' => CardinalPromptAction::Pass,
        _ => CardinalPromptAction::Ignored,
    }
}

/// `input.md §10,§11` echoed labels the shared direction prompts
/// print after a cardinal/Pass keystroke. The caller is responsible
/// for emitting the prefix (the spell prompt prints `Direction-` and
/// the adjacent-tile prompt prints its caller-owned verb prefix); the
/// echo is the cardinal name or `Pass`.
pub const DIRECTION_PROMPT_LABEL_NORTH: &str = "North";
pub const DIRECTION_PROMPT_LABEL_SOUTH: &str = "South";
pub const DIRECTION_PROMPT_LABEL_EAST: &str = "East";
pub const DIRECTION_PROMPT_LABEL_WEST: &str = "West";
pub const DIRECTION_PROMPT_LABEL_PASS: &str = "Pass";

/// `input.md §11`: lead-in prefix the spell direction prompt prints
/// before its echoed cardinal label or `Pass`. The adjacent-tile
/// prompt's prefix is caller-owned (e.g. `Search-`, `Jimmy-`, etc.).
pub const SPELL_DIRECTION_PROMPT_PREFIX: &str = "Direction-";

/// `input.md §10,§11`: echoed label for one [`CardinalPromptAction`]
/// outcome. `Ignored` returns `None` because the prompt does not
/// echo anything on a re-poll.
pub const fn direction_prompt_label(action: CardinalPromptAction) -> Option<&'static str> {
    Some(match action {
        CardinalPromptAction::Cardinal(InputDirection::North) => DIRECTION_PROMPT_LABEL_NORTH,
        CardinalPromptAction::Cardinal(InputDirection::South) => DIRECTION_PROMPT_LABEL_SOUTH,
        CardinalPromptAction::Cardinal(InputDirection::East) => DIRECTION_PROMPT_LABEL_EAST,
        CardinalPromptAction::Cardinal(InputDirection::West) => DIRECTION_PROMPT_LABEL_WEST,
        CardinalPromptAction::Pass => DIRECTION_PROMPT_LABEL_PASS,
        // Diagonals cannot reach this helper through
        // `cardinal_direction_prompt_action` (the classifier filters
        // them to Ignored), and the prompt's re-poll path does not
        // emit an echo either.
        CardinalPromptAction::Cardinal(_) | CardinalPromptAction::Ignored => return None,
    })
}

/// `input.md §2` prompt-mode discriminator. The shared wait-for-input
/// routine reads the resident *prompt-character* byte: a printable
/// ASCII byte (`0x20..=0x7E`) means a Y/N or text prompt is open and
/// the world tick is suppressed; any non-printable value is the idle
/// sentinel and the world tick (NPC schedules excluded) runs between
/// keystrokes.
pub const fn input_prompt_mode_active(prompt_character_byte: u8) -> bool {
    matches!(prompt_character_byte, 0x20..=0x7E)
}

/// `input.md §4` function-key remap. F1 through F10 become the
/// contiguous internal byte range `0xC9..=0xD2`, disjoint from
/// printable ASCII and the direction codes.
pub const INPUT_CODE_F1: u8 = 0xC9;
pub const INPUT_CODE_F10: u8 = 0xD2;
pub const INPUT_CODE_FUNCTION_FIRST: u8 = INPUT_CODE_F1;
pub const INPUT_CODE_FUNCTION_LAST: u8 = INPUT_CODE_F10;

/// `input.md §4`: returns `Some(1..=10)` for the function-key index a
/// remapped byte represents, or `None` for non-function bytes. F1 maps
/// to 1, F10 maps to 10.
pub const fn input_function_key_index(byte: u8) -> Option<u8> {
    if byte < INPUT_CODE_FUNCTION_FIRST || byte > INPUT_CODE_FUNCTION_LAST {
        None
    } else {
        Some(byte - INPUT_CODE_FUNCTION_FIRST + 1)
    }
}

/// `input.md §4`: return the remapped byte for a physical function-key
/// index. F1 maps to `0xC9`; F10 maps to `0xD2`.
pub const fn input_function_key_code(index: u8) -> Option<u8> {
    if index == 0 || index > 10 {
        None
    } else {
        Some(INPUT_CODE_FUNCTION_FIRST + index - 1)
    }
}

/// `input.md §3` cursor-blink defaults: glyph code `4` is the first
/// frame; the blink modulus wraps the phase counter every `4658` poll
/// iterations. These are mutable resident values; callers that need
/// real-time pacing derive a visually similar cadence from elapsed
/// time while preserving the no-advance/erase contract.
pub const CURSOR_BLINK_BASE_GLYPH: u8 = 4;
pub const CURSOR_BLINK_MODULUS: u16 = 4658;

/// `input.md §5` direction codes (eight directions plus the "no direction"
/// case that the upper layer represents by simply not seeing one of these
/// bytes).
pub const INPUT_CODE_NORTHWEST: u8 = 0xD3;
pub const INPUT_CODE_SOUTHWEST: u8 = 0xD4;
pub const INPUT_CODE_NORTHEAST: u8 = 0xD5;
pub const INPUT_CODE_SOUTHEAST: u8 = 0xD6;
pub const INPUT_CODE_WEST: u8 = 0x01;
pub const INPUT_CODE_EAST: u8 = 0x02;
pub const INPUT_CODE_NORTH: u8 = 0x03;
pub const INPUT_CODE_SOUTH: u8 = 0x04;

/// `input.md §5` first byte of the contiguous diagonal-direction range
/// (`0xD3..=0xD6`). Diagonals sit just above the function-key remap
/// range so a single byte unambiguously distinguishes the three
/// families.
pub const INPUT_CODE_DIAGONAL_FIRST: u8 = INPUT_CODE_NORTHWEST;
/// `input.md §5` last byte of the diagonal-direction range.
pub const INPUT_CODE_DIAGONAL_LAST: u8 = INPUT_CODE_SOUTHEAST;
/// `input.md §5` first byte of the contiguous cardinal-direction
/// range (`0x01..=0x04`). Cardinals occupy the low code block —
/// disjoint from printable ASCII and from the diagonal/function-key
/// high blocks — and are accepted by every movement consumer. A typed
/// Control character that reaches this range is biased into a high
/// pseudo-code by the keyboard peek, so a scancode-sourced direction
/// and a typed control byte never collide.
pub const INPUT_CODE_CARDINAL_FIRST: u8 = INPUT_CODE_WEST;
/// `input.md §5` last byte of the cardinal-direction range.
pub const INPUT_CODE_CARDINAL_LAST: u8 = INPUT_CODE_SOUTH;

/// `input.md §5` direction-code classification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputDirection {
    North,
    South,
    East,
    West,
    Northwest,
    Northeast,
    Southwest,
    Southeast,
}

impl InputDirection {
    /// Whether this direction is one of the four cardinals; world, town,
    /// dungeon, and combat movement consumers accept only this subset.
    pub const fn is_cardinal(self) -> bool {
        matches!(
            self,
            InputDirection::North
                | InputDirection::South
                | InputDirection::East
                | InputDirection::West
        )
    }
}

/// `input.md §5`: return the final input-layer byte for a semantic direction.
pub const fn input_direction_code(direction: InputDirection) -> u8 {
    match direction {
        InputDirection::North => INPUT_CODE_NORTH,
        InputDirection::South => INPUT_CODE_SOUTH,
        InputDirection::East => INPUT_CODE_EAST,
        InputDirection::West => INPUT_CODE_WEST,
        InputDirection::Northwest => INPUT_CODE_NORTHWEST,
        InputDirection::Northeast => INPUT_CODE_NORTHEAST,
        InputDirection::Southwest => INPUT_CODE_SOUTHWEST,
        InputDirection::Southeast => INPUT_CODE_SOUTHEAST,
    }
}

/// `input.md §5`: classify a final input-layer byte as a direction.
/// Returns `None` for any byte outside the eight published direction
/// codes; non-direction bytes are letters, function keys, or the prompt
/// dispatcher's own control bytes.
pub const fn input_code_direction(byte: u8) -> Option<InputDirection> {
    Some(match byte {
        INPUT_CODE_NORTH => InputDirection::North,
        INPUT_CODE_SOUTH => InputDirection::South,
        INPUT_CODE_EAST => InputDirection::East,
        INPUT_CODE_WEST => InputDirection::West,
        INPUT_CODE_NORTHWEST => InputDirection::Northwest,
        INPUT_CODE_NORTHEAST => InputDirection::Northeast,
        INPUT_CODE_SOUTHWEST => InputDirection::Southwest,
        INPUT_CODE_SOUTHEAST => InputDirection::Southeast,
        _ => return None,
    })
}

/// `input.md §5`: keypad-style direction translation for physical digits.
/// The input layer uses `7/8/9`, `4/6`, and `1/2/3` as the compass grid;
/// `0` and `5` are not directions.
pub const fn input_keypad_digit_direction_code(digit: u8) -> Option<u8> {
    Some(match digit {
        7 => INPUT_CODE_NORTHWEST,
        8 => INPUT_CODE_NORTH,
        9 => INPUT_CODE_NORTHEAST,
        4 => INPUT_CODE_WEST,
        6 => INPUT_CODE_EAST,
        1 => INPUT_CODE_SOUTHWEST,
        2 => INPUT_CODE_SOUTH,
        3 => INPUT_CODE_SOUTHEAST,
        _ => return None,
    })
}

/// `combat.md §8.3` / `input.md §5`, the shared input routine's internal
/// **numpad flag**. The routine sets it in two ways:
///
/// 1. "A key arriving as an extended scancode is matched against a small
///    table - the four arrow keys plus Home / End / PgUp / PgDn - and
///    translated to the direction codes; the flag is set."
/// 2. "A key arriving as one of the ASCII characters `1` through `9` causes
///    the routine to read the BIOS keyboard shift-status byte and set the
///    flag if **either shift key is down OR NumLock is currently active**."
///
/// This helper is arm 2. `§8.3` closes with the porting rule: "Whether
/// NumLock is on in a given player's session is an environment fact, not a
/// game fact; an engine should expose it as one" - hence the second
/// argument rather than a global. `input.md §5` gives the fallback for a
/// host that cannot see the lock state: "an implementation that lacks
/// NumLock state can treat shift held as the trigger."
pub const fn input_numpad_flag(shift_held: bool, num_lock_active: bool) -> bool {
    shift_held || num_lock_active
}

/// `combat.md §8.3`: "When the flag is set, the command reader remaps the
/// typed digits through a fixed table: `1` south-west, `2` south, `3`
/// south-east, `4` west, `6` east, `7` north-west, `8` north, `9`
/// north-east. `5` passes through unchanged, and `0` is outside the window
/// and is never remapped."
///
/// Returns the remapped direction code, or `None` when the digit reaches
/// the mode loop as an ordinary character - which is the case for every
/// digit with the flag clear, and for `0` and `5` always. The published
/// consequences follow directly: with the flag clear a typed digit still
/// reaches the combat active-player-selection handler, and "member
/// selection by digit is therefore only reachable with NumLock off"; only
/// `0` and `5` are unconditionally inert inside the targeting cursor.
pub const fn input_typed_digit_direction_code(digit: u8, numpad_flag: bool) -> Option<u8> {
    if !numpad_flag {
        return None;
    }
    input_keypad_digit_direction_code(digit)
}

/// `input.md §4` keyboard-layer return-byte family. After the peek
/// routine reads, classifies, and translates a raw byte, the value it
/// hands the rest of the engine falls into one of these three
/// non-overlapping ranges (plus the catch-all "no key" outcome that
/// the upper layer represents by simply not seeing one of the bytes
/// below).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputByteClass {
    /// Printable ASCII, plus the small set of accepted control bytes
    /// (Enter `0x0D`, Backspace `0x08`, Escape `0x1B`). The keyboard
    /// layer passes regular keys straight through.
    RegularAscii,
    /// Function-key remap: F1..F10 become the contiguous internal
    /// byte range `0xC9..=0xD2`, disjoint from printable ASCII and
    /// the direction codes.
    FunctionKey,
    /// One of the eight direction codes `0xD3..=0xD6` (diagonals) or
    /// `0x01..=0x04` (cardinals). World/town/dungeon/combat movement
    /// consumes only the cardinal subset; diagonals reach specialised
    /// prompts and otherwise fall through as non-movement input.
    Direction,
    /// Any other byte — the keyboard layer treats it as "no key" and
    /// the upper layer continues polling without firing a command.
    None,
}

/// `input.md §4`: classify a final post-translation keyboard byte
/// into its return family. Backspace, Enter, and Escape are accepted
/// as control bytes inside the regular-ASCII band; other low control
/// bytes (`0x00..=0x1F` outside that trio) fall through to the
/// "no key" branch because the engine does not bind them.
pub const fn input_byte_class(byte: u8) -> InputByteClass {
    match byte {
        0x08 | 0x0D | 0x1B => InputByteClass::RegularAscii,
        0x20..=0x7E => InputByteClass::RegularAscii,
        INPUT_CODE_CARDINAL_FIRST..=INPUT_CODE_CARDINAL_LAST => InputByteClass::Direction,
        0xC9..=0xD2 => InputByteClass::FunctionKey,
        INPUT_CODE_DIAGONAL_FIRST..=INPUT_CODE_DIAGONAL_LAST => InputByteClass::Direction,
        _ => InputByteClass::None,
    }
}

/// `input.md §9` party-member selector outcome from one keystroke.
/// The shared selector is slot-based: visible digits `1..=6`
/// directly choose the matching active-party slot; `0`, Space, and
/// Enter confirm the currently highlighted slot (or the explicit-
/// none branch when applicable); Escape cancels. Other bytes are
/// silently discarded so the prompt re-reads input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PartyTargetSelectorAction {
    /// Digit `1..=6` — chose party slot `digit - 1`. Caller still
    /// caps against the live party size.
    SelectSlot(u8),
    /// `0`, Space, or Enter — confirm. Resolves to the currently
    /// highlighted slot or the explicit-none branch per the
    /// caller's rules.
    Confirm,
    /// Escape — cancel the prompt.
    Cancel,
    /// North or West — cycle to the previous highlighted party slot.
    PreviousSlot,
    /// South or East — cycle to the next highlighted party slot.
    NextSlot,
    /// Any other byte — silently discarded; prompt re-reads input.
    Discard,
}

/// `input.md §9`: classify one keystroke for the shared
/// party-member selector, including cardinal highlight navigation. Caller has
/// already applied the case fold from [`input_case_fold`]; this helper does no
/// further translation.
pub const fn party_target_selector_action(byte: u8) -> PartyTargetSelectorAction {
    match byte {
        b'1'..=b'6' => PartyTargetSelectorAction::SelectSlot(byte - b'1'),
        b'0' | b' ' | 0x0D | 0x0A => PartyTargetSelectorAction::Confirm,
        0x1B => PartyTargetSelectorAction::Cancel,
        INPUT_CODE_NORTH | INPUT_CODE_WEST => PartyTargetSelectorAction::PreviousSlot,
        INPUT_CODE_SOUTH | INPUT_CODE_EAST => PartyTargetSelectorAction::NextSlot,
        _ => PartyTargetSelectorAction::Discard,
    }
}

/// `input.md §9` semantic result family the party-member selector
/// returns to its caller. The spec defines three families:
/// a non-negative selected slot, an Escape/cancel negative result,
/// and the distinct explicit-none negative result produced by the
/// `0` key. Most callers collapse both negative results to one
/// no-target branch; compatibility code that inspects them
/// separately can use this enum.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PartyTargetSelectorResult {
    /// Zero-based active-party slot the player chose.
    Slot(u8),
    /// `0` key — explicit "none" result, distinct from cancel.
    ExplicitNone,
    /// Escape — cancel the prompt.
    Cancel,
}

/// `input.md §9`: classify one keystroke into the published
/// three-family selector result, projecting Space/Enter/`0` from
/// [`party_target_selector_action`]'s `Confirm` into the explicit
/// branches. Visible digits `1..=6` resolve to `Slot(digit - 1)`;
/// the caller still caps the slot index against the live party size.
/// Returns `None` for keystrokes that should re-prompt without
/// producing a result (the underlying [`PartyTargetSelectorAction::Discard`]
/// branch, plus `Confirm` keystrokes that are not the explicit-none
/// `0` key — Space and Enter belong to the caller-driven highlight
/// confirmation rather than to this distinct three-family result).
pub const fn party_target_selector_result(byte: u8) -> Option<PartyTargetSelectorResult> {
    Some(match byte {
        b'1'..=b'6' => PartyTargetSelectorResult::Slot(byte - b'1'),
        b'0' => PartyTargetSelectorResult::ExplicitNone,
        0x1B => PartyTargetSelectorResult::Cancel,
        _ => return None,
    })
}

/// `input.md §8` free-text-input prompt action classified from one
/// keystroke. The line buffer reader appends printable ASCII into
/// the caller's small line buffer, pops on Backspace, terminates on
/// Enter, cancels on Escape (when the prompt allows it), and
/// silently discards every other byte (function keys, direction
/// codes, etc.).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FreeTextInputAction {
    /// Printable ASCII (`0x20..=0x7E`) — append to the line buffer
    /// (subject to a caller-supplied max length) and echo at the
    /// cursor.
    Append(u8),
    /// Backspace (`0x08`) — pop the most recent character from the
    /// buffer and overwrite the previous cell with a space. No-op
    /// when the buffer is already empty.
    Backspace,
    /// Enter (`0x0D` or `0x0A`) — terminate the prompt and return
    /// the accumulated string to the caller.
    Submit,
    /// Escape (`0x1B`) — terminate with the cancelled indication on
    /// prompts that allow it; the line buffer is cleared.
    Cancel,
    /// Any other byte (function keys, direction codes, raw control
    /// bytes) — silently discarded.
    Discard,
}

/// `input.md §8`: classify one input byte for the free-text prompt
/// reader. Caller already has the byte case-folded by
/// [`input_case_fold`]; this helper does no further translation.
pub const fn free_text_input_action(byte: u8) -> FreeTextInputAction {
    match byte {
        0x08 => FreeTextInputAction::Backspace,
        0x0A | 0x0D => FreeTextInputAction::Submit,
        0x1B => FreeTextInputAction::Cancel,
        0x20..=0x7E => FreeTextInputAction::Append(byte),
        _ => FreeTextInputAction::Discard,
    }
}

/// `input.md §8` numeric-prompt apply step. The shared numeric
/// reader accumulates digits as `value = value * 10 + digit`,
/// treats Backspace as `value = value / 10`, terminates on Enter,
/// and silently discards anything else. The caller still owns the
/// saturating cap on the accumulator (so a numeric prompt for a
/// byte-sized counter can clamp at 255).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumericPromptAction {
    /// Decimal digit `0..=9` — multiply the accumulator by ten and
    /// add the digit. Carries the digit value (`0..=9`).
    AppendDigit(u8),
    /// Backspace — integer-divide the accumulator by ten.
    Pop,
    /// Enter — terminate the prompt and return the accumulator.
    Submit,
    /// Any other byte (escape, function keys, direction codes) — the
    /// shared numeric reader silently discards the byte and re-polls.
    Discard,
}

/// `input.md §8`: classify one byte for the shared numeric-prompt
/// reader. The byte is already case-folded by [`input_case_fold`];
/// this helper does no further translation.
pub const fn numeric_prompt_action(byte: u8) -> NumericPromptAction {
    match byte {
        b'0'..=b'9' => NumericPromptAction::AppendDigit(byte - b'0'),
        0x08 => NumericPromptAction::Pop,
        0x0A | 0x0D => NumericPromptAction::Submit,
        _ => NumericPromptAction::Discard,
    }
}

/// `input.md §8`: apply one [`NumericPromptAction`] to a `u16`
/// accumulator. Returns the next accumulator value (saturating, so
/// callers that need a tighter cap should clamp after this call).
/// Submit/Discard leave the accumulator unchanged; Pop divides by ten.
pub const fn numeric_prompt_apply(value: u16, action: NumericPromptAction) -> u16 {
    match action {
        NumericPromptAction::AppendDigit(digit) => {
            value.saturating_mul(10).saturating_add(digit as u16)
        }
        NumericPromptAction::Pop => value / 10,
        NumericPromptAction::Submit | NumericPromptAction::Discard => value,
    }
}

/// `input.md §6` case fold: lowercase ASCII letters are folded to upper
/// case by simple subtraction; other bytes pass through unchanged. This
/// is locale-free and table-free, and is a no-op for higher-byte codes
/// (function keys, direction codes, control bytes).
pub const fn input_case_fold(byte: u8) -> u8 {
    if byte >= b'a' && byte <= b'z' {
        byte - (b'a' - b'A')
    } else {
        byte
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cardinal_direction_codes_occupy_the_published_low_block() {
        // `input.md §5`: "The four cardinals land in a low code block
        // and the four diagonals in a high one" with the final table
        // West `0x01`, East `0x02`, North `0x03`, South `0x04`. §14
        // lists this cardinal renumbering as a settled revision, so
        // the high tail `0xFB..=0xFE` an earlier revision published is
        // no longer a direction code at all.
        assert_eq!(INPUT_CODE_WEST, 0x01);
        assert_eq!(INPUT_CODE_EAST, 0x02);
        assert_eq!(INPUT_CODE_NORTH, 0x03);
        assert_eq!(INPUT_CODE_SOUTH, 0x04);
        assert_eq!(INPUT_CODE_CARDINAL_FIRST, 0x01);
        assert_eq!(INPUT_CODE_CARDINAL_LAST, 0x04);
        for byte in 0xFBu8..=0xFE {
            assert_eq!(input_code_direction(byte), None, "{byte:#04x}");
            assert_eq!(input_byte_class(byte), InputByteClass::None, "{byte:#04x}");
        }
    }

    #[test]
    fn low_cardinal_block_classifies_as_direction() {
        // `input.md §4`: the direction codes are one of the three
        // non-overlapping return families. A test vector written
        // against the published table (`0x01` = West) must not be
        // dropped as "no key".
        for (byte, direction) in [
            (INPUT_CODE_WEST, InputDirection::West),
            (INPUT_CODE_EAST, InputDirection::East),
            (INPUT_CODE_NORTH, InputDirection::North),
            (INPUT_CODE_SOUTH, InputDirection::South),
        ] {
            assert_eq!(input_byte_class(byte), InputByteClass::Direction);
            assert_eq!(input_code_direction(byte), Some(direction));
            assert!(input_code_direction(byte).unwrap().is_cardinal());
            assert_eq!(input_direction_code(direction), byte);
        }
        // The bytes either side of the block stay unbound, and the
        // accepted control trio keeps its regular-ASCII classification.
        assert_eq!(input_byte_class(0x00), InputByteClass::None);
        assert_eq!(input_byte_class(0x05), InputByteClass::None);
        assert_eq!(input_byte_class(0x08), InputByteClass::RegularAscii);
        assert_eq!(input_byte_class(0x0D), InputByteClass::RegularAscii);
        assert_eq!(input_byte_class(0x1B), InputByteClass::RegularAscii);
    }

    #[test]
    fn low_cardinal_block_does_not_collide_with_the_engine_control_keys() {
        // `input.md §5`: the cardinal block must not collide with any
        // byte another layer already binds. The play-loop control keys
        // are the only low-range bytes this engine reserves.
        for reserved in [
            crate::PLAY_IGNORED_INPUT_KEY,
            crate::PLAY_TYPEAHEAD_TOGGLE_KEY,
            crate::PLAY_MUSIC_TOGGLE_KEY,
            crate::PLAY_EXIT_TO_DOS_KEY,
        ] {
            let byte = reserved as u32;
            assert!(
                byte < INPUT_CODE_CARDINAL_FIRST as u32 || byte > INPUT_CODE_CARDINAL_LAST as u32,
                "{reserved:?} collides with the cardinal block"
            );
        }
    }

    /// `combat.md §8.3`: the shared input routine sets its numpad flag when
    /// an ASCII `1`..`9` arrives and "either shift key is down OR NumLock is
    /// currently active". "When the flag is set, the command reader remaps
    /// the typed digits through a fixed table: `1` south-west, `2` south, `3`
    /// south-east, `4` west, `6` east, `7` north-west, `8` north, `9`
    /// north-east. `5` passes through unchanged, and `0` is outside the
    /// window and is never remapped."
    #[test]
    fn typed_digits_remap_only_while_the_numpad_flag_is_set() {
        assert!(!input_numpad_flag(false, false));
        assert!(input_numpad_flag(true, false));
        assert!(input_numpad_flag(false, true));
        assert!(input_numpad_flag(true, true));

        // Flag clear: every digit reaches the mode loop as an ordinary
        // character, which is what keeps combat member selection by digit
        // reachable ("Member selection by digit is therefore only reachable
        // with NumLock off").
        for digit in 0..=9u8 {
            assert_eq!(input_typed_digit_direction_code(digit, false), None);
        }

        // Flag set: eight of the ten digits become direction codes.
        let remapped = [
            (1u8, INPUT_CODE_SOUTHWEST),
            (2, INPUT_CODE_SOUTH),
            (3, INPUT_CODE_SOUTHEAST),
            (4, INPUT_CODE_WEST),
            (6, INPUT_CODE_EAST),
            (7, INPUT_CODE_NORTHWEST),
            (8, INPUT_CODE_NORTH),
            (9, INPUT_CODE_NORTHEAST),
        ];
        for (digit, code) in remapped {
            assert_eq!(input_typed_digit_direction_code(digit, true), Some(code));
        }
        assert_eq!(remapped.len(), 8);

        // "Only `0` and `5` are unconditionally inert inside the cursor."
        assert_eq!(input_typed_digit_direction_code(0, true), None);
        assert_eq!(input_typed_digit_direction_code(5, true), None);
    }

    #[test]
    fn keypad_digits_translate_into_the_published_direction_vocabulary() {
        // `input.md §5` numpad table: 4/6/8/2 are the cardinals and
        // 7/9/1/3 the diagonals; 0 and 5 are not directions.
        assert_eq!(input_keypad_digit_direction_code(4), Some(0x01));
        assert_eq!(input_keypad_digit_direction_code(6), Some(0x02));
        assert_eq!(input_keypad_digit_direction_code(8), Some(0x03));
        assert_eq!(input_keypad_digit_direction_code(2), Some(0x04));
        assert_eq!(input_keypad_digit_direction_code(7), Some(0xD3));
        assert_eq!(input_keypad_digit_direction_code(1), Some(0xD4));
        assert_eq!(input_keypad_digit_direction_code(9), Some(0xD5));
        assert_eq!(input_keypad_digit_direction_code(3), Some(0xD6));
        assert_eq!(input_keypad_digit_direction_code(5), None);
        assert_eq!(input_keypad_digit_direction_code(0), None);
    }
}
