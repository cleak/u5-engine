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
/// 32-byte character record's `+0x0B` field.
///
/// `'P'` is **poisoned**, and nothing else. This doc used to say the
/// value was "shared by poison and one revive-style helper transitioning
/// a dead slot back to a live state", and the variant was named
/// `PoisonedOrRevived` to match. Both came from a draft of
/// `systems/traps.md §3` that is **withdrawn**: the helper used by trap
/// effect ids 1 and 3 is a poison primitive, not a revival one. It skips
/// already-dead slots and leaves them dead.
///
/// That wording was not harmless. We implemented it
/// (`if status == 'D' { status = 'P' }`), so poison traps did nothing at
/// all to a healthy party and gas traps resurrected the dead instead of
/// poisoning the living. The variant name is the part worth noting: it
/// asserted the retracted contract in a place no review reads as a
/// claim, and it survived the code fix by three commits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CharacterStatus {
    Good,
    Poisoned,
    Sleeping,
    Charmed,
    Dead,
    Ashes,
}

impl CharacterStatus {
    pub const fn save_byte(self) -> u8 {
        match self {
            CharacterStatus::Good => b'G',
            CharacterStatus::Poisoned => b'P',
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
/// `formats/saved-gam.md §3`: per-character name-field byte
/// length. Anchored to [`crate::SAVE_CHARACTER_NAME_LEN`] so the
/// character-record-module alias and the constants-module
/// declaration share one source of truth.
pub const SAVE_CHARACTER_NAME_LEN_BYTES: usize = crate::SAVE_CHARACTER_NAME_LEN;
/// `formats/saved-gam.md §3.1` per-character byte offsets. Each
/// of these constants is anchored to the canonical constants.rs
/// declaration (which lives in the per-byte chain rooted at
/// SAVE_CHARACTER_STATUS_OFFSET) so the duplicate aliases here
/// cannot drift from the single source of truth.
pub const SAVE_CHARACTER_STRENGTH_OFFSET: usize = crate::constants::SAVE_CHARACTER_STR_OFFSET;
pub const SAVE_CHARACTER_DEXTERITY_OFFSET: usize = crate::constants::SAVE_CHARACTER_DEX_OFFSET;
pub const SAVE_CHARACTER_INTELLIGENCE_OFFSET: usize = crate::constants::SAVE_CHARACTER_INT_OFFSET;
pub const SAVE_CHARACTER_MAGIC_POINTS_OFFSET: usize = crate::constants::SAVE_CHARACTER_MANA_OFFSET;
pub const SAVE_CHARACTER_HP_CURRENT_OFFSET: usize = crate::constants::SAVE_CHARACTER_HP_OFFSET;
pub const SAVE_CHARACTER_HP_MAX_OFFSET: usize = crate::constants::SAVE_CHARACTER_MAX_HP_OFFSET;
pub const SAVE_CHARACTER_EXPERIENCE_OFFSET: usize =
    crate::constants::SAVE_CHARACTER_EXPERIENCE_OFFSET;
pub const SAVE_CHARACTER_LEVEL_OFFSET: usize = crate::constants::SAVE_CHARACTER_LEVEL_OFFSET;
pub const SAVE_CHARACTER_MONTH_COUNTER_OFFSET: usize =
    crate::constants::SAVE_CHARACTER_STAY_COUNTER_OFFSET;
/// `formats/saved-gam.md §3.1`: per-record combat defense byte
/// sits immediately after the one-byte month counter and
/// immediately before the six-byte equipment band. Anchored to
/// SAVE_CHARACTER_MONTH_COUNTER_OFFSET + 1 so the
/// MonthCounter→Defense→Equipment chain has one source of truth.
pub const SAVE_CHARACTER_DEFENSE_BYTE_OFFSET: usize = SAVE_CHARACTER_MONTH_COUNTER_OFFSET + 1;

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
        CharacterStatus::Good | CharacterStatus::Poisoned | CharacterStatus::Sleeping,
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
/// loop. `cleak/u5-spec#47` currently promotes ordinary rest as time
/// advancement only, so no status receives a direct HP tick here.
pub const fn rest_with_watch_recovers_hp(status: CharacterStatus) -> bool {
    let _ = status;
    false
}

/// `rest-and-camp.md §6` sleep-ambush rest-local status restoration.
/// Before the rest handler hands the selected ambush row to combat
/// setup, it restores each rest-local snapshot status: members who
/// entered rest Poisoned stay Poisoned, and other eligible
/// sleepers/watch participants are restored to Good. Dead, Charmed,
/// and Ashes members are not turned into active combatants.
///
/// Returns the status the rest helper should write back for the
/// member's combat-entry status, given the status the member held at
/// rest entry.
pub const fn sleep_ambush_restored_status(entry_status: CharacterStatus) -> CharacterStatus {
    match entry_status {
        CharacterStatus::Poisoned => CharacterStatus::Poisoned,
        CharacterStatus::Good | CharacterStatus::Sleeping => CharacterStatus::Good,
        other => other,
    }
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
        b'P' => CharacterStatus::Poisoned,
        b'S' => CharacterStatus::Sleeping,
        b'C' => CharacterStatus::Charmed,
        b'D' => CharacterStatus::Dead,
        b'A' => CharacterStatus::Ashes,
        _ => return None,
    })
}
