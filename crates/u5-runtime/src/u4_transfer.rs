//! Ultima IV transfer helpers for producing a fresh U5 saved game.

use std::io;
use std::path::Path;

use crate::*;

pub const U5_TRANSFER_MALE_BYTE: u8 = SAVE_GENDER_MALE_BYTE;
pub const U5_TRANSFER_FEMALE_BYTE: u8 = SAVE_GENDER_FEMALE_BYTE;

/// `u4-transfer.md §4` published filenames the transfer flow reads and
/// writes. The U5-side transfer seed pair is `INIT.GAM` (save image)
/// and `INIT.OOL` (object overlay), the U4-side source is the player
/// disk's `PARTY.SAV`, and the commit destination is the ordinary U5
/// save pair `SAVED.GAM` / `SAVED.OOL`. The seeds are read-only; only
/// the commit step writes anything to disk.
///
/// `BRIT.GAM` is **withdrawn** (`cleak/u5-spec#88`) and this pair used
/// to name it. It is not merely the wrong seed: `BRIT.GAM` does not
/// ship at all, so the old constant opened a filename with nothing
/// behind it and the commit path could never have succeeded. Verified
/// against the shipped install rather than from the retraction:
/// `BRIT.GAM` is absent; `INIT.GAM` is 4192 bytes, the same length as
/// `SAVED.GAM`. The overlay seed was wrong in a quieter way -
/// `BRIT.OOL` ships as 256 zero bytes, so seeding from it produced an
/// empty object overlay, while `INIT.OOL` carries the seed records and
/// is byte-identical to `UNDER.OOL`.
pub const U4_TRANSFER_U5_SEED_GAM_FILENAME: &str = crate::INIT_GAM_FILENAME;
/// `u4-transfer.md §4` U5-side seed object-overlay filename. Anchored
/// to [`crate::INIT_OOL_FILENAME`] so the transfer seed alias and the
/// canonical filename stay one value.
pub const U4_TRANSFER_U5_SEED_OOL_FILENAME: &str = crate::INIT_OOL_FILENAME;
pub const U4_TRANSFER_U4_SOURCE_FILENAME: &str = "PARTY.SAV";
// `u4-transfer.md §5.1`/`§5.2`/`§5.3` (`cleak/u5-spec#88`): the
// `PARTY.SAV` layout, the validation gate and the Avatarhood test all
// live in [`crate::u4_transfer_preview`], which is now the only
// `PARTY.SAV` parser in the engine.
//
// This module used to carry a second, older parser
// (`parse_u4_transfer_source_from_party_sav`) written against a
// withdrawn revision of §5. It was wrong on three counts and it is
// retired rather than repaired, because two parsers for one file
// format are how the disagreement survived review:
//
// - It rejected all-zero virtue standings as "no transferable data".
//   §5.3 settles that this is exactly backwards: **all-zero is the
//   Avatar success condition**, and "no value of this block ever
//   prevents a transfer". The old gate turned away precisely the
//   completed Ultima IV Avatar this path exists to import.
// - It validated party-wide counters (move, moon, dungeon, gold,
//   food, gems, torches, keys, sextants). §5.2: "No party-wide counter
//   is validated", and §5.4 adds that the original structurally cannot
//   validate them - the party-wide block is not read until after
//   validation has passed and the leading record has been copied.
// - It read the name at file offset `0x001A` and the class at
//   `0x0019`. §5.1/§5.4: the record base **is** the first read's seek
//   target `0x0008`, so the name is at file offset `0x001C` and the
//   class byte at `0x002D`.
//
// The constants that described those wrong reads are deleted with the
// parser; the surviving published layout is exported from
// [`crate::u4_transfer_preview`].

/// `u4-transfer.md §7`: when §5.3's Avatarhood test passed, the class
/// letter translated from the source class index is overwritten with
/// the Avatar class letter. Anchored to
/// [`crate::CHARGEN_AVATAR_SEED_CLASS_BYTE`] so chargen and transfer
/// spell "Avatar" in the save image with one value.
pub const U4_TRANSFER_AVATAR_CLASS_BYTE: u8 = crate::CHARGEN_AVATAR_SEED_CLASS_BYTE;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct U4TransferSource {
    pub name: Vec<u8>,
    pub male: bool,
    pub class_index: u8,
    pub strength: u16,
    pub dexterity: u16,
    pub intelligence: u16,
    pub experience: u32,
    /// `u4-transfer.md §5.3`: set when all eight virtue standings are
    /// individually zero. It never rejects a transfer. Its only
    /// observable effects are `§7`'s class override to
    /// [`U4_TRANSFER_AVATAR_CLASS_BYTE`] and `§6.3`/`§6.5`/`§6.6`'s
    /// alternate display strings. The flag is a one-shot latch per
    /// transfer attempt and is never cleared.
    pub is_avatar: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct U4TransferOverrides {
    pub name: Option<Vec<u8>>,
    pub male: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct U4TransferAvatar {
    pub name: [u8; SAVE_CHARACTER_NAME_LEN],
    pub male: bool,
    /// `u4-transfer.md §7`: the translated class letter, or
    /// [`U4_TRANSFER_AVATAR_CLASS_BYTE`] when `§5.3`'s Avatarhood test
    /// passed.
    pub class_byte: u8,
    pub strength: u8,
    pub dexterity: u8,
    pub intelligence: u8,
    pub experience: u16,
    pub level: u8,
    pub hp: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum U4TransferError {
    InvalidClassIndex(u8),
    InvalidNameByte(u8),
    BlankName,
    SaveTooShort(usize),
}

impl std::fmt::Display for U4TransferError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidClassIndex(index) => {
                write!(f, "U4 transfer class index must be 0..7, got {index}")
            }
            Self::InvalidNameByte(byte) => {
                write!(f, "U4 transfer name contains invalid byte 0x{byte:02x}")
            }
            Self::BlankName => write!(f, "U4 transfer name must not be blank"),
            Self::SaveTooShort(len) => write!(
                f,
                "U5 transfer seed must contain slot 0 record, got {len} bytes"
            ),
        }
    }
}

impl std::error::Error for U4TransferError {}

pub fn u4_transfer_class_byte(class_index: u8) -> Option<u8> {
    match class_index {
        0 => Some(b'M'),
        1 => Some(b'B'),
        2 => Some(b'F'),
        3 => Some(b'D'),
        4 => Some(b'T'),
        5 => Some(b'P'),
        6 => Some(b'R'),
        7 => Some(b'S'),
        _ => None,
    }
}

/// `u4-transfer.md §7` primary-attribute translator band boundaries.
/// Values `0..LOW_BAND_END` (0..=9) pass through unchanged; values
/// `LOW_BAND_END..MID_BAND_END` (10..=29) halve their excess over
/// `LOW_BAND_BIAS` and add `MID_BAND_BIAS` (= 10); values
/// `MID_BAND_END..` (30+) quarter their excess over `HIGH_BAND_BIAS`
/// and add `HIGH_BAND_BIAS_OUT` (= 20). Promote the band edges,
/// biases, and divisors so the translator's piecewise formula has
/// one named source of truth.
pub const U4_TRANSFER_ATTRIBUTE_LOW_BAND_END: u16 = 10;
pub const U4_TRANSFER_ATTRIBUTE_MID_BAND_END: u16 = 30;
pub const U4_TRANSFER_ATTRIBUTE_LOW_BAND_BIAS: u16 = 9;
pub const U4_TRANSFER_ATTRIBUTE_MID_BAND_DIVISOR: u16 = 2;
pub const U4_TRANSFER_ATTRIBUTE_MID_BAND_BIAS_OUT: u16 = 10;
pub const U4_TRANSFER_ATTRIBUTE_HIGH_BAND_BIAS: u16 = 30;
pub const U4_TRANSFER_ATTRIBUTE_HIGH_BAND_DIVISOR: u16 = 4;
pub const U4_TRANSFER_ATTRIBUTE_HIGH_BAND_BIAS_OUT: u16 = 20;
/// `u4-transfer.md §7` Strength floor: after the band-translator
/// runs, Strength alone is floored to 20. Dexterity and Intelligence
/// are not floored. This floor matches the chargen Strength floor
/// (the questionnaire pass also caps Strength below at 20).
/// Anchored to [`crate::CHARGEN_STR_FLOOR`] so the published "20
/// is the minimum starting Strength" rule has one source of truth
/// for both chargen and U4 transfer.
pub const U4_TRANSFER_STRENGTH_FLOOR: u8 = crate::CHARGEN_STR_FLOOR;
/// `u4-transfer.md §7` experience-translator divisor. Source XP is
/// divided by this value, truncating toward zero.
pub const U4_TRANSFER_EXPERIENCE_DIVISOR: u32 = 10;

pub fn u4_transfer_attribute_to_u5(value: u16) -> u8 {
    let converted = if value < U4_TRANSFER_ATTRIBUTE_LOW_BAND_END {
        value
    } else if value < U4_TRANSFER_ATTRIBUTE_MID_BAND_END {
        (value - U4_TRANSFER_ATTRIBUTE_LOW_BAND_BIAS) / U4_TRANSFER_ATTRIBUTE_MID_BAND_DIVISOR
            + U4_TRANSFER_ATTRIBUTE_MID_BAND_BIAS_OUT
    } else {
        (value - U4_TRANSFER_ATTRIBUTE_HIGH_BAND_BIAS) / U4_TRANSFER_ATTRIBUTE_HIGH_BAND_DIVISOR
            + U4_TRANSFER_ATTRIBUTE_HIGH_BAND_BIAS_OUT
    };
    converted.min(u8::MAX as u16) as u8
}

pub fn u4_transfer_strength_to_u5(value: u16) -> u8 {
    u4_transfer_attribute_to_u5(value).max(U4_TRANSFER_STRENGTH_FLOOR)
}

pub fn u4_transfer_experience_to_u5(value: u32) -> u16 {
    (value / U4_TRANSFER_EXPERIENCE_DIVISOR).min(u16::MAX as u32) as u16
}

/// `u4-transfer.md §5`: read the Ultima IV player disk's `PARTY.SAV`
/// and return the leading transferable record.
///
/// There is exactly one `PARTY.SAV` parser in the engine
/// ([`parse_u4_preview_source`]); this is the commit-side view of its
/// result. The terminal path used to call a second parser written
/// against a withdrawn revision of §5 - see the module note above and
/// `cleak/u5-spec#88`.
pub fn read_u4_transfer_source_from_party_sav(game_dir: &Path) -> io::Result<U4TransferSource> {
    let bytes = read_disk_file(&game_dir.join(U4_TRANSFER_U4_SOURCE_FILENAME))?;
    let preview = crate::u4_transfer_preview::parse_u4_preview_source(&bytes)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    Ok(preview.to_transfer_source())
}

pub fn apply_u4_transfer_to_save(
    save: &mut [u8],
    source: &U4TransferSource,
    overrides: Option<&U4TransferOverrides>,
) -> Result<U4TransferAvatar, U4TransferError> {
    if save.len() < SAVE_ROSTER_OFFSET + SAVE_CHARACTER_RECORD_LEN {
        return Err(U4TransferError::SaveTooShort(save.len()));
    }
    // `u4-transfer.md §7`: the source class index is translated into a
    // U5 class letter first, and that letter is *then* overwritten with
    // the Avatar letter when `§5.3`'s Avatarhood test passed. Transfer
    // therefore leaves roster slot 0 with a non-Avatar class only when
    // the source character had not attained all eight virtues.
    let translated = u4_transfer_class_byte(source.class_index)
        .ok_or(U4TransferError::InvalidClassIndex(source.class_index))?;
    let class_byte = if source.is_avatar {
        U4_TRANSFER_AVATAR_CLASS_BYTE
    } else {
        translated
    };
    let name_bytes = overrides
        .and_then(|overrides| overrides.name.as_deref())
        .unwrap_or(&source.name);
    let name = normalize_u4_transfer_name(name_bytes)?;
    let male = overrides
        .and_then(|overrides| overrides.male)
        .unwrap_or(source.male);
    let strength = u4_transfer_strength_to_u5(source.strength);
    let dexterity = u4_transfer_attribute_to_u5(source.dexterity);
    let intelligence = u4_transfer_attribute_to_u5(source.intelligence);
    let experience = u4_transfer_experience_to_u5(source.experience);
    let level = recompute_level_from_experience(experience);
    let hp = u16::from(level) * 30;

    let record = SAVE_ROSTER_OFFSET;
    save[record..record + SAVE_CHARACTER_NAME_LEN].copy_from_slice(&name);
    save[record + SAVE_CHARACTER_GENDER_OFFSET] = if male {
        U5_TRANSFER_MALE_BYTE
    } else {
        U5_TRANSFER_FEMALE_BYTE
    };
    save[record + SAVE_CHARACTER_CLASS_OFFSET] = class_byte;
    save[record + SAVE_CHARACTER_STATUS_OFFSET] = b'G';
    save[record + SAVE_CHARACTER_STR_OFFSET] = strength;
    save[record + SAVE_CHARACTER_DEX_OFFSET] = dexterity;
    save[record + SAVE_CHARACTER_INT_OFFSET] = intelligence;
    save[record + SAVE_CHARACTER_MANA_OFFSET] = intelligence;
    write_u16_at(save, record + SAVE_CHARACTER_HP_OFFSET, hp);
    write_u16_at(save, record + SAVE_CHARACTER_MAX_HP_OFFSET, hp);
    write_u16_at(save, record + SAVE_CHARACTER_EXPERIENCE_OFFSET, experience);
    save[record + SAVE_CHARACTER_LEVEL_OFFSET] = level;

    Ok(U4TransferAvatar {
        name,
        male,
        class_byte,
        strength,
        dexterity,
        intelligence,
        experience,
        level,
        hp,
    })
}

pub fn commit_u4_transfer_save(
    game_dir: &Path,
    source: &U4TransferSource,
    overrides: Option<&U4TransferOverrides>,
) -> io::Result<U4TransferAvatar> {
    let mut save = read_save_image_file(
        &game_dir.join(U4_TRANSFER_U5_SEED_GAM_FILENAME),
        U4_TRANSFER_U5_SEED_GAM_FILENAME,
    )?;
    let brit_ool = read_brit_ool_plane(game_dir)?;
    let avatar = apply_u4_transfer_to_save(&mut save, source, overrides)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;

    let mut saved_ool = vec![0; SAVED_OOL_LEN];
    saved_ool[OOL_PLANE_LEN..].copy_from_slice(&brit_ool);
    write_disk_file(&game_dir.join(SAVED_OOL_FILENAME), saved_ool)?;
    write_disk_file(&game_dir.join(SAVED_GAM_FILENAME), save)?;

    Ok(avatar)
}

fn normalize_u4_transfer_name(
    name_bytes: &[u8],
) -> Result<[u8; SAVE_CHARACTER_NAME_LEN], U4TransferError> {
    let mut name = [0; SAVE_CHARACTER_NAME_LEN];
    let mut copied = 0;
    let mut has_non_space = false;
    for &byte in name_bytes.iter().take(SAVE_CHARACTER_NAME_LEN - 1) {
        if byte == 0 {
            break;
        }
        if !(0x20..=0x7e).contains(&byte) {
            return Err(U4TransferError::InvalidNameByte(byte));
        }
        if byte != b' ' {
            has_non_space = true;
        }
        name[copied] = byte;
        copied += 1;
    }
    for &byte in name_bytes
        .iter()
        .take(SAVE_CHARACTER_NAME_LEN - 1)
        .skip(copied)
    {
        if byte != 0 && !(0x20..=0x7e).contains(&byte) {
            return Err(U4TransferError::InvalidNameByte(byte));
        }
    }
    if !has_non_space {
        return Err(U4TransferError::BlankName);
    }
    Ok(name)
}

fn read_brit_ool_plane(game_dir: &Path) -> io::Result<Vec<u8>> {
    let bytes = read_disk_file(&game_dir.join(U4_TRANSFER_U5_SEED_OOL_FILENAME))?;
    if bytes.len() != OOL_PLANE_LEN {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{U4_TRANSFER_U5_SEED_OOL_FILENAME} must be {OOL_PLANE_LEN} bytes, got {}",
                bytes.len()
            ),
        ));
    }
    Ok(bytes)
}
