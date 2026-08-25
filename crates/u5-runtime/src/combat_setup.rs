//! Combat setup helpers that bridge encounters, arena records, and class data.

use std::io;

use crate::*;

/// `encounters.md §4` outdoor arena bank size: sixteen 11x11 arenas
/// stored in the on-disk `outdoor combat arena bank`. Anchored to
/// [`BRIT_CBT_RECORDS`] so the encounters-side arena count and
/// the format-side `.CBT` record count stay one value.
pub const OUTDOOR_ARENA_COUNT: usize = BRIT_CBT_RECORDS;

pub const OUTDOOR_COMBAT_TYPE_FIRST: u8 = 0x40;

/// `combat.md §4`: renderer-facing actor byte for a seated party member.
/// The four human combat classes are Mage, Bard, Fighter, and Avatar;
/// unsupported class letters leave the presentation byte at zero.
pub const fn combat_party_actor_byte(class_byte: u8) -> u8 {
    match class_byte.to_ascii_uppercase() {
        b'M' | b'D' => 0x40,
        b'B' | b'S' | b'T' => 0x44,
        b'F' | b'P' | b'R' => 0x48,
        b'A' => 0x4C,
        _ => 0,
    }
}
pub const OUTDOOR_PIRATE_TYPE_FIRST: u8 = 0x2c;
pub const OUTDOOR_PIRATE_TYPE_LAST: u8 = 0x2f;
pub const OUTDOOR_PIRATE_COMBAT_CLASS: u8 = 1;

/// `combat.md §6.1`: placement marks ordinary self-acting monsters with
/// `0x40`. The two passive/neutral class rows are non-acting `0x20` records.
pub const fn combat_monster_placement_flags(class: u8) -> u8 {
    if class == 8 || class == 9 {
        COMBAT_ACTOR_FLAG_MARKED_DEAD
    } else {
        COMBAT_ACTOR_FLAG_SELECTABLE_40
    }
}

pub const fn outdoor_type_is_pirate(type_byte: u8) -> bool {
    type_byte & 0xfc == OUTDOOR_PIRATE_TYPE_FIRST
}

/// `encounters.md §4`: resolve the combat class independently of the arena.
/// The low two animation-frame bits of ordinary active-object types are
/// discarded; the pirate ship family has the fixed class-one override.
pub const fn outdoor_combat_class_id(type_byte: u8) -> Option<u8> {
    if type_byte >= OUTDOOR_COMBAT_TYPE_FIRST {
        return Some((type_byte - OUTDOOR_COMBAT_TYPE_FIRST) / 4);
    }
    if outdoor_type_is_pirate(type_byte) {
        return Some(OUTDOOR_PIRATE_COMBAT_CLASS);
    }
    None
}

pub fn outdoor_combat_banner_name(type_byte: u8) -> Option<&'static str> {
    if outdoor_type_is_pirate(type_byte) {
        return Some("Pirates");
    }
    outdoor_combat_class_id(type_byte)
        .and_then(combat_class_stats)
        .map(|stats| stats.name)
}

/// `encounters.md §4` water predicate before aquatic-class forcing.
pub const fn outdoor_combat_terrain_is_water(terrain: u8) -> bool {
    terrain < 4 || ((terrain >= 0x60 && terrain <= 0x6f) && terrain != 0x6a && terrain != 0x6b)
}

/// `encounters.md §4`: select one of the sixteen outdoor arenas from the
/// combat class, hostile object's underlying terrain, party transport, and
/// scene-byte fallback. Combat class and arena selection deliberately remain
/// separate operations.
pub const fn outdoor_combat_arena_index(
    type_byte: u8,
    hostile_terrain: u8,
    aboard_ship: bool,
    scene_byte: u8,
) -> Option<usize> {
    let class = match outdoor_combat_class_id(type_byte) {
        Some(class) => class,
        None => return None,
    };
    if class == 47 {
        return Some(10);
    }

    let ship_target = outdoor_type_is_pirate(type_byte);
    let water = outdoor_combat_terrain_is_water(hostile_terrain) || (class >= 16 && class <= 19);
    if aboard_ship && ship_target {
        return Some(14);
    }
    if aboard_ship && water {
        return Some(11);
    }
    if aboard_ship {
        return Some(13);
    }
    if ship_target {
        return Some(12);
    }
    if water {
        return Some(15);
    }

    Some(match hostile_terrain {
        4 => 1,
        5 => 2,
        6 | 8 => 3,
        7 | 30 | 31 => 4,
        9 | 10 => 5,
        11..=15 => 6,
        29 | 72 | 73 | 106 | 107 => 7,
        68 => 8,
        _ if scene_byte == 0 => 2,
        _ => 8,
    })
}

/// `active-objects.md §8`: the generic adjacent-hostile arm reaches the
/// shared impact payload only for the exact terrain/transport combination.
pub const fn generic_adjacent_hostile_uses_impact(
    party_terrain: u8,
    party_transport_marker: u8,
) -> bool {
    party_terrain <= 0x03
        && ((party_transport_marker >= 0x14 && party_transport_marker <= 0x15)
            || (party_transport_marker >= 0x28 && party_transport_marker <= 0x2b))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerrainCombatSetup {
    pub arena_index: usize,
    pub underworld_variant: bool,
    pub base_tile: u8,
    pub terrain: [[u8; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE],
    pub setup_table_a: [u8; 6],
    pub setup_table_b: [u8; 6],
    pub placement_slots: Vec<CombatPlacementSlot>,
    pub base_class: Option<CombatClassStats>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DungeonRoomCombatSetup {
    pub arena_index: usize,
    pub terrain: [[u8; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE],
    pub placement_slots: Vec<CombatPlacementSlot>,
    pub party_positions: [(u8, u8); COMBAT_PARTY_ACTOR_SLOTS],
    pub setup_sources: Vec<DungeonRoomSetupSource>,
    pub scan_sources: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerrainCombatInstance {
    pub active_objects: Vec<ActiveObject>,
    pub actors: [CombatActorDescriptor; COMBAT_ACTOR_SLOTS],
    pub requested_count: u8,
    pub placed_count: u8,
    pub unplaced_count: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CombatPlacementSlot {
    pub slot: usize,
    pub x: u8,
    pub y: u8,
}

pub const TERRAIN_COMBAT_PARTY_POSITIONS: [(u8, u8); COMBAT_PARTY_ACTOR_SLOTS] =
    [(5, 5), (4, 5), (6, 5), (5, 4), (5, 6), (4, 6)];

pub const fn dungeon_room_entry_seed_for_direction(direction: Direction) -> u8 {
    match direction {
        Direction::North => 0,
        Direction::East => 1,
        Direction::South => 2,
        Direction::West => 3,
        Direction::NorthWest => 4,
        Direction::NorthEast => 5,
        Direction::SouthWest => 6,
        Direction::SouthEast => 7,
    }
}

/// Public issue #3 resident terrain-combat replacement-tile table.
/// Spawn counts now come from the combat-class stat row's
/// `default_spawn_count` field, not from a per-arena raw row.
pub const TERRAIN_COMBAT_REPLACEMENT_TILES_RAW: [u8; OUTDOOR_ARENA_COUNT] = [
    0x21, 0x01, 0x01, 0x03, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x0a, 0x04, 0x0c, 0x0d, 0x0e, 0x0f,
];

pub const fn terrain_combat_raw_replacement_tile_for_arena(arena_index: usize) -> Option<u8> {
    if arena_index < OUTDOOR_ARENA_COUNT {
        Some(TERRAIN_COMBAT_REPLACEMENT_TILES_RAW[arena_index])
    } else {
        None
    }
}

pub fn terrain_combat_setup_from_record_at_arena(
    plane: WorldPlane,
    trigger: ActiveObject,
    arena_index: usize,
    record: &CombatArenaRecord,
) -> io::Result<TerrainCombatSetup> {
    if arena_index >= OUTDOOR_ARENA_COUNT {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("outdoor combat arena index {arena_index} is out of range"),
        ));
    }
    if outdoor_combat_class_id(trigger.type_byte).is_none() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "active-object type 0x{:02X} has no outdoor combat class",
                trigger.type_byte
            ),
        ));
    }
    let placement_slots = combat_placement_slots_from_record(record);

    Ok(TerrainCombatSetup {
        arena_index,
        underworld_variant: plane == WorldPlane::Underworld || trigger.z < 0,
        base_tile: trigger.tile,
        terrain: record.terrain_grid(),
        setup_table_a: record.outdoor_setup_table_a(),
        setup_table_b: record.outdoor_setup_table_b(),
        placement_slots,
        base_class: terrain_combat_base_class(trigger),
    })
}

pub fn dungeon_room_combat_setup_from_record(
    arena_index: usize,
    record: &CombatArenaRecord,
) -> DungeonRoomCombatSetup {
    dungeon_room_combat_setup_from_record_for_entry(arena_index, record, 0, true)
}

pub fn dungeon_room_combat_setup_from_record_for_entry(
    arena_index: usize,
    record: &CombatArenaRecord,
    entry_seed: u8,
    scan_sources: bool,
) -> DungeonRoomCombatSetup {
    DungeonRoomCombatSetup {
        arena_index,
        terrain: record.terrain_grid(),
        placement_slots: combat_placement_slots_from_record(record),
        party_positions: record.dungeon_room_party_positions_for_seed(entry_seed),
        setup_sources: record.dungeon_room_setup_sources_with_scan(scan_sources),
        scan_sources,
    }
}

pub fn combat_placement_slots_from_record(record: &CombatArenaRecord) -> Vec<CombatPlacementSlot> {
    let placement_x = record.outdoor_placement_x();
    let placement_y = record.outdoor_placement_y();
    placement_x
        .into_iter()
        .zip(placement_y)
        .enumerate()
        .map(|(slot, (x, y))| CombatPlacementSlot { slot, x, y })
        .collect()
}

pub fn dungeon_room_source_sprite(source: u8) -> Option<u8> {
    match DungeonRoomSetupSourceKind::from_source(source) {
        DungeonRoomSetupSourceKind::OrdinaryCombatant { .. } => Some((source & 0x7f) | 0x80),
        _ => None,
    }
}

pub fn dungeon_room_combat_instance_from_setup(
    setup: &DungeonRoomCombatSetup,
    z: i8,
) -> TerrainCombatInstance {
    let mut prng_state = 0;
    dungeon_room_combat_instance_from_setup_with_prng(setup, z, &mut prng_state)
}

pub fn dungeon_room_combat_instance_from_setup_with_prng(
    setup: &DungeonRoomCombatSetup,
    z: i8,
    prng_state: &mut u16,
) -> TerrainCombatInstance {
    let mut active_objects = vec![ActiveObject::empty(); OOL_SLOTS];
    let mut actors = [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS];
    let mut placed_count = 0u8;
    let random_special_setup_ids =
        dungeon_room_random_special_setup_ids(setup.scan_sources, prng_state);

    for source in &setup.setup_sources {
        let active_object_slot = COMBAT_PARTY_ACTOR_SLOTS + usize::from(placed_count);
        if active_object_slot >= OOL_SLOTS || active_object_slot >= COMBAT_ACTOR_SLOTS {
            continue;
        }
        match source.kind {
            DungeonRoomSetupSourceKind::OrdinaryCombatant { .. } => {
                let Some(tile) = dungeon_room_source_sprite(source.source) else {
                    continue;
                };
                let Some(stats) = combat_class_stats_for_sprite_byte(tile) else {
                    continue;
                };
                active_objects[active_object_slot] = ActiveObject {
                    type_byte: tile,
                    tile,
                    x: usize::from(source.x),
                    y: usize::from(source.y),
                    z,
                    phase: STEADY_PHASE,
                    aux1: 0,
                    aux3: 0,
                };
                actors[active_object_slot] = CombatActorDescriptor::for_monster_placement(
                    stats,
                    active_object_slot as u8,
                    source.x,
                    source.y,
                    combat_monster_placement_flags(stats.class),
                    0,
                );
                placed_count = placed_count.saturating_add(1);
            }
            DungeonRoomSetupSourceKind::AbsorbableField => {
                active_objects[active_object_slot] = ActiveObject {
                    type_byte: source.source,
                    tile: source.source,
                    x: usize::from(source.x),
                    y: usize::from(source.y),
                    z,
                    phase: STEADY_PHASE,
                    aux1: 0,
                    aux3: 0,
                };
                placed_count = placed_count.saturating_add(1);
            }
            DungeonRoomSetupSourceKind::SpecialPlacement(placement) => {
                active_objects[active_object_slot] =
                    dungeon_room_special_marker_active_object(source, z, placement, prng_state);
                placed_count = placed_count.saturating_add(1);
            }
            DungeonRoomSetupSourceKind::RandomSpecialPlacement { selector } => {
                let setup_id = random_special_setup_ids
                    .get(usize::from(selector))
                    .copied()
                    .unwrap_or(source.source);
                let placement = DungeonRoomSpecialPlacement::from_setup_id(setup_id);
                active_objects[active_object_slot] =
                    dungeon_room_special_marker_active_object(source, z, placement, prng_state);
                placed_count = placed_count.saturating_add(1);
            }
        }
    }

    TerrainCombatInstance {
        active_objects,
        actors,
        requested_count: setup.setup_sources.len() as u8,
        placed_count,
        unplaced_count: (setup.setup_sources.len() as u8).saturating_sub(placed_count),
    }
}

pub fn dungeon_room_random_special_setup_ids(
    scan_sources: bool,
    prng_state: &mut u16,
) -> [u8; DUNGEON_ROOM_RANDOM_SPECIAL_ROLL_COUNT] {
    let mut ids = [0u8; DUNGEON_ROOM_RANDOM_SPECIAL_ROLL_COUNT];
    if scan_sources {
        for id in &mut ids {
            let palette_index = u5_prng_range_u16(
                prng_state,
                0,
                (DUNGEON_ROOM_RANDOM_SPECIAL_SETUP_PALETTE.len() - 1) as u16,
            ) as usize;
            *id = DUNGEON_ROOM_RANDOM_SPECIAL_SETUP_PALETTE[palette_index];
        }
    }
    ids
}

pub fn dungeon_room_special_aux1(
    post_write: DungeonRoomSpecialPostWrite,
    z: i8,
    prng_state: &mut u16,
) -> u8 {
    let z = z.max(0) as u8;
    match post_write {
        DungeonRoomSpecialPostWrite::LevelTimesThreePlusSeven => z.saturating_mul(3) + 7,
        DungeonRoomSpecialPostWrite::LevelScaledRandom => {
            let high = u16::from(z) * 10 + 10;
            u5_prng_range_u16(prng_state, 1, high) as u8
        }
        DungeonRoomSpecialPostWrite::RandomRange { low, high } => {
            u5_prng_range_u16(prng_state, u16::from(low), u16::from(high)) as u8
        }
        DungeonRoomSpecialPostWrite::RandomRangePlus { base, low, high } => base
            .saturating_add(u5_prng_range_u16(prng_state, u16::from(low), u16::from(high)) as u8),
        DungeonRoomSpecialPostWrite::Constant(value) => value,
        DungeonRoomSpecialPostWrite::None => 0,
    }
}

fn dungeon_room_special_marker_active_object(
    source: &DungeonRoomSetupSource,
    z: i8,
    placement: DungeonRoomSpecialPlacement,
    prng_state: &mut u16,
) -> ActiveObject {
    ActiveObject {
        type_byte: placement.setup_id,
        tile: placement.setup_id,
        x: usize::from(source.x),
        y: usize::from(source.y),
        z,
        phase: STEADY_PHASE,
        aux1: dungeon_room_special_aux1(placement.post_write, z, prng_state),
        aux3: 0,
    }
}

pub fn terrain_combat_base_class(trigger: ActiveObject) -> Option<CombatClassStats> {
    outdoor_combat_class_id(trigger.type_byte).and_then(combat_class_stats)
}

pub fn resolve_terrain_combat_setup_count(
    base_count: u8,
    fortunes_of_war: u8,
    first_roll_seed: u8,
    fortunes_roll_seed: u8,
    town_style_override: bool,
) -> u8 {
    if town_style_override {
        return 1;
    }
    resolve_combat_spawn_count(
        base_count,
        first_roll_seed,
        (fortunes_of_war != 0).then_some(fortunes_roll_seed),
    )
}

/// `combat.md §5` terrain-replacement early-spawn divisor. The
/// early-spawn band — spawn indexes that may roll for the per-arena
/// replacement tile — is `[0, count / DIVISOR + BIAS)`. Promote
/// both halves so the threshold helper no longer encodes the
/// "count / 4 + 1" rule as bare literals.
pub const TERRAIN_COMBAT_REPLACEMENT_EARLY_SPAWN_DIVISOR: u8 = 4;
pub const TERRAIN_COMBAT_REPLACEMENT_EARLY_SPAWN_BIAS: u8 = 1;

pub fn terrain_combat_replacement_threshold(count: u8) -> u8 {
    count / TERRAIN_COMBAT_REPLACEMENT_EARLY_SPAWN_DIVISOR
        + TERRAIN_COMBAT_REPLACEMENT_EARLY_SPAWN_BIAS
}

/// `encounters.md §4` + `combat.md §5` terrain-replacement chance
/// denominator. Each spawn index below
/// [`terrain_combat_replacement_threshold`] rolls modulo this
/// denominator; only a zero result swaps the arena's base tile for
/// the per-arena replacement tile. Later spawn indexes never roll
/// for the replacement.
pub const TERRAIN_COMBAT_REPLACEMENT_DENOMINATOR: u8 = 9;

/// `encounters.md §4` + `combat.md §5`: returns `true` when an
/// early-spawn replacement roll selects the per-arena replacement
/// tile. The caller is responsible for the spawn-index threshold
/// gate; this helper only encodes the one-in-nine die.
pub const fn terrain_combat_replacement_roll_picks_replacement(replacement_roll_seed: u8) -> bool {
    replacement_roll_seed % TERRAIN_COMBAT_REPLACEMENT_DENOMINATOR == 0
}

/// `encounters.md §8` shipped dungeon-encounter arena bank size.
/// 112 arenas are stored in the on-disk `DUNGEON.CBT` file and are
/// indexed as `bank * 16 + (tile & 0x0F)` from the dungeon-room
/// trigger tile.
pub const DUNGEON_CBT_ARENA_COUNT: usize = 112;

/// `encounters.md §8`: returns `true` when a computed dungeon arena
/// index lies inside the shipped 112-record bank. Out-of-range
/// indices indicate either an unrecognised dungeon scene or a
/// corrupted room tile and should not be passed to the loader.
pub const fn dungeon_room_arena_index_in_range(arena_index: usize) -> bool {
    arena_index < DUNGEON_CBT_ARENA_COUNT
}

pub fn terrain_combat_tile_for_spawn_index(
    spawn_index: u8,
    count: u8,
    base_tile: u8,
    replacement_tile: Option<u8>,
    replacement_roll_seed: u8,
) -> u8 {
    if spawn_index == 0 {
        return base_tile;
    }
    if spawn_index >= terrain_combat_replacement_threshold(count) {
        return base_tile;
    }
    match replacement_tile {
        Some(tile) if terrain_combat_replacement_roll_picks_replacement(replacement_roll_seed) => {
            tile
        }
        _ => base_tile,
    }
}

pub fn terrain_combat_instance_from_setup(
    setup: &TerrainCombatSetup,
    requested_count: u8,
    replacement_tile: Option<u8>,
    replacement_roll_seeds: &[u8],
) -> io::Result<TerrainCombatInstance> {
    let base_class = setup.base_class.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "terrain combat arena {} has no base monster class for tile 0x{:02X}",
                setup.arena_index, setup.base_tile
            ),
        )
    })?;
    let max_placeable =
        (COMBAT_ACTOR_SLOTS - COMBAT_PARTY_ACTOR_SLOTS).min(setup.placement_slots.len());
    let placed_count = usize::from(requested_count).min(max_placeable);
    let mut active_objects = vec![ActiveObject::empty(); COMBAT_ACTOR_SLOTS];
    let mut actors = [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS];
    let z = if setup.underworld_variant {
        WorldPlane::Underworld.save_floor()
    } else {
        WorldPlane::Britannia.save_floor()
    };

    for spawn_index in 0..placed_count {
        let placement = setup.placement_slots[spawn_index];
        let roll_seed = replacement_roll_seeds
            .get(spawn_index)
            .copied()
            .unwrap_or_default();
        let tile = terrain_combat_tile_for_spawn_index(
            spawn_index as u8,
            requested_count,
            setup.base_tile,
            replacement_tile,
            roll_seed,
        );
        let stats = combat_class_stats_for_sprite_byte(tile).unwrap_or(base_class);
        let active_object_slot = COMBAT_PARTY_ACTOR_SLOTS + spawn_index;
        active_objects[active_object_slot] = ActiveObject {
            type_byte: tile,
            tile,
            x: usize::from(placement.x),
            y: usize::from(placement.y),
            z,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        };
        actors[active_object_slot] = CombatActorDescriptor::for_monster_placement(
            stats,
            active_object_slot as u8,
            placement.x,
            placement.y,
            combat_monster_placement_flags(stats.class),
            0,
        );
    }

    Ok(TerrainCombatInstance {
        active_objects,
        actors,
        requested_count,
        placed_count: placed_count as u8,
        unplaced_count: requested_count.saturating_sub(placed_count as u8),
    })
}

impl PlayState {
    pub fn enter_dungeon_room_combat(
        &mut self,
        game_dir: &std::path::Path,
        scene: DungeonScene,
        level: u8,
        room_slot: u8,
        arena_index: usize,
        entry_seed: u8,
        scan_sources: bool,
        enter_endgame_after_successful_absorbable_combat: bool,
    ) -> io::Result<String> {
        let bank = load_dungeon_cbt(game_dir)?;
        let record = bank.record(arena_index).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("DUNGEON.CBT has no arena record {arena_index}"),
            )
        })?;
        let setup = dungeon_room_combat_setup_from_record_for_entry(
            arena_index,
            record,
            entry_seed,
            scan_sources,
        );
        let has_absorbable_field = setup
            .setup_sources
            .iter()
            .any(|source| source.kind == DungeonRoomSetupSourceKind::AbsorbableField);
        let mut instance = dungeon_room_combat_instance_from_setup_with_prng(
            &setup,
            level as i8,
            &mut self.prng_state,
        );
        self.populate_dungeon_room_combat_party(
            &mut instance.active_objects,
            &mut instance.actors,
            level as i8,
            &setup.party_positions,
        );
        let placed_count = instance.placed_count;
        let requested_count = instance.requested_count;
        self.enter_combat_frame_with_terrain(
            instance.active_objects,
            instance.actors,
            setup.terrain,
        )?;
        if !enter_endgame_after_successful_absorbable_combat {
            if let Some(snapshot) = &mut self.combat_frame_snapshot {
                snapshot.dungeon_room_clear_on_success =
                    Some(PendingDungeonRoomClear { scene, room_slot });
            }
        }
        if enter_endgame_after_successful_absorbable_combat && has_absorbable_field {
            let endgame_messages = require_endgame_messages(game_dir)?;
            let endgame_tableau_map =
                require_miscmaps_cutscene_map(game_dir, ENDGAME_TABLEAU_CUTSCENE_MAP_RECORD)?;
            if let Some(snapshot) = &mut self.combat_frame_snapshot {
                snapshot.endgame_messages = Some(endgame_messages);
                snapshot.endgame_tableau_map = Some(endgame_tableau_map);
            }
        }
        Ok(format!(
            "entered dungeon combat from {} level {level} using DUNGEON.CBT arena {arena_index}; placed {placed_count} ordinary combatant(s) from {requested_count} room source marker(s)",
            scene.key()
        ))
    }

    pub fn enter_dungeon_active_monster_combat(
        &mut self,
        level: u8,
        object: ActiveObject,
    ) -> io::Result<String> {
        let mut active_objects = vec![ActiveObject::empty(); OOL_SLOTS];
        let mut actors = [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS];
        self.populate_terrain_combat_party(&mut active_objects, &mut actors, level as i8);

        let monster_slot = COMBAT_PARTY_ACTOR_SLOTS;
        let stats = combat_class_stats(object.aux1).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "dungeon active-object DEP1 0x{:02X} has no combat class",
                    object.aux1
                ),
            )
        })?;
        active_objects[monster_slot] = ActiveObject {
            type_byte: object.type_byte,
            tile: object.tile,
            x: 6,
            y: 5,
            z: level as i8,
            phase: STEADY_PHASE,
            aux1: object.aux1,
            aux3: object.aux3,
        };
        actors[monster_slot] = CombatActorDescriptor::for_monster_placement(
            stats,
            monster_slot as u8,
            6,
            5,
            combat_monster_placement_flags(stats.class),
            0,
        );
        self.enter_combat_frame_with_terrain(active_objects, actors, DUNGEON_AMBUSH_ARENA_TERRAIN)?;
        Ok(format!(
            "entered dungeon combat against {} from active monster tile {}",
            stats.name, object.tile
        ))
    }

    pub fn enter_sleep_ambush_combat(
        &mut self,
        monster: SleepAmbushMonster,
        z: i8,
    ) -> io::Result<String> {
        let tile = sleep_ambush_monster_sprite(monster);
        let stats = combat_class_stats_for_sprite_byte(tile).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("sleep-ambush monster sprite 0x{tile:02X} has no combat class"),
            )
        })?;
        let mut active_objects = vec![ActiveObject::empty(); OOL_SLOTS];
        let mut actors = [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS];
        self.populate_terrain_combat_party(&mut active_objects, &mut actors, z);

        let requested_count =
            self.roll_terrain_combat_setup_count(stats.default_spawn_count, false);
        let placement_count = requested_count
            .min((OOL_SLOTS - COMBAT_PARTY_ACTOR_SLOTS) as u8)
            .min((COMBAT_ACTOR_SLOTS - COMBAT_PARTY_ACTOR_SLOTS) as u8);
        for spawn in 0..placement_count {
            let slot = COMBAT_PARTY_ACTOR_SLOTS + usize::from(spawn);
            let x = 2 + (spawn % 4) * 2;
            let y = 2 + (spawn / 4) * 2;
            active_objects[slot] = ActiveObject {
                type_byte: tile,
                tile,
                x: usize::from(x),
                y: usize::from(y),
                z,
                phase: STEADY_PHASE,
                aux1: 0,
                aux3: 0,
            };
            actors[slot] = CombatActorDescriptor::for_monster_placement(
                stats,
                slot as u8,
                x,
                y,
                combat_monster_placement_flags(stats.class),
                0,
            );
        }

        self.enter_combat_frame(active_objects, actors)?;
        Ok(format!(
            "sleep ambush entered combat against {placement_count} of {requested_count} requested {} combatant(s)",
            stats.name
        ))
    }

    pub fn enter_terrain_combat_from_world_object(
        &mut self,
        game_dir: &std::path::Path,
        plane: WorldPlane,
        object_slot: usize,
        object: ActiveObject,
    ) -> io::Result<String> {
        let hostile_terrain = self.grid[world_cell_index(object.x, object.y)];
        let aboard_ship = matches!(self.player.transport, TransportState::Ship { .. });
        let arena_index = outdoor_combat_arena_index(
            object.type_byte,
            hostile_terrain,
            aboard_ship,
            SCENE_OVERWORLD,
        )
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "active-object type 0x{:02X} has no outdoor combat class",
                    object.type_byte
                ),
            )
        })?;
        let bank = load_brit_cbt(game_dir)?;
        let record = bank.record(arena_index).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("BRIT.CBT has no arena record {arena_index}"),
            )
        })?;
        let setup = terrain_combat_setup_from_record_at_arena(plane, object, arena_index, record)?;
        let base_class = setup.base_class.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "terrain combat arena {arena_index} has no base monster class for tile 0x{:02X}",
                    object.tile
                ),
            )
        })?;
        // The identity-gap classes carry all-zero stat rows. Terrain setup
        // still creates the lead actor once and adds no followers; in
        // particular, it must not feed zero to a modulo-based count roll.
        let requested_count = if base_class.default_spawn_count == 0 {
            1
        } else {
            self.roll_terrain_combat_setup_count(base_class.default_spawn_count, false)
        };
        let replacement_tile = terrain_combat_raw_replacement_tile_for_arena(arena_index);
        let replacement_roll_seeds =
            self.terrain_combat_replacement_roll_seeds(requested_count, replacement_tile);
        let mut instance = terrain_combat_instance_from_setup(
            &setup,
            requested_count,
            replacement_tile,
            &replacement_roll_seeds,
        )?;
        self.populate_combat_party_at_placement_slots(
            &mut instance.active_objects,
            &mut instance.actors,
            object.z,
            &setup.placement_slots,
            usize::from(instance.placed_count),
        );
        let placed_count = instance.placed_count;
        let requested_count = instance.requested_count;
        self.enter_combat_frame_with_terrain(
            instance.active_objects,
            instance.actors,
            setup.terrain,
        )?;
        self.pending_combat_terrain_trigger_slot = Some(object_slot);
        Ok(format!(
            "entered terrain combat using BRIT.CBT arena {arena_index}; spawned {} of {} requested {} combatant(s)",
            placed_count,
            requested_count,
            outdoor_combat_banner_name(object.type_byte).unwrap_or(base_class.name)
        ))
    }

    pub fn populate_terrain_combat_party(
        &self,
        active_objects: &mut [ActiveObject],
        actors: &mut [CombatActorDescriptor; COMBAT_ACTOR_SLOTS],
        z: i8,
    ) {
        self.populate_combat_party_with_positions(
            active_objects,
            actors,
            z,
            &TERRAIN_COMBAT_PARTY_POSITIONS,
        );
    }

    pub fn populate_combat_party_at_placement_slots(
        &self,
        active_objects: &mut [ActiveObject],
        actors: &mut [CombatActorDescriptor; COMBAT_ACTOR_SLOTS],
        z: i8,
        placement_slots: &[CombatPlacementSlot],
        start_index: usize,
    ) {
        let mut positions = TERRAIN_COMBAT_PARTY_POSITIONS;
        for (slot, position) in positions.iter_mut().enumerate() {
            if let Some(placement) = placement_slots.get(start_index + slot) {
                *position = (placement.x, placement.y);
            }
        }
        self.populate_combat_party_with_positions(active_objects, actors, z, &positions);
    }

    pub fn populate_dungeon_room_combat_party(
        &self,
        active_objects: &mut [ActiveObject],
        actors: &mut [CombatActorDescriptor; COMBAT_ACTOR_SLOTS],
        z: i8,
        positions: &[(u8, u8); COMBAT_PARTY_ACTOR_SLOTS],
    ) {
        self.populate_combat_party_with_positions(active_objects, actors, z, positions);
    }

    fn populate_combat_party_with_positions(
        &self,
        active_objects: &mut [ActiveObject],
        actors: &mut [CombatActorDescriptor; COMBAT_ACTOR_SLOTS],
        z: i8,
        positions: &[(u8, u8); COMBAT_PARTY_ACTOR_SLOTS],
    ) {
        for (slot, member) in self.party.iter().take(COMBAT_PARTY_ACTOR_SLOTS).enumerate() {
            if !member.conscious() {
                continue;
            }
            let (x, y) = positions[slot];
            let base_step = combat_class_stats(member.class_byte)
                .map(|stats| stats.speed_seed)
                .unwrap_or(1);
            let actor_byte = combat_party_actor_byte(member.class_byte);
            active_objects[slot] = ActiveObject {
                type_byte: actor_byte,
                tile: actor_byte,
                x: usize::from(x),
                y: usize::from(y),
                z,
                phase: STEADY_PHASE,
                aux1: slot as u8,
                aux3: 0,
            };
            actors[slot] = CombatActorDescriptor::from_row([
                member.hp.min(u16::from(u8::MAX)) as u8,
                base_step,
                COMBAT_ACTOR_FLAG_SELECTABLE_80,
                slot as u8,
                slot as u8,
                0,
                x,
                y,
            ]);
        }
    }

    pub fn resolve_terrain_combat_setup_count(
        &self,
        base_count: u8,
        first_roll_seed: u8,
        fortunes_roll_seed: u8,
        town_style_override: bool,
    ) -> u8 {
        resolve_terrain_combat_setup_count(
            base_count,
            self.fortunes_of_war,
            first_roll_seed,
            fortunes_roll_seed,
            town_style_override,
        )
    }

    pub fn roll_terrain_combat_setup_count(
        &mut self,
        base_count: u8,
        town_style_override: bool,
    ) -> u8 {
        if town_style_override {
            return 1;
        }
        let count = match base_count {
            0 => 0,
            1 | 8 | 16 => base_count,
            max => {
                let mut rolled = self.random_range_u8(1, max);
                if self.fortunes_of_war != 0 {
                    rolled = self.random_range_u8(1, max);
                }
                rolled
            }
        };
        count.min(COMBAT_SPAWN_COUNT_CAP)
    }

    pub fn terrain_combat_replacement_roll_seeds(
        &mut self,
        requested_count: u8,
        replacement_tile: Option<u8>,
    ) -> Vec<u8> {
        let threshold = terrain_combat_replacement_threshold(requested_count);
        (0..requested_count)
            .map(|spawn| {
                if replacement_tile.is_some() && spawn != 0 && spawn < threshold {
                    self.random_mod_u8(TERRAIN_COMBAT_REPLACEMENT_DENOMINATOR)
                } else {
                    1
                }
            })
            .collect()
    }
}
