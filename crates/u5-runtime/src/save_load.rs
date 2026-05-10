//! Loaders that turn SAVED.GAM/SAVED.OOL/INIT.GAM into PlayOptions.

use std::collections::{HashMap, VecDeque};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::*;

pub fn load_play_options_from_save(game_dir: &Path) -> io::Result<PlayOptions> {
    let mut options = load_play_options_from_save_file(game_dir, "SAVED.GAM", "--from-save", true)?;
    refresh_saved_ool_mirrors_for_load(game_dir)?;
    options.save_template_source = SaveTemplateSource::SavedGame;
    Ok(options)
}

pub fn load_play_options_from_init(game_dir: &Path) -> io::Result<PlayOptions> {
    let mut options = load_play_options_from_save_file(game_dir, "INIT.GAM", "--from-init", false)?;
    options.initial_britannia_overlay = Some(load_init_overlay_objects(game_dir)?);
    options.save_template_source = SaveTemplateSource::InitGame;
    Ok(options)
}

pub fn load_save_image_template(game_dir: &Path, source: SaveTemplateSource) -> io::Result<Vec<u8>> {
    match source {
        SaveTemplateSource::SavedGame => {
            read_save_image_file(&game_dir.join("SAVED.GAM"), "SAVED.GAM")
        }
        SaveTemplateSource::InitGame => {
            read_save_image_file(&game_dir.join("INIT.GAM"), "INIT.GAM")
        }
        SaveTemplateSource::PreferSavedGame => {
            let saved = game_dir.join("SAVED.GAM");
            if saved.exists() {
                return read_save_image_file(&saved, "SAVED.GAM");
            }
            let init = game_dir.join("INIT.GAM");
            if init.exists() {
                return read_save_image_file(&init, "INIT.GAM");
            }
            Err(io::Error::new(
                io::ErrorKind::NotFound,
                "saving requires an existing SAVED.GAM or INIT.GAM template",
            ))
        }
    }
}

pub fn read_save_image_file(path: &Path, file_name: &str) -> io::Result<Vec<u8>> {
    let bytes = read(path)?;
    if bytes.len() != SAVED_GAM_LEN {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{file_name} must be {SAVED_GAM_LEN} bytes, got {}",
                bytes.len()
            ),
        ));
    }
    Ok(bytes)
}

pub fn load_play_options_from_save_file(
    game_dir: &Path,
    file_name: &str,
    option_name: &str,
    include_active_objects: bool,
) -> io::Result<PlayOptions> {
    let bytes = read(&game_dir.join(file_name))?;
    play_options_from_save_bytes_named(&bytes, file_name, option_name, include_active_objects)
}

#[cfg(test)]
pub fn play_options_from_save_bytes(bytes: &[u8]) -> io::Result<PlayOptions> {
    play_options_from_save_bytes_named(bytes, "SAVED.GAM", "--from-save", true)
}

pub fn play_options_from_save_bytes_named(
    bytes: &[u8],
    file_name: &str,
    option_name: &str,
    include_active_objects: bool,
) -> io::Result<PlayOptions> {
    if bytes.len() != SAVED_GAM_LEN {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{file_name} must be {SAVED_GAM_LEN} bytes, got {}",
                bytes.len()
            ),
        ));
    }
    let _avatar_name_present = saved_game_has_avatar_name(bytes);
    let scene_byte = bytes[SAVE_SCENE_OFFSET];
    if scene_byte > 40 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "{option_name} currently supports overworld, town-family, or stock dungeon scenes only; scene is {scene_byte}"
            ),
        ));
    }
    let z = bytes[SAVE_Z_OFFSET];
    let x = bytes[SAVE_X_OFFSET] as usize;
    let y = bytes[SAVE_Y_OFFSET] as usize;
    let (target, floor) = if scene_byte == 0 {
        if x >= WORLD_SIDE || y >= WORLD_SIDE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("saved world position must be inside 0..255, got ({x}, {y})"),
            ));
        }
        let plane = WorldPlane::from_save_z(z);
        (PlayTarget::World(plane), plane.save_floor())
    } else if scene_byte <= 32 {
        if x >= 32 || y >= 32 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("saved town position must be inside 0..31, got ({x}, {y})"),
            ));
        }
        (PlayTarget::Town(Scene::new(scene_byte)?), z as i8)
    } else {
        if z > 7 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("saved dungeon level must be inside 0..7, got {z}"),
            ));
        }
        if x >= DUNGEON_SIDE || y >= DUNGEON_SIDE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("saved dungeon position must be inside 0..7, got ({x}, {y})"),
            ));
        }
        (PlayTarget::Dungeon(DungeonScene::new(scene_byte)?), z as i8)
    };

    let mut spell_charges = [0; SPELL_COUNT];
    spell_charges.copy_from_slice(
        &bytes[SAVE_SPELL_CHARGES_OFFSET..SAVE_SPELL_CHARGES_OFFSET + SPELL_COUNT],
    );
    let moonstone_slots = decode_moonstone_gate_slots(bytes);
    let reagents = decode_reagent_stock(bytes);
    let avatar_stats = decode_avatar_stats(bytes);

    Ok(PlayOptions {
        target,
        floor,
        start: Some((x, y)),
        clock: GameClock::with_date(
            u16_at(bytes, SAVE_YEAR_OFFSET),
            bytes[SAVE_MONTH_OFFSET],
            bytes[SAVE_DAY_OFFSET],
            bytes[SAVE_HOUR_OFFSET],
            bytes[SAVE_MINUTE_OFFSET],
        )?,
        food: u16_at(bytes, SAVE_FOOD_STOCK_OFFSET),
        gold: u16_at(bytes, SAVE_GOLD_STOCK_OFFSET),
        keys: bytes[SAVE_KEY_STOCK_OFFSET],
        gems: bytes[SAVE_GEM_STOCK_OFFSET],
        climbing_gear: DEFAULT_CLIMBING_GEAR,
        party: decode_save_party(bytes),
        spell_charges,
        reagents,
        moonstone_slots,
        shrine_ordained_mask: bytes[SAVE_SHRINE_ORDAINED_MASK_OFFSET],
        shrine_codex_mask: bytes[SAVE_SHRINE_CODEX_MASK_OFFSET],
        shrine_standing: [0; VIRTUE_COUNT],
        avatar_stats,
        torches: bytes[SAVE_TORCH_STOCK_OFFSET],
        torch_counter: bytes[SAVE_TORCH_COUNTER_OFFSET],
        light_spell_counter: bytes[SAVE_LIGHT_SPELL_COUNTER_OFFSET],
        wind: WindState::from_save_byte(bytes[SAVE_WIND_OFFSET]),
        wind_save_byte: bytes[SAVE_WIND_OFFSET],
        timing_status: TimingStatusTag::from_save_byte(bytes[SAVE_TIMING_STATUS_TAG_OFFSET]),
        time_stop_counter: 0,
        active_effect_tag: None,
        active_effect_counter: 0,
        transport: transport_from_save_marker(bytes[SAVE_TRANSPORT_MARKER_OFFSET]),
        pending_vehicle: None,
        initial_britannia_overlay: None,
        debug_enter: None,
        saved_active_objects: if include_active_objects {
            Some(decode_saved_active_objects(bytes)?)
        } else {
            None
        },
        save_template_source: SaveTemplateSource::PreferSavedGame,
    })
}

pub fn decode_reagent_stock(bytes: &[u8]) -> [u8; REAGENT_COUNT] {
    let mut reagents = [0; REAGENT_COUNT];
    for (save_index, recipe_index) in REAGENT_SAVE_ORDER.iter().copied().enumerate() {
        reagents[recipe_index] = bytes[SAVE_REAGENTS_OFFSET + save_index];
    }
    reagents
}

pub fn encode_reagent_stock(bytes: &mut [u8], reagents: [u8; REAGENT_COUNT]) {
    for (save_index, recipe_index) in REAGENT_SAVE_ORDER.iter().copied().enumerate() {
        bytes[SAVE_REAGENTS_OFFSET + save_index] = reagents[recipe_index];
    }
}

pub fn decode_avatar_stats(bytes: &[u8]) -> AvatarStats {
    let avatar_record = SAVE_ROSTER_OFFSET;
    AvatarStats {
        strength: bytes[avatar_record + SAVE_CHARACTER_STR_OFFSET],
        dexterity: bytes[avatar_record + SAVE_CHARACTER_DEX_OFFSET],
        intelligence: bytes[avatar_record + SAVE_CHARACTER_INT_OFFSET],
    }
}

pub fn decode_save_party(bytes: &[u8]) -> Vec<PartyMember> {
    let party_size = bytes[SAVE_PARTY_SIZE_OFFSET] as usize;
    if !(1..=6).contains(&party_size) {
        return default_party();
    }

    (0..party_size)
        .map(|slot| {
            let record = SAVE_ROSTER_OFFSET + slot * SAVE_CHARACTER_RECORD_LEN;
            let status = bytes[record + SAVE_CHARACTER_STATUS_OFFSET];
            let climb_stat = bytes[record + SAVE_CHARACTER_DEX_OFFSET];
            let mana = bytes[record + SAVE_CHARACTER_MANA_OFFSET];
            let hp = u16_at(bytes, record + SAVE_CHARACTER_HP_OFFSET);
            let max_hp = u16_at(bytes, record + SAVE_CHARACTER_MAX_HP_OFFSET);
            let level = bytes[record + SAVE_CHARACTER_LEVEL_OFFSET];
            PartyMember {
                slot: slot as u8,
                status,
                climb_stat,
                mana,
                hp,
                max_hp,
                level,
            }
        })
        .collect()
}

pub fn decode_moonstone_gate_slots(bytes: &[u8]) -> [MoonstoneGateSlot; MOONSTONE_SLOT_COUNT] {
    let mut slots = [MoonstoneGateSlot::invalid(); MOONSTONE_SLOT_COUNT];
    for (slot_index, slot) in slots.iter_mut().enumerate() {
        *slot = MoonstoneGateSlot {
            x: bytes[SAVE_MOONSTONE_X_OFFSET + slot_index],
            y: bytes[SAVE_MOONSTONE_Y_OFFSET + slot_index],
            scene: bytes[SAVE_MOONSTONE_SCENE_OFFSET + slot_index],
            z: bytes[SAVE_MOONSTONE_Z_OFFSET + slot_index],
        };
    }
    slots
}

pub fn saved_game_has_avatar_name(bytes: &[u8]) -> bool {
    bytes
        .get(SAVE_AVATAR_NAME_OFFSET..SAVE_AVATAR_NAME_OFFSET + SAVE_AVATAR_NAME_LEN)
        .is_some_and(|name| name.iter().any(|byte| *byte != 0))
}
