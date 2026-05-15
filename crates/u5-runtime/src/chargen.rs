//! Character-creation save producer.

use std::fs;
use std::io;
use std::path::Path;

use crate::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChargenStats {
    pub strength: u8,
    pub dexterity: u8,
    pub intelligence: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChargenAvatar {
    pub name: [u8; SAVE_CHARACTER_NAME_LEN],
    pub male: bool,
    pub stats: ChargenStats,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChargenError {
    BlankName,
    InvalidNameByte(u8),
    SameVirtuePair,
    SaveTooShort(usize),
}

impl std::fmt::Display for ChargenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BlankName => write!(f, "character name must not be blank"),
            Self::InvalidNameByte(byte) => {
                write!(f, "character name contains invalid byte 0x{byte:02x}")
            }
            Self::SameVirtuePair => write!(f, "chargen question pair must use two virtues"),
            Self::SaveTooShort(len) => write!(
                f,
                "chargen seed must contain slot 0 record, got {len} bytes"
            ),
        }
    }
}

impl std::error::Error for ChargenError {}

/// `chargen.md §6` — total questions in the questionnaire tournament:
/// round 1 = 4 questions, round 2 = 2 questions, round 3 = 1 question.
pub const CHARGEN_QUESTION_COUNT: usize = 7;
/// `chargen.md §6` — number of tournament rounds.
pub const CHARGEN_ROUND_COUNT: usize = 3;
/// `chargen.md §6` — questions per round, indexed by round (0..3).
pub const CHARGEN_QUESTIONS_PER_ROUND: [usize; CHARGEN_ROUND_COUNT] = [4, 2, 1];

/// `chargen.md §4` Avatar name-prompt input limit. The free-text
/// prompt accepts up to eight characters; shorter names are
/// null-padded into the eight-byte name slice. The save record's
/// name field is nine bytes wide; chargen leaves the ninth byte as
/// the seed padding.
pub const CHARGEN_NAME_INPUT_MAX_LEN: usize = 8;
pub const CHARGEN_NAME_FIELD_LEN: usize = 9;

pub fn chargen_question_record_for_pair(
    first: ShrineVirtue,
    second: ShrineVirtue,
) -> Result<usize, ChargenError> {
    let a = first.index();
    let b = second.index();
    if a == b {
        return Err(ChargenError::SameVirtuePair);
    }
    let (low, high) = if a < b { (a, b) } else { (b, a) };
    Ok(2 + (0..low).map(|row| 7 - row).sum::<usize>() + (high - low - 1))
}

pub fn chargen_virtue_stat_delta(virtue: ShrineVirtue) -> ChargenStats {
    match virtue {
        ShrineVirtue::Honesty => ChargenStats {
            strength: 0,
            dexterity: 0,
            intelligence: 2,
        },
        ShrineVirtue::Compassion => ChargenStats {
            strength: 0,
            dexterity: 2,
            intelligence: 0,
        },
        ShrineVirtue::Valor => ChargenStats {
            strength: 2,
            dexterity: 0,
            intelligence: 0,
        },
        ShrineVirtue::Justice => ChargenStats {
            strength: 0,
            dexterity: 1,
            intelligence: 1,
        },
        ShrineVirtue::Sacrifice => ChargenStats {
            strength: 1,
            dexterity: 1,
            intelligence: 0,
        },
        ShrineVirtue::Honor => ChargenStats {
            strength: 1,
            dexterity: 0,
            intelligence: 1,
        },
        ShrineVirtue::Spirituality => ChargenStats {
            strength: 1,
            dexterity: 1,
            intelligence: 1,
        },
        ShrineVirtue::Humility => ChargenStats {
            strength: 0,
            dexterity: 0,
            intelligence: 0,
        },
    }
}

/// `chargen.md §7`: STR floor applied after summing the questionnaire
/// totals. Because the maximum per-question STR contribution is two and
/// the questionnaire has seven questions, the floor always fires for the
/// questionnaire path and every newly-created avatar emerges with exactly
/// twenty Strength.
pub const CHARGEN_STR_FLOOR: u8 = 20;

/// `chargen.md §8`: starting party size for a fresh-from-questionnaire
/// save (Avatar plus Iolo and Shamino in scene 13, Iolo's Hut).
pub const CHARGEN_STARTING_PARTY_SIZE: u8 = 3;

pub fn chargen_stats_from_winners(winners: &[ShrineVirtue]) -> ChargenStats {
    let mut strength = 0u8;
    let mut dexterity = 0u8;
    let mut intelligence = 0u8;
    for winner in winners {
        let delta = chargen_virtue_stat_delta(*winner);
        strength = strength.saturating_add(delta.strength);
        dexterity = dexterity.saturating_add(delta.dexterity);
        intelligence = intelligence.saturating_add(delta.intelligence);
    }
    ChargenStats {
        strength: strength.max(CHARGEN_STR_FLOOR),
        dexterity,
        intelligence,
    }
}

pub fn apply_chargen_to_save(
    save: &mut [u8],
    name_bytes: &[u8],
    male: bool,
    stats: ChargenStats,
) -> Result<ChargenAvatar, ChargenError> {
    if save.len() < SAVE_ROSTER_OFFSET + SAVE_CHARACTER_RECORD_LEN {
        return Err(ChargenError::SaveTooShort(save.len()));
    }
    let name = normalize_chargen_name(name_bytes)?;
    let record = SAVE_ROSTER_OFFSET;
    save[record..record + SAVE_CHARACTER_NAME_LEN - 1]
        .copy_from_slice(&name[..SAVE_CHARACTER_NAME_LEN - 1]);
    let mut saved_name = name;
    saved_name[SAVE_CHARACTER_NAME_LEN - 1] = save[record + SAVE_CHARACTER_NAME_LEN - 1];
    save[record + SAVE_CHARACTER_GENDER_OFFSET] = if male {
        SAVE_GENDER_MALE_BYTE
    } else {
        SAVE_GENDER_FEMALE_BYTE
    };
    save[record + SAVE_CHARACTER_STR_OFFSET] = stats.strength;
    save[record + SAVE_CHARACTER_DEX_OFFSET] = stats.dexterity;
    save[record + SAVE_CHARACTER_INT_OFFSET] = stats.intelligence;
    save[record + SAVE_CHARACTER_MANA_OFFSET] = stats.intelligence;

    Ok(ChargenAvatar {
        name: saved_name,
        male,
        stats,
    })
}

pub fn commit_chargen_save(
    game_dir: &Path,
    name_bytes: &[u8],
    male: bool,
    stats: ChargenStats,
) -> io::Result<ChargenAvatar> {
    let mut save = read_save_image_file(&game_dir.join("INIT.GAM"), "INIT.GAM")?;
    let init_ool = read_init_ool_plane(game_dir)?;
    let avatar = apply_chargen_to_save(&mut save, name_bytes, male, stats)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))?;

    let mut saved_ool = vec![0; SAVED_OOL_LEN];
    saved_ool[OOL_PLANE_LEN..].copy_from_slice(&init_ool);
    fs::write(game_dir.join("SAVED.OOL"), saved_ool)?;
    fs::write(game_dir.join("SAVED.GAM"), save)?;

    Ok(avatar)
}

fn normalize_chargen_name(
    name_bytes: &[u8],
) -> Result<[u8; SAVE_CHARACTER_NAME_LEN], ChargenError> {
    let mut name = [0; SAVE_CHARACTER_NAME_LEN];
    let mut has_non_space = false;
    for (index, &byte) in name_bytes
        .iter()
        .take(SAVE_CHARACTER_NAME_LEN - 1)
        .enumerate()
    {
        if !(0x20..=0x7e).contains(&byte) {
            return Err(ChargenError::InvalidNameByte(byte));
        }
        if byte != b' ' {
            has_non_space = true;
        }
        name[index] = byte;
    }
    if !has_non_space {
        return Err(ChargenError::BlankName);
    }
    Ok(name)
}

fn read_init_ool_plane(game_dir: &Path) -> io::Result<Vec<u8>> {
    let bytes = fs::read(game_dir.join("INIT.OOL"))?;
    if bytes.len() != OOL_PLANE_LEN {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "INIT.OOL must be {OOL_PLANE_LEN} bytes, got {}",
                bytes.len()
            ),
        ));
    }
    Ok(bytes)
}
