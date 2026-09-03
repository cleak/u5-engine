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

/// Numeric combat-stat row for the four published human combat classes.
pub const fn combat_party_class_id(class_byte: u8) -> Option<u8> {
    match class_byte.to_ascii_uppercase() {
        b'M' | b'D' => Some(0),
        b'B' | b'S' | b'T' => Some(1),
        b'F' | b'P' | b'R' => Some(2),
        b'A' => Some(3),
        _ => None,
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

/// `encounters.md §4`: the encounter's base class id "is what drives the
/// plural encounter banner", so combat entry prints one plural group
/// name above the conflict banner.
///
/// **The plural encounter-banner table itself is unpublished.**
/// `catalogs/monster-bestiary.md §2` says the shipped name data is "two
/// parallel tables" and that the "**plural** encounter-banner table"
/// exists, but the catalog publishes only the *singular* rows. The one
/// plural string this project has seen is the bat encounter's `BATS`,
/// read cell by cell out of a capture of the original's own combat entry
/// and matched against the shipped `IBM.CH`. Nothing published states the
/// relation between the two tables, so no rule is derived from that one
/// sample: every class the capture has not shown prints no name line
/// rather than an invented one. The forty-eight-row plural table is
/// pending publication as `cleak/u5-spec#185`.
///
/// *runtime observation, spec silent.*
pub fn combat_class_plural_banner_name(class: u8) -> Option<&'static str> {
    match class {
        // Bat. Observed in the original at combat entry, one blank row
        // under the direction echo and one above the conflict banner.
        21 => Some("BATS"),
        _ => None,
    }
}

/// The plural encounter-banner line for one outdoor hostile sprite byte.
///
/// `encounters.md §4` says the ship family's "banner for this case prints
/// a fixed pirate plural literal rather than the class-1 plural name",
/// and `catalogs/monster-bestiary.md §2` says the plural table "does name
/// class 8, and that banner is where the name "Pirate" used for row 8
/// above comes from". Neither publishes the literal's own spelling, and
/// it has not been observed, so this path prints no name line rather than
/// guessing between the singular and plural spellings. Pending
/// `cleak/u5-spec#185`.
pub fn outdoor_combat_plural_banner_name(type_byte: u8) -> Option<&'static str> {
    if outdoor_type_is_pirate(type_byte) {
        return None;
    }
    outdoor_combat_class_id(type_byte).and_then(combat_class_plural_banner_name)
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

/// `encounters.md §6` sleep-ambush party seats. Ordinary terrain
/// combat seats the party from the selected arena record's own party
/// entry coordinates (`combat.md §5`), never from this list; it remains
/// only as the fallback the sleep-ambush entry uses, which loads no
/// arena record.
pub const TERRAIN_COMBAT_PARTY_POSITIONS: [(u8, u8); COMBAT_PARTY_ACTOR_SLOTS] =
    [(5, 5), (4, 5), (6, 5), (5, 4), (5, 6), (4, 6)];

/// `dungeon-mode.md §14.1`: the wandering-monster launch reads no
/// arena record from disk, so its synthesised record has no bank index.
/// Zero is a placeholder carried only in the setup struct's diagnostic
/// `arena_index` field.
pub const DUNGEON_AMBUSH_SYNTHETIC_ARENA_INDEX: usize = 0;

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

/// `combat.md §5` + `combat.md §6.3` monster sprite identity: an
/// ordinary monster class's renderer-facing tile is
/// `class * 4 + 0x40` (§6.3 states the identity explicitly for
/// Insect Swarm, `31 * 4 + 0x40 = 0xBC`).
pub const fn combat_class_sprite_byte(class: u8) -> u8 {
    class
        .wrapping_mul(4)
        .wrapping_add(OUTDOOR_COMBAT_TYPE_FIRST)
}

/// `audio.md §8.3.1` summon tile flash: the class-parallel *flash* tile.
///
/// "The rule is `flash tile = creature class x 4 + 320`, and the settle tile
/// that replaces it is `creature class x 4 + 64`" - the latter is exactly
/// [`combat_class_sprite_byte`]. The flash bank sits above the 8-bit tile
/// range, so this returns `u16` where the sprite byte returns `u8`.
///
/// The flash is drawn by "one invocation of the engine's shared single-cell
/// pseudorandom pixel converge - the same primitive, the same driver path, as
/// the moongate tile shimmer", in the 256-position order this engine already
/// models as [`crate::return_to_view::return_to_view_single_cell_write_coordinates`].
/// It "needs no separate implementation".
///
/// `§8.3.1` marks two limits on this rule that a caller must respect: the
/// arithmetic "was verified ... for the daemon and cross-checked against the
/// Conjure and Swarm rows", but "two spot-checked classes render as
/// recognisable, unrelated pictures", so "the rule must not be assumed to give
/// a sensible flash for every class"; and the whole 256-plot result is
/// EGA-only, the other three shipped drivers being unchecked.
pub const fn combat_class_summon_flash_tile(class: u8) -> u16 {
    class as u16 * 4 + 320
}

/// `combat.md §5`: "A town-style single-attacker override applies
/// before the lookup: if the pre-combat scene was a town, dwelling,
/// castle, or keep, the party is on the surface, and the base class
/// is not 12 (Guard), the count is forced to one." The scene half of
/// that condition is the town-family scene band - the same partition
/// [`outdoor_combat_arena_index`] treats as non-overworld.
pub const fn scene_is_town_dwelling_castle_or_keep(scene_byte: u8) -> bool {
    scene_byte >= SCENE_TOWN_FAMILY_FIRST && scene_byte <= SCENE_TOWN_FAMILY_LAST
}

/// `combat.md §5` player-facing combat banner. "A short combat banner
/// ("CONFLICT") is printed at the start of setup, before any monsters
/// are placed" - step 3 of the strict order of operations, after
/// party seating and before the count roll.
pub const COMBAT_BANNER: &str = "CONFLICT";

/// The character the original's combat banner flanks `CONFLICT` with.
///
/// The banner row was decoded cell by cell out of the original's own
/// combat-entry capture and each cell matched against the shipped
/// `IBM.CH`: the row is exactly `*** CONFLICT ***`, and the flank cell's
/// eight-by-eight bitmap matches **one** slot of the shipped font,
/// `0x2A` - the ASCII asterisk, which this font draws as a five-pixel
/// pointed diamond. `formats/font-ch.md §3` gives every glyph an
/// "eight-by-eight pixel cell" and §4 says "the printable ASCII region
/// maps directly to matching glyph positions", which is what makes that
/// unique bitmap match an identification of the *character* rather than
/// of a picture.
///
/// *runtime observation, spec silent* - `combat.md §5` publishes the
/// banner word but not its decoration.
pub const COMBAT_BANNER_FLANK_GLYPH: char = '*';

/// The complete combat banner line as the original prints it: three
/// flank asterisks, a space, `CONFLICT`, a space, three more asterisks -
/// exactly the sixteen cells of the message window's row.
///
/// *runtime observation, spec silent* (see [`COMBAT_BANNER_FLANK_GLYPH`]).
pub fn combat_banner_line() -> String {
    let flank: String = std::iter::repeat_n(COMBAT_BANNER_FLANK_GLYPH, 3).collect();
    format!("{flank} {COMBAT_BANNER} {flank}")
}

/// Combat placements initialise byte seven of the renderer-facing active
/// object to the all-ones "no linked descriptor" marker. The parallel combat
/// descriptor owns the actual link in the other direction.
pub const COMBAT_ACTIVE_OBJECT_NO_DESCRIPTOR: u8 = u8::MAX;

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

/// `formats/cbt.md §5` + `combat.md §5`: the six per-arena party
/// entry coordinates, indexed by party slot. "For party slot `i`, X
/// comes from column `11 + i` and Y from column `17 + i`", so table A
/// is the X slice and table B the Y slice. Party seats come from this
/// table and never from the monster placement slots.
pub fn terrain_combat_party_entry_positions(
    setup: &TerrainCombatSetup,
) -> [(u8, u8); COMBAT_PARTY_ACTOR_SLOTS] {
    let mut positions = [(0u8, 0u8); COMBAT_PARTY_ACTOR_SLOTS];
    for (slot, position) in positions.iter_mut().enumerate() {
        *position = (setup.setup_table_a[slot], setup.setup_table_b[slot]);
    }
    positions
}

/// `active-objects.md §7` and `combat.md §5`: the two combat tables are
/// allocated by two *independent* first-free scans. "Descriptors are
/// scanned from index zero for party members and from index six for
/// monsters, taking the first descriptor whose flags byte is zero.
/// Active-object records are scanned from index zero for *everyone*,
/// taking the first record whose tile byte is zero."
///
/// Party seating runs before any monster is placed, so a party with dead
/// members packs into fewer records and leaves the monster placer a
/// lower first-free record than the six-slot party band would imply:
/// "a party of four puts the first monster at descriptor six and
/// active-object record four". The descriptor's active-object link byte
/// is the authoritative pairing, not index equality.
pub fn first_free_combat_active_object_record(active_objects: &[ActiveObject]) -> Option<usize> {
    active_objects.iter().position(|object| object.tile == 0)
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

/// `formats/cbt.md §5` ordinary-source sprite. The `0xEC..0xEF` vermin
/// family has no sprite derivable from its source byte: its class is
/// substituted from the pre-rolled palette first, and the tile follows
/// the substituted class.
pub fn dungeon_room_source_sprite(source: u8) -> Option<u8> {
    match DungeonRoomSetupSourceKind::from_source(source) {
        DungeonRoomSetupSourceKind::OrdinaryCombatant {
            setup_class,
            palette_selector: None,
        } => Some(combat_class_sprite_byte(setup_class)),
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
    // No party has been seated into these tables, so the room sources
    // continue from the full six-slot party band.
    dungeon_room_combat_instance_from_setup_after_party(
        setup,
        z,
        prng_state,
        vec![ActiveObject::empty(); OOL_SLOTS],
        [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS],
        COMBAT_PARTY_ACTOR_SLOTS,
    )
}

/// `formats/cbt.md §5` room-source scan run over tables the party has
/// already been seated into. The ordinary path "allocates a combat actor
/// descriptor (from the first free monster slot, above the six party
/// slots) *and* a renderer-facing active-object record, links them",
/// while `active-objects.md §7` puts the record itself at the lowest
/// free index left by the seated party. `first_free_record` is the
/// seated-party cursor those records continue from.
pub fn dungeon_room_combat_instance_from_setup_after_party(
    setup: &DungeonRoomCombatSetup,
    z: i8,
    prng_state: &mut u16,
    mut active_objects: Vec<ActiveObject>,
    mut actors: [CombatActorDescriptor; COMBAT_ACTOR_SLOTS],
    first_free_record: usize,
) -> TerrainCombatInstance {
    let mut placed_count = 0u8;
    let random_special_setup_ids =
        dungeon_room_random_special_setup_ids(setup.scan_sources, prng_state);

    for source in &setup.setup_sources {
        // The scans are independent. Special placements consume only a
        // renderer record, so they must not create holes in the descriptor
        // scan used by a later ordinary source.
        let Some(record_slot) = active_objects
            .iter()
            .enumerate()
            .skip(first_free_record)
            .find_map(|(slot, object)| (object.tile == 0).then_some(slot))
        else {
            continue;
        };
        match source.kind {
            DungeonRoomSetupSourceKind::OrdinaryCombatant {
                setup_class,
                palette_selector,
            } => {
                // `formats/cbt.md §5`: the vermin family keeps the
                // ordinary placement path and only replaces its derived
                // setup class with the pre-rolled palette id selected by
                // the source's low two bits. "It is placed on the
                // ordinary path with a full combat actor and the tile
                // derived from the substituted class, exactly like any
                // other ordinary source."
                let (tile, stats) = match palette_selector {
                    Some(selector) => {
                        let class = random_special_setup_ids
                            .get(usize::from(selector))
                            .copied()
                            .unwrap_or(setup_class);
                        let Some(stats) = combat_class_stats(class) else {
                            continue;
                        };
                        (combat_class_sprite_byte(class), stats)
                    }
                    None => {
                        let Some(tile) = dungeon_room_source_sprite(source.source) else {
                            continue;
                        };
                        let Some(stats) = combat_class_stats(setup_class) else {
                            continue;
                        };
                        (tile, stats)
                    }
                };
                let Some(descriptor_slot) = actors
                    .iter()
                    .enumerate()
                    .skip(COMBAT_PARTY_ACTOR_SLOTS)
                    .find_map(|(slot, actor)| actor.is_free_for_allocation().then_some(slot))
                else {
                    continue;
                };
                active_objects[record_slot] = ActiveObject {
                    type_byte: tile,
                    tile,
                    x: usize::from(source.x),
                    y: usize::from(source.y),
                    z,
                    phase: STEADY_PHASE,
                    aux1: stats.max_hp,
                    aux3: COMBAT_ACTIVE_OBJECT_NO_DESCRIPTOR,
                };
                // `combat.md §5`: "Each ordinary monster placement then
                // consumes one speed-variation draw." The draw is the
                // uniform `0..7` adjustment feeding the base-step, and
                // the phase counter is thirty-six minus that base-step.
                // "The later source setup scans indexes in ascending
                // order, so actor placement and speed draws occur in
                // ascending occupied-source order."
                let speed_adjust_roll = u5_prng_range_u16(
                    prng_state,
                    u16::from(COMBAT_PLACEMENT_SPEED_ADJUST_ROLL_LOW),
                    u16::from(COMBAT_PLACEMENT_SPEED_ADJUST_ROLL_HIGH),
                ) as u8;
                actors[descriptor_slot] = combat_placement_descriptor(
                    stats,
                    record_slot as u8,
                    source.x,
                    source.y,
                    combat_monster_placement_flags(stats.class),
                    speed_adjust_roll,
                );
                placed_count = placed_count.saturating_add(1);
            }
            DungeonRoomSetupSourceKind::AbsorbableField => {
                active_objects[record_slot] = ActiveObject {
                    type_byte: source.source,
                    tile: source.source,
                    x: usize::from(source.x),
                    y: usize::from(source.y),
                    z,
                    phase: STEADY_PHASE,
                    aux1: source.source,
                    aux3: COMBAT_ACTIVE_OBJECT_NO_DESCRIPTOR,
                };
                placed_count = placed_count.saturating_add(1);
            }
            DungeonRoomSetupSourceKind::SpecialPlacement(placement) => {
                active_objects[record_slot] =
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
        DungeonRoomSpecialPostWrite::DegenerateDrawThenConstant(value) => {
            let _ = u5_prng_range_u16(prng_state, 0, 0);
            value
        }
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
        aux1: match placement.post_write {
            DungeonRoomSpecialPostWrite::None => placement.setup_id,
            post_write => dungeon_room_special_aux1(post_write, z, prng_state),
        },
        aux3: COMBAT_ACTIVE_OBJECT_NO_DESCRIPTOR,
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

/// `combat.md §5` + `catalogs/monster-bestiary.md §2.1`: the combat
/// class a terrain-combat spawn index is created with. "The first
/// monster always uses the encounter's base combat class. Subsequent
/// monsters normally reuse that same class. For early spawn indexes
/// below the `count / 4 + 1` threshold, each monster rolls a
/// one-in-nine check; only a zero result substitutes the base class's
/// **companion class** from the per-class companion table." The
/// substituted value is a CLASS id, and nothing about the
/// substitution is keyed to the arena.
pub fn terrain_combat_class_for_spawn_index(
    spawn_index: u8,
    count: u8,
    base_class: u8,
    companion_class: Option<u8>,
    replacement_roll_seed: u8,
) -> u8 {
    if spawn_index == 0 {
        return base_class;
    }
    if spawn_index >= terrain_combat_replacement_threshold(count) {
        return base_class;
    }
    match companion_class {
        Some(class) if terrain_combat_replacement_roll_picks_replacement(replacement_roll_seed) => {
            class
        }
        _ => base_class,
    }
}

pub fn terrain_combat_instance_from_setup(
    setup: &TerrainCombatSetup,
    requested_count: u8,
    companion_class: Option<u8>,
    replacement_roll_seeds: &[u8],
    speed_adjust_roll_seeds: &[u8],
) -> io::Result<TerrainCombatInstance> {
    let mut active_objects = vec![ActiveObject::empty(); COMBAT_ACTOR_SLOTS];
    let mut actors = [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS];
    // No party has been seated into these tables, so the monster
    // records continue from the full six-slot party band.
    let placed_count = terrain_combat_place_monsters_after_party(
        setup,
        requested_count,
        companion_class,
        replacement_roll_seeds,
        speed_adjust_roll_seeds,
        &mut active_objects,
        &mut actors,
        COMBAT_PARTY_ACTOR_SLOTS,
    )?;
    Ok(TerrainCombatInstance {
        active_objects,
        actors,
        requested_count,
        placed_count,
        unplaced_count: requested_count.saturating_sub(placed_count),
    })
}

/// `combat.md §5` step 5: place the monsters into the sixteen
/// placement slots of an arena whose party seats have already been
/// written by step 2. Seating is not this pass's business - "party
/// seats never depend on the monster count and never consume a
/// placement slot."
#[allow(clippy::too_many_arguments)]
pub fn terrain_combat_place_monsters(
    setup: &TerrainCombatSetup,
    requested_count: u8,
    companion_class: Option<u8>,
    replacement_roll_seeds: &[u8],
    speed_adjust_roll_seeds: &[u8],
    active_objects: &mut [ActiveObject],
    actors: &mut [CombatActorDescriptor; COMBAT_ACTOR_SLOTS],
) -> io::Result<u8> {
    // This entry point is handed tables no party has been seated into,
    // so its records continue from the full six-slot party band.
    terrain_combat_place_monsters_after_party(
        setup,
        requested_count,
        companion_class,
        replacement_roll_seeds,
        speed_adjust_roll_seeds,
        active_objects,
        actors,
        COMBAT_PARTY_ACTOR_SLOTS,
    )
}

/// `active-objects.md §7`: the same step 5 placement pass, told
/// explicitly which active-object record the seated party left free.
/// Monster *descriptors* still come from the index-six scan, so
/// "a party of four puts the first monster at descriptor six and
/// active-object record four".
#[allow(clippy::too_many_arguments)]
pub fn terrain_combat_place_monsters_after_party(
    setup: &TerrainCombatSetup,
    requested_count: u8,
    companion_class: Option<u8>,
    replacement_roll_seeds: &[u8],
    speed_adjust_roll_seeds: &[u8],
    active_objects: &mut [ActiveObject],
    actors: &mut [CombatActorDescriptor; COMBAT_ACTOR_SLOTS],
    first_free_record: usize,
) -> io::Result<u8> {
    let base_class = setup.base_class.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "terrain combat arena {} has no base monster class for tile 0x{:02X}",
                setup.arena_index, setup.base_tile
            ),
        )
    })?;
    let max_placeable = (COMBAT_ACTOR_SLOTS - COMBAT_PARTY_ACTOR_SLOTS)
        .min(setup.placement_slots.len())
        .min(
            active_objects
                .len()
                .saturating_sub(COMBAT_PARTY_ACTOR_SLOTS),
        );
    let placed_count = usize::from(requested_count).min(max_placeable);
    let z = if setup.underworld_variant {
        WorldPlane::Underworld.save_floor()
    } else {
        WorldPlane::Britannia.save_floor()
    };

    let mut written = 0usize;
    for spawn_index in 0..placed_count {
        let placement = setup.placement_slots[spawn_index];
        let roll_seed = replacement_roll_seeds
            .get(spawn_index)
            .copied()
            .unwrap_or_default();
        let chosen_class = terrain_combat_class_for_spawn_index(
            spawn_index as u8,
            requested_count,
            base_class.class,
            companion_class,
            roll_seed,
        );
        // `combat.md §5`: "A spawned actor's renderer-facing tile is
        // then derived from whichever class was chosen." The base
        // class keeps the triggering object's own tile byte so its
        // animation frame survives; a substituted companion class
        // derives its sprite from the class identity.
        let stats = combat_class_stats(chosen_class).unwrap_or(base_class);
        let tile = if stats.class == base_class.class {
            setup.base_tile
        } else {
            combat_class_sprite_byte(stats.class)
        };
        // `active-objects.md §7`: the descriptor comes from the
        // monster-side scan that starts at index six, while the
        // renderer-facing record comes from the lowest free record.
        // "A party of four puts the first monster at descriptor six and
        // active-object record four."
        let descriptor_slot = COMBAT_PARTY_ACTOR_SLOTS + spawn_index;
        let record_slot = first_free_record + spawn_index;
        if record_slot >= active_objects.len() || descriptor_slot >= COMBAT_ACTOR_SLOTS {
            written = spawn_index;
            break;
        }
        active_objects[record_slot] = ActiveObject {
            type_byte: tile,
            tile,
            x: usize::from(placement.x),
            y: usize::from(placement.y),
            z,
            phase: STEADY_PHASE,
            aux1: stats.max_hp,
            aux3: COMBAT_ACTIVE_OBJECT_NO_DESCRIPTOR,
        };
        // `combat.md §5`: "Each ordinary monster placement then consumes
        // one speed-variation draw." The caller pre-rolls one uniform
        // `0..7` value per spawn, in placement order.
        let speed_adjust_roll = speed_adjust_roll_seeds
            .get(spawn_index)
            .copied()
            .unwrap_or(COMBAT_PLACEMENT_SPEED_ADJUST_ROLL_NEUTRAL);
        actors[descriptor_slot] = combat_placement_descriptor(
            stats,
            // The link byte is the authoritative pairing.
            record_slot as u8,
            placement.x,
            placement.y,
            combat_monster_placement_flags(stats.class),
            speed_adjust_roll,
        );
        written = spawn_index + 1;
    }

    Ok(written as u8)
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
        // `combat.md §5` / `formats/cbt.md §5`: the room helper reads
        // its party-entry coordinates and seats the party *before* it
        // scans the sixteen room sources, so the sources continue from
        // the first record the seated party left free.
        let mut active_objects = vec![ActiveObject::empty(); OOL_SLOTS];
        let mut actors = [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS];
        self.populate_dungeon_room_combat_party(
            &mut active_objects,
            &mut actors,
            level as i8,
            &setup.party_positions,
        );
        let first_free_record = first_free_combat_active_object_record(&active_objects)
            .unwrap_or(COMBAT_PARTY_ACTOR_SLOTS);
        let instance = dungeon_room_combat_instance_from_setup_after_party(
            &setup,
            level as i8,
            &mut self.prng_state,
            active_objects,
            actors,
            first_free_record,
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

    /// `dungeon-mode.md §14.1`: "a shuffled permutation of the sixteen
    /// slot indices is built" before the source band is written.
    pub fn dungeon_ambush_source_permutation(&mut self) -> [u8; DUNGEON_ROOM_SOURCE_COUNT] {
        let mut permutation = [0u8; DUNGEON_ROOM_SOURCE_COUNT];
        for (index, slot) in permutation.iter_mut().enumerate() {
            *slot = index as u8;
        }
        for index in 0..DUNGEON_ROOM_SOURCE_COUNT {
            let pick = usize::from(self.random_range_u8(0, 15));
            permutation.swap(index, pick);
        }
        permutation
    }

    /// The dormant terrain helper differs by stopping before index fifteen.
    /// No shipped caller enables it, but keeping the exact algorithm makes the
    /// clean implementation deterministic for custom callers and tests.
    pub fn dormant_terrain_combat_source_permutation(&mut self) -> [u8; DUNGEON_ROOM_SOURCE_COUNT] {
        let mut permutation = [0u8; DUNGEON_ROOM_SOURCE_COUNT];
        for (index, slot) in permutation.iter_mut().enumerate() {
            *slot = index as u8;
        }
        for index in 0..DUNGEON_ROOM_SOURCE_COUNT - 1 {
            let pick = usize::from(self.random_range_u8(0, 15));
            permutation.swap(index, pick);
        }
        permutation
    }

    /// `dungeon-mode.md §14.1` wandering-monster combat. The launch does
    /// not read a `DUNGEON.CBT` record: it synthesises the arena in the
    /// room buffer - party-entry rows one through four, the
    /// facing-selected source coordinate rows, and `count` copies of the
    /// ordinary source byte `class * 4 + 0x40` in a shuffled permutation
    /// of the sixteen source slots - and then hands that record to the
    /// ordinary room-combat setup helper, with the party-entry seed set
    /// to the party's current dungeon facing.
    pub fn enter_dungeon_active_monster_combat(
        &mut self,
        level: u8,
        object: ActiveObject,
    ) -> io::Result<String> {
        let stats = combat_class_stats(object.aux1).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "dungeon active-object DEP1 0x{:02X} has no combat class",
                    object.aux1
                ),
            )
        })?;
        let facing_seed = dungeon_room_entry_seed_for_direction(self.player.facing);
        let permutation = self.dungeon_ambush_source_permutation();
        // The live painter always consumes this draw and only then replaces
        // the result for the exact-count sentinels eight and sixteen.
        let rolled_count = self.random_range_u8(1, stats.default_spawn_count.max(1));
        let count = if matches!(stats.default_spawn_count, 8 | 16) {
            stats.default_spawn_count
        } else {
            rolled_count
        }
        .max(1);
        let record = CombatArenaRecord::synthesise_dungeon_ambush(
            DUNGEON_AMBUSH_ARENA_FLOOR_TILE,
            facing_seed,
            stats.class,
            count,
            permutation,
        );
        let setup = dungeon_room_combat_setup_from_record_for_entry(
            DUNGEON_AMBUSH_SYNTHETIC_ARENA_INDEX,
            &record,
            facing_seed,
            true,
        );
        // Seat the party first, then scan the synthesised room sources
        // from the first free record (`combat.md §5` order of
        // operations).
        let mut active_objects = vec![ActiveObject::empty(); OOL_SLOTS];
        let mut actors = [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS];
        self.populate_dungeon_room_combat_party(
            &mut active_objects,
            &mut actors,
            level as i8,
            &setup.party_positions,
        );
        let first_free_record = first_free_combat_active_object_record(&active_objects)
            .unwrap_or(COMBAT_PARTY_ACTOR_SLOTS);
        let instance = dungeon_room_combat_instance_from_setup_after_party(
            &setup,
            level as i8,
            &mut self.prng_state,
            active_objects,
            actors,
            first_free_record,
        );
        let placed_count = instance.placed_count;
        self.enter_combat_frame_with_terrain(
            instance.active_objects,
            instance.actors,
            setup.terrain,
        )?;
        Ok(format!(
            "entered dungeon combat against {placed_count} of {count} {} from active monster tile {}",
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
        let mut placed = 0u8;
        for spawn in 0..placement_count {
            // `active-objects.md §7`: descriptor from the monster-side
            // scan at index six, record from the lowest free record left
            // by the seated party; the link byte pairs them.
            let descriptor_slot = COMBAT_PARTY_ACTOR_SLOTS + usize::from(spawn);
            let Some(record_slot) = first_free_combat_active_object_record(&active_objects) else {
                break;
            };
            let x = 2 + (spawn % 4) * 2;
            let y = 2 + (spawn / 4) * 2;
            active_objects[record_slot] = ActiveObject {
                type_byte: tile,
                tile,
                x: usize::from(x),
                y: usize::from(y),
                z,
                phase: STEADY_PHASE,
                aux1: stats.max_hp,
                aux3: COMBAT_ACTIVE_OBJECT_NO_DESCRIPTOR,
            };
            // `combat.md §5`: one speed-variation draw per ordinary
            // placement, phase counter of thirty-six minus the base-step.
            let speed_adjust_roll = self.combat_placement_speed_adjust_roll();
            actors[descriptor_slot] = combat_placement_descriptor(
                stats,
                record_slot as u8,
                x,
                y,
                combat_monster_placement_flags(stats.class),
                speed_adjust_roll,
            );
            placed += 1;
        }
        let placement_count = placed;

        self.enter_combat_frame(active_objects, actors)?;
        if let Some(snapshot) = self.combat_frame_snapshot.as_mut() {
            // `combat.md §6.3`: sleep/camp alternate-entry modes 4 and 6
            // carry bit 0x04, which suppresses the faint helper's world tick.
            snapshot.suppress_controlled_faint_sleep_tick = true;
        }
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
        self.enter_terrain_combat_from_world_object_in_scene(
            game_dir,
            plane,
            object_slot,
            object,
            SCENE_OVERWORLD,
        )
    }

    /// `combat.md §5` ordinary terrain-combat setup. The order of
    /// operations is strict: clear both tables, seat the party from the
    /// arena record's own party entry coordinates, print the combat
    /// banner, choose the monster count, then place the monsters.
    /// `pre_combat_scene` is the scene the fight was launched from; it
    /// feeds both the arena selector's scene fallback and the
    /// town-style single-attacker override.
    pub fn enter_terrain_combat_from_world_object_in_scene(
        &mut self,
        game_dir: &std::path::Path,
        plane: WorldPlane,
        object_slot: usize,
        object: ActiveObject,
        pre_combat_scene: u8,
    ) -> io::Result<String> {
        let hostile_terrain = self.grid[world_cell_index(object.x, object.y)];
        self.enter_terrain_combat_from_object_in_scene_with_terrain(
            game_dir,
            plane,
            object_slot,
            object,
            pre_combat_scene,
            hostile_terrain,
        )
    }

    /// `town-mode.md §14` / `encounters.md §7`: the town NPC-conflict
    /// chain "hands the target NPC's linked active-object slot to the
    /// same terrain-combat entry the overworld uses". Only the ground
    /// sample differs - the world arm reads the 256-wide world grid,
    /// while a location arm reads its own 32x32 floor - so the terrain
    /// byte is a parameter and everything downstream is shared.
    pub fn enter_terrain_combat_from_object_in_scene_with_terrain(
        &mut self,
        game_dir: &std::path::Path,
        plane: WorldPlane,
        object_slot: usize,
        object: ActiveObject,
        pre_combat_scene: u8,
        hostile_terrain: u8,
    ) -> io::Result<String> {
        let aboard_ship = matches!(self.player.transport, TransportState::Ship { .. });
        let arena_index = outdoor_combat_arena_index(
            object.type_byte,
            hostile_terrain,
            aboard_ship,
            pre_combat_scene,
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
        // `encounters.md §4` Shadow Lord arena branch: "if the party is
        // carrying the Sceptre of Lord British, entering that fight
        // ... clears the sceptre flag."
        let sceptre_reclaimed = base_class.class == COMBAT_CLASS_SHADOW_LORD
            && self.special_items[SPECIAL_ITEM_SCEPTRE_LB_INDEX] != 0;
        if sceptre_reclaimed {
            self.special_items[SPECIAL_ITEM_SCEPTRE_LB_INDEX] = 0;
        }

        // Step 1/2: clear both tables, then seat the party from the
        // arena record's six party entry X/Y coordinates. Seating runs
        // before the count roll and reads its own coordinate table, so
        // party seats never depend on the monster count.
        let mut active_objects = vec![ActiveObject::empty(); COMBAT_ACTOR_SLOTS];
        let mut actors = [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS];
        let party_positions = terrain_combat_party_entry_positions(&setup);
        self.populate_combat_party_with_positions(
            &mut active_objects,
            &mut actors,
            object.z,
            &party_positions,
        );
        // `active-objects.md §7`: monster records "continue from the
        // first record left free by the seated party". Scanned, not
        // counted - see the note on
        // [`Self::populate_combat_party_with_positions`].
        let first_free_record = first_free_combat_active_object_record(&active_objects)
            .unwrap_or(COMBAT_PARTY_ACTOR_SLOTS);

        // Step 3: the encounter-name line and the combat banner, before
        // any monster is placed. `combat.md §5.3` groups them as one
        // draw-free step - "Conflict banner, arena-record load,
        // encounter-name print" - and `encounters.md §4` says the base
        // class id "drives the plural encounter banner". The name line is
        // printed only for a class whose plural string is known; see
        // [`combat_class_plural_banner_name`], which never invents one.
        //
        // `text-output.md §11` / `combat.md §5`: the banner is produced
        // by setup and every production caller of this entry point
        // overwrites the message slot with its own diagnostic before the
        // next turn-composition flush, so these lines have to be emitted
        // into the transcript at the moment they are produced rather than
        // parked in the slot.
        //
        // The blank row under the direction echo is `text-output.md
        // §10.4`'s derived blank: the completed verb echo leaves the
        // cursor at column 0 of a fresh row and "the next cycle's leading
        // line feed advances again — producing exactly one blank row
        // after each completed command turn". This transcript is
        // line-oriented rather than cell-based, so that derived row is
        // materialised as an explicit blank entry.
        //
        // The second blank row - between the encounter name and the
        // banner - and the print order, name above banner, are *runtime
        // observation, spec silent*: they were read off a capture of the
        // original's own combat entry, and `combat.md §5.3` lists banner
        // and encounter-name print in one unordered row.
        self.push_explicit_blank_message_entry();
        if let Some(plural) = outdoor_combat_plural_banner_name(object.type_byte)
            .or_else(|| combat_class_plural_banner_name(base_class.class))
        {
            self.emit_centered_message_line(plural);
            self.push_explicit_blank_message_entry();
        }
        self.emit_centered_message_line(combat_banner_line());

        // Step 4: choose the monster count. The identity-gap classes
        // carry all-zero stat rows; terrain setup still creates the lead
        // actor once and adds no followers, and in particular must not
        // feed zero to a modulo-based count roll.
        let town_style_override = scene_is_town_dwelling_castle_or_keep(pre_combat_scene)
            && !setup.underworld_variant
            && base_class.class != COMBAT_CLASS_GUARD;
        let requested_count = if base_class.default_spawn_count == 0 {
            1
        } else {
            self.roll_terrain_combat_setup_count(
                base_class.default_spawn_count,
                town_style_override,
            )
        };

        // Step 5: place the monsters.
        let companion_class = combat_class_companion(base_class.class);
        let (replacement_roll_seeds, speed_adjust_roll_seeds) =
            self.terrain_combat_placement_roll_seeds(requested_count, companion_class);
        let placed_count = terrain_combat_place_monsters_after_party(
            &setup,
            requested_count,
            companion_class,
            &replacement_roll_seeds,
            &speed_adjust_roll_seeds,
            &mut active_objects,
            &mut actors,
            first_free_record,
        )?;
        self.enter_combat_frame_with_terrain(active_objects, actors, setup.terrain)?;
        self.pending_combat_terrain_trigger_slot = Some(object_slot);
        Ok(format!(
            "entered terrain combat using BRIT.CBT arena {arena_index}; spawned {} of {} requested {} combatant(s){}",
            placed_count,
            requested_count,
            outdoor_combat_banner_name(object.type_byte).unwrap_or(base_class.name),
            if sceptre_reclaimed {
                "; sceptre of Lord British reclaimed"
            } else {
                ""
            }
        ))
    }

    pub fn populate_terrain_combat_party(
        &mut self,
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

    pub fn populate_dungeon_room_combat_party(
        &mut self,
        active_objects: &mut [ActiveObject],
        actors: &mut [CombatActorDescriptor; COMBAT_ACTOR_SLOTS],
        z: i8,
        positions: &[(u8, u8); COMBAT_PARTY_ACTOR_SLOTS],
    ) {
        self.populate_combat_party_with_positions(active_objects, actors, z, positions);
    }

    /// `active-objects.md §7`: party members are "allocated the first
    /// free records, one per live (non-dead) member in roster order, so
    /// a full party occupies records zero through five and a party with
    /// dead members packs into fewer". `combat.md §5` says the same for
    /// descriptors - a dead member "is skipped entirely: no descriptor,
    /// no active-object record, no arena presence. The remaining members
    /// therefore pack into the low descriptor indexes rather than
    /// keeping their roster index" - and adds that for party members
    /// "the two scans run in lockstep ... so a party member's descriptor
    /// index and its active-object index are always equal".
    ///
    /// The *seat coordinates* are still taken by roster slot: `§5` places
    /// the member at the arena entry coordinates "indexed by *party
    /// slot* (the roster index, not the packed descriptor index)". The
    /// descriptor's owner/target/class field records that roster index,
    /// which is what lets every later reader undo the packing - see
    /// [`PlayState::combat_roster_slot_for_actor_slot`].
    pub fn populate_combat_party_with_positions(
        &mut self,
        active_objects: &mut [ActiveObject],
        actors: &mut [CombatActorDescriptor; COMBAT_ACTOR_SLOTS],
        z: i8,
        positions: &[(u8, u8); COMBAT_PARTY_ACTOR_SLOTS],
    ) {
        let roster: Vec<PartyMember> = self
            .party
            .iter()
            .copied()
            .take(COMBAT_PARTY_ACTOR_SLOTS)
            .collect();
        let mut cleared_active_player = false;
        let mut packed_slot = 0usize;
        for (roster_slot, member) in roster.into_iter().enumerate() {
            // `combat.md §5`: only `'D'` skips. An `'S'` (asleep) member
            // "is seated and then immediately marked asleep", so the
            // seater must not gate on consciousness.
            if member.status == PARTY_STATUS_DEAD {
                continue;
            }
            let record_slot = packed_slot;
            packed_slot += 1;
            let (x, y) = positions[roster_slot];
            // `combat.md §5` party descriptor seeding: base-step is "The
            // character's dexterity", and the phase counter is
            // "Thirty-six minus the base-step". The class stat table's
            // speed seed is a *monster* placement input and has no part
            // in party seating; §5 also charges no speed-variation draw
            // to a party seat, so nothing here touches the shared PRNG.
            //
            // The raw DEX byte is used as published. Whether a very low
            // dexterity is floored before the subtraction is open in
            // `cleak/u5-spec#178`; until that is answered a DEX-3 Avatar
            // legitimately seats at phase 33.
            let base_step = member.dexterity();
            // The save stores an ASCII profession letter, while the combat
            // stat table uses the four numeric human-class rows; that
            // mapping still selects the *sprite*, and an unrecognised
            // letter still leaves the presentation byte at zero.
            let actor_byte = combat_party_actor_byte(member.class_byte);
            // `combat.md §5`: an `'S'` member's "presentation record
            // shows the prone marker". `§5` names the marker only by
            // that phrase; the published combat asleep presentation tile
            // is `catalogs/item-list.md §7.2`'s
            // [`COMBAT_POTION_SLEEP_DISPLAY_TILE`], which "replace[s] the
            // selected member's ordinary displayed tile ... until the
            // normal wake path restores it" - the same displayed-tile
            // override this seater needs, and the one
            // [`PlayState::apply_combat_sleep_wake_dispatch`] already
            // reverses by restoring the record's base/type byte.
            let asleep = member.status == b'S';
            if asleep && self.active_player == Some(roster_slot) {
                // `combat.md §5`: "the active-player sentinel is cleared
                // if that member was the active player."
                cleared_active_player = true;
            }
            active_objects[record_slot] = ActiveObject {
                type_byte: actor_byte,
                tile: if asleep {
                    COMBAT_POTION_SLEEP_DISPLAY_TILE
                } else {
                    actor_byte
                },
                x: usize::from(x),
                y: usize::from(y),
                z,
                phase: STEADY_PHASE,
                aux1: roster_slot as u8,
                aux3: COMBAT_ACTIVE_OBJECT_NO_DESCRIPTOR,
            };
            // `combat.md §5` party descriptor seeding: the flags byte
            // takes "the party-side marker (bit `0x80`)" and "the
            // asleep/magically-disabled bit is additionally set when the
            // status byte is neither `'G'` (good) nor `'P'` (poisoned)".
            let mut flags = COMBAT_ACTOR_FLAG_SELECTABLE_80;
            if !matches!(member.status, b'G' | b'P') {
                flags |= COMBAT_ACTOR_FLAG_STATUS_DISABLED;
            }
            actors[record_slot] = CombatActorDescriptor::from_row([
                0,
                base_step,
                flags,
                // Owner/target/class is "the character's roster slot
                // index"; the link byte is "the allocated
                // combat-instance active-object index". With packing
                // these two are no longer the same number.
                roster_slot as u8,
                record_slot as u8,
                // `combat.md §5`: "Phase counter | Thirty-six minus the
                // base-step."
                COMBAT_PLACEMENT_PHASE_BASE.saturating_sub(base_step),
                x,
                y,
            ]);
        }
        if cleared_active_player {
            self.active_player = None;
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
                // `encounters.md §5`: the second roll draws over the
                // FIRST ROLL'S RESULT, so the damper "can only *lower*
                // the count. It is a damper, not a doubler."
                let mut rolled = self.random_range_u8(1, max);
                if self.fortunes_of_war != 0 {
                    rolled = self.random_range_u8(1, rolled);
                }
                rolled
            }
        };
        count.min(COMBAT_SPAWN_COUNT_CAP)
    }

    pub fn terrain_combat_replacement_roll_seeds(
        &mut self,
        requested_count: u8,
        companion_class: Option<u8>,
    ) -> Vec<u8> {
        (0..requested_count)
            .map(|spawn| {
                self.terrain_combat_replacement_roll_seed(spawn, requested_count, companion_class)
            })
            .collect()
    }

    /// `combat.md §5` "Picking a class per monster": one spawn index's
    /// one-in-nine companion check. Spawn zero and every index at or above
    /// the `count / 4 + 1` threshold never roll, and consume no draw.
    pub fn terrain_combat_replacement_roll_seed(
        &mut self,
        spawn: u8,
        requested_count: u8,
        companion_class: Option<u8>,
    ) -> u8 {
        let threshold = terrain_combat_replacement_threshold(requested_count);
        if companion_class.is_some() && spawn != 0 && spawn < threshold {
            self.random_mod_u8(TERRAIN_COMBAT_REPLACEMENT_DENOMINATOR)
        } else {
            1
        }
    }

    /// `combat.md §5` step 5 draw order: each monster is created in
    /// placement order, so its class check (when it rolls one at all) and
    /// the one speed-variation draw its placement consumes are taken back
    /// to back rather than in two separate sweeps. Returns the per-spawn
    /// class-replacement seeds and the per-spawn speed-variation seeds.
    pub fn terrain_combat_placement_roll_seeds(
        &mut self,
        requested_count: u8,
        companion_class: Option<u8>,
    ) -> (Vec<u8>, Vec<u8>) {
        let mut replacement_roll_seeds = Vec::with_capacity(usize::from(requested_count));
        let mut speed_adjust_roll_seeds = Vec::with_capacity(usize::from(requested_count));
        for spawn in 0..requested_count {
            replacement_roll_seeds.push(self.terrain_combat_replacement_roll_seed(
                spawn,
                requested_count,
                companion_class,
            ));
            speed_adjust_roll_seeds.push(self.combat_placement_speed_adjust_roll());
        }
        (replacement_roll_seeds, speed_adjust_roll_seeds)
    }
}

#[cfg(test)]
mod combat_setup_batch_tests {
    use super::*;
    use crate::test_fixtures::{debug_game_dir, open_world_grid, world_state};
    use std::fs;

    /// A `BRIT.CBT` record with distinguishable party-entry and
    /// placement-slot metadata: party seats at `(0xA0+i, 0xB0+i)` and the
    /// sixteen monster placement slots at `(i, 15 - i)`.
    fn batch_arena_record() -> Vec<u8> {
        let mut record = vec![0u8; COMBAT_ARENA_RECORD_LEN];
        for row in 0..COMBAT_ARENA_SIDE {
            let row_start = row * COMBAT_ARENA_ROW_STRIDE;
            for x in 0..COMBAT_ARENA_SIDE {
                record[row_start + x] = (row as u8) * 16 + x as u8;
            }
        }
        for index in 0..COMBAT_PARTY_ACTOR_SLOTS {
            record[3 * COMBAT_ARENA_ROW_STRIDE + 11 + index] = 0xa0 + index as u8;
            record[3 * COMBAT_ARENA_ROW_STRIDE + 17 + index] = 0xb0 + index as u8;
        }
        for index in 0..CBT_PLACEMENT_SLOT_COUNT {
            record[6 * COMBAT_ARENA_ROW_STRIDE + 11 + index] = index as u8;
            record[7 * COMBAT_ARENA_ROW_STRIDE + 11 + index] = 15 - index as u8;
        }
        record
    }

    fn batch_party_member(slot: u8, status: u8) -> PartyMember {
        PartyMember {
            slot,
            class_byte: b'F',
            status,
            climb_stat: DEFAULT_CLIMB_STAT,
            mana: 0,
            hp: if status == b'D' { 0 } else { 20 },
            max_hp: 20,
            level: 3,
        }
    }

    fn batch_combat_state(statuses: &[u8]) -> (PlayState, std::path::PathBuf) {
        let dir = debug_game_dir();
        fs::write(
            dir.join(BRIT_CBT_FILE),
            batch_arena_record().repeat(BRIT_CBT_RECORDS),
        )
        .unwrap();
        let mut state = world_state(open_world_grid(), 5, 5);
        let len = statuses.len();
        state.party = statuses
            .iter()
            .enumerate()
            .map(|(slot, status)| batch_party_member(slot as u8, *status))
            .collect();
        state.party_names = default_party_names(len);
        state.party_experience = default_party_experience(len);
        state.party_stay_counters = default_party_stay_counters(len);
        state.party_strengths = default_party_strengths(len);
        state.party_intelligence = default_party_intelligence(len);
        state.party_equipment = default_party_equipment(len);
        state.party_roster = default_party_roster(len);
        (state, dir)
    }

    fn batch_trigger() -> ActiveObject {
        ActiveObject {
            type_byte: 0xc0,
            tile: 0xc0,
            x: 6,
            y: 5,
            z: WorldPlane::Britannia.save_floor(),
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        }
    }

    /// A `batch_arena_record` variant with in-arena party seats, so the
    /// seated members can be looked up through the renderer.
    fn seated_arena_record(seats: &[(u8, u8)]) -> Vec<u8> {
        let mut record = batch_arena_record();
        for (slot, (x, y)) in seats.iter().copied().enumerate() {
            record[3 * COMBAT_ARENA_ROW_STRIDE + 11 + slot] = x;
            record[3 * COMBAT_ARENA_ROW_STRIDE + 17 + slot] = y;
        }
        record
    }

    /// `combat.md §5` "Seating the party": every party slot that is not
    /// `'D'` is "placed at arena `(X, Y)` taken from the selected arena's
    /// party entry coordinate slices, indexed by *party slot*", and §5.4
    /// adds that placement runs no terrain validation, so no seat is ever
    /// dropped or relocated. A play-test reported only two of three party
    /// members reaching the arena; this pins one rendered sprite per
    /// living member, each on its own authored seat.
    #[test]
    fn every_living_party_member_reaches_its_own_authored_arena_seat() {
        // The seat rows the shipped forest/hill outdoor arenas use.
        let seats = [(5u8, 8u8), (6, 9), (4, 9), (5, 10), (7, 10), (3, 10)];
        let (mut state, dir) = batch_combat_state(&[b'G', b'G', b'G']);
        fs::write(
            dir.join(BRIT_CBT_FILE),
            seated_arena_record(&seats).repeat(BRIT_CBT_RECORDS),
        )
        .unwrap();

        state
            .enter_terrain_combat_from_world_object(&dir, WorldPlane::Britannia, 1, batch_trigger())
            .unwrap();

        for (roster_slot, (x, y)) in seats.iter().copied().take(3).enumerate() {
            let descriptor = state
                .combat_actors
                .iter()
                .find(|actor| {
                    actor.flags & COMBAT_ACTOR_FLAG_SELECTABLE_80 != 0
                        && usize::from(actor.owner_target_class) == roster_slot
                })
                .copied()
                .unwrap_or_else(|| panic!("roster slot {roster_slot} has no party descriptor"));
            assert_eq!(
                (descriptor.x, descriptor.y),
                (x, y),
                "roster slot {roster_slot} seat"
            );
            assert_eq!(
                state.combat_render_actor_byte_at(usize::from(x), usize::from(y)),
                Some(combat_party_actor_byte(state.party[roster_slot].class_byte)),
                "roster slot {roster_slot} must render on its own seat"
            );
        }

        let rendered_party_cells = (0..COMBAT_ARENA_SIDE)
            .flat_map(|y| (0..COMBAT_ARENA_SIDE).map(move |x| (x, y)))
            .filter(|(x, y)| {
                state
                    .combat_render_actor_byte_at(*x, *y)
                    .is_some_and(|byte| byte & 0xf0 == 0x40)
            })
            .count();
        assert_eq!(rendered_party_cells, 3);
        let _ = fs::remove_dir_all(dir);
    }

    /// `active-objects.md §9`: the party seating writes "arena
    /// coordinates over the low records, record zero included", and
    /// "record zero is overwritten because it is the first record the
    /// party seating allocates, not because combat reserves a player
    /// slot"; the framer restores the world value on combat exit. §5
    /// scopes the world compositor's slot-zero refresh to "every
    /// **world** frame", so nothing may refresh record zero from the
    /// world globals while combat is live. This is play-test defect 18:
    /// the refresh used to fire on the first combat turn and erase the
    /// first party member's arena tile and coordinates.
    #[test]
    fn a_combat_turn_does_not_refresh_record_zero_from_the_world_globals() {
        let seats = [(5u8, 8u8), (6, 9), (4, 9), (5, 10), (7, 10), (3, 10)];
        let (mut state, dir) = batch_combat_state(&[b'G', b'G', b'G']);
        fs::write(
            dir.join(BRIT_CBT_FILE),
            seated_arena_record(&seats).repeat(BRIT_CBT_RECORDS),
        )
        .unwrap();

        state
            .enter_terrain_combat_from_world_object(&dir, WorldPlane::Britannia, 1, batch_trigger())
            .unwrap();
        let seated_record_zero = state.active_objects[0];
        assert_eq!(
            (seated_record_zero.x, seated_record_zero.y),
            (usize::from(seats[0].0), usize::from(seats[0].1))
        );

        state.sync_player_object();

        assert_eq!(state.active_objects[0], seated_record_zero);
        for (roster_slot, (x, y)) in seats.iter().copied().take(3).enumerate() {
            assert_eq!(
                state.combat_render_actor_byte_at(usize::from(x), usize::from(y)),
                Some(combat_party_actor_byte(state.party[roster_slot].class_byte)),
                "roster slot {roster_slot} must still render on its own seat"
            );
        }
        let _ = fs::remove_dir_all(dir);
    }

    /// `encounters.md §4`: the base class id "drives the plural encounter
    /// banner", and `combat.md §5.3` groups the encounter-name print with
    /// the banner. The plural table is unpublished, so a class whose
    /// plural string has not been observed prints the banner alone rather
    /// than an invented name; the trigger here is an Orc (class 32), one
    /// of those.
    ///
    /// The blank row is `text-output.md §10.4`'s derived blank and the
    /// centring is `text-output.md §3`'s centre flag.
    #[test]
    fn combat_entry_prints_the_conflict_banner_under_one_blank_row() {
        let (mut state, dir) = batch_combat_state(&[b'G']);

        state
            .enter_terrain_combat_from_world_object(&dir, WorldPlane::Britannia, 1, batch_trigger())
            .unwrap();

        let tail: Vec<(String, bool, bool)> = state
            .message_entries()
            .iter()
            .rev()
            .take(2)
            .rev()
            .map(|entry| (entry.text.clone(), entry.centered, entry.explicit_blank))
            .collect();
        assert_eq!(
            tail,
            vec![
                (String::new(), false, true),
                (combat_banner_line(), true, false),
            ]
        );
        assert!(
            state
                .message_entries()
                .iter()
                .all(|entry| entry.text != "ORCS"),
            "no plural name may be synthesised for an unpublished class"
        );
        let _ = fs::remove_dir_all(dir);
    }

    /// The one plural encounter-banner string this project has observed:
    /// the bat encounter prints `BATS` centred one blank row above the
    /// banner. Read cell by cell out of a capture of the original's own
    /// combat entry and matched against the shipped `IBM.CH`.
    #[test]
    fn a_bat_encounter_prints_the_observed_plural_group_name_above_the_banner() {
        let (mut state, dir) = batch_combat_state(&[b'G']);
        let mut trigger = batch_trigger();
        // `encounters.md §4`: `class = (sprite_byte - 0x40) / 4`, so
        // `0x94` is class 21, Bat.
        trigger.type_byte = 0x94;
        trigger.tile = 0x94;

        state
            .enter_terrain_combat_from_world_object(&dir, WorldPlane::Britannia, 1, trigger)
            .unwrap();

        let tail: Vec<(String, bool, bool)> = state
            .message_entries()
            .iter()
            .rev()
            .take(4)
            .rev()
            .map(|entry| (entry.text.clone(), entry.centered, entry.explicit_blank))
            .collect();
        assert_eq!(
            tail,
            vec![
                (String::new(), false, true),
                ("BATS".to_string(), true, false),
                (String::new(), false, true),
                (combat_banner_line(), true, false),
            ]
        );
        let _ = fs::remove_dir_all(dir);
    }

    /// The plural encounter-banner table is unpublished
    /// (`catalogs/monster-bestiary.md §2` names only that it exists), so
    /// exactly one class - the observed bat - has a name here, and no
    /// class name is ever transformed into one. In particular the engine
    /// catalog's own descriptive labels, such as class 7's
    /// "Bard (second row)", must never reach the screen.
    #[test]
    fn only_the_observed_plural_encounter_banner_name_is_published_to_the_screen() {
        assert_eq!(outdoor_combat_plural_banner_name(0x94), Some("BATS"));
        assert_eq!(combat_class_plural_banner_name(21), Some("BATS"));
        for class in 0..=47u8 {
            if class == 21 {
                continue;
            }
            assert_eq!(
                combat_class_plural_banner_name(class),
                None,
                "class {class} has no published plural banner name"
            );
        }
        // `encounters.md §4`'s ship family: the fixed pirate plural
        // literal exists but its spelling is unpublished and unobserved.
        assert_eq!(outdoor_combat_plural_banner_name(0x2c), None);
    }

    /// The combat banner row of the original's capture decodes, cell by
    /// cell against the shipped `IBM.CH`, to sixteen characters:
    /// `*** CONFLICT ***`, the flank cell matching font slot `0x2A`
    /// uniquely.
    #[test]
    fn combat_banner_line_is_the_sixteen_cell_asterisk_flanked_literal() {
        assert_eq!(combat_banner_line(), "*** CONFLICT ***");
        assert_eq!(combat_banner_line().len(), 16);
        assert_eq!(COMBAT_BANNER_FLANK_GLYPH, '*');
    }

    /// `combat.md §5`: "A short combat banner ("CONFLICT") is printed at
    /// the start of setup, before any monsters are placed." The banner is
    /// a produced line, not a parked slot value: every production caller
    /// of terrain-combat entry overwrites `message` with its own
    /// diagnostic before the next flush boundary, so the banner has to
    /// reach the transcript at the moment setup produces it.
    #[test]
    fn combat_banner_reaches_the_transcript_before_a_later_writer_takes_the_slot() {
        let (mut state, dir) = batch_combat_state(&[b'G']);

        state
            .enter_terrain_combat_from_world_object(&dir, WorldPlane::Britannia, 1, batch_trigger())
            .unwrap();
        // Stand in for the production callers, which all assign the slot
        // directly right after this call returns.
        state.message = "entered terrain combat using BRIT.CBT arena 1".to_string();

        assert!(
            state
                .message_entries()
                .iter()
                .any(|entry| entry.text == combat_banner_line()),
            "combat banner missing from transcript: {:?}",
            state
                .message_entries()
                .iter()
                .map(|entry| entry.text.clone())
                .collect::<Vec<_>>()
        );
        let _ = fs::remove_dir_all(dir);
    }

    /// `active-objects.md §7`: "descriptor six pairs with the first
    /// active-object record left free by the seated party, so a party of
    /// four puts the first monster at descriptor six and active-object
    /// record four. The descriptor's active-object link byte is the
    /// authoritative pairing."
    #[test]
    fn monster_descriptors_start_at_six_while_records_continue_from_the_seated_party() {
        let (mut state, dir) = batch_combat_state(&[b'G', b'G', b'G', b'G']);

        state
            .enter_terrain_combat_from_world_object(&dir, WorldPlane::Britannia, 1, batch_trigger())
            .unwrap();

        let first_monster = state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS];
        assert!(
            !first_monster.is_empty(),
            "no monster descriptor at index six"
        );
        // A live party of four leaves record four free.
        assert_eq!(first_monster.active_object_slot, 4);
        let linked = state.active_objects[usize::from(first_monster.active_object_slot)];
        assert_eq!(linked.tile, 0xc0);
        assert_eq!(
            (linked.x, linked.y),
            (usize::from(first_monster.x), usize::from(first_monster.y))
        );
        // Descriptor index six is *not* the record index, and record six
        // is untouched when only one monster is placed at record four.
        assert_ne!(
            usize::from(first_monster.active_object_slot),
            COMBAT_PARTY_ACTOR_SLOTS
        );
        let _ = fs::remove_dir_all(dir);
    }

    /// Distinguishable seats so a seat read by the packed descriptor
    /// index instead of by the roster index shows up as a wrong number.
    const BATCH_SEATS: [(u8, u8); COMBAT_PARTY_ACTOR_SLOTS] =
        [(0, 5), (1, 6), (2, 7), (3, 8), (4, 9), (5, 10)];

    fn seat_batch_party(
        state: &mut PlayState,
    ) -> (
        Vec<ActiveObject>,
        [CombatActorDescriptor; COMBAT_ACTOR_SLOTS],
    ) {
        let mut active_objects = vec![ActiveObject::empty(); COMBAT_ACTOR_SLOTS];
        let mut actors = [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS];
        state.populate_combat_party_with_positions(
            &mut active_objects,
            &mut actors,
            0,
            &BATCH_SEATS,
        );
        (active_objects, actors)
    }

    /// `active-objects.md §7`: party members are "allocated the first
    /// free records, one per live (non-dead) member in roster order ...
    /// a party with dead members packs into fewer". `combat.md §5`: the
    /// dead member is "skipped entirely", "the remaining members
    /// therefore pack into the low descriptor indexes rather than
    /// keeping their roster index", each seat is still taken from the
    /// arena entry table "indexed by *party slot* (the roster index, not
    /// the packed descriptor index)", and the owner/target/class field
    /// is seeded with "the character's roster slot index".
    #[test]
    fn a_dead_member_packs_the_survivors_into_the_low_descriptor_indexes() {
        let (mut state, dir) = batch_combat_state(&[b'G', b'D', b'G', b'G']);
        let (active_objects, actors) = seat_batch_party(&mut state);

        // Three survivors, packed into descriptors zero through two.
        assert_eq!(actors[0].owner_target_class, 0);
        assert_eq!(actors[1].owner_target_class, 2);
        assert_eq!(actors[2].owner_target_class, 3);
        assert!(
            actors[3].is_empty(),
            "a party of four with one dead member must occupy three descriptors, not four"
        );
        assert_eq!(active_objects[3].tile, 0);

        // The two scans run in lockstep for party members, so the link
        // byte equals the descriptor index.
        for packed in 0..3 {
            assert_eq!(usize::from(actors[packed].active_object_slot), packed);
        }

        // Seats and the record's roster byte still follow the roster
        // index, so the third roster member sits at seat two.
        assert_eq!((actors[1].x, actors[1].y), BATCH_SEATS[2]);
        assert_eq!(
            (active_objects[1].x, active_objects[1].y),
            (usize::from(BATCH_SEATS[2].0), usize::from(BATCH_SEATS[2].1))
        );
        assert_eq!(active_objects[1].aux1, 2);
        let _ = fs::remove_dir_all(dir);
    }

    /// `combat.md §5`: experience, names and equipment are reached
    /// through the descriptor's owner/target/class byte - "the
    /// character's roster slot index" - so a packed roster credits the
    /// character the descriptor names, not the one at the descriptor's
    /// own index.
    #[test]
    fn packed_experience_and_names_follow_the_descriptor_owner_byte() {
        let (mut state, dir) = batch_combat_state(&[b'G', b'D', b'G', b'G']);
        state.party_names[2][..5].copy_from_slice(b"Iolo\0");
        state.party_names[1][..5].copy_from_slice(b"Gwen\0");
        let (active_objects, actors) = seat_batch_party(&mut state);
        state.active_objects = active_objects;
        state.combat_actors = actors;

        assert_eq!(
            state.credit_combat_party_attacker_experience(1, 10),
            Some(10),
            "descriptor one is the third roster member and is alive"
        );
        assert_eq!(state.party_experience[2], 10);
        assert_eq!(state.party_experience[1], 0);
        assert_eq!(state.party_experience[0], 0);

        assert_eq!(
            state.combat_charm_target_display_name(1).as_deref(),
            Some("Iolo")
        );
        let _ = fs::remove_dir_all(dir);
    }

    /// `combat.md §5`: "A member whose status byte is `'S'` (asleep) is
    /// seated and then immediately marked asleep: status stays `'S'`,
    /// the descriptor's disabled bit is set, the presentation record
    /// shows the prone marker, and the active-player sentinel is cleared
    /// if that member was the active player." Only `'D'` skips seating.
    #[test]
    fn an_asleep_member_is_seated_disabled_prone_and_clears_the_active_player() {
        let (mut state, dir) = batch_combat_state(&[b'G', b'S', b'G']);
        state.active_player = Some(1);
        let (active_objects, actors) = seat_batch_party(&mut state);

        // Seated, not skipped: three members occupy three descriptors
        // and the asleep member keeps its own roster index and seat.
        assert_eq!(actors[1].owner_target_class, 1);
        assert_eq!(actors[2].owner_target_class, 2);
        assert_eq!((actors[1].x, actors[1].y), BATCH_SEATS[1]);

        assert!(
            actors[1].is_status_disabled(),
            "an asleep member's descriptor must carry the disabled bit"
        );
        assert_eq!(
            active_objects[1].tile, COMBAT_POTION_SLEEP_DISPLAY_TILE,
            "the presentation record must show the prone marker"
        );
        assert_eq!(
            active_objects[1].type_byte,
            combat_party_actor_byte(b'F'),
            "the base/type byte keeps the class sprite so the wake path can restore it"
        );
        assert_eq!(state.party[1].status, b'S', "status stays 'S'");
        assert_eq!(state.active_player, None);

        // A good member is neither disabled nor prone.
        assert!(!actors[0].is_status_disabled());
        assert_eq!(active_objects[0].tile, combat_party_actor_byte(b'F'));
        let _ = fs::remove_dir_all(dir);
    }

    /// `combat.md §5`: with a packed party the descriptor index is no
    /// longer the roster index, so an effect aimed at a descriptor has
    /// to reach the character that descriptor's owner/target/class byte
    /// names. This is the shared route every converted reader takes -
    /// [`PlayState::combat_roster_slot_for_actor_slot`].
    #[test]
    fn packed_party_damage_wounds_the_character_the_descriptor_names() {
        let (mut state, dir) = batch_combat_state(&[b'G', b'D', b'G', b'G']);
        let (active_objects, actors) = seat_batch_party(&mut state);
        state.active_objects = active_objects;
        state.combat_actors = actors;

        // Descriptor one is the packed seat of roster slot two.
        assert_eq!(state.combat_roster_slot_for_actor_slot(1), Some(2));
        assert_eq!(state.combat_roster_slot_for_actor_slot(2), Some(3));
        // An unseated index has a zero flags byte and so no roster field
        // to read; `§5` calls such a descriptor free.
        assert_eq!(state.combat_roster_slot_for_actor_slot(3), Some(3));
        assert_eq!(
            state.combat_roster_slot_for_actor_slot(COMBAT_PARTY_ACTOR_SLOTS),
            None
        );

        state
            .apply_combat_weapon_damage_to_target(None, 1, 5, false)
            .expect("descriptor one is a seated party member");
        assert_eq!(state.party[2].hp, 15);
        assert_eq!(state.party[0].hp, 20);
        assert_eq!(state.party[3].hp, 20);
        let _ = fs::remove_dir_all(dir);
    }

    /// `combat.md §5`: the active-player sentinel names a roster slot,
    /// so with a packed party the combat cursor has to find the
    /// descriptor whose owner/target/class byte names that character
    /// instead of indexing the descriptor table by the sentinel.
    #[test]
    fn the_combat_cursor_finds_the_packed_descriptor_of_the_active_player() {
        let (mut state, dir) = batch_combat_state(&[b'G', b'D', b'G', b'G']);
        let (active_objects, actors) = seat_batch_party(&mut state);
        state.active_objects = active_objects;
        state.combat_actors = actors;
        state.active_player = Some(3);

        assert_eq!(
            state.combat_party_descriptor_slot_for_roster_slot(3),
            Some(2)
        );
        assert_eq!(state.combat_cursor_actor_cell(), Some(BATCH_SEATS[3]));
        let _ = fs::remove_dir_all(dir);
    }

    /// `combat.md §7`: the cursor box is drawn "around the eligible active
    /// player's arena cell", and §8 prompts one combatant at a time, so the
    /// selector the box reads has to follow the actor the round walk parked
    /// on rather than whichever member was selected outside the fight.
    #[test]
    fn the_pending_combat_actor_becomes_the_active_player() {
        let (mut state, dir) = batch_combat_state(&[b'G', b'D', b'G', b'G']);
        let (active_objects, actors) = seat_batch_party(&mut state);
        state.active_objects = active_objects;
        state.combat_actors = actors;
        state.active_player = Some(0);

        // Descriptor two carries roster slot three in a party packed by the
        // dead member at roster slot one.
        state.pending_combat_actor_slot = Some(2);
        state.select_active_player_for_pending_combat_actor();

        assert_eq!(state.active_player, Some(3));
        assert_eq!(state.combat_cursor_actor_cell(), Some(BATCH_SEATS[3]));

        // With nobody parked on, the selector is left where it stands.
        state.pending_combat_actor_slot = None;
        state.select_active_player_for_pending_combat_actor();

        assert_eq!(state.active_player, Some(3));
        let _ = fs::remove_dir_all(dir);
    }
}
