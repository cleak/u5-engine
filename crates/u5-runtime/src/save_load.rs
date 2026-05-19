//! Loaders that turn SAVED.GAM/SAVED.OOL/INIT.GAM into PlayOptions.

use std::io;
use std::path::Path;

use crate::*;

pub fn load_play_options_from_save(game_dir: &Path) -> io::Result<PlayOptions> {
    let mut options =
        load_play_options_from_save_file(game_dir, SAVED_GAM_FILENAME, "--from-save", true)?;
    refresh_saved_ool_mirrors_for_load(game_dir)?;
    load_world_progress_state(game_dir)?.apply_to_play_options(&mut options);
    options.blackthorn_story = load_blackthorn_story_state(game_dir)?;
    options.save_template_source = SaveTemplateSource::SavedGame;
    Ok(options)
}

pub fn load_play_options_from_init(game_dir: &Path) -> io::Result<PlayOptions> {
    let mut options =
        load_play_options_from_save_file(game_dir, INIT_GAM_FILENAME, "--from-init", false)?;
    options.initial_britannia_overlay = Some(load_init_overlay_objects(game_dir)?);
    options.save_template_source = SaveTemplateSource::InitGame;
    Ok(options)
}

pub fn load_save_image_template(
    game_dir: &Path,
    source: SaveTemplateSource,
) -> io::Result<Vec<u8>> {
    match source {
        SaveTemplateSource::SavedGame => {
            read_save_image_file(&game_dir.join(SAVED_GAM_FILENAME), SAVED_GAM_FILENAME)
        }
        SaveTemplateSource::InitGame => {
            read_save_image_file(&game_dir.join(INIT_GAM_FILENAME), INIT_GAM_FILENAME)
        }
        SaveTemplateSource::PreferSavedGame => {
            let saved = game_dir.join(SAVED_GAM_FILENAME);
            if saved.exists() {
                return read_save_image_file(&saved, SAVED_GAM_FILENAME);
            }
            let init = game_dir.join(INIT_GAM_FILENAME);
            if init.exists() {
                return read_save_image_file(&init, INIT_GAM_FILENAME);
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
    if file_name.eq_ignore_ascii_case("SAVED.GAM") && !saved_game_has_avatar_name(bytes) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "No active game. Please create a character or transfer one from Ultima IV.",
        ));
    }
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
    let mut scroll_stock = [0; SCROLL_COUNT];
    scroll_stock
        .copy_from_slice(&bytes[SAVE_SCROLL_STOCK_OFFSET..SAVE_SCROLL_STOCK_OFFSET + SCROLL_COUNT]);
    let mut potion_stock = [0; POTION_COUNT];
    potion_stock
        .copy_from_slice(&bytes[SAVE_POTION_STOCK_OFFSET..SAVE_POTION_STOCK_OFFSET + POTION_COUNT]);
    let moonstone_slots = decode_moonstone_gate_slots(bytes);
    let reagents = decode_reagent_stock(bytes);
    let mut dungeon_room_clear_bitmap = [0; SAVE_DUNGEON_ROOM_CLEAR_BITMAP_LEN];
    dungeon_room_clear_bitmap.copy_from_slice(
        &bytes[SAVE_DUNGEON_ROOM_CLEAR_BITMAP_OFFSET
            ..SAVE_DUNGEON_ROOM_CLEAR_BITMAP_OFFSET + SAVE_DUNGEON_ROOM_CLEAR_BITMAP_LEN],
    );
    let saved_dungeon_working_buffer = (scene_byte > 32).then(|| {
        bytes[SAVE_DUNGEON_WORKING_BUFFER_OFFSET
            ..SAVE_DUNGEON_WORKING_BUFFER_OFFSET + SAVE_DUNGEON_WORKING_BUFFER_LEN]
            .to_vec()
    });
    let avatar_stats = decode_avatar_stats(bytes);
    let party = decode_save_party(bytes);
    let party_size = party.len();
    let party_names = decode_party_names(bytes);
    let party_experience = decode_party_experience(bytes, party.len());
    let party_stay_counters = decode_party_stay_counters(bytes, party.len());
    let party_strengths = decode_party_strengths(bytes, party.len());
    let party_intelligence = decode_party_intelligence(bytes, party.len());
    let party_equipment = decode_party_equipment(bytes, party.len());
    let party_roster = decode_party_roster(bytes);
    let equipment_stock = decode_equipment_stock(bytes);
    let special_items = decode_special_items(bytes);
    let inn_registry = decode_inn_registry(bytes);

    let transport_marker = bytes[SAVE_TRANSPORT_MARKER_OFFSET];

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
        climbing_gear: bytes[SAVE_GRAPPLE_OFFSET],
        special_items,
        party,
        party_names,
        party_experience,
        party_stay_counters,
        party_strengths,
        party_intelligence,
        party_equipment,
        party_roster,
        equipment_stock,
        spell_charges,
        scroll_stock,
        potion_stock,
        reagents,
        rare_reagent_harvest_days: [RARE_REAGENT_HARVEST_UNSEEN_DAY;
            RARE_REAGENT_HARVEST_POINT_COUNT],
        fixed_hidden_treasure_found: [0; FIXED_HIDDEN_TREASURE_FOUND_BYTES],
        fixed_hidden_treasure_daily_day: FIXED_HIDDEN_TREASURE_DAILY_UNSEEN_DAY,
        dungeon_room_clear_bitmap,
        saved_dungeon_working_buffer,
        moonstone_slots,
        shadowlord_hideouts: DEFAULT_SHADOWLORD_HIDEOUTS,
        shrine_ordained_mask: bytes[SAVE_SHRINE_ORDAINED_MASK_OFFSET],
        shrine_codex_mask: bytes[SAVE_SHRINE_CODEX_MASK_OFFSET],
        shrine_standing: [0; VIRTUE_COUNT],
        moral_standing: bytes[SAVE_MORAL_STANDING_OFFSET],
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
        fortunes_of_war: bytes[SAVE_FORTUNES_OF_WAR_OFFSET],
        active_player: decode_active_player_slot(bytes[SAVE_ACTIVE_PLAYER_OFFSET], party_size),
        combat_round_counter: bytes[SAVE_COMBAT_ROUND_COUNTER_OFFSET],
        transport: transport_from_save_marker(transport_marker),
        facing: transport_marker_facing(transport_marker),
        pending_vehicle: None,
        inn_registry,
        blackthorn_story: BlackthornStoryState::default(),
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

pub fn decode_special_items(bytes: &[u8]) -> [u8; SPECIAL_ITEM_COUNT] {
    let mut special_items = [0; SPECIAL_ITEM_COUNT];
    special_items.copy_from_slice(
        &bytes[SAVE_SPECIAL_ITEM_OFFSET..SAVE_SPECIAL_ITEM_OFFSET + SPECIAL_ITEM_COUNT],
    );
    special_items
}

pub fn decode_equipment_stock(bytes: &[u8]) -> [u8; EQUIPMENT_COUNT] {
    let mut stock = [0; EQUIPMENT_COUNT];
    stock.copy_from_slice(
        &bytes[SAVE_EQUIPMENT_STOCK_OFFSET..SAVE_EQUIPMENT_STOCK_OFFSET + EQUIPMENT_COUNT],
    );
    stock
}

pub fn decode_party_strengths(bytes: &[u8], party_size: usize) -> Vec<u8> {
    (0..party_size)
        .map(|slot| {
            let record = SAVE_ROSTER_OFFSET + slot * SAVE_CHARACTER_RECORD_LEN;
            bytes[record + SAVE_CHARACTER_STR_OFFSET]
        })
        .collect()
}

pub fn decode_party_experience(bytes: &[u8], party_size: usize) -> Vec<u16> {
    (0..party_size)
        .map(|slot| {
            let record = SAVE_ROSTER_OFFSET + slot * SAVE_CHARACTER_RECORD_LEN;
            u16_at(bytes, record + SAVE_CHARACTER_EXPERIENCE_OFFSET)
        })
        .collect()
}

pub fn decode_party_stay_counters(bytes: &[u8], party_size: usize) -> Vec<u8> {
    (0..party_size)
        .map(|slot| {
            let record = SAVE_ROSTER_OFFSET + slot * SAVE_CHARACTER_RECORD_LEN;
            bytes[record + SAVE_CHARACTER_STAY_COUNTER_OFFSET]
        })
        .collect()
}

pub fn decode_party_intelligence(bytes: &[u8], party_size: usize) -> Vec<u8> {
    (0..party_size)
        .map(|slot| {
            let record = SAVE_ROSTER_OFFSET + slot * SAVE_CHARACTER_RECORD_LEN;
            bytes[record + SAVE_CHARACTER_INT_OFFSET]
        })
        .collect()
}

pub fn decode_party_equipment(bytes: &[u8], party_size: usize) -> Vec<[u8; EQUIPMENT_SLOT_COUNT]> {
    (0..party_size)
        .map(|slot| {
            let record = SAVE_ROSTER_OFFSET + slot * SAVE_CHARACTER_RECORD_LEN;
            let mut equipment = [EQUIPMENT_EMPTY; EQUIPMENT_SLOT_COUNT];
            equipment.copy_from_slice(
                &bytes[record + SAVE_CHARACTER_EQUIPMENT_OFFSET
                    ..record + SAVE_CHARACTER_EQUIPMENT_OFFSET + EQUIPMENT_SLOT_COUNT],
            );
            equipment
        })
        .collect()
}

pub fn decode_party_roster(bytes: &[u8]) -> Vec<PartyRosterRecord> {
    (0..SAVE_ROSTER_SLOT_COUNT)
        .map(|slot| {
            let record = SAVE_ROSTER_OFFSET + slot * SAVE_CHARACTER_RECORD_LEN;
            let class_byte = match bytes[record + SAVE_CHARACTER_CLASS_OFFSET] {
                0 => b'A',
                value => value,
            };
            let mut name = [0; SAVE_CHARACTER_NAME_LEN];
            name.copy_from_slice(&bytes[record..record + SAVE_CHARACTER_NAME_LEN]);
            let mut equipment = [EQUIPMENT_EMPTY; EQUIPMENT_SLOT_COUNT];
            equipment.copy_from_slice(
                &bytes[record + SAVE_CHARACTER_EQUIPMENT_OFFSET
                    ..record + SAVE_CHARACTER_EQUIPMENT_OFFSET + EQUIPMENT_SLOT_COUNT],
            );
            PartyRosterRecord {
                member: PartyMember {
                    slot: slot as u8,
                    class_byte,
                    status: bytes[record + SAVE_CHARACTER_STATUS_OFFSET],
                    climb_stat: bytes[record + SAVE_CHARACTER_DEX_OFFSET],
                    mana: bytes[record + SAVE_CHARACTER_MANA_OFFSET],
                    hp: u16_at(bytes, record + SAVE_CHARACTER_HP_OFFSET),
                    max_hp: u16_at(bytes, record + SAVE_CHARACTER_MAX_HP_OFFSET),
                    level: bytes[record + SAVE_CHARACTER_LEVEL_OFFSET],
                },
                name,
                experience: u16_at(bytes, record + SAVE_CHARACTER_EXPERIENCE_OFFSET),
                stay_counter: bytes[record + SAVE_CHARACTER_STAY_COUNTER_OFFSET],
                strength: bytes[record + SAVE_CHARACTER_STR_OFFSET],
                intelligence: bytes[record + SAVE_CHARACTER_INT_OFFSET],
                equipment,
            }
        })
        .collect()
}

pub fn decode_inn_registry(bytes: &[u8]) -> Vec<InnGuestRecord> {
    (0..SAVE_INN_REGISTRY_COUNT)
        .filter_map(|slot| {
            let record = SAVE_INN_REGISTRY_OFFSET + slot * SAVE_CHARACTER_RECORD_LEN;
            let scene_marker = bytes[record];
            if scene_marker == 0 {
                return None;
            }
            let class_byte = match bytes[record + SAVE_CHARACTER_CLASS_OFFSET] {
                0 => b'A',
                value => value,
            };
            let mut equipment = [EQUIPMENT_EMPTY; EQUIPMENT_SLOT_COUNT];
            equipment.copy_from_slice(
                &bytes[record + SAVE_CHARACTER_EQUIPMENT_OFFSET
                    ..record + SAVE_CHARACTER_EQUIPMENT_OFFSET + EQUIPMENT_SLOT_COUNT],
            );
            Some(InnGuestRecord {
                scene_marker,
                name: {
                    let mut name = [0; SAVE_CHARACTER_NAME_LEN];
                    name.copy_from_slice(&bytes[record..record + SAVE_CHARACTER_NAME_LEN]);
                    name[0] = 0;
                    name
                },
                member: PartyMember {
                    slot: slot as u8,
                    class_byte,
                    status: bytes[record + SAVE_CHARACTER_STATUS_OFFSET],
                    climb_stat: bytes[record + SAVE_CHARACTER_DEX_OFFSET],
                    mana: bytes[record + SAVE_CHARACTER_MANA_OFFSET],
                    hp: u16_at(bytes, record + SAVE_CHARACTER_HP_OFFSET),
                    max_hp: u16_at(bytes, record + SAVE_CHARACTER_MAX_HP_OFFSET),
                    level: bytes[record + SAVE_CHARACTER_LEVEL_OFFSET],
                },
                strength: bytes[record + SAVE_CHARACTER_STR_OFFSET],
                intelligence: bytes[record + SAVE_CHARACTER_INT_OFFSET],
                experience: u16_at(bytes, record + SAVE_CHARACTER_EXPERIENCE_OFFSET),
                equipment,
                stay_counter: bytes[record + SAVE_CHARACTER_STAY_COUNTER_OFFSET],
            })
        })
        .collect()
}

pub fn encode_inn_registry(bytes: &mut [u8], registry: &[InnGuestRecord]) {
    for slot in 0..SAVE_INN_REGISTRY_COUNT {
        let record = SAVE_INN_REGISTRY_OFFSET + slot * SAVE_CHARACTER_RECORD_LEN;
        bytes[record] = 0;
    }

    for (slot, guest) in registry.iter().take(SAVE_INN_REGISTRY_COUNT).enumerate() {
        let record = SAVE_INN_REGISTRY_OFFSET + slot * SAVE_CHARACTER_RECORD_LEN;
        bytes[record] = guest.scene_marker;
        bytes[record..record + SAVE_CHARACTER_NAME_LEN].copy_from_slice(&guest.name);
        bytes[record] = guest.scene_marker;
        bytes[record + SAVE_CHARACTER_CLASS_OFFSET] = guest.member.class_byte;
        bytes[record + SAVE_CHARACTER_STATUS_OFFSET] = guest.member.status;
        bytes[record + SAVE_CHARACTER_STR_OFFSET] = guest.strength;
        bytes[record + SAVE_CHARACTER_DEX_OFFSET] = guest.member.climb_stat;
        bytes[record + SAVE_CHARACTER_INT_OFFSET] = guest.intelligence;
        bytes[record + SAVE_CHARACTER_MANA_OFFSET] = guest.member.mana;
        write_u16_at(bytes, record + SAVE_CHARACTER_HP_OFFSET, guest.member.hp);
        write_u16_at(
            bytes,
            record + SAVE_CHARACTER_MAX_HP_OFFSET,
            guest.member.max_hp,
        );
        write_u16_at(
            bytes,
            record + SAVE_CHARACTER_EXPERIENCE_OFFSET,
            guest.experience,
        );
        bytes[record + SAVE_CHARACTER_LEVEL_OFFSET] = guest.member.level;
        bytes[record + SAVE_CHARACTER_STAY_COUNTER_OFFSET] =
            guest.stay_counter.min(INN_STAY_COUNTER_CAP);
        bytes[record + SAVE_CHARACTER_EQUIPMENT_OFFSET
            ..record + SAVE_CHARACTER_EQUIPMENT_OFFSET + EQUIPMENT_SLOT_COUNT]
            .copy_from_slice(&guest.equipment);
    }
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
            let class_byte = match bytes[record + SAVE_CHARACTER_CLASS_OFFSET] {
                0 => b'A',
                value => value,
            };
            let status = bytes[record + SAVE_CHARACTER_STATUS_OFFSET];
            let climb_stat = bytes[record + SAVE_CHARACTER_DEX_OFFSET];
            let mana = bytes[record + SAVE_CHARACTER_MANA_OFFSET];
            let hp = u16_at(bytes, record + SAVE_CHARACTER_HP_OFFSET);
            let max_hp = u16_at(bytes, record + SAVE_CHARACTER_MAX_HP_OFFSET);
            let level = bytes[record + SAVE_CHARACTER_LEVEL_OFFSET];
            PartyMember {
                slot: slot as u8,
                class_byte,
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

pub fn decode_party_names(bytes: &[u8]) -> Vec<[u8; SAVE_CHARACTER_NAME_LEN]> {
    let party_size = bytes[SAVE_PARTY_SIZE_OFFSET] as usize;
    if !(1..=6).contains(&party_size) {
        return Vec::new();
    }

    (0..party_size)
        .map(|slot| {
            let record = SAVE_ROSTER_OFFSET + slot * SAVE_CHARACTER_RECORD_LEN;
            let mut name = [0; SAVE_CHARACTER_NAME_LEN];
            name.copy_from_slice(&bytes[record..record + SAVE_CHARACTER_NAME_LEN]);
            name
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
        .get(SAVE_AVATAR_NAME_OFFSET)
        .is_some_and(|byte| *byte != 0)
}

/// `save-load.md §4.2` step 6: scene-byte sentinel for the overworld
/// stream. The save image stores it at [`SAVE_SCENE_OFFSET`].
pub const SAVE_SCENE_OVERWORLD: u8 = 0;

/// `save-load.md §4.2` step 6: the load flow enters the underworld
/// disk-swap loop only when the save indicates the player was standing
/// on the underworld surface at save time — scene byte equals the
/// overworld scene and party Z is non-zero. Returns `true` when the
/// disk-swap loop should fire.
pub const fn save_load_needs_underworld_disk_swap(scene_byte: u8, party_z: u8) -> bool {
    scene_byte == SAVE_SCENE_OVERWORLD && party_z != 0
}

/// `save-load.md §5.2` step 5: the save handler writes the underworld
/// staging half to `UNDER.OOL` once unconditionally, then a second time
/// as a defensive re-flush when the entry disk-prompt mode was *not*
/// already [`DISK_PROMPT_MODE_CANONICAL`]. Returns `true` when the
/// second write should run.
pub const fn save_flow_double_writes_underworld(entry_disk_prompt_mode: u8) -> bool {
    entry_disk_prompt_mode != DISK_PROMPT_MODE_CANONICAL
}

/// `screen-mode-dispatch.md §5`: canonical disk-prompt mode the
/// normalizer folds the historical values `2` and `5` to.
pub const DISK_PROMPT_MODE_CANONICAL: u8 = 1;
/// `screen-mode-dispatch.md §5`: first historical disk-prompt mode
/// value that the normalizer collapses to
/// [`DISK_PROMPT_MODE_CANONICAL`]. Preserve the alias mapping rather
/// than the input values themselves; gameplay-side callers should
/// never depend on the raw input value.
pub const DISK_PROMPT_MODE_ALIAS_A: u8 = 2;
pub const DISK_PROMPT_MODE_ALIAS_B: u8 = 5;

/// `screen-mode-dispatch.md §5`: the disk-prompt request normalizes the
/// historical mode values `2` and `5` to mode `1`; other values pass
/// through unchanged.
pub const fn normalize_disk_prompt_mode(requested_mode: u8) -> u8 {
    if requested_mode == DISK_PROMPT_MODE_ALIAS_A || requested_mode == DISK_PROMPT_MODE_ALIAS_B {
        DISK_PROMPT_MODE_CANONICAL
    } else {
        requested_mode
    }
}

/// `save-load.md §3.1`: file lengths in bytes for the `.OOL`
/// family. SAVED.OOL packs both per-plane mirrors (surface and
/// underworld); BRIT.OOL / UNDER.OOL / INIT.OOL each carry one
/// 256-byte plane. Anchored to the format-side constants so the
/// save-load file-length contract and the .OOL plane layout
/// stay one value.
pub const SAVED_OOL_FILE_LEN: usize = crate::SAVED_OOL_LEN;
pub const PER_PLANE_OOL_FILE_LEN: usize = crate::OOL_PLANE_LEN;
pub const INIT_OOL_FILE_LEN: usize = crate::OOL_PLANE_LEN;
/// `formats/ool.md §2`: published `.OOL` file names for the four
/// roles the `.OOL` family fills.
pub const SAVED_OOL_FILENAME: &str = "SAVED.OOL";
pub const BRIT_OOL_FILENAME: &str = "BRIT.OOL";
pub const UNDER_OOL_FILENAME: &str = "UNDER.OOL";
pub const INIT_OOL_FILENAME: &str = "INIT.OOL";

/// `save-load.md §5.2` published Save-flow narration strings the
/// Quit-and-Save handler prints in sequence: prompt the player,
/// echo the chosen branch label, announce the disk activity, and
/// confirm completion.
pub const SAVE_PROMPT_MESSAGE: &str = "Save game?";
pub const SAVE_PROMPT_YES_REPLY: &str = "Yes";
pub const SAVE_PROMPT_NO_REPLY: &str = "No";
pub const SAVE_IN_PROGRESS_MESSAGE: &str = "Saving...";
pub const SAVE_DONE_MESSAGE: &str = "Done.";

/// `save-load.md §4.2` three-line "No active game" notice the load
/// flow prints when the empty-save guard fires. Lines are emitted
/// in order; the intro then waits for a keystroke before returning
/// to the title menu.
pub const LOAD_EMPTY_SAVE_LINE_1: &str = "No active game";
pub const LOAD_EMPTY_SAVE_LINE_2: &str = "Please create a character";
pub const LOAD_EMPTY_SAVE_LINE_3: &str = "or transfer one from Ultima IV";

/// `save-load.md §5.2` "Save game?" prompt confirmation. The
/// handler accepts only `Y` or `N`; any other key loops the prompt.
/// Returns `Some(true)` for `Y` (commit save), `Some(false)` for
/// `N` (return to gameplay), and `None` for any other byte
/// (continue polling). Comparison is case-insensitive.
pub const fn save_prompt_decision(byte: u8) -> Option<bool> {
    match byte {
        b'Y' | b'y' => Some(true),
        b'N' | b'n' => Some(false),
        _ => None,
    }
}

/// `save-load.md §4.2` empty-save guard. Loading checks the byte at
/// file offset `0x0002` — the first byte of the Avatar name field.
/// A zero byte means the save is uninitialised; the intro prints
/// the three-line "No active game" notice and returns to the title
/// menu without entering gameplay.
pub fn save_image_has_active_avatar(image: &[u8]) -> bool {
    image
        .get(SAVE_AVATAR_NAME_OFFSET)
        .copied()
        .map_or(false, |byte| byte != 0)
}

/// `chargen.md §3` shipped factory-seed `INIT.GAM` size in bytes.
/// The seed image clones into the in-memory save buffer at chargen
/// entry; chargen then overwrites only the Avatar's customisation
/// slice (name / gender / STR / DEX / INT / MP) before the image is
/// written out as `SAVED.GAM` (which has the same length).
/// `formats/saved-gam.md §1`: INIT.GAM is the factory seed for
/// SAVED.GAM and ships at the same 4,192-byte total file length.
/// Anchored to [`crate::SAVED_GAM_LEN`] so the seed and save
/// images stay one value.
pub const INIT_GAM_FILE_LEN: usize = crate::SAVED_GAM_LEN;
pub const INIT_GAM_FILENAME: &str = "INIT.GAM";

/// `formats/saved-gam.md §1` published runtime working-save filename
/// (the 4,192-byte sibling of `INIT.GAM` that Journey Onward reads
/// and Q-Save writes).
pub const SAVED_GAM_FILENAME: &str = "SAVED.GAM";

/// `formats/ool.md §3`: each plane table holds 32 active-object
/// records (8 bytes each = 256 bytes per plane). Anchored to the
/// format-side [`crate::OOL_SLOTS`] / [`crate::OOL_RECORD_LEN`] /
/// [`crate::OOL_PLANE_LEN`] so the save-load contract and the
/// .OOL format share one source of truth.
pub const OOL_PLANE_RECORD_COUNT: usize = crate::OOL_SLOTS;
pub const OOL_PLANE_RECORD_LEN: usize = crate::OOL_RECORD_LEN;
pub const OOL_PLANE_TABLE_LEN: usize = crate::OOL_PLANE_LEN;

/// `save-load.md §3.1`: the "above-ground / no z" sentinel used in the
/// eight-byte `.OOL` record's `z` byte.
pub const OOL_NO_Z_SENTINEL: u8 = 0xFF;
