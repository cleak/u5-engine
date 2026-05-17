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

/// `chargen.md §6` — number of tournament rounds.
pub const CHARGEN_ROUND_COUNT: usize = 3;
/// `chargen.md §6` — questions per round, indexed by round (0..3).
pub const CHARGEN_QUESTIONS_PER_ROUND: [usize; CHARGEN_ROUND_COUNT] = [4, 2, 1];
/// `chargen.md §6` — total questions in the questionnaire
/// tournament. Anchored to the sum of CHARGEN_QUESTIONS_PER_ROUND
/// so the total derives from the per-round breakdown.
pub const CHARGEN_QUESTION_COUNT: usize = CHARGEN_QUESTIONS_PER_ROUND[0]
    + CHARGEN_QUESTIONS_PER_ROUND[1]
    + CHARGEN_QUESTIONS_PER_ROUND[2];

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

/// `chargen.md §8` factory-seed Avatar header values the seed image
/// dictates for a freshly questionnaire-created character. Chargen
/// only customises the name, gender, STR, DEX, INT, and MP fields;
/// these record bytes are inherited from `INIT.GAM` unchanged.
///
/// The freshly seeded current-HP value matches the global
/// [`crate::DEFAULT_PARTY_HP`] starting-HP anchor; the seeded
/// max-HP value equals the current-HP value because chargen sets
/// both to the same starting value. Anchor both through to those
/// chains so a chargen-HP rebalance flows through one source of
/// truth.
pub const CHARGEN_AVATAR_SEED_CURRENT_HP: u16 = crate::DEFAULT_PARTY_HP;
pub const CHARGEN_AVATAR_SEED_MAX_HP: u16 = CHARGEN_AVATAR_SEED_CURRENT_HP;
pub const CHARGEN_AVATAR_SEED_EXPERIENCE: u16 = 150;
pub const CHARGEN_AVATAR_SEED_LEVEL: u8 = 2;
pub const CHARGEN_AVATAR_SEED_CLASS_BYTE: u8 = b'A';
pub const CHARGEN_AVATAR_SEED_STATUS_BYTE: u8 = b'G';

/// `chargen.md §8`: starting party size for a fresh-from-questionnaire
/// save (Avatar plus Iolo and Shamino in scene 13, Iolo's Hut).
pub const CHARGEN_STARTING_PARTY_SIZE: u8 = 3;

/// `chargen.md §8` seeded starting-inventory counters for a fresh
/// questionnaire-created save. Chargen does not write these; they
/// come from `INIT.GAM` unchanged. Listing them as named constants
/// lets fixture builders and verification checks compare a freshly
/// generated save against the published seed values.
///
/// The seeded counters are the same values as the global
/// `DEFAULT_*` initial-inventory anchors in `constants.rs`, so
/// they are aliased through to those constants. A future
/// rebalance of either the chargen seed or the global default
/// flows through both names.
pub const CHARGEN_SEED_FOOD: u16 = crate::DEFAULT_FOOD_STOCK;
pub const CHARGEN_SEED_GOLD: u16 = crate::DEFAULT_GOLD_STOCK;
pub const CHARGEN_SEED_KEYS: u8 = crate::DEFAULT_KEY_STOCK;
pub const CHARGEN_SEED_GEMS: u8 = crate::DEFAULT_GEM_STOCK;
pub const CHARGEN_SEED_TORCHES: u8 = crate::DEFAULT_TORCH_STOCK;
pub const CHARGEN_SEED_MAGIC_POWDER: u8 = 0;

/// `chargen.md §8` seeded reagent counters for a fresh
/// questionnaire-created save. Mandrake, spider silk, and sulfurous
/// ash start at zero — the player must source them before mixing the
/// spells those reagents gate.
///
/// Each CHARGEN_SEED_REAGENT_* equals the matching
/// `crate::DEFAULT_REAGENTS[crate::REAGENT_*]` slot. Anchor each
/// seed through the storage-indexed array so a rebalance of either
/// the chargen seed or the global default reagent stock flows
/// through one source of truth.
pub const CHARGEN_SEED_REAGENT_BLACK_PEARL: u8 =
    crate::DEFAULT_REAGENTS[crate::REAGENT_BLACK_PEARL];
pub const CHARGEN_SEED_REAGENT_BLOOD_MOSS: u8 =
    crate::DEFAULT_REAGENTS[crate::REAGENT_BLOOD_MOSS];
pub const CHARGEN_SEED_REAGENT_GARLIC: u8 =
    crate::DEFAULT_REAGENTS[crate::REAGENT_GARLIC];
pub const CHARGEN_SEED_REAGENT_GINSENG: u8 =
    crate::DEFAULT_REAGENTS[crate::REAGENT_GINSENG];
pub const CHARGEN_SEED_REAGENT_MANDRAKE: u8 =
    crate::DEFAULT_REAGENTS[crate::REAGENT_MANDRAKE];
pub const CHARGEN_SEED_REAGENT_NIGHTSHADE: u8 =
    crate::DEFAULT_REAGENTS[crate::REAGENT_NIGHTSHADE];
pub const CHARGEN_SEED_REAGENT_SPIDER_SILK: u8 =
    crate::DEFAULT_REAGENTS[crate::REAGENT_SPIDER_SILK];
pub const CHARGEN_SEED_REAGENT_SULFUROUS_ASH: u8 =
    crate::DEFAULT_REAGENTS[crate::REAGENT_SULFUR_ASH];

/// `chargen.md §4` maximum visible characters the name prompt accepts.
/// The shipped prompt prints "By what name shalt thou be known?" and
/// opens a free-text input prompt with this many characters of room.
/// The save record's name field is one byte longer
/// ([`SAVE_CHARACTER_NAME_LEN`] = 9) so the trailing byte stays as
/// seed padding when the player enters the maximum length.
pub const CHARGEN_NAME_INPUT_LIMIT: usize = SAVE_CHARACTER_NAME_LEN - 1;

/// `chargen.md §8` starting map tuple for a fresh-from-questionnaire
/// save. The chargen writer seeds scene 13 (Iolo's Hut) on floor /
/// Z 0 at local cell (15, 15) with a zero saved-scene scratch byte.
/// These bytes come from `INIT.GAM`; chargen does not customise them.
pub const CHARGEN_STARTING_SCENE: u8 = 13;
/// `chargen.md §8` / `town-mode.md §3`: the chargen exit cell
/// uses the engine-wide town-entry default column (X = 15).
/// Anchored to [`crate::LOCATION_DEFAULT_ENTRY_X`] so the
/// town-entry default and the chargen exit column stay one value.
pub const CHARGEN_STARTING_X: u8 = crate::LOCATION_DEFAULT_ENTRY_X as u8;
pub const CHARGEN_STARTING_Y: u8 = 15;
pub const CHARGEN_STARTING_Z: u8 = 0;
pub const CHARGEN_STARTING_SAVED_SCENE_SCRATCH: u8 = 0;

/// `formats/saved-gam.md §3.1` per-character defense byte the
/// factory seed ships for every roster slot. No traced writer
/// currently recomputes this byte from readied equipment, so the
/// shipped value is also the runtime value the combat damage
/// path's random defense subtraction reads.
pub const CHARGEN_SEED_DEFENSE_BYTE: u8 = 7;

/// `chargen.md §8` starting calendar values for a fresh-from-
/// questionnaire save. The save clock begins at year 139, month 4,
/// day 5, 08:35 of the in-world calendar. These bytes come from
/// `INIT.GAM`; chargen does not customise them.
pub const CHARGEN_STARTING_YEAR: u16 = 139;
pub const CHARGEN_STARTING_MONTH: u8 = 4;
pub const CHARGEN_STARTING_DAY: u8 = 5;
pub const CHARGEN_STARTING_HOUR: u8 = 8;
pub const CHARGEN_STARTING_MINUTE: u8 = 35;

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
    let mut save = read_save_image_file(
        &game_dir.join(INIT_GAM_FILENAME),
        INIT_GAM_FILENAME,
    )?;
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
        .take(CHARGEN_NAME_INPUT_LIMIT)
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
    let bytes = fs::read(game_dir.join(INIT_OOL_FILENAME))?;
    if bytes.len() != OOL_PLANE_LEN {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{INIT_OOL_FILENAME} must be {OOL_PLANE_LEN} bytes, got {}",
                bytes.len()
            ),
        ));
    }
    Ok(bytes)
}
