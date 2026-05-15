//! Combat setup helpers that bridge encounters, arena records, and class data.

use std::io;

use crate::*;

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
    pub setup_sources: Vec<DungeonRoomSetupSource>,
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

pub fn terrain_combat_setup_from_record(
    plane: WorldPlane,
    trigger: ActiveObject,
    record: &CombatArenaRecord,
) -> io::Result<TerrainCombatSetup> {
    let arena_index = outdoor_combat_arena_index_for_object(trigger).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "active-object type 0x{:02X} tile 0x{:02X} has no outdoor combat arena",
                trigger.type_byte, trigger.tile
            ),
        )
    })?;
    let placement_x = record.outdoor_placement_x();
    let placement_y = record.outdoor_placement_y();
    let placement_slots = placement_x
        .into_iter()
        .zip(placement_y)
        .enumerate()
        .map(|(slot, (x, y))| CombatPlacementSlot { slot, x, y })
        .collect();

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
    DungeonRoomCombatSetup {
        arena_index,
        terrain: record.terrain_grid(),
        setup_sources: record.dungeon_room_setup_sources(),
    }
}

pub fn terrain_combat_base_class(trigger: ActiveObject) -> Option<CombatClassStats> {
    if (0xe0..=0xe3).contains(&trigger.tile) {
        return None;
    }
    combat_class_stats_for_sprite_byte(trigger.tile)
        .or_else(|| combat_class_stats_for_sprite_byte(trigger.type_byte))
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

pub fn terrain_combat_replacement_threshold(count: u8) -> u8 {
    (count / 4) + 1
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
        Some(tile) if replacement_roll_seed % 9 == 0 => tile,
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
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
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
    pub fn enter_terrain_combat_from_world_object(
        &mut self,
        game_dir: &std::path::Path,
        plane: WorldPlane,
        object_slot: usize,
        object: ActiveObject,
    ) -> io::Result<String> {
        let arena_index = outdoor_combat_arena_index_for_object(object).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "active-object type 0x{:02X} tile 0x{:02X} has no outdoor combat arena",
                    object.type_byte, object.tile
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
        let setup = terrain_combat_setup_from_record(plane, object, record)?;
        let base_class = setup.base_class.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "terrain combat arena {arena_index} has no base monster class for tile 0x{:02X}",
                    object.tile
                ),
            )
        })?;
        let requested_count = self.resolve_terrain_combat_setup_count(
            base_class.default_spawn_count,
            self.terrain_combat_trigger_seed(object_slot, object, 0x11),
            self.terrain_combat_trigger_seed(object_slot, object, 0x29),
            false,
        );
        let replacement_roll_seeds = (0..requested_count)
            .map(|spawn| self.terrain_combat_trigger_seed(object_slot, object, 0x40 ^ spawn))
            .collect::<Vec<_>>();
        let mut instance = terrain_combat_instance_from_setup(
            &setup,
            requested_count,
            None,
            &replacement_roll_seeds,
        )?;
        self.populate_terrain_combat_party(
            &mut instance.active_objects,
            &mut instance.actors,
            object.z,
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
            placed_count, requested_count, base_class.name
        ))
    }

    pub fn terrain_combat_trigger_seed(
        &self,
        object_slot: usize,
        object: ActiveObject,
        salt: u8,
    ) -> u8 {
        self.turn as u8
            ^ (self.player.x as u8).wrapping_mul(3)
            ^ (self.player.y as u8).wrapping_mul(5)
            ^ (object_slot as u8).wrapping_mul(7)
            ^ object.type_byte.wrapping_mul(11)
            ^ object.tile.wrapping_mul(13)
            ^ salt
    }

    pub fn populate_terrain_combat_party(
        &self,
        active_objects: &mut [ActiveObject],
        actors: &mut [CombatActorDescriptor; COMBAT_ACTOR_SLOTS],
        z: i8,
    ) {
        for (slot, member) in self.party.iter().take(COMBAT_PARTY_ACTOR_SLOTS).enumerate() {
            if !member.living() {
                continue;
            }
            let (x, y) = TERRAIN_COMBAT_PARTY_POSITIONS[slot];
            active_objects[slot] = ActiveObject {
                type_byte: PLAYER_TILE,
                tile: PLAYER_TILE,
                x: usize::from(x),
                y: usize::from(y),
                z,
                phase: STEADY_PHASE,
                aux1: slot as u8,
                aux3: 0,
            };
            actors[slot] = CombatActorDescriptor::from_row([
                member.hp.min(u16::from(u8::MAX)) as u8,
                1,
                COMBAT_ACTOR_FLAG_SELECTABLE_80,
                member.class_byte,
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
}
