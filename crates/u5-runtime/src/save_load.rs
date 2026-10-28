//! Loaders that turn SAVED.GAM/SAVED.OOL/INIT.GAM into PlayOptions.

use std::collections::HashMap;
use std::io;
use std::path::Path;

use crate::*;

pub fn load_play_options_from_save(game_dir: &Path) -> io::Result<PlayOptions> {
    let bytes = read_save_image_file(&game_dir.join(SAVED_GAM_FILENAME), SAVED_GAM_FILENAME)?;
    let needs_underworld_disk_swap =
        save_load_needs_underworld_disk_swap(bytes[SAVE_SCENE_OFFSET], bytes[SAVE_Z_OFFSET]);
    let mut options =
        play_options_from_save_bytes_named(&bytes, SAVED_GAM_FILENAME, "--from-save", true)?;
    refresh_saved_ool_mirrors_for_load(game_dir, needs_underworld_disk_swap)?;
    options.town_npc_mutations = load_town_npc_mutations(game_dir)?;
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
    let bytes = read_disk_file(path)?;
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
    let bytes = read_save_image_file(&game_dir.join(file_name), file_name)?;
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
    validate_party_names(&party_names)?;
    let party_experience = decode_party_experience(bytes, party.len());
    let party_stay_counters = decode_party_stay_counters(bytes, party.len());
    let party_strengths = decode_party_strengths(bytes, party.len());
    let party_combat_defense = decode_party_combat_defense(bytes, party.len());
    let party_intelligence = decode_party_intelligence(bytes, party.len());
    let party_equipment = decode_party_equipment(bytes, party.len());
    let party_roster = decode_party_roster(bytes);
    let equipment_stock = decode_equipment_stock(bytes);
    let special_items = decode_special_items(bytes);
    let inn_registry = decode_inn_registry(bytes);
    let mut fixed_hidden_treasure_found = [0; FIXED_HIDDEN_TREASURE_FOUND_BYTES];
    fixed_hidden_treasure_found.copy_from_slice(
        &bytes[SAVE_FIXED_HIDDEN_TREASURE_FOUND_OFFSET
            ..SAVE_FIXED_HIDDEN_TREASURE_FOUND_OFFSET + FIXED_HIDDEN_TREASURE_FOUND_BYTES],
    );
    let mut shadowlord_hideouts = [0; SHADOWLORD_COUNT];
    shadowlord_hideouts.copy_from_slice(
        &bytes[SAVE_SHADOWLORD_HIDEOUTS_OFFSET..SAVE_SHADOWLORD_HIDEOUTS_OFFSET + SHADOWLORD_COUNT],
    );
    let removed_town_npc_flags = decode_npc_mask_bank(bytes, SAVE_NPC_REMOVED_MASKS_OFFSET);
    let talk_branch_flags = decode_npc_mask_bank(bytes, SAVE_NPC_NAME_KNOWN_MASKS_OFFSET);
    let mut combat_interference_sources = [0; COMBAT_ACTOR_SLOTS];
    combat_interference_sources.copy_from_slice(
        &bytes[SAVE_COMBAT_INTERFERENCE_SOURCE_MAP_OFFSET
            ..SAVE_COMBAT_INTERFERENCE_SOURCE_MAP_OFFSET + SAVE_COMBAT_INTERFERENCE_SOURCE_MAP_LEN],
    );

    let transport_marker = bytes[SAVE_TRANSPORT_MARKER_OFFSET];
    let mut transport = transport_from_save_marker(transport_marker);
    if let TransportState::Ship {
        ref mut hull,
        ref mut skiffs,
        ..
    } = transport
    {
        *hull = bytes[SAVE_ACTIVE_OBJECTS_OFFSET + 5];
        *skiffs = bytes[SAVE_ACTIVE_OBJECTS_OFFSET + 7];
    }
    let door_tracker = bytes
        [SAVE_DOOR_TRACKER_PREVIOUS_TILE_OFFSET..=SAVE_DOOR_TRACKER_COUNTDOWN_OFFSET]
        .iter()
        .any(|byte| *byte != 0)
        .then(|| DoorTracker {
            previous_tile: bytes[SAVE_DOOR_TRACKER_PREVIOUS_TILE_OFFSET],
            x: usize::from(bytes[SAVE_DOOR_TRACKER_X_OFFSET]),
            y: usize::from(bytes[SAVE_DOOR_TRACKER_Y_OFFSET]),
            turns_remaining: bytes[SAVE_DOOR_TRACKER_COUNTDOWN_OFFSET],
        });

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
        party_combat_defense,
        party_intelligence,
        party_equipment,
        party_roster,
        equipment_stock,
        spell_charges,
        scroll_stock,
        potion_stock,
        reagents,
        rare_reagent_harvest_days: bytes[SAVE_RARE_REAGENT_HARVEST_COOKIES_OFFSET
            ..SAVE_RARE_REAGENT_HARVEST_COOKIES_OFFSET + RARE_REAGENT_HARVEST_POINT_COUNT]
            .try_into()
            .expect("fixed rare-reagent harvest cookie slice"),
        fixed_hidden_treasure_found,
        fixed_hidden_treasure_daily_day: bytes[SAVE_FIXED_HIDDEN_TREASURE_DAILY_COOKIE_OFFSET],
        dungeon_room_clear_bitmap,
        saved_dungeon_working_buffer,
        moonstone_slots,
        shadowlord_hideouts,
        // `formats/saved-gam.md §9.1`: `0x0325` values `0..2` name the last
        // summoned Shadowlord (R585); anything else (new-game `0xFF`) means
        // no summon has succeeded yet.
        summoned_shadowlord: Some(usize::from(bytes[SAVE_ACTIVE_SHADOWLORD_OFFSET]))
            .filter(|index| *index < SHADOWLORD_COUNT),
        removed_town_npc_flags,
        talk_branch_flags,
        shrine_ordained_mask: bytes[SAVE_SHRINE_ORDAINED_MASK_OFFSET],
        shrine_codex_mask: bytes[SAVE_SHRINE_CODEX_MASK_OFFSET],
        word_of_power_seal_flags: bytes[SAVE_WORD_OF_POWER_SEAL_FLAGS_OFFSET
            ..SAVE_WORD_OF_POWER_SEAL_FLAGS_OFFSET + SAVE_WORD_OF_POWER_SEAL_FLAG_COUNT]
            .try_into()
            .expect("fixed Word-of-Power seal flag slice"),
        shrine_ruin_flags: bytes[SAVE_SHRINE_RUIN_FLAGS_OFFSET
            ..SAVE_SHRINE_RUIN_FLAGS_OFFSET + SAVE_SHRINE_RUIN_FLAG_COUNT]
            .try_into()
            .expect("fixed shrine-ruin flag slice"),
        moral_standing: bytes[SAVE_MORAL_STANDING_OFFSET],
        toll_progress: bytes[SAVE_TOLL_PROGRESS_OFFSET],
        cleanup_previous_hour: bytes[SAVE_SAVED_HOUR_SNAPSHOT_OFFSET],
        // `time.md §11` (spec `0170809`): the byte at `0x02DE` is written
        // on a snapshot mismatch and then "counted down toward zero by the
        // ambient-audio tick"; it is save-backed state, not a value to
        // rederive from the clock.
        twelve_hour_audio_repeats: bytes[SAVE_TWELVE_HOUR_AUDIO_REPEAT_OFFSET],
        // `formats/saved-gam.md §5.1` (spec `0170809`): "An engine that
        // treats the pair as disposable, or that recomputes the phase by a
        // different route on load, sends the party through the wrong gate."
        cached_moon_glyph_bytes: [
            bytes[SAVE_CACHED_TRAMMEL_GLYPH_OFFSET],
            bytes[SAVE_CACHED_FELUCCA_GLYPH_OFFSET],
        ],
        // `formats/saved-gam.md §10` (spec `0170809`): a stored ambient
        // value of 51 or higher "makes the recompute skip entirely and
        // freezes ambient light for that call", so the saved byte has to
        // reach the recompute rather than being replaced by a fresh zero.
        ambient_light: bytes[SAVE_AMBIENT_LIGHT_OFFSET],
        // `overworld.md §9.1` (spec HEAD c00bf63): the gate-presence
        // counter is save-backed, so a game saved mid-rise reloads at
        // the same gate height.
        natural_moongate_counter: bytes[SAVE_NATURAL_MOONGATE_COUNTER_OFFSET],
        // `animation.md §9`: the driver-side animation layer is "transient
        // in the same sense — nothing about it is saved". A save image
        // therefore carries no phase to restore, and "Loading a saved game
        // does not restore pristine artwork" either: a load inside a running
        // program must inherit the phases the program has already reached.
        // A caller that loads a save mid-run should overwrite this with the
        // live [`PlayState::animation_asset_buffer`]; the boot value below is
        // correct for the only load path the engine currently has, which is
        // the front end starting a program run (`§6.1`).
        animation_asset_buffer: AnimationAssetBuffer::AT_BOOT,
        avatar_stats,
        torches: bytes[SAVE_TORCH_STOCK_OFFSET],
        torch_counter: bytes[SAVE_TORCH_COUNTER_OFFSET],
        light_spell_counter: bytes[SAVE_LIGHT_SPELL_COUNTER_OFFSET],
        wind: WindState::from_save_byte(bytes[SAVE_WIND_OFFSET]),
        wind_save_byte: bytes[SAVE_WIND_OFFSET],
        time_stop_counter: 0,
        active_effect_tag: (bytes[SAVE_ACTIVE_EFFECT_CODE_OFFSET] != 0)
            .then_some(bytes[SAVE_ACTIVE_EFFECT_CODE_OFFSET]),
        active_effect_counter: bytes[SAVE_ACTIVE_EFFECT_DURATION_OFFSET],
        fortunes_of_war: bytes[SAVE_FORTUNES_OF_WAR_OFFSET],
        // `rest-and-camp.md §5` / `formats/saved-gam.md §10`,
        // published in the answer to cleak/u5-spec#95.
        camp_cooldown: bytes[SAVE_CAMP_COOLDOWN_OFFSET],
        camp_month_cookie: bytes[SAVE_CAMP_MONTH_COOKIE_OFFSET],
        world_tick_gate: bytes[SAVE_WORLD_TICK_GATE_OFFSET],
        active_player: decode_active_player_slot(bytes[SAVE_ACTIVE_PLAYER_OFFSET], party_size),
        active_player_save_byte: bytes[SAVE_ACTIVE_PLAYER_OFFSET],
        combat_round_counter: bytes[SAVE_COMBAT_ROUND_COUNTER_OFFSET],
        combat_interference_sources,
        transport,
        // `dungeon-mode.md §4.1`: "Loading a saved game never runs [the
        // entry seed], so a saved facing survives untouched and the party
        // resumes facing whatever the save recorded ... a save whose facing
        // field is zero starts the party facing north" (playtest E11).
        facing: if scene_byte > 32 {
            dungeon_facing_from_save_code(bytes[SAVE_NPC_BAND_DUNGEON_FACING_OFFSET])
                .or(transport_marker_facing(transport_marker))
        } else {
            transport_marker_facing(transport_marker)
        },
        door_tracker,
        loaded_transport_marker: Some(transport_marker),
        pending_vehicle: PendingVehicleSaveState {
            x: bytes[SAVE_PENDING_VEHICLE_X_OFFSET],
            y: bytes[SAVE_PENDING_VEHICLE_Y_OFFSET],
            class_byte: bytes[SAVE_PENDING_VEHICLE_CLASS_OFFSET],
        }
        .acquisition(),
        pending_vehicle_save: PendingVehicleSaveState {
            x: bytes[SAVE_PENDING_VEHICLE_X_OFFSET],
            y: bytes[SAVE_PENDING_VEHICLE_Y_OFFSET],
            class_byte: bytes[SAVE_PENDING_VEHICLE_CLASS_OFFSET],
        },
        inn_registry,
        initial_britannia_overlay: None,
        debug_enter: None,
        saved_active_objects: if include_active_objects {
            Some(decode_saved_active_objects(bytes)?)
        } else {
            None
        },
        saved_player_aux: include_active_objects.then(|| {
            (
                bytes[SAVE_ACTIVE_OBJECTS_OFFSET + 5],
                bytes[SAVE_ACTIVE_OBJECTS_OFFSET + 7],
            )
        }),
        // `formats/saved-gam.md §12.3`: a town-family scene reaches the
        // preserving entry, whose NPC family "comes from the save image";
        // `npc-schedules.md §13`: "loading resumes from there".
        saved_npc_band: (include_active_objects && scene_byte_is_town_family(scene_byte))
            .then(|| read_npc_band(bytes)),
        town_npc_mutations: Vec::new(),
        save_template_source: SaveTemplateSource::PreferSavedGame,
    })
}

/// Decode one native 32-scene NPC mask bank. Zero entries are omitted
/// from the runtime map but reproduce as zero when encoded.
pub fn decode_npc_mask_bank(bytes: &[u8], offset: usize) -> HashMap<u8, u32> {
    let mut masks = HashMap::new();
    for scene_index in 0..SAVE_NPC_MASK_SCENE_COUNT {
        let start = offset + scene_index * SAVE_NPC_MASK_BYTES_PER_SCENE;
        let mask = u32::from_le_bytes(bytes[start..start + 4].try_into().unwrap());
        if mask != 0 {
            masks.insert((scene_index + 1) as u8, mask);
        }
    }
    masks
}

/// Encode one native 32-scene NPC mask bank in scene order.
pub fn encode_npc_mask_bank(bytes: &mut [u8], offset: usize, masks: &HashMap<u8, u32>) {
    for scene_index in 0..SAVE_NPC_MASK_SCENE_COUNT {
        let start = offset + scene_index * SAVE_NPC_MASK_BYTES_PER_SCENE;
        let scene = (scene_index + 1) as u8;
        bytes[start..start + 4]
            .copy_from_slice(&masks.get(&scene).copied().unwrap_or(0).to_le_bytes());
    }
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

/// `combat.md §12`: "For party-member defenders, the damage roll reads
/// the cached combat-defense byte in the character record at offset
/// `+0x18`; factory-seed records carry value `7`. This is not one of the
/// stat bytes earlier in the record". The byte is read per record, not
/// assumed constant across the roster: `7` is the value a factory-seed
/// record carries, not a rule about every record.
pub fn decode_party_combat_defense(bytes: &[u8], party_size: usize) -> Vec<u8> {
    (0..party_size)
        .map(|slot| {
            let record = SAVE_ROSTER_OFFSET + slot * SAVE_CHARACTER_RECORD_LEN;
            bytes[record + SAVE_CHARACTER_DEFENSE_BYTE_OFFSET]
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
                // `formats/saved-gam.md §3.1`: field offset `0x09`, width
                // one byte — "Gender. `0x0B` for male, `0x0C` for female."
                // The same section pins the order as gender-then-class-
                // then-status, so this byte sits between the nine-byte name
                // and the ASCII class letter already read above.
                gender: bytes[record + SAVE_CHARACTER_GENDER_OFFSET],
                experience: u16_at(bytes, record + SAVE_CHARACTER_EXPERIENCE_OFFSET),
                stay_counter: bytes[record + SAVE_CHARACTER_STAY_COUNTER_OFFSET],
                strength: bytes[record + SAVE_CHARACTER_STR_OFFSET],
                intelligence: bytes[record + SAVE_CHARACTER_INT_OFFSET],
                equipment,
                defense: bytes[record + crate::SAVE_CHARACTER_DEFENSE_BYTE_OFFSET],
                inn_marker: bytes[record + SAVE_CHARACTER_INN_MARKER_OFFSET],
            }
        })
        .collect()
}

/// `formats/saved-gam.md` §3.1: the byte at record offset `0x1F` is "This
/// record's own inn-lodging marker: the scene number of the inn where the
/// character is lodged, or zero when it is not lodged" (u5-spec#299 Q14; the
/// earlier "marker byte when this record is viewed through the shifted inn
/// guest table" is `RETRACTIONS.md` R569).
pub const INN_REGISTRY_EMPTY_MARKER: u8 = 0;

/// `formats/saved-gam.md` §3.1/§3.3: the inn-lodging marker is each roster
/// record's own last byte, `+0x1F`.
pub const SAVE_CHARACTER_INN_MARKER_OFFSET: usize = SAVE_CHARACTER_RECORD_LEN - 1;

/// **Conservative, not published.** `systems/shops.md` §8.4 publishes the
/// occupied test only as a comparison against the *current* inn scene: "the
/// engine compares all sixteen markers with the current scene", and "guest
/// enumeration treats any nonmatching marker as 'not a guest at this inn'".
/// It publishes no standalone "is this record lodged at all" predicate beyond
/// zero meaning not lodged.
///
/// `0xFF` is not a matchable scene: [`crate::Scene::new`] rejects it, so no
/// inn can ever select a slot carrying it. Both the factory seed and every
/// shipped save carry `0xFF` in that byte for ordinary roster records that
/// are not lodged anywhere. Decoding those as guests invented thirteen
/// phantom lodgers on the shipped image. Treating `0xFF` as "not a guest" is
/// therefore the conservative reading: it can only drop records no published
/// consumer can reach. (`systems/blackthorn.md` §5's executed-companion
/// tombstone is `0x7F`, not this value, and is read back by the shrine/Codex
/// presentation - u5-spec#298 Q5, `RETRACTIONS.md` R546 - so it decodes as an
/// ordinary registry record that no inn's scene matches.)
pub const INN_REGISTRY_UNMATCHABLE_MARKER: u8 = 0xff;

/// Whether a record's inn-lodging marker (`+0x1F`) denotes a lodged guest.
///
/// Published part: zero is empty/cleared (`formats/saved-gam.md` §3.1).
/// Conservative part: [`INN_REGISTRY_UNMATCHABLE_MARKER`] is also treated as
/// not-a-guest, for the reasons documented on that constant.
pub const fn inn_registry_marker_is_guest(marker: u8) -> bool {
    marker != INN_REGISTRY_EMPTY_MARKER && marker != INN_REGISTRY_UNMATCHABLE_MARKER
}

/// Roster record that holds the guest of registry slot `slot`.
///
/// `formats/saved-gam.md` §3.3 (u5-spec#299 Q14): "There is no separate guest
/// table. A companion lodged at an inn is an ordinary roster record outside
/// the travelling party. What marks it as lodged is the record's own last
/// byte, `+0x1F`". So the guest of slot `N` *is* roster record `N`, intact.
/// Reading the guest's fields relative to the marker byte instead shifted
/// every lodged record one byte into its neighbour (playtest E43).
pub const fn inn_registry_guest_record_offset(slot: usize) -> usize {
    SAVE_ROSTER_OFFSET + slot * SAVE_CHARACTER_RECORD_LEN
}

/// Byte holding roster record `slot`'s own inn-lodging marker, its last byte
/// (`+0x1F`, `formats/saved-gam.md` §3.3). The withdrawn "leading marker of a
/// shifted registry slot" (`RETRACTIONS.md` R569) named the same byte.
pub const fn inn_registry_marker_offset(slot: usize) -> usize {
    SAVE_INN_REGISTRY_OFFSET + slot * SAVE_CHARACTER_RECORD_LEN
}

/// The travelling-party prefix of the roster. `formats/saved-gam.md` §3.1:
/// the marker "has no meaning for a travelling member", so a travelling
/// record is never read as a lodged guest.
fn inn_registry_active_prefix(bytes: &[u8]) -> usize {
    match bytes[SAVE_PARTY_SIZE_OFFSET] {
        size @ 1..=SAVE_PARTY_SIZE_MAX => usize::from(size),
        _ => 1,
    }
}

pub fn decode_inn_registry(bytes: &[u8]) -> Vec<InnGuestRecord> {
    let active = inn_registry_active_prefix(bytes);
    (active..SAVE_INN_REGISTRY_COUNT)
        .filter_map(|slot| {
            let record = inn_registry_guest_record_offset(slot);
            let scene_marker = bytes[inn_registry_marker_offset(slot)];
            if !inn_registry_marker_is_guest(scene_marker) {
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
                registry_slot: slot as u8,
                scene_marker,
                name: {
                    let mut name = [0; SAVE_CHARACTER_NAME_LEN];
                    name.copy_from_slice(&bytes[record..record + SAVE_CHARACTER_NAME_LEN]);
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

/// Write the inn-guest registry back into the save image.
///
/// `formats/saved-gam.md` §3.3 (u5-spec#299 Q14, `RETRACTIONS.md` R569/R570):
/// the registry is the roster itself - each lodged guest is roster record `N`
/// with its own marker in that record's byte `0x1F` (see
/// [`inn_registry_guest_record_offset`]), so a write at the wrong slot lands
/// on another character record. No inventory byte belongs to it.
///
/// Two rules follow, and both are load-bearing:
///
/// 1. **Never repack.** Each guest is written back at
///    [`InnGuestRecord::registry_slot`], the slot it was decoded from or
///    allocated into, never at its position in the guest list. Repacking from
///    slot zero shifted whole roster records and destroyed companions.
/// 2. **Never blanket-clear.** Records that hold no guest keep the marker the
///    roster writer stored for them (the record's own byte, which travels
///    with it). The one exception is a record whose marker still names a real
///    inn scene while no guest occupies it - a guest the inn flow returned,
///    and §8.4 pickup "clears the returned record's marker to zero".
pub fn encode_inn_registry(bytes: &mut [u8], registry: &[InnGuestRecord]) {
    let mut occupied = [false; SAVE_INN_REGISTRY_COUNT];
    let active = inn_registry_active_prefix(bytes);

    for guest in registry {
        let slot = guest.registry_slot as usize;
        // A travelling record is never a guest; writing one there would
        // overwrite a party member the roster writer just stored.
        if slot >= SAVE_INN_REGISTRY_COUNT || slot < active {
            continue;
        }
        occupied[slot] = true;
        let record = inn_registry_guest_record_offset(slot);
        bytes[record..record + SAVE_CHARACTER_NAME_LEN].copy_from_slice(&guest.name);
        bytes[inn_registry_marker_offset(slot)] = guest.scene_marker;
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
        // `formats/saved-gam.md` §3.1: the month counter is aged only by
        // *the time system*, which adds one below 25 and leaves 25 or more
        // unchanged (R587). Clamping on write instead
        // silently rewrites an inherited byte the engine only ever read, so
        // the raw value is preserved here and the cap stays in the ageing pass
        // that owns it.
        bytes[record + SAVE_CHARACTER_STAY_COUNTER_OFFSET] = guest.stay_counter;
        bytes[record + SAVE_CHARACTER_EQUIPMENT_OFFSET
            ..record + SAVE_CHARACTER_EQUIPMENT_OFFSET + EQUIPMENT_SLOT_COUNT]
            .copy_from_slice(&guest.equipment);
    }

    for (slot, slot_occupied) in occupied.iter().copied().enumerate() {
        // `formats/saved-gam.md` §3.1: a travelling record's marker "has no
        // meaning for a travelling member", and §3.3 / Section 2 say to
        // "Preserve the marker byte-for-byte". The returned-guest clear
        // belongs to pickup, which already zeroes the record it returns, so
        // the travelling prefix keeps whatever its records carry (playtest
        // N25).
        if slot_occupied || slot < active {
            continue;
        }
        let marker = inn_registry_marker_offset(slot);
        if inn_registry_marker_is_guest(bytes[marker]) {
            bytes[marker] = INN_REGISTRY_EMPTY_MARKER;
        }
    }
}

/// Lowest registry slot not already claimed by a guest. Inn Leave does not
/// use it: `systems/shops.md` §8.4 (u5-spec#299 Q14) moves the leaving record
/// to slot 15. It remains the fallback of the Blackthorn park.
pub fn free_inn_registry_slot(registry: &[InnGuestRecord]) -> Option<u8> {
    (0..SAVE_INN_REGISTRY_COUNT as u8)
        .find(|slot| !registry.iter().any(|guest| guest.registry_slot == *slot))
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
    // `save-load.md §4.2`: the load path "validates nothing", and
    // `blackthorn.md §7` gives an empty roster its outcome - it "also falls
    // into the third case", the rescue. A size of zero therefore loads as
    // no members at all; inventing a default Avatar played on with a party
    // the save never had and wrote size 1 back on the next Q (playtest
    // P12-02). Sizes past six still take the fallback.
    if party_size == 0 {
        return Vec::new();
    }
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

/// `save-load.md §5.2` step 5: after reading both per-plane mirrors,
/// the save handler writes the staged underworld bytes back once unless
/// its entry required-disk state was already Britannia index 1. There
/// is never a corresponding save-time `BRIT.OOL` write.
pub const fn save_flow_writes_underworld_mirror(entry_required_disk: RequiredDisk) -> bool {
    !matches!(entry_required_disk, RequiredDisk::Britannia)
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
/// The same prompt as the input cursor sees it.
///
/// Runtime observation, spec silent: `save-load.md §5.2` step 1 gives
/// the prompt text but not its trailing space. A capture of the
/// original shows `Save game? ` with the barber-pole cursor in the cell
/// after that space, and the `Yes` reply landing in the same cell.
pub const SAVE_PROMPT_LINE: &str = "Save game? ";
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

/// `stats-panel.md §3`: a party row's name is "printed from the record and
/// padded to a fixed name column"; only rows *outside* the travelling-party
/// size are cleared with spaces. An unreadable name record inside the
/// travelling party is therefore a save/loader fault, not something the
/// panel may paper over with a synthesised `Party 2`-style placeholder.
/// [`decode_party_names`] only yields records inside the saved party size,
/// so every record it returns must be readable.
pub fn validate_party_names(party_names: &[[u8; SAVE_CHARACTER_NAME_LEN]]) -> io::Result<()> {
    for (slot, name) in party_names.iter().enumerate() {
        if party_name_to_string(name).is_none() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "save roster slot {slot} is inside the travelling party of {} but carries an empty character name record",
                    party_names.len()
                ),
            ));
        }
    }
    Ok(())
}
