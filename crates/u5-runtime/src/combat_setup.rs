//! Combat setup helpers that bridge encounters, arena records, and class data.

use std::io;
use std::path::Path;

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

/// `catalogs/monster-bestiary.md §2.2` group banner names: "A second
/// forty-eight-entry resident table, parallel to the stat rows and to
/// the singular name table and indexed by the same class id, holds the
/// **group banner name** - the caption printed when a terrain fight
/// begins".
///
/// "It is a shipped table of finished strings. There is no suffix rule
/// to derive it from: nothing appends an `S`, and the banner never
/// consults the monster count, so the group form is printed even for a
/// single attacker." Twenty-two entries happen to be the singular name
/// uppercased with an `S`; the other twenty-six are not, so the table
/// is shipped verbatim.
///
/// The six `x` entries (classes 3, 9, 13, 29, 42, 43) are the shipped
/// one-character placeholders and are **not** a cue to fall back on the
/// singular name: "An engine that falls back to the singular name when
/// the group entry looks empty will print `Avatar`, `Wanderer` or
/// `Crawler` where the original prints a single `x`."
pub const COMBAT_CLASS_GROUP_BANNER_NAMES: [&str; COMBAT_CLASS_COUNT] = [
    "WIZARDS",      // 0 Mage
    "BARD",         // 1 Bard
    "FIGHTER",      // 2 Fighter
    "x",            // 3 Avatar - shipped placeholder
    "VILLAGER",     // 4 Villager
    "MERCHANT",     // 5 Merchant
    "JESTER",       // 6 Jester
    "BARD",         // 7 Bard (second row)
    "PIRATES",      // 8 Pirate - no singular counterpart
    "x",            // 9 Unnamed reserved - shipped placeholder
    "CHILD",        // 10 Child
    "BEGGAR",       // 11 Beggar
    "GUARDS",       // 12 Guard
    "x",            // 13 Wanderer - shipped placeholder
    "BLACKTHORN",   // 14 Blackthorn - proper noun
    "LORD BRITISH", // 15 Lord British - proper noun, the longest entry
    "SEA HORSES",   // 16 Sea Horse
    "SQUIDS",       // 17 Squid
    "SEA SERPENTS", // 18 Sea Serpent
    "SHARKS",       // 19 Shark
    "GIANT RATS",   // 20 Giant Rat
    "BATS",         // 21 Bat
    "SPIDERS",      // 22 Giant Spider - different word
    "GHOSTS",       // 23 Ghost
    "SLIME",        // 24 Slime - singular form
    "GREMLINS",     // 25 Gremlin
    "MIMICS",       // 26 Mimic
    "REAPERS",      // 27 Reaper
    "GAZERS",       // 28 Gazer
    "x",            // 29 Crawler - shipped placeholder
    "GARGOYLE",     // 30 Gargoyle - singular form
    "INSECTS",      // 31 Insect Swarm - different word
    "ORCS",         // 32 Orc
    "SKELETONS",    // 33 Skeleton
    "SNAKES",       // 34 Python - different word
    "ETTINS",       // 35 Ettin
    "HEADLESSES",   // 36 Headless - irregular plural
    "WISPS",        // 37 Wisp
    "DAEMONS",      // 38 Daemon
    "DRAGONS",      // 39 Dragon
    "SAND TRAPS",   // 40 Sand Trap
    "TROLLS",       // 41 Troll
    "x",            // 42 Reserved gap - shipped placeholder
    "x",            // 43 Reserved gap - shipped placeholder
    "MONGBATS",     // 44 Mongbat
    "CORPSERS",     // 45 Corpser
    "ROTWORMS",     // 46 Rot Worm - different word, no space
    "SHADOW LORD",  // 47 Shadow Lord - singular form
];

/// `encounters.md §4`: the encounter's base class id "is what drives the
/// group-name encounter banner", so combat entry prints one group name
/// above the conflict banner.
///
/// `catalogs/monster-bestiary.md §2.2` publishes the whole forty-eight
/// entry table, so no name is derived and none is withheld. `combat.md
/// §4.1`: "The banner is count-independent ... a lone attacker still
/// gets the group name: one bat announces `BATS`. There is no singular
/// form of this banner anywhere in the game." The Shadow Lord fight,
/// always a single opponent, announces `SHADOW LORD` - "No article, no
/// "The", no separate singular caption."
pub fn combat_class_group_banner_name(class: u8) -> Option<&'static str> {
    COMBAT_CLASS_GROUP_BANNER_NAMES
        .get(usize::from(class))
        .copied()
}

/// `encounters.md §4` group-banner fallback literal: a hostile whose
/// masked sprite byte is below `0x40` "never indexes it and prints the
/// fixed literal `PIRATES` - seven characters, uppercase, no punctuation
/// and no line feed of its own".
pub const COMBAT_GROUP_BANNER_PIRATE_LITERAL: &str = "PIRATES";

/// The group encounter-banner line for one outdoor hostile sprite byte.
///
/// `encounters.md §4`, "The banner fallback is a range test, not a ship
/// test": "The banner is chosen before the class table is consulted: if
/// the masked sprite byte is **below `0x40`** the banner code prints the
/// fixed literal [`COMBAT_GROUP_BANNER_PIRATE_LITERAL`] ... and never
/// touches the group-name table. Implement the guard, not the instance."
/// The reason is arithmetic - `(masked - 0x40) / 4` would go negative -
/// and the ship family still takes combat class 1 for its stats, "so the
/// banner and the stat row disagree by design: a boarded ship announces
/// `PIRATES` and fights with the Bard row."
///
/// *(Corrected: an earlier revision of this engine tied the case to a
/// masked sprite byte of `0x2C` and withheld the literal as unpublished.
/// `RETRACTIONS.md` R350.)*
pub fn outdoor_combat_group_banner_name(type_byte: u8) -> Option<&'static str> {
    if type_byte & 0xfc < OUTDOOR_COMBAT_TYPE_FIRST {
        return Some(COMBAT_GROUP_BANNER_PIRATE_LITERAL);
    }
    outdoor_combat_class_id(type_byte).and_then(combat_class_group_banner_name)
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
/// entry coordinates (`combat.md §5`), never from this list.
///
/// The camp-ambush entry does load an arena record - `combat.md §5`: "The
/// camp ambush loads arena record 0" - but it "skips the pre-placement
/// pass that clears the combat tables and seats the party from the arena
/// record's party-seat rows", and "**where the party's arena coordinates
/// come from on that route is not established** [...] do not assume the
/// arena-record party seats apply there". This list is the engine's
/// conservative stand-in until that is settled.
pub const TERRAIN_COMBAT_PARTY_POSITIONS: [(u8, u8); COMBAT_PARTY_ACTOR_SLOTS] =
    [(5, 5), (4, 5), (6, 5), (5, 4), (5, 6), (4, 6)];

/// `combat.md §5`: "The camp ambush loads arena record 0, whose sixteen
/// monster cells are all distinct and all on grass, spread around the
/// arena's corners and edges."
pub const CAMP_AMBUSH_ARENA_INDEX: usize = 0;

/// Stand-in placement cells for the camp-ambush entry when no `BRIT.CBT`
/// is available to supply arena record 0's authored sixteen (fixtures and
/// headless harnesses). They are sixteen distinct interior cells on the
/// even grid, so the `combat.md §5` fifteen-transposition shuffle stays
/// observable.
///
/// These are engine scaffolding, **not** published data and not a spec
/// contract, so they are crate-private: no frontend or test outside this
/// crate may treat them as the camp ambush's placement cells. With shipped
/// assets present the route uses arena record 0's authored cells instead.
pub(crate) const SLEEP_AMBUSH_FALLBACK_PLACEMENT_SLOTS: [(u8, u8); DUNGEON_ROOM_SOURCE_COUNT] = [
    (2, 2),
    (4, 2),
    (6, 2),
    (8, 2),
    (2, 4),
    (4, 4),
    (6, 4),
    (8, 4),
    (2, 6),
    (4, 6),
    (6, 6),
    (8, 6),
    (2, 8),
    (4, 8),
    (6, 8),
    (8, 8),
];

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

/// `combat.md §4.1` player-facing conflict banner word. Terrain setup
/// "prints it at the start, before any monster is placed (Section 5,
/// step 3)", after party seating and before the count roll.
pub const COMBAT_BANNER: &str = "CONFLICT";

/// `encounters.md §4` Shadow Lord branch, step 1: "Print exactly
/// `The Sceptre is reclaimed!` followed by one newline. There is no
/// terminating period and no leading blank line."
pub const SCEPTRE_RECLAIMED_LINE: &str = "The Sceptre is reclaimed!";

/// The character the original's conflict banner flanks `CONFLICT` with.
///
/// `combat.md §4.1`: "**The flank glyph is character code `0x2A`**,
/// three per side - the ASCII asterisk code point, **not** `0x2B`
/// (`+`). The distinction is visible: in the 8x8 gameplay font, `0x2A`
/// is drawn as a solid four-pointed diamond that reads as a **bold
/// cross** at cell size, while `0x2B` is a thin two-pixel cross. A
/// transcript that renders this banner with literal `+` characters
/// differs from the original in glyph shape, which is why player-facing
/// transcripts of this line are commonly written `+++ CONFLICT +++`."
///
/// Independently confirmed here: the banner row was decoded cell by cell
/// out of the original's own combat-entry capture and each cell matched
/// against the shipped `IBM.CH`, where the flank bitmap matches slot
/// `0x2A` uniquely.
pub const COMBAT_BANNER_FLANK_GLYPH: char = '*';
/// The published code point behind [`COMBAT_BANNER_FLANK_GLYPH`].
pub const COMBAT_BANNER_FLANK_GLYPH_CODE: u8 = 0x2A;
/// `combat.md §4.1`: three flank glyphs per side.
pub const COMBAT_BANNER_FLANK_GLYPH_COUNT: usize = 3;

/// The complete conflict banner line as the original prints it.
///
/// `combat.md §4.1`: "`*** CONFLICT ***` followed by one line feed.
/// Exactly sixteen printable characters: three flank glyphs, one space,
/// the eight letters of `CONFLICT`, one space, three flank glyphs." It
/// "fills the message window edge to edge, absolute columns 24 through
/// 39, on one row. Sixteen characters is exactly the window's capacity."
/// It "is not centred, and centring would not move it ... a
/// sixteen-character caption in a sixteen-cell window has exactly one
/// centred position, column zero." Its trailing line feed "costs no
/// row", so no blank row appears under the banner.
pub fn combat_banner_line() -> String {
    let flank: String =
        std::iter::repeat_n(COMBAT_BANNER_FLANK_GLYPH, COMBAT_BANNER_FLANK_GLYPH_COUNT).collect();
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
                    phase: COMBAT_PLACEMENT_ACTIVE_OBJECT_PHASE,
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

/// `combat.md §5` "Arena-centre special": the magic-field marker tile the
/// setup pass looks for on the arena's centre cell.
pub const COMBAT_ARENA_CENTRE_SPECIAL_TILE: u8 = 0xDC;

/// `combat.md §5` "Arena-centre special": the setup id the converted centre
/// cell is given.
pub const COMBAT_ARENA_CENTRE_SPECIAL_SETUP_ID: u8 = 1;

/// `combat.md §5` "Arena-centre special": the cell tested, "row five,
/// column five".
pub const COMBAT_ARENA_CENTRE_CELL: (usize, usize) = (5, 5);

/// `combat.md §5`, "Arena-centre special": "If the loaded arena's centre
/// cell (row five, column five) holds terrain byte `0xDC`, the setup pass
/// converts that cell into a special active object with setup id one."
///
/// Three properties are settled there:
///
/// * **The auxiliary-byte rule is setup id one's, and it is draw-free** -
///   "three times the current level index plus seven, computed
///   arithmetically, with no random draw of any kind", i.e.
///   [`DungeonRoomSpecialPostWrite::LevelTimesThreePlusSeven`]. "Do not
///   generalise it to the sibling id: **setup id two draws**".
/// * **The destination is the stamped object's quantity/loot byte** - "not
///   any combat-descriptor field. The object is placed as an ordinary
///   world-object stamp at arena `(5, 5)` ... so it produces a
///   world-object row with **no** combat descriptor: nothing acts on that
///   cell during the round."
/// * **The arm is gated on the centre cell already holding `0xDC`**; "it is
///   not an unconditional conversion of the centre cell".
///
/// *Retracted (`RETRACTIONS.md` R362).* An earlier revision of this comment
/// carried `§5`'s "No shipped outdoor arena carries that tile at that cell,
/// so this is inert for stock `BRIT.CBT` data and is documented only so a
/// custom arena behaves the same way." **The inertness conclusion is
/// withdrawn.** The arena-file half is stronger than before - "**No shipped
/// arena record carries `0xDC` anywhere in its grid**" - but "the
/// qualifying byte is painted at run time rather than loaded from an arena
/// record": the dungeon room painter's underfoot-icon table stamps it at
/// the arena centre for a chest cell, so "the live trigger is ...
/// *dungeon-room combat entered while the party stands on a chest cell*,
/// and the step is **not** inert in stock play". Of the three paths that
/// enter this setup pass "only the dungeon-room path can present a
/// qualifying centre cell". Confidence there is **probable** for the icon
/// class and for the runtime sequencing; **established** for everything
/// this function implements.
///
pub fn combat_arena_centre_special_active_object(
    terrain: &[[u8; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE],
    z: i8,
    prng_state: &mut u16,
) -> Option<ActiveObject> {
    let (row, column) = COMBAT_ARENA_CENTRE_CELL;
    if terrain[row][column] != COMBAT_ARENA_CENTRE_SPECIAL_TILE {
        return None;
    }
    let placement =
        DungeonRoomSpecialPlacement::from_setup_id(COMBAT_ARENA_CENTRE_SPECIAL_SETUP_ID);
    Some(ActiveObject {
        type_byte: placement.setup_id,
        tile: placement.setup_id,
        x: column,
        y: row,
        z,
        phase: STEADY_PHASE,
        aux1: match placement.post_write {
            DungeonRoomSpecialPostWrite::None => placement.setup_id,
            post_write => dungeon_room_special_aux1(post_write, z, prng_state),
        },
        aux3: COMBAT_ACTIVE_OBJECT_NO_DESCRIPTOR,
    })
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
            phase: COMBAT_PLACEMENT_ACTIVE_OBJECT_PHASE,
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
        // `combat.md §5` "Arena-centre special" (`RETRACTIONS.md` R362): the
        // step is wired on all three setup callers. This one loads its arena
        // from `DUNGEON.CBT`, and §5's arena-file negative is **established**
        // - "**No shipped arena record carries `0xDC` anywhere in its
        // grid**" - so no stock record can present a qualifying centre cell
        // here; the byte "is painted at run time", by the painter path in
        // [`Self::enter_dungeon_active_monster_combat`]. A record read from
        // disk also carries no floor-fill byte this engine owns, and §14.1
        // publishes none as a value, so the overwrite argument is `None`
        // rather than a guess.
        let mut setup_terrain = setup.terrain;
        self.place_combat_arena_centre_special(
            &mut setup_terrain,
            None,
            level as i8,
            &mut active_objects,
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
            setup_terrain,
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

    /// `combat.md §5`, the placement-slot shuffle. **It is live.** The
    /// section's retraction is explicit: "Earlier revisions of this section
    /// said the helper's placement-shuffle branch was dormant because 'the
    /// complete caller census has one caller and it always leaves the branch
    /// inactive'. That is withdrawn. There are exactly two routes into the
    /// terrain setup helper and they pass different setup flags" - the
    /// ordinary wilderness or town encounter leaves the shuffle bit clear
    /// and places monsters in identity slot order, while the **surface camp
    /// ambush** (overworld `H` Hole up) "reaches the same setup helper
    /// through its CMDS wrapper, and reaches it **only** with the shuffle
    /// bit set".
    ///
    /// The permutation is *not* a uniform shuffle and must not be replaced
    /// by one: "initialise slots `0..15`, then for each current index
    /// `0..14` draw an independent index from the full inclusive range
    /// `0..15` and swap the two entries. That is fifteen random
    /// transpositions, and it does **not** produce a uniform permutation -
    /// an engine that substitutes a correct Fisher-Yates shuffle will not
    /// reproduce the original's distribution."
    ///
    /// It differs from [`Self::dungeon_ambush_source_permutation`], the room
    /// painter's own sixteen-swap, only by stopping before index fifteen;
    /// §5.3 keeps the two mechanisms in separate scopes.
    ///
    /// `§5.3` step 3a charges this route "exactly fifteen uniform `[0, 15]`
    /// draws, taken after seating and before the banner", and the ordinary
    /// encounter route zero.
    pub fn terrain_combat_placement_slot_permutation(&mut self) -> [u8; DUNGEON_ROOM_SOURCE_COUNT] {
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
        // `combat.md §5` "Arena-centre special" (`RETRACTIONS.md` R362) and
        // `dungeon-mode.md §14.1`, "The centre-icon classes": the painter
        // "selects one byte from a small icon table that is stamped into the
        // arena grid's centre cell", and the chest class "stamps the byte the
        // combat setup pass tests at the arena centre ... That is the only
        // way that byte ever reaches the centre cell - no shipped arena
        // record carries it - so dungeon-room combat entered while the party
        // stands on a chest cell is the sole live trigger for that step."
        //
        // This is the engine's only painter path and its only combat entry
        // whose underfoot cell can be a chest:
        // [`Self::enter_dungeon_room_combat`], the loaded-record route, is
        // reached from a room-helper (`0xA?`) or room-trigger (`0xF?`) cell,
        // never a `0x4?` one. Only the chest class's byte is published,
        // so no other class stamps anything here. Confidence follows §5:
        // **probable** for the icon-class identification.
        let centre_icon = matches!(
            dungeon_cell_class_of(self.dungeon_cell(level, self.player.x, self.player.y)),
            DungeonCellClass::Chest
        )
        .then_some(COMBAT_ARENA_CENTRE_SPECIAL_TILE);
        let record = CombatArenaRecord::synthesise_dungeon_ambush(
            DUNGEON_AMBUSH_ARENA_FLOOR_TILE,
            facing_seed,
            stats.class,
            count,
            permutation,
            centre_icon,
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
        // `combat.md §5` third settled property, confidence **established**:
        // "The step's last act overwrites that centre cell with the room's
        // floor-fill terrain byte, erasing the `0xDC` the room painter had
        // stamped there." This is the path that owns a floor-fill byte -
        // §14.1's painter "fills the eleven-by-eleven terrain grid with the
        // current corridor fill byte", and the synthesis above used exactly
        // this constant - so the overwrite runs here with the caller's own
        // fill rather than a guessed one, and it runs on the path the
        // centre-icon stamp above can actually fire on.
        let mut setup_terrain = setup.terrain;
        self.place_combat_arena_centre_special(
            &mut setup_terrain,
            Some(DUNGEON_AMBUSH_ARENA_FLOOR_TILE),
            level as i8,
            &mut active_objects,
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
            setup_terrain,
        )?;
        Ok(format!(
            "entered dungeon combat against {placed_count} of {count} {} from active monster tile {}",
            stats.name, object.tile
        ))
    }

    /// `combat.md §5`: "The camp ambush loads arena record 0, whose sixteen
    /// monster cells are all distinct and all on grass, spread around the
    /// arena's corners and edges, so the permutation is observable in play
    /// rather than a no-op."
    ///
    /// The same section says the camp route "skips the pre-placement pass
    /// that clears the combat tables and seats the party from the arena
    /// record's party-seat rows", and that "**where the party's arena
    /// coordinates come from on that route is not established** and should
    /// be settled by observation before it is implemented; do not assume the
    /// arena-record party seats apply there". The party therefore keeps the
    /// engine's conservative [`TERRAIN_COMBAT_PARTY_POSITIONS`] seats rather
    /// than adopting arena 0's party-entry rows.
    pub fn enter_sleep_ambush_combat(
        &mut self,
        monster: SleepAmbushMonster,
        z: i8,
        game_dir: &Path,
    ) -> io::Result<String> {
        let tile = sleep_ambush_monster_sprite(monster);
        let stats = combat_class_stats_for_sprite_byte(tile).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("sleep-ambush monster sprite 0x{tile:02X} has no combat class"),
            )
        })?;
        // The arena record is optional here so the entry keeps working for
        // harnesses and fixtures with no `BRIT.CBT` on disk; when it is
        // missing the fallback grid below stands in for the authored cells.
        let record = load_brit_cbt(game_dir)
            .ok()
            .and_then(|bank| bank.record(CAMP_AMBUSH_ARENA_INDEX).cloned());
        let placement_slots: Vec<(u8, u8)> = match record.as_ref() {
            Some(record) => combat_placement_slots_from_record(record)
                .into_iter()
                .map(|slot| (slot.x, slot.y))
                .collect(),
            None => SLEEP_AMBUSH_FALLBACK_PLACEMENT_SLOTS.to_vec(),
        };
        let mut active_objects = vec![ActiveObject::empty(); OOL_SLOTS];
        let mut actors = [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS];
        self.populate_terrain_combat_party(&mut active_objects, &mut actors, z);

        // `combat.md §5.3` step 3a: the surface camp ambush "sets and
        // forwards the [shuffle] bit ... and draws exactly fifteen uniform
        // `[0, 15]` draws, taken after seating and before the banner".
        //
        // OPEN SPEC QUESTION - do not pin this gate with a test. §5.3 names
        // the *surface* camp ambush and says the row "does not cover the
        // dungeon entries", which reads as identity order underground; but
        // `rest-and-camp.md §6` describes both rest interruptions as reaching
        // the same CMDS `H` Hole-up alternate setup, and `combat.md §5` says
        // the terrain setup helper's camp-ambush caller reaches it "only with
        // the shuffle bit set" - which reads as shuffling on both surfaces.
        // §5.3's exclusion most plausibly names the dungeon entries that go
        // through the room painter and never reach this helper at all. The
        // surface-only gate below is the conservative stand-in until the spec
        // settles it; it is deliberately left untested so that adopting the
        // other reading is a one-line change, not a test rewrite.
        let surface_camp_ambush = matches!(self.area, Area::World { .. });
        let permutation =
            surface_camp_ambush.then(|| self.terrain_combat_placement_slot_permutation());

        // `combat.md` §4.1: the conflict banner "is **unconditional**. The
        // test that precedes it cannot fail, so every terrain-setup entry
        // prints it." The published exception list for the group name names
        // exactly one entry that reaches terrain setup without the
        // world-side entry step - "the **surface** camp ambush, which
        // reaches terrain setup through its command-overlay wrapper" - and
        // that entry "gets the conflict banner but **no** group name".
        //
        // Gated on the same surface test as the shuffle above, and for the
        // same reason: §4.1 and §5 both say *surface* camp ambush, and §5
        // puts dungeon fights on the room-combat setup helper, "a separate
        // mechanism on a different setup target [that is] outside this
        // contract". Nothing published says a dungeon rest interruption
        // reaches terrain setup and prints this banner, so it is not printed
        // there - see the open question recorded on the shuffle gate, which
        // this shares verbatim. Left untested underground for the same
        // reason.
        //
        // Ordered here rather than earlier because §5.3 step 3a puts the
        // camp route's fifteen shuffle draws "after seating and before the
        // banner".
        if surface_camp_ambush {
            self.emit_centered_message_line(combat_banner_line());
            self.combat_transcript_row_open = false;
        }

        let requested_count =
            self.roll_terrain_combat_setup_count(stats.default_spawn_count, false);
        // `combat.md §5` reachable-count invariant: "a conforming engine may
        // treat the sixteen placement slots as sufficient for every terrain
        // encounter".
        let placement_count = requested_count
            .min((OOL_SLOTS - COMBAT_PARTY_ACTOR_SLOTS) as u8)
            .min((COMBAT_ACTOR_SLOTS - COMBAT_PARTY_ACTOR_SLOTS) as u8)
            .min(placement_slots.len() as u8);
        let mut placed = 0u8;
        for spawn in 0..placement_count {
            // `active-objects.md §7`: descriptor from the monster-side
            // scan at index six, record from the lowest free record left
            // by the seated party; the link byte pairs them.
            let descriptor_slot = COMBAT_PARTY_ACTOR_SLOTS + usize::from(spawn);
            let Some(record_slot) = first_free_combat_active_object_record(&active_objects) else {
                break;
            };
            // `combat.md §5`: "With `N` monsters the permuted order makes
            // them occupy a random `N`-subset of the sixteen authored cells
            // in a random order, rather than the first `N`."
            let placement_slot = permutation
                .map(|permutation| usize::from(permutation[usize::from(spawn)]))
                .unwrap_or(usize::from(spawn));
            let (x, y) = placement_slots[placement_slot.min(placement_slots.len() - 1)];
            active_objects[record_slot] = ActiveObject {
                type_byte: tile,
                tile,
                x: usize::from(x),
                y: usize::from(y),
                z,
                phase: COMBAT_PLACEMENT_ACTIVE_OBJECT_PHASE,
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

        match record.as_ref() {
            Some(record) => {
                self.enter_combat_frame_with_terrain(
                    active_objects,
                    actors,
                    record.terrain_grid(),
                )?;
            }
            None => {
                self.enter_combat_frame(active_objects, actors)?;
            }
        }
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
        // reclaims it ... the branch runs entirely inside encounter setup,
        // before the combat scene is entered."
        //
        // Only the *test* runs here. The three published consequences are
        // ordered against the banner and are performed below: "the line
        // completes before the sting starts, and the flag clears last."
        let sceptre_reclaimed = base_class.class == COMBAT_CLASS_SHADOW_LORD
            && self.special_items[SPECIAL_ITEM_SCEPTRE_LB_INDEX] != 0;

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
        // `combat.md §5` "Arena-centre special", published between the
        // party descriptor seeding above and the monster count below: a
        // `0xDC` centre cell becomes a special active object with setup id
        // one, draw-free, so it changes no existing entry. The outdoor
        // terrain path is one of `§5`'s three callers, and the one the
        // three-caller census says can never present a qualifying centre
        // cell, so it owns no floor-fill byte (`RETRACTIONS.md` R362
        // withdraws the "inert for stock `BRIT.CBT`" conclusion this comment
        // used to draw: the byte is painted at run time on the dungeon-room
        // path, not loaded from an arena record).
        let mut setup_terrain = setup.terrain;
        self.place_combat_arena_centre_special(
            &mut setup_terrain,
            None,
            object.z,
            &mut active_objects,
        );

        // `active-objects.md §7`: monster records "continue from the
        // first record left free by the seated party". Scanned, not
        // counted - see the note on
        // [`Self::populate_combat_party_with_positions`].
        let first_free_record = first_free_combat_active_object_record(&active_objects)
            .unwrap_or(COMBAT_PARTY_ACTOR_SLOTS);

        // Step 3: the group-name line and the conflict banner, before any
        // monster is placed.
        //
        // `combat.md §4.1`: "Two separate banners are printed when a
        // terrain fight begins, by two different stages". Banner one is
        // the group name, emitted by the world-side terrain-combat entry
        // step "*before* it calls the framer", in the order: one line
        // feed, centre-output on, "the group name for the encounter's
        // class", centre-output off, "two line feeds | ends the name's
        // row and leaves one blank row below it". Banner two is the
        // conflict banner, which "Terrain setup prints ... at the start,
        // before any monster is placed", and it "is **unconditional**.
        // The test that precedes it cannot fail, so every terrain-setup
        // entry prints it" - once, not per group: nothing in either path
        // loops over the enemy set.
        //
        // The full published entry transcript is echo / blank / centred
        // group name / blank / `*** CONFLICT ***` filling the row, which
        // is exactly what this sequence emits.
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
        // The second blank row - between the group name and the conflict
        // banner - is the group name's own trailing "two line feeds", and
        // the print order, name above banner, is now published: the group
        // name is emitted "a whole stage before the framer is entered".
        // `combat.md §5.3`'s PRNG-order row formerly read "Conflict
        // banner, arena-record load, encounter-name print"; only the
        // conflict banner belongs there, and the table now says so. No
        // draw counts change - all three items consume none.
        self.push_explicit_blank_message_entry();
        if let Some(group_name) = outdoor_combat_group_banner_name(object.type_byte)
            .or_else(|| combat_class_group_banner_name(base_class.class))
        {
            self.emit_centered_message_line(group_name);
            self.push_explicit_blank_message_entry();
        }

        // `encounters.md §4` Shadow Lord branch, in the published order:
        //
        //   1. Print exactly `The Sceptre is reclaimed!`
        //   2. Play the sceptre-reclaimed sting (`audio.md §8.4.1`).
        //   3. Clear the sceptre flag.
        //
        // "The order matters for a transcript: the line completes before the
        // sting starts, and the flag clears last."
        //
        // It goes *before* the conflict banner. `combat.md §4.1`'s full entry
        // transcript is echo / blank / group name / blank / `*** CONFLICT ***`
        // "with `The Sceptre is reclaimed!` inserted after the group name on
        // the Shadow Lord branch when the sceptre is held", and
        // `encounters.md §4` puts the branch "entirely inside encounter setup,
        // before the combat scene is entered" while the conflict banner is
        // printed by terrain setup inside the framer. Nothing between the two
        // consumes a PRNG draw (`combat.md §5.3` step 4: "None"), so the
        // placement is draw-neutral.
        //
        // *Hedge.* Whether the group name's own trailing blank row falls above
        // or below this line is not published: §4.1's banner-one emission
        // table ends the group-name stage with "two line feeds ... leaves one
        // blank row below it", which puts the blank first as written here,
        // while §4.1's transcript table numbers the blank as its own row 4 and
        // says only "after the group name". `encounters.md §4`'s "no leading
        // blank line" describes the stored string, not the row above it.
        //
        // `audio.md §8.4.1`: entering this fight while carrying the sceptre
        // "is the only caller of this recipe".
        if sceptre_reclaimed {
            self.emit_message_line(SCEPTRE_RECLAIMED_LINE);
            self.emit_sound_effect(SoundEffect::SceptreReclaimed);
            self.special_items[SPECIAL_ITEM_SCEPTRE_LB_INDEX] = 0;
        }

        self.emit_centered_message_line(combat_banner_line());
        // `combat.md §4.1`: the banner fills the row edge to edge, so "the
        // row is full when the line feed is reached ... the printer's
        // full-row suppression consumes it. The cursor is left at column 0
        // of the following row and **no blank row** appears under the
        // banner." The next print's own line feed is what costs the blank.
        self.combat_transcript_row_open = false;

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
        self.enter_combat_frame_with_terrain(active_objects, actors, setup_terrain)?;
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

    /// `combat.md §5` "Arena-centre special". Allocates the marker's
    /// active-object record by the ordinary rule - "the first record whose
    /// tile byte is zero" - and no combat descriptor, which is what `§5`
    /// requires of every marker-only placement.
    ///
    /// `room_floor_fill` is the room's floor-fill terrain byte, for `§5`'s
    /// third settled property: "**The terrain byte under the converted cell
    /// is not left as loaded.** The step's last act overwrites that centre
    /// cell with the room's floor-fill terrain byte, erasing the `0xDC` the
    /// room painter had stamped there. An engine that leaves the grid
    /// untouched keeps a `0xDC` under the converted object, and every later
    /// grid consumer - the round loop's restraint test (Section 7.1), the
    /// step-validity predicate, the standing-cell hazard pass - then reads
    /// the wrong byte for that cell."
    ///
    /// It is `None` for a caller that owns no room floor-fill byte: the
    /// outdoor terrain path, which `§5`'s three-caller census says can never
    /// present a qualifying centre cell, and the loaded-room path, whose
    /// record comes off disk with no fill byte attached and which `§5`'s
    /// **established** arena-file negative says can never carry the
    /// qualifying byte anyway. Filed as a spec question rather than filled
    /// in with a guess. The painter path
    /// ([`Self::enter_dungeon_active_monster_combat`]) is the one that can
    /// present the byte, and it passes the fill it built the grid with.
    pub fn place_combat_arena_centre_special(
        &mut self,
        terrain: &mut [[u8; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE],
        room_floor_fill: Option<u8>,
        z: i8,
        active_objects: &mut [ActiveObject],
    ) -> Option<usize> {
        let marker = combat_arena_centre_special_active_object(terrain, z, &mut self.prng_state)?;
        let record = first_free_combat_active_object_record(active_objects)?;
        active_objects[record] = marker;
        if let Some(fill) = room_floor_fill {
            let (row, column) = COMBAT_ARENA_CENTRE_CELL;
            terrain[row][column] = fill;
        }
        Some(record)
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
            // `combat.md §5`/`§5.3` step 3: "The ring vanish check runs
            // **before** the member is placed: one uniform draw in
            // `[0, 15]` if the member wears the Ring of Invisibility or the
            // Ring of Regeneration, destroying the ring on the single
            // outcome `11`." The ordering is load-bearing for PRNG
            // reproduction - "one vanish lowers every subsequent count in
            // the same seating pass" (`RETRACTIONS.md` R307).
            self.apply_combat_seating_ring_vanish_check(roster_slot);
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
                phase: COMBAT_PLACEMENT_ACTIVE_OBJECT_PHASE,
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
            // `combat.md §5`: "After the member is placed, a ring-effect
            // step runs - but **only** for members whose status byte is
            // exactly `'G'` (good) or `'P'` (poisoned)."
            if matches!(member.status, b'G' | b'P') {
                self.apply_combat_seating_ring_effect_step(
                    roster_slot,
                    record_slot,
                    active_objects,
                    actors,
                );
            }
            // `combat.md §5.3` step 3, per-slot item 5: "A member whose
            // status is `'S'` (asleep) takes a branch that runs a **full
            // world tick**, itself a variable consumer - so seating is not
            // draw-bounded at all whenever anyone in the party is asleep."
            //
            // It is the fifth and last item of the per-slot order, after
            // the ring-effect step of item 4, and it is charged once per
            // asleep member rather than once per seating pass. The tick is
            // the shared world step of `animation.md §13.2`; its own draw
            // count is that step's contract, and `§5.3` publishes no
            // maximum for it.
            if asleep {
                self.advance_visual_tick();
            }
        }
        if cleared_active_player {
            self.active_player = None;
        }
    }

    /// `combat.md §5`/`§5.3` step 3.2: the pre-placement vanish check for
    /// one roster slot. Draws nothing unless that member wears one of the
    /// two magic rings.
    fn apply_combat_seating_ring_vanish_check(&mut self, roster_slot: usize) {
        let Some(ring) = self
            .party_equipment
            .get(roster_slot)
            .map(|equipment| equipment[EQUIP_SLOT_RING])
        else {
            return;
        };
        if !crate::is_combat_magic_ring_id(ring) {
            return;
        }
        let vanish_roll = self.combat_magic_ring_vanish_roll(roster_slot);
        if !crate::combat_magic_ring_vanishes(ring, vanish_roll) {
            return;
        }
        // `audio.md §8.1` terrain-combat-entry path, in its published
        // order: print the line, play the 40-update action snap, then
        // remove the item.
        self.message = COMBAT_RING_VANISHED_MESSAGE.to_string();
        self.emit_sound_effect(SoundEffect::ActionSnap);
        self.party_equipment[roster_slot][EQUIP_SLOT_RING] = EQUIPMENT_EMPTY;
    }

    /// `combat.md §5`: the post-placement ring-effect step for the member
    /// just seated, gated by the caller on status exactly `'G'` or `'P'`.
    ///
    /// The Invisibility arm marks that one member hidden. The Regeneration
    /// arm is a **whole-party regeneration sweep**, "not a single tick for
    /// the member that triggered it: it draws one uniform value in
    /// `[0, 7]` for every party member who is alive and wearing the
    /// regeneration ring *at that moment*, including members whose status
    /// is neither good nor poisoned" (`RETRACTIONS.md` R307). With two
    /// eligible wearers in good condition the entry pass therefore runs two
    /// sweeps of two draws each.
    fn apply_combat_seating_ring_effect_step(
        &mut self,
        roster_slot: usize,
        record_slot: usize,
        active_objects: &mut [ActiveObject],
        actors: &mut [CombatActorDescriptor; COMBAT_ACTOR_SLOTS],
    ) {
        let Some(ring) = self
            .party_equipment
            .get(roster_slot)
            .map(|equipment| equipment[EQUIP_SLOT_RING])
        else {
            return;
        };
        if ring as usize == EQUIPMENT_ID_RING_INVISIBILITY {
            if let Some(actor) = actors.get_mut(record_slot) {
                let _ = crate::apply_combat_linked_invisibility(actor, active_objects);
            }
            return;
        }
        if ring as usize != EQUIPMENT_ID_RING_REGENERATION {
            return;
        }
        let sweep: Vec<usize> = (0..self.party.len())
            .filter(|&slot| {
                self.party[slot].living()
                    && self.party_equipment.get(slot).is_some_and(|equipment| {
                        equipment[EQUIP_SLOT_RING] == EQUIPMENT_ID_RING_REGENERATION as u8
                    })
            })
            .collect();
        for slot in sweep {
            let regeneration_roll = self.combat_magic_ring_regeneration_roll(slot);
            let amount = crate::combat_ring_regeneration_amount(
                self.party[slot],
                EQUIPMENT_ID_RING_REGENERATION as u8,
                regeneration_roll,
            );
            if amount != 0 {
                let _ = self.party[slot].heal_by(amount);
            }
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

    /// `combat.md §5.3` steps 5 and 6. Step 5 is the count roll: "The count is
    /// rolled only when the class's spawn count rating is not one of the three
    /// exact-count sentinels 1, 8 and 16. When it is rolled it is one uniform
    /// draw in `[1, rating]`; and when the early-game damper flag is set, a
    /// second uniform draw in `[1, result of the first]` immediately follows".
    ///
    /// Step 6 rides on the same branch: "The same non-sentinel branch that
    /// rolls a count runs a **full world tick before any monster is placed**.
    /// That tick is a variable PRNG consumer with three distinct drawing arms,
    /// and they draw **in this order**: 1. The **active-object animation
    /// pass** ... 2. The **autonomous wind-drift roll**. 3. The **viewport
    /// composite** ... which takes one uniform `[0, 3]` draw **only** for a
    /// composited actor standing on one of the five selecting terrain rows of
    /// `systems/visibility.md` Section 8, and **zero** otherwise."
    ///
    /// [`Self::advance_visual_tick`] is that shared world tick and runs the
    /// arms in exactly that order. "Arena terrain almost never carries a
    /// selecting row ... So in ordinary combat entry the composite arm
    /// contributes **nothing**", which is why adding the tick here does not
    /// add a `[0, 3]` draw to combat entry - `RETRACTIONS.md` R329 withdrew
    /// the per-tick visibility draw an earlier revision published, and R331
    /// withdrew the reversed arm order that went with it.
    ///
    /// Sentinel ratings, the zero rating and the town-style override all skip
    /// the branch entirely and so "consume nothing here" - no count roll and
    /// no tick.
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
                // Step 6, on this same branch and before any placement.
                //
                // GAP, recorded rather than papered over: this shared tick
                // has its own `timing.md §8.2` scene gate and returns without
                // doing anything for scene values `0x21..=0x7F`. `§5.3`
                // scopes only step 3a out of the dungeon entries, and says
                // nothing about step 6 there. If some route ever reaches this
                // helper while the scene byte is still a dungeon value, that
                // entry would take step 5's count draws and not step 6's
                // tick. Both of today's callers are the terrain-combat setup
                // helper, which `§5` describes on the surface; the dungeon
                // entries go through the room painter and do not reach here.
                // Taken to the spec as a question rather than resolved by
                // hoisting the tick past its own published gate.
                self.advance_visual_tick();
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

    /// `active-objects.md §7`: combat setup "first clears all thirty-two
    /// records, then seats the party, then places monsters", and the bytes it
    /// then writes are enumerated - byte 0, byte 1, bytes 2 and 3, byte 4,
    /// byte 5 and byte 7. **Byte 6 is not among them**, so an arena record
    /// keeps the zero the clear left.
    ///
    /// That value is load-bearing. `active-objects.md §3` gives the animator's
    /// gates in order: an all-ones low nibble makes it "bail immediately,
    /// **writing nothing**", while a low nibble of zero "fall[s] through to the
    /// eligibility gates ..., which may advance the script step and rewrite the
    /// byte". Seeding [`STEADY_PHASE`] at placement therefore freezes every
    /// sprite in the arena for the whole fight - measured against the original
    /// as sixteen identical bat tiles where the original shows the four frames
    /// of the family spread 5/5/4/2 across the same sixteen cells.
    #[test]
    fn combat_placement_leaves_arena_records_at_a_decision_point() {
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

        let occupied: Vec<usize> = (0..state.active_objects.len())
            .filter(|slot| !state.active_objects[*slot].is_empty())
            .collect();
        assert!(
            occupied.len() > 3,
            "the arena must hold the seated party and at least one monster, got {occupied:?}"
        );
        for slot in occupied {
            let object = state.active_objects[slot];
            assert_eq!(
                object.phase, COMBAT_PLACEMENT_ACTIVE_OBJECT_PHASE,
                "record {slot} (type {:#04x}) must keep the clear's zero in byte 6",
                object.type_byte
            );
            assert_ne!(
                object.phase & 0x0f,
                STEADY_PHASE,
                "record {slot} must not carry the freeze sentinel"
            );
        }
        let _ = fs::remove_dir_all(dir);
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
        state.party_combat_defense = default_party_combat_defense(len);
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

    /// `combat.md §4.1`, "The full entry transcript": echo, blank, "the
    /// centred group name", blank, "`*** CONFLICT ***`, filling the row".
    /// The trigger here is an Orc (class 32), whose published group
    /// banner is `ORCS` (`catalogs/monster-bestiary.md §2.2`).
    ///
    /// *(Re-derived: the former assertion that no name line is printed
    /// for a class outside the one observed sample is withdrawn - the
    /// whole forty-eight-entry table is published. `RETRACTIONS.md` R350
    /// covers the pirate half of the same paragraph.)*
    #[test]
    fn combat_entry_prints_the_group_name_and_conflict_banner_around_one_blank_row() {
        let (mut state, dir) = batch_combat_state(&[b'G']);

        state
            .enter_terrain_combat_from_world_object(&dir, WorldPlane::Britannia, 1, batch_trigger())
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
                ("ORCS".to_string(), true, false),
                (String::new(), false, true),
                (combat_banner_line(), true, false),
            ]
        );
        // "nothing in either path loops over the enemy set, so
        // `*** CONFLICT ***` appears exactly once."
        assert_eq!(
            state
                .message_entries()
                .iter()
                .filter(|entry| entry.text == combat_banner_line())
                .count(),
            1
        );
        let _ = fs::remove_dir_all(dir);
    }

    /// `combat.md §4.1`: "The banner is count-independent ... a lone
    /// attacker still gets the group name: one bat announces `BATS`."
    #[test]
    fn a_bat_encounter_prints_the_published_group_name_above_the_banner() {
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

    /// `catalogs/monster-bestiary.md §2.2` is a shipped table of finished
    /// strings, not a rule: "nothing appends an `S`, and the banner never
    /// consults the monster count". Twenty-two of the forty-eight are the
    /// singular name uppercased with an `S`; twenty-six are not, which is
    /// why the table has to be shipped verbatim.
    #[test]
    fn the_group_banner_table_is_the_published_forty_eight_entry_table() {
        assert_eq!(COMBAT_CLASS_GROUP_BANNER_NAMES.len(), COMBAT_CLASS_COUNT);
        assert_eq!(combat_class_group_banner_name(21), Some("BATS"));
        assert_eq!(outdoor_combat_group_banner_name(0x94), Some("BATS"));
        // The five that "use a different word from the singular table"
        // and the one irregular plural.
        assert_eq!(combat_class_group_banner_name(0), Some("WIZARDS"));
        assert_eq!(combat_class_group_banner_name(22), Some("SPIDERS"));
        assert_eq!(combat_class_group_banner_name(31), Some("INSECTS"));
        assert_eq!(combat_class_group_banner_name(34), Some("SNAKES"));
        assert_eq!(combat_class_group_banner_name(46), Some("ROTWORMS"));
        assert_eq!(combat_class_group_banner_name(36), Some("HEADLESSES"));
        // The two proper nouns with no plural, and two singular forms.
        assert_eq!(combat_class_group_banner_name(14), Some("BLACKTHORN"));
        assert_eq!(combat_class_group_banner_name(15), Some("LORD BRITISH"));
        assert_eq!(combat_class_group_banner_name(24), Some("SLIME"));
        assert_eq!(combat_class_group_banner_name(30), Some("GARGOYLE"));
        // "**The Shadow Lord fight announces `SHADOW LORD`.** No article,
        // no "The", no separate singular caption."
        assert_eq!(combat_class_group_banner_name(47), Some("SHADOW LORD"));
        // "classes 3, 9, 13, 29, 42 and 43 all carry the one-character
        // placeholder `x` as their group banner, yet classes 3, 13 and 29
        // have perfectly real singular names (Avatar, Wanderer,
        // Crawler)." The engine must print the placeholder, not the name.
        for placeholder in [3u8, 9, 13, 29, 42, 43] {
            assert_eq!(
                combat_class_group_banner_name(placeholder),
                Some("x"),
                "class {placeholder} carries the shipped placeholder"
            );
        }
        // "twenty-two of the forty-eight are [the singular name uppercased
        // with an `S` appended], and twenty-six are not". Class 8 is
        // skipped: the bestiary lists its singular name as *(none)* - "one
        // with no singular counterpart at all - 8" - and this engine's own
        // class-stat row borrows the name `Pirate` *from* the banner, so
        // counting it would report twenty-three.
        let derived = (0u8..COMBAT_CLASS_COUNT as u8)
            .filter(|&class| {
                if class == 8 {
                    return false;
                }
                let Some(banner) = combat_class_group_banner_name(class) else {
                    return false;
                };
                combat_class_stats(class)
                    .map(|stats| format!("{}S", stats.name.to_ascii_uppercase()) == banner)
                    .unwrap_or(false)
            })
            .count();
        assert_eq!(derived, 22);
        // "Every entry is uppercase, letters and spaces only, with no
        // trailing punctuation ... The longest is twelve characters, so
        // no banner ever wraps in the sixteen-column message window."
        for banner in COMBAT_CLASS_GROUP_BANNER_NAMES {
            assert!(banner.len() <= 12, "{banner:?} is longer than twelve cells");
            assert!(
                banner == "x"
                    || banner
                        .bytes()
                        .all(|byte| byte == b' ' || byte.is_ascii_uppercase()),
                "{banner:?} is not uppercase letters and spaces"
            );
        }
    }

    /// `encounters.md §4` / `RETRACTIONS.md` R350: "The banner fallback
    /// is selected by **masked sprite byte `< 0x40`**, not by equality
    /// with `0x2C`, and the literal is exactly `PIRATES` (seven
    /// characters, uppercase, no punctuation, no line feed). ...
    /// Implement the range guard, not the instance."
    #[test]
    fn the_group_banner_pirate_literal_is_selected_by_the_sub_0x40_range_guard() {
        assert_eq!(COMBAT_GROUP_BANNER_PIRATE_LITERAL, "PIRATES");
        assert_eq!(COMBAT_GROUP_BANNER_PIRATE_LITERAL.len(), 7);
        for type_byte in OUTDOOR_PIRATE_TYPE_FIRST..=OUTDOOR_PIRATE_TYPE_LAST {
            assert_eq!(
                outdoor_combat_group_banner_name(type_byte),
                Some(COMBAT_GROUP_BANNER_PIRATE_LITERAL)
            );
        }
        // The guard, not the instance: every masked value below `0x40`
        // takes the literal, because "the class formula would go negative
        // below `0x40`, so the table cannot be indexed there at all".
        for type_byte in 0x00u8..0x40 {
            assert_eq!(
                outdoor_combat_group_banner_name(type_byte),
                Some(COMBAT_GROUP_BANNER_PIRATE_LITERAL),
                "masked sprite byte {type_byte:#04x} is below 0x40"
            );
        }
        // "The ship family's **stat** class is still 1 ... the banner and
        // the stat row disagree by design."
        assert_eq!(
            outdoor_combat_class_id(OUTDOOR_PIRATE_TYPE_FIRST),
            Some(OUTDOOR_PIRATE_COMBAT_CLASS)
        );
        assert_eq!(combat_class_group_banner_name(1), Some("BARD"));
        // At or above `0x40` the table is indexed normally.
        assert_eq!(outdoor_combat_group_banner_name(0x40), Some("WIZARDS"));
    }

    /// `combat.md §4.1`: "`*** CONFLICT ***` followed by one line feed.
    /// Exactly sixteen printable characters ... **The flank glyph is
    /// character code `0x2A`**, three per side - the ASCII asterisk code
    /// point, **not** `0x2B` (`+`)." Sixteen characters "is exactly the
    /// window's capacity", so the row fills columns 24..=39, and its one
    /// centred position is column zero.
    #[test]
    fn combat_banner_line_is_the_sixteen_cell_asterisk_flanked_literal() {
        assert_eq!(combat_banner_line(), "*** CONFLICT ***");
        assert_eq!(combat_banner_line().len(), MESSAGE_WINDOW_WIDTH);
        assert_eq!(COMBAT_BANNER_FLANK_GLYPH, '*');
        assert_eq!(
            COMBAT_BANNER_FLANK_GLYPH as u8,
            COMBAT_BANNER_FLANK_GLYPH_CODE
        );
        assert_ne!(COMBAT_BANNER_FLANK_GLYPH_CODE, b'+');
        assert_eq!(
            crate::text_window_centred_start_column(
                MESSAGE_WINDOW_WIDTH as u8,
                combat_banner_line().len() as u8
            ),
            0
        );
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
    ///
    /// It must do so **without** writing the resident active-player
    /// selector: `stats-panel.md §4.1` draws the roster's `0x1A` arrow
    /// from that selector, and a capture of the original's combat panel
    /// shows the acting member's row inverted with no arrow on it.
    #[test]
    fn the_combat_cursor_follows_the_pending_actor_without_moving_the_roster_marker() {
        let (mut state, dir) = batch_combat_state(&[b'G', b'D', b'G', b'G']);
        let (active_objects, actors) = seat_batch_party(&mut state);
        state.active_objects = active_objects;
        state.combat_actors = actors;
        state.active_player = None;

        // Descriptor two carries roster slot three in a party packed by the
        // dead member at roster slot one.
        state.pending_combat_actor_slot = Some(2);

        assert_eq!(state.combat_cursor_roster_slot(), Some(3));
        assert_eq!(state.combat_cursor_actor_cell(), Some(BATCH_SEATS[3]));
        assert_eq!(
            state.active_player, None,
            "the round walk must not move the roster arrow"
        );

        // With nobody parked on, the box falls back to the player's own
        // `0`-command selection.
        state.pending_combat_actor_slot = None;
        state.active_player = Some(3);

        assert_eq!(state.combat_cursor_actor_cell(), Some(BATCH_SEATS[3]));
        let _ = fs::remove_dir_all(dir);
    }

    /// `combat.md §5.3` step 3, per-slot item 5: "A member whose status is
    /// `'S'` (asleep) takes a branch that runs a **full world tick**, itself
    /// a variable consumer - so seating is not draw-bounded at all whenever
    /// anyone in the party is asleep."
    ///
    /// The tick is charged per asleep member and lands after item 4's
    /// ring-effect step. The all-good control is `§5.3` step 3's other half:
    /// with no ring and no `'S'`, seating "is genuinely draw-free in the
    /// default case".
    #[test]
    fn seating_an_asleep_member_runs_one_full_world_tick() {
        let (mut awake, awake_dir) = batch_combat_state(&[b'G', b'G', b'G']);
        awake.prng_state = 0x1234;
        let _ = seat_batch_party(&mut awake);
        assert_eq!(
            awake.prng_state, 0x1234,
            "an all-good roster with no rings seats draw-free"
        );
        let _ = fs::remove_dir_all(awake_dir);

        let (mut asleep, asleep_dir) = batch_combat_state(&[b'G', b'S', b'G']);
        asleep.prng_state = 0x1234;
        let _ = seat_batch_party(&mut asleep);

        // The reference is the same fixture taking exactly one world tick
        // and nothing else, so the asleep branch is pinned to one tick
        // rather than to some other number of draws.
        let (mut reference, reference_dir) = batch_combat_state(&[b'G', b'S', b'G']);
        reference.prng_state = 0x1234;
        reference.advance_visual_tick();

        assert_ne!(
            asleep.prng_state, 0x1234,
            "the `'S'` branch must consume the world tick's draws"
        );
        assert_eq!(
            asleep.prng_state, reference.prng_state,
            "seating an asleep member costs exactly one full world tick"
        );
        let _ = fs::remove_dir_all(asleep_dir);
        let _ = fs::remove_dir_all(reference_dir);
    }

    /// A second `'S'` member charges a second tick: `§5.3` puts the branch
    /// inside the per-slot walk, not once per seating pass.
    #[test]
    fn each_asleep_member_charges_its_own_world_tick() {
        let (mut one, one_dir) = batch_combat_state(&[b'G', b'S', b'G']);
        one.prng_state = 0x1234;
        let _ = seat_batch_party(&mut one);

        let (mut two, two_dir) = batch_combat_state(&[b'G', b'S', b'S']);
        two.prng_state = 0x1234;
        let _ = seat_batch_party(&mut two);

        let (mut reference, reference_dir) = batch_combat_state(&[b'G', b'S', b'S']);
        reference.prng_state = 0x1234;
        reference.advance_visual_tick();
        reference.advance_visual_tick();

        assert_ne!(two.prng_state, one.prng_state);
        assert_eq!(two.prng_state, reference.prng_state);
        let _ = fs::remove_dir_all(one_dir);
        let _ = fs::remove_dir_all(two_dir);
        let _ = fs::remove_dir_all(reference_dir);
    }

    /// `combat.md §5` "Arena-centre special": "If the loaded arena's centre
    /// cell (row five, column five) holds the magic-field marker tile
    /// `0xDC`, the setup pass converts that cell into a special active
    /// object with setup id one, using the same auxiliary-byte rule the
    /// dungeon-room loader applies to that id."
    ///
    /// Setup id one's rule is level-times-three-plus-seven, which draws
    /// nothing. `§5` also makes every marker-only placement descriptor-free,
    /// so the converted cell "never takes a turn and never appears to the
    /// target picker".
    #[test]
    fn an_arena_centre_magic_field_becomes_a_setup_id_one_special_object() {
        let (mut state, dir) = batch_combat_state(&[b'G', b'G', b'G', b'G']);
        let mut record = batch_arena_record();
        record[5 * COMBAT_ARENA_ROW_STRIDE + 5] = COMBAT_ARENA_CENTRE_SPECIAL_TILE;
        fs::write(dir.join(BRIT_CBT_FILE), record.repeat(BRIT_CBT_RECORDS)).unwrap();
        let trigger = batch_trigger();
        let prng_before = state.prng_state;

        state
            .enter_terrain_combat_from_world_object(&dir, WorldPlane::Britannia, 1, trigger)
            .unwrap();

        // A live party of four fills records 0..3; the marker takes the
        // next free record by the ordinary "first record whose tile byte is
        // zero" rule.
        let marker = state.active_objects[4];
        assert_eq!(marker.type_byte, COMBAT_ARENA_CENTRE_SPECIAL_SETUP_ID);
        assert_eq!(marker.tile, COMBAT_ARENA_CENTRE_SPECIAL_SETUP_ID);
        assert_eq!((marker.x, marker.y), COMBAT_ARENA_CENTRE_CELL);
        // Setup id one's auxiliary-byte rule: level * 3 + 7, on the
        // trigger's plane (surface, level zero).
        assert_eq!(marker.aux1, 7);
        assert_eq!(marker.aux3, COMBAT_ACTIVE_OBJECT_NO_DESCRIPTOR);

        // Marker-only placement allocates no descriptor, so nothing links
        // back to record four.
        assert!(
            state
                .combat_actors
                .iter()
                .all(|actor| actor.is_empty() || usize::from(actor.active_object_slot) != 4),
            "the centre special must not own a combat descriptor"
        );

        // The conversion is draw-free, so it cannot perturb the `§5.3`
        // entry stream ahead of the count roll. Re-running the same entry
        // on the plain arena has to leave the PRNG in the same place.
        let (mut plain, plain_dir) = batch_combat_state(&[b'G', b'G', b'G', b'G']);
        plain.prng_state = prng_before;
        plain
            .enter_terrain_combat_from_world_object(&plain_dir, WorldPlane::Britannia, 1, trigger)
            .unwrap();
        assert_eq!(state.prng_state, plain.prng_state);

        // And with no `0xDC` at the centre there is no marker at all: the
        // clause is inert for stock `BRIT.CBT`, whose records carry no
        // `0xDC` on cell (5, 5).
        assert!(
            plain.active_objects[4].type_byte != COMBAT_ARENA_CENTRE_SPECIAL_SETUP_ID
                || plain.active_objects[4].aux3 != COMBAT_ACTIVE_OBJECT_NO_DESCRIPTOR
                || (plain.active_objects[4].x, plain.active_objects[4].y)
                    != COMBAT_ARENA_CENTRE_CELL,
            "a non-`0xDC` centre cell must not produce the special object"
        );

        let _ = fs::remove_dir_all(dir);
        let _ = fs::remove_dir_all(plain_dir);
    }
}
