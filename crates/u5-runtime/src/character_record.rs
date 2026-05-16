//! Save-image character-record byte vocabulary per
//! `formats/saved-gam.md` §3.1.

/// `formats/saved-gam.md §3.1` published class-letter values for the
/// 32-byte character record's `+0x0A` field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CharacterClass {
    Avatar,
    Bard,
    Fighter,
    Mage,
    Druid,
    Tinker,
    Paladin,
    Ranger,
    Shepherd,
}

impl CharacterClass {
    /// `formats/saved-gam.md §3.1`: ASCII byte the engine writes for
    /// this class.
    pub const fn save_byte(self) -> u8 {
        match self {
            CharacterClass::Avatar => b'A',
            CharacterClass::Bard => b'B',
            CharacterClass::Fighter => b'F',
            CharacterClass::Mage => b'M',
            CharacterClass::Druid => b'D',
            CharacterClass::Tinker => b'T',
            CharacterClass::Paladin => b'P',
            CharacterClass::Ranger => b'R',
            CharacterClass::Shepherd => b'S',
        }
    }

    /// `formats/saved-gam.md §3.1`: human-readable display name.
    pub const fn display_name(self) -> &'static str {
        match self {
            CharacterClass::Avatar => "Avatar",
            CharacterClass::Bard => "Bard",
            CharacterClass::Fighter => "Fighter",
            CharacterClass::Mage => "Mage",
            CharacterClass::Druid => "Druid",
            CharacterClass::Tinker => "Tinker",
            CharacterClass::Paladin => "Paladin",
            CharacterClass::Ranger => "Ranger",
            CharacterClass::Shepherd => "Shepherd",
        }
    }
}

/// `formats/saved-gam.md §3.1`: classify a class byte at `+0x0A`.
/// Returns `None` for any byte outside the published table.
pub const fn character_class_for_byte(byte: u8) -> Option<CharacterClass> {
    Some(match byte {
        b'A' => CharacterClass::Avatar,
        b'B' => CharacterClass::Bard,
        b'F' => CharacterClass::Fighter,
        b'M' => CharacterClass::Mage,
        b'D' => CharacterClass::Druid,
        b'T' => CharacterClass::Tinker,
        b'P' => CharacterClass::Paladin,
        b'R' => CharacterClass::Ranger,
        b'S' => CharacterClass::Shepherd,
        _ => return None,
    })
}

/// `formats/saved-gam.md §3.1` published status-letter values for the
/// 32-byte character record's `+0x0B` field. The `'P'` value is shared
/// by poison and one revive-style helper transitioning a dead slot
/// back to a live state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CharacterStatus {
    Good,
    PoisonedOrRevived,
    Sleeping,
    Charmed,
    Dead,
    Ashes,
}

impl CharacterStatus {
    pub const fn save_byte(self) -> u8 {
        match self {
            CharacterStatus::Good => b'G',
            CharacterStatus::PoisonedOrRevived => b'P',
            CharacterStatus::Sleeping => b'S',
            CharacterStatus::Charmed => b'C',
            CharacterStatus::Dead => b'D',
            CharacterStatus::Ashes => b'A',
        }
    }
}

/// `formats/saved-gam.md §3.1` per-character byte offsets inside the
/// 32-byte character record. The roster lays each record out
/// contiguously from offset `0x00` (first name byte) through `0x1F`
/// (last equipment-and-padding tail byte).
pub const SAVE_CHARACTER_NAME_OFFSET: usize = 0x00;
pub const SAVE_CHARACTER_NAME_LEN_BYTES: usize = 9;
pub const SAVE_CHARACTER_STRENGTH_OFFSET: usize = 0x0C;
pub const SAVE_CHARACTER_DEXTERITY_OFFSET: usize = 0x0D;
pub const SAVE_CHARACTER_INTELLIGENCE_OFFSET: usize = 0x0E;
pub const SAVE_CHARACTER_MAGIC_POINTS_OFFSET: usize = 0x0F;
pub const SAVE_CHARACTER_HP_CURRENT_OFFSET: usize = 0x10;
pub const SAVE_CHARACTER_HP_MAX_OFFSET: usize = 0x12;
pub const SAVE_CHARACTER_EXPERIENCE_OFFSET: usize = 0x14;
pub const SAVE_CHARACTER_LEVEL_OFFSET: usize = 0x16;
pub const SAVE_CHARACTER_MONTH_COUNTER_OFFSET: usize = 0x17;
pub const SAVE_CHARACTER_DEFENSE_BYTE_OFFSET: usize = 0x18;

/// `formats/saved-gam.md §3.1` per-character record stride.
pub const SAVE_CHARACTER_RECORD_LEN: usize = 32;

/// `formats/saved-gam.md §3.1`: returns the file offset of the
/// supplied `field_offset` within roster slot `slot` (0..=15).
pub const fn save_character_field_offset(slot: usize, field_offset: usize) -> usize {
    // Roster begins two leading save-image bytes into the file.
    0x0002 + slot * SAVE_CHARACTER_RECORD_LEN + field_offset
}

/// `rest-and-camp.md §5` rest-with-watch participation classification.
/// Good, Poisoned, and Sleeping members participate in the watch
/// path; Charmed, Dead, and Ashes members are skipped and have no
/// dedicated H-Hole-up status transition.
pub const fn rest_with_watch_participates(status: CharacterStatus) -> bool {
    matches!(
        status,
        CharacterStatus::Good
            | CharacterStatus::PoisonedOrRevived
            | CharacterStatus::Sleeping,
    )
}

/// `rest-and-camp.md §5` town-hours rest pass: only Good members are
/// temporarily marked Sleeping for the elapsed-rest loop. The cleanup
/// pass then restores all Sleeping members to Good. Non-Good members
/// are not changed by this temporary sleep-marking.
pub const fn town_rest_temp_sleep_marked(status: CharacterStatus) -> bool {
    matches!(status, CharacterStatus::Good)
}

/// `rest-and-camp.md §5` cleanup transition: Sleeping members are
/// restored to Good during cleanup (regardless of whether they were
/// temporarily marked or already Sleeping at entry).
pub const fn rest_cleanup_transitions_to_good(status: CharacterStatus) -> bool {
    matches!(status, CharacterStatus::Sleeping)
}

/// `rest-and-camp.md §5`: returns `true` when a rest-with-watch
/// participant is eligible for ordinary HP recovery during the rest
/// loop. Good and Sleeping members recover HP (capped at maximum HP);
/// Poisoned members participate in the watch but do not receive
/// ordinary rest HP recovery, and Charmed/Dead/Ashes members do not
/// participate at all. This is the HP-tick eligibility predicate, not
/// the broader watch-participation predicate.
pub const fn rest_with_watch_recovers_hp(status: CharacterStatus) -> bool {
    matches!(status, CharacterStatus::Good | CharacterStatus::Sleeping)
}

/// `rest-and-camp.md §4` rest-handler duration prompt outcome. The
/// shared rest-with-watch handler echoes a digit 1..=9 as the
/// requested rest duration, cancels on Space or `0`, and silently
/// re-prompts on any other key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RestDurationInput {
    /// Digit `1..=9` — accepted duration (in hours for the
    /// rest-with-watch handler, or as the relative target-hour offset
    /// for the town bed-rest path).
    Hours(u8),
    /// Space or `0` — cancel; rest does not run.
    Cancel,
    /// Any other key — silently re-prompt; the handler reads the
    /// next byte without echoing or advancing.
    Discard,
}

/// `rest-and-camp.md §4`: classify one keystroke for the rest-handler
/// duration prompt.
pub const fn rest_duration_input(byte: u8) -> RestDurationInput {
    match byte {
        b'1'..=b'9' => RestDurationInput::Hours(byte - b'0'),
        b'0' | b' ' => RestDurationInput::Cancel,
        _ => RestDurationInput::Discard,
    }
}

/// `formats/saved-gam.md §3.1`: classify a status byte at `+0x0B`.
pub const fn character_status_for_byte(byte: u8) -> Option<CharacterStatus> {
    Some(match byte {
        b'G' => CharacterStatus::Good,
        b'P' => CharacterStatus::PoisonedOrRevived,
        b'S' => CharacterStatus::Sleeping,
        b'C' => CharacterStatus::Charmed,
        b'D' => CharacterStatus::Dead,
        b'A' => CharacterStatus::Ashes,
        _ => return None,
    })
}
