//! Tile/glyph rendering, NPC tile helpers, transport conversion, direction phase helpers, hashing, world scroll math, byte readers.

use std::collections::{HashMap, VecDeque};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::*;

pub fn place_pending_vehicle_acquisition(
    active_objects: &mut Vec<ActiveObject>,
    plane: WorldPlane,
    pending: PendingVehicleAcquisition,
) -> io::Result<usize> {
    let object = pending.active_object(plane.save_floor());
    if let Some(slot) = active_objects
        .iter()
        .enumerate()
        .skip(1)
        .find_map(|(slot, object)| object.is_empty().then_some(slot))
    {
        active_objects[slot] = object;
        return Ok(slot);
    }
    if active_objects.len() < OOL_SLOTS {
        active_objects.push(object);
        return Ok(active_objects.len() - 1);
    }
    Err(io::Error::new(
        io::ErrorKind::InvalidData,
        "no active-object slot for pending vehicle acquisition",
    ))
}

pub fn dungeon_cell_index(level: u8, x: usize, y: usize) -> usize {
    level as usize * DUNGEON_LEVEL_LEN + y * DUNGEON_SIDE + x
}

/// `formats/dungeon-dat.md §2`: absolute file offset of cell
/// `(level, x, y)` inside the eight-record `DUNGEON.DAT`. The file is
/// dungeon-major (records are 512 bytes each), each record is
/// level-major (eight 64-byte levels), and each level is row-major Y
/// then X.
pub const fn dungeon_file_offset(record_index: u8, level: u8, x: u8, y: u8) -> usize {
    (record_index as usize) * DUNGEON_RECORD_LEN
        + (level as usize) * DUNGEON_LEVEL_LEN
        + (y as usize) * DUNGEON_SIDE
        + (x as usize)
}

pub fn first_dungeon_walkable(grid: &[u8], level: u8) -> Option<(usize, usize)> {
    (0..DUNGEON_SIDE)
        .flat_map(|y| (0..DUNGEON_SIDE).map(move |x| (x, y)))
        .find(|(x, y)| is_dungeon_walkable(grid[dungeon_cell_index(level, *x, *y)]))
}

pub fn is_dungeon_walkable(tile: u8) -> bool {
    !matches!(tile >> 4, 0x0b..=0x0f)
}

pub fn dungeon_minimap_expands(tile: u8) -> bool {
    !matches!(tile >> 4, 0x0b..=0x0d)
}

pub fn is_dungeon_fall_trap(tile: u8) -> bool {
    matches!(tile, 0x61 | 0x69)
}

pub fn is_dungeon_bomb_trap(tile: u8) -> bool {
    matches!(tile, 0x62 | 0x6a)
}

pub fn dungeon_field_effect(tile: u8) -> Option<DungeonFieldEffect> {
    match tile {
        0x80 | 0x88 => Some(DungeonFieldEffect::Sleep),
        0x81 | 0x89 => Some(DungeonFieldEffect::PoisonGas),
        0x82 | 0x8a => Some(DungeonFieldEffect::Fire),
        0x83 | 0x8b => Some(DungeonFieldEffect::Electric),
        0x84..=0x9f => Some(DungeonFieldEffect::Energy),
        _ => None,
    }
}

pub fn is_dungeon_room_trigger(tile: u8) -> bool {
    tile >> 4 == 0x0f
}

pub fn is_dungeon_room_helper_state(tile: u8) -> bool {
    tile >> 4 == 0x0a
}

pub fn dungeon_room_slot(tile: u8) -> u8 {
    tile & 0x0f
}

pub fn dungeon_room_arena_index(scene: DungeonScene, tile: u8) -> usize {
    let bank = if scene.record <= 1 {
        0
    } else {
        scene.record - 1
    };
    bank * 16 + dungeon_room_slot(tile) as usize
}

pub fn stair_delta(tile: u8, intent: ClimbIntent) -> Option<i8> {
    if !(80..=87).contains(&tile) {
        return None;
    }
    // The public spec identifies the stair/ladder family but leaves the exact
    // subtype table open, so this first-playable hook follows the request.
    Some(town_climb_delta(intent))
}

pub fn town_walk_on_stair_delta(tile: u8, direction: Direction) -> Option<i8> {
    if !(0xc4..=0xc7).contains(&tile) {
        return None;
    }
    let direction = town_cardinal_direction_code(direction)?;
    let selector = tile & 0x03;
    if selector == direction {
        Some(1)
    } else if selector == ((direction + 2) & 0x03) {
        Some(-1)
    } else {
        None
    }
}

fn town_cardinal_direction_code(direction: Direction) -> Option<u8> {
    match direction {
        Direction::North => Some(0),
        Direction::East => Some(1),
        Direction::South => Some(2),
        Direction::West => Some(3),
        _ => None,
    }
}

pub fn town_climb_delta(intent: ClimbIntent) -> i8 {
    match intent {
        ClimbIntent::Up => 1,
        ClimbIntent::Down => -1,
    }
}

pub fn dungeon_ladder_delta(tile: u8, intent: ClimbIntent) -> Option<i8> {
    match (tile >> 4, intent) {
        (0x1, ClimbIntent::Up) => Some(-1),
        (0x2, ClimbIntent::Down) => Some(1),
        (0x3, ClimbIntent::Up) => Some(-1),
        (0x3, ClimbIntent::Down) => Some(1),
        _ => None,
    }
}

pub fn render_glyph(tile: u8) -> char {
    // Moongate frames have a dedicated glyph regardless of their numeric
    // range; the actual sprite ids are 0xD4..=0xD7 (see constants.rs).
    if (MOONGATE_TILE_BASE..MOONGATE_TILE_BASE + MOONGATE_ANIMATION_FRAMES).contains(&tile) {
        return '^';
    }
    match tile {
        0 => ' ',
        1..=4 => match tile {
            1 => '~',
            2 => '=',
            3 => '-',
            _ => '_',
        },
        5..=15 => ',',
        16..=23 => '.',
        24..=63 => '#',
        64..=79 => 'f',
        80..=87 => '<',
        88..=95 => '?',
        96..=103 => '+',
        104..=127 => '*',
        128..=159 => '^',
        160..=191 => 'v',
        192..=255 => 'n',
    }
}

pub fn surface_view_class(tile: u8) -> u8 {
    match tile {
        0x00 | 0xc0..=0xc3 | 0xcc..=0xcf | 0xff => 0x00,
        0x05 | 0x30..=0x37 => 0x01,
        0x09..=0x0a | 0x2d => 0x02,
        0x07
        | 0x1c
        | 0x1e..=0x1f
        | 0x40
        | 0x44
        | 0x48..=0x49
        | 0x6a..=0x6b
        | 0x70..=0x7f
        | 0x87
        | 0x8c
        | 0x8f
        | 0xaa
        | 0xbc
        | 0xdd => 0x03,
        0x1d
        | 0x38
        | 0x47
        | 0x5a
        | 0x5c..=0x5d
        | 0x94..=0x96
        | 0x9a..=0x9c
        | 0xab..=0xac
        | 0xbe => 0x04,
        0x10..=0x1b
        | 0x29..=0x2b
        | 0x2e..=0x2f
        | 0x41..=0x43
        | 0x4c
        | 0x58..=0x59
        | 0x5b
        | 0x5e..=0x5f
        | 0x80..=0x85
        | 0x88..=0x8b
        | 0x8d..=0x8e
        | 0x90..=0x93
        | 0x9d..=0xa9
        | 0xad..=0xb7
        | 0xbd
        | 0xbf
        | 0xc8..=0xcb
        | 0xde..=0xdf
        | 0xe8..=0xeb
        | 0xfa..=0xfd => 0x05,
        0x0d
        | 0x45
        | 0x4a..=0x4b
        | 0x86
        | 0x97..=0x99
        | 0xb8..=0xbb
        | 0xc4..=0xc7
        | 0xec..=0xf9 => 0x06,
        0x0c | 0x27..=0x28 | 0x39..=0x3f | 0x46 | 0x4d..=0x57 | 0xd0..=0xd3 | 0xfe => 0x07,
        0x0b | 0x0e..=0x0f => 0x08,
        0x06 | 0x08 | 0x2c => 0x09,
        0x03 | 0x60..=0x69 | 0x6c..=0x6f | 0xe4..=0xe7 => 0x0a,
        0x02 | 0xd4..=0xd7 => 0x0b,
        0x01 => 0x0c,
        0x04 => 0x0d,
        0xe0..=0xe3 => 0x0e,
        0xd8..=0xdc => 0x0f,
        0x20..=0x26 => 0x10,
    }
}

pub fn render_surface_view_class(class: u8) -> char {
    match class {
        0x00 => ' ',
        0x01..=0x09 => (b'0' + class) as char,
        0x0a..=0x0f => (b'A' + (class - 0x0a)) as char,
        0x10 => 'G',
        0x5a => 'W',
        _ => '?',
    }
}

pub fn render_dungeon_glyph(tile: u8) -> char {
    match tile {
        0x00..=0x07 => ' ',
        0x08..=0x0f => '.',
        0x10..=0x1f => '<',
        0x20..=0x2f => '>',
        0x30..=0x3f => 'H',
        0x40..=0x4f => '$',
        0x50..=0x5f => 'f',
        0x60 => 'o',
        0x61 | 0x69 => 'v',
        0x68 => '.',
        0x62..=0x6f => '!',
        0x70..=0x7f => ' ',
        0x80..=0x8f => '*',
        0x90..=0x9f => ' ',
        0xa0..=0xaf => '+',
        0xb0 => '#',
        0xb1..=0xbf => '#',
        0xc0..=0xcf => '#',
        0xd0..=0xdf => '#',
        0xe0..=0xef => '+',
        0xf0..=0xff => '+',
    }
}

pub fn npc_tile(type_byte: u8) -> u8 {
    if (192..=255).contains(&type_byte) {
        type_byte
    } else {
        192
    }
}

pub fn npc_active_object(type_byte: u8, x: usize, y: usize, z: u8) -> ActiveObject {
    let tile = npc_tile(type_byte);
    ActiveObject {
        type_byte: tile,
        tile,
        x,
        y,
        z: z as i8,
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 0,
    }
}

pub fn active_object_matches_runtime_npc(
    object: ActiveObject,
    npc: &RuntimeNpc,
    floor: u8,
) -> bool {
    if object.is_empty()
        || object.x != npc.x
        || object.y != npc.y
        || object.z != floor as i8
        || npc.z != floor
    {
        return false;
    }
    if npc.is_player_phantom() {
        object.type_byte == PLAYER_NPC_SENTINEL_TYPE
    } else {
        object.type_byte == npc_tile(npc.type_byte)
    }
}

pub fn player_phantom_active_object(x: usize, y: usize, z: u8) -> ActiveObject {
    ActiveObject {
        type_byte: PLAYER_NPC_SENTINEL_TYPE,
        tile: 0,
        x,
        y,
        z: z as i8,
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 0,
    }
}

pub fn step_toward(from: (usize, usize), to: (usize, usize)) -> Option<(usize, usize)> {
    if from == to {
        return None;
    }
    let mut next = from;
    if from.0 != to.0 {
        next.0 = if from.0 < to.0 {
            from.0 + 1
        } else {
            from.0.saturating_sub(1)
        };
    } else if from.1 != to.1 {
        next.1 = if from.1 < to.1 {
            from.1 + 1
        } else {
            from.1.saturating_sub(1)
        };
    }
    Some(next)
}

pub fn tile_class(tile: u8) -> &'static str {
    match tile {
        0 => "sentinel",
        1..=4 => "water",
        5..=15 => "terrain",
        16..=23 => "path",
        24..=63 => "wall",
        64..=95 => "furniture",
        96..=103 => "door",
        104..=127 => "decoration",
        128..=159 => "special",
        160..=191 => "vehicle",
        192..=255 => "npc-sprite",
    }
}

pub fn transport_from_vehicle_object(
    type_byte: u8,
    tile: u8,
    aux1: u8,
    aux3: u8,
) -> Option<TransportState> {
    match tile {
        160..=167 => Some(TransportState::Horse { type_byte, tile }),
        168..=175 => Some(TransportState::Ship {
            type_byte,
            tile,
            sails_hoisted: false,
            hull: aux1,
            skiffs: aux3,
        }),
        176..=183 => Some(TransportState::Skiff { type_byte, tile }),
        184..=187 => Some(TransportState::Carpet { type_byte, tile }),
        188..=191 => None,
        _ => None,
    }
}

pub fn transport_from_save_marker(marker: u8) -> TransportState {
    transport_from_vehicle_object(marker, marker, 0, 0).unwrap_or_default()
}

pub fn active_object_frame_tile(type_byte: u8, phase: u8) -> Option<u8> {
    if type_byte == PLAYER_TILE {
        return None;
    }
    let low = phase & 0x0f;
    match type_byte {
        128..=191 => Some((type_byte & !0x03) + (low & 0x03)),
        192..=255 => Some((type_byte & !0x01) + (low & 0x01)),
        _ => None,
    }
}

pub fn is_ambient_wanderer_object(object: ActiveObject) -> bool {
    (192..=255).contains(&object.type_byte) || (192..=255).contains(&object.tile)
}

pub fn is_ship_object(object: ActiveObject) -> bool {
    (168..=175).contains(&object.type_byte) || (168..=175).contains(&object.tile)
}

pub fn is_whirlpool_object(object: ActiveObject) -> bool {
    (0xec..=0xef).contains(&object.type_byte) || (0xec..=0xef).contains(&object.tile)
}

pub fn outdoor_combat_arena_index_for_object(object: ActiveObject) -> Option<usize> {
    outdoor_combat_arena_index_for_byte(object.type_byte)
        .or_else(|| outdoor_combat_arena_index_for_byte(object.tile))
}

pub fn outdoor_combat_arena_index_for_byte(byte: u8) -> Option<usize> {
    match byte {
        0x2c..=0x2f => Some(1),
        0x40..=0x7f => Some(((byte - 0x40) / 4) as usize),
        _ => None,
    }
}

pub fn direction_from_active_object_phase(phase: u8) -> Option<Direction> {
    match phase >> 4 {
        0 => Some(Direction::North),
        1 => Some(Direction::NorthEast),
        2 => Some(Direction::East),
        3 => Some(Direction::SouthEast),
        4 => Some(Direction::South),
        5 => Some(Direction::SouthWest),
        6 => Some(Direction::West),
        7 => Some(Direction::NorthWest),
        _ => None,
    }
}

pub fn active_object_phase_from_direction(direction: Direction, low_nibble: u8) -> u8 {
    let high_nibble = match direction {
        Direction::North => 0,
        Direction::NorthEast => 1,
        Direction::East => 2,
        Direction::SouthEast => 3,
        Direction::South => 4,
        Direction::SouthWest => 5,
        Direction::West => 6,
        Direction::NorthWest => 7,
    };
    (high_nibble << 4) | (low_nibble & 0x0f)
}

pub fn active_object_phase_toward_player(dx: i8, dy: i8) -> u8 {
    let x_step = -dx.signum();
    let y_step = -dy.signum();
    let direction = match (x_step, y_step) {
        (-1, -1) => Direction::NorthWest,
        (0, -1) => Direction::North,
        (1, -1) => Direction::NorthEast,
        (-1, 0) => Direction::West,
        (1, 0) => Direction::East,
        (-1, 1) => Direction::SouthWest,
        (0, 1) => Direction::South,
        (1, 1) => Direction::SouthEast,
        _ => Direction::South,
    };
    active_object_phase_from_direction(direction, 0)
}

pub fn cardinal_direction_from_active_object_phase(phase: u8) -> Option<Direction> {
    direction_from_active_object_phase(phase).filter(|direction| direction.is_cardinal())
}

pub fn is_vehicle_object_tile(tile: u8) -> bool {
    (160..=191).contains(&tile)
}

/// `dungeon-mode.md §8` pit/trap family classification for the
/// `0x6?` band. Only the three named exact bytes (`0x60`, `0x61`+
/// `0x69`, `0x62`+`0x6A`) carry mechanical effects; other `0x6?`
/// bytes inspect as the generic pit/trap class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DungeonPitTrap {
    /// `0x60` — plain pit. Look inspects it; K-Klimb fires the
    /// surface-reset helper.
    PlainPit,
    /// `0x61` and `0x69` — automatic fall traps. Stepping on either
    /// drops the party to the same X/Y on the next level.
    FallTrap,
    /// `0x62` and `0x6A` — bomb traps. Stepping on either prints the
    /// bomb messages, clears the cell, and does not change Z.
    BombTrap,
    /// Other `0x6?` bytes — generic pit/trap inspection only.
    GenericPitFamily,
}

/// `dungeon-mode.md §8`: classify a `0x6?` pit/trap byte. Returns
/// `None` for any byte outside the `0x60..=0x6F` family band.
pub const fn dungeon_pit_trap_kind(tile: u8) -> Option<DungeonPitTrap> {
    if tile < 0x60 || tile > 0x6F {
        return None;
    }
    Some(match tile {
        0x60 => DungeonPitTrap::PlainPit,
        0x61 | 0x69 => DungeonPitTrap::FallTrap,
        0x62 | 0x6A => DungeonPitTrap::BombTrap,
        _ => DungeonPitTrap::GenericPitFamily,
    })
}

/// `dungeon-mode.md §8`: Search rewrites `0x61` (secret-door reveal)
/// to `0x60` for the current visit and marks the same X/Y cell one
/// level below with the visit bit `0x08` (when not already on the
/// deepest level). The deepest dungeon level is `7`.
pub const DUNGEON_DEEPEST_LEVEL: u8 = 7;
pub const DUNGEON_VISIT_MARKER_BIT: u8 = 0x08;

/// `dungeon-mode.md §8`: stepping into an automatic fall trap
/// (`0x61`/`0x69`) lands the party at the same X/Y on the next level
/// and marks bit `0x08` in the destination cell *only when* it is
/// below the wall/door band (`< 0x90`).
pub const DUNGEON_WALL_DOOR_BAND_FIRST: u8 = 0x90;
pub const fn dungeon_fall_destination_marks_visit(destination_byte: u8) -> bool {
    destination_byte < DUNGEON_WALL_DOOR_BAND_FIRST
}

/// `dungeon-mode.md §8`: search-rewrite targets for the flavour-class
/// (`0xC?`) and wall-class (`0xD?`) hidden-passage paths. Each rewrite
/// preserves only the visit-marker bit on the original cell.
pub const DUNGEON_SEARCH_FLAVOR_REWRITE_PRIMARY: u8 = 0xB0;
pub const DUNGEON_SEARCH_FLAVOR_REWRITE_VISITED: u8 = 0xB8;
pub const DUNGEON_SEARCH_WALL_REWRITE_PRIMARY: u8 = 0xE0;
pub const DUNGEON_SEARCH_WALL_REWRITE_VISITED: u8 = 0xE8;

/// `dungeon-mode.md §8` fountain effect derived from the low nibble
/// of a fountain cell byte (high nibble `0x5`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FountainEffect {
    /// Sub-type 0 — Cure: sets status to Good.
    Cure,
    /// Sub-type 1 — Heal: refills HP without status change.
    Heal,
    /// Sub-type 2 — Poison: sets status to Poisoned.
    Poison,
    /// Sub-types 3..=15 — Bad taste: random `0..=7` HP damage.
    BadTaste,
}

/// `dungeon-mode.md §8`: classify a fountain cell byte's low nibble.
pub const fn fountain_effect_from_byte(tile: u8) -> FountainEffect {
    match tile & 0x0F {
        0 => FountainEffect::Cure,
        1 => FountainEffect::Heal,
        2 => FountainEffect::Poison,
        _ => FountainEffect::BadTaste,
    }
}

/// `dungeon-mode.md §8` energy-field sub-type derived from the low
/// nibble of an energy-field cell byte. Magic field placement preserves
/// the dungeon visit-marker bit, producing the matching `0x88..=0x8B`
/// variants of these base bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnergyFieldKind {
    /// `0x80` — sleep field (status `'S'` on contact).
    Sleep,
    /// `0x81` — poison gas (status `'P'` on contact, no cell rewrite).
    Poison,
    /// `0x82` — wall of fire (fire damage on contact).
    Fire,
    /// `0x83` — electric field (electric damage + forced step).
    Electric,
    /// Other `0x8?` sub-types collapse to the generic energy-field
    /// description.
    Generic,
}

/// `dungeon-mode.md §8`: classify an energy-field cell byte.
/// Recognises the four named base bytes `0x80..=0x83`; everything else
/// in the `0x8_` band collapses to the generic energy-field family.
pub const fn energy_field_kind_from_byte(tile: u8) -> EnergyFieldKind {
    match tile {
        0x80 => EnergyFieldKind::Sleep,
        0x81 => EnergyFieldKind::Poison,
        0x82 => EnergyFieldKind::Fire,
        0x83 => EnergyFieldKind::Electric,
        _ => EnergyFieldKind::Generic,
    }
}

/// `dungeon-mode.md §3` typed dungeon-cell class derived from the cell
/// byte's high nibble. Renderer wall checks, L-Look description routing,
/// and most special-cell handlers branch on this.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DungeonCellClass {
    Passage,
    UpLadder,
    DownLadder,
    TwoWayLadder,
    Chest,
    Fountain,
    PitTrap,
    PassageVariant,
    EnergyField,
    EnergyFieldSecondary,
    RoomHelperState,
    Wall,
    HeavyDoorOrRoomTrigger,
}

/// `dungeon-mode.md §5`: visit-local patch a room-trigger cell
/// receives after the room encounter resolves. The high nibble drops
/// from `0xF` (room trigger) to `0xA` (room-helper state) while the
/// low nibble (room-arena slot id) is preserved. Returns `None` for
/// any byte that is not in the `0xF?` room-trigger range — the
/// caller should not patch other classes.
pub const fn dungeon_room_post_combat_patch_byte(tile: u8) -> Option<u8> {
    if tile >> 4 != 0xF {
        return None;
    }
    Some(0xA0 | (tile & 0x0F))
}

/// `dungeon-mode.md §3`: L-Look description-byte normalisation. The
/// exact byte `0x61` is rewritten to `0x00` before the cell-class
/// description string is looked up, so it reports as passage even
/// though the underlying tile remains a pit-family variant. Other
/// `0x6?` trap bytes (e.g. `0x69`, `0x62`, `0x6A`) keep their `0x6`
/// pit/trap class description.
pub const DUNGEON_LOOK_PASSAGE_NORMALISED_BYTE: u8 = 0x61;

/// `dungeon-mode.md §3`: returns the cell byte L-Look should hand to
/// the description-string lookup. Only the exact `0x61` byte is
/// normalised; every other byte is returned unchanged.
pub const fn dungeon_look_description_byte(tile: u8) -> u8 {
    if tile == DUNGEON_LOOK_PASSAGE_NORMALISED_BYTE {
        0x00
    } else {
        tile
    }
}

/// `dungeon-mode.md §3`: classify a dungeon-cell byte by its high nibble.
pub const fn dungeon_cell_class_of(tile: u8) -> DungeonCellClass {
    match tile >> 4 {
        0x0 => DungeonCellClass::Passage,
        0x1 => DungeonCellClass::UpLadder,
        0x2 => DungeonCellClass::DownLadder,
        0x3 => DungeonCellClass::TwoWayLadder,
        0x4 => DungeonCellClass::Chest,
        0x5 => DungeonCellClass::Fountain,
        0x6 => DungeonCellClass::PitTrap,
        0x7 => DungeonCellClass::PassageVariant,
        0x8 => DungeonCellClass::EnergyField,
        0x9 => DungeonCellClass::EnergyFieldSecondary,
        0xA => DungeonCellClass::RoomHelperState,
        0xB..=0xE => DungeonCellClass::Wall,
        // 0xF and any (impossible) higher value
        _ => DungeonCellClass::HeavyDoorOrRoomTrigger,
    }
}

impl DungeonCellClass {
    /// `dungeon-mode.md §3`: solid-blocker wall classes (high nibble
    /// `0xB..=0xE`). The renderer's wall checks branch on this.
    pub const fn is_wall(self) -> bool {
        matches!(self, DungeonCellClass::Wall)
    }

    /// `dungeon-mode.md §3`: classes that K-Klimb can act on.
    pub const fn is_ladder(self) -> bool {
        matches!(
            self,
            DungeonCellClass::UpLadder
                | DungeonCellClass::DownLadder
                | DungeonCellClass::TwoWayLadder
        )
    }

    /// `dungeon-mode.md §3`: classes that render as walkable passage in
    /// the first-person renderer.
    pub const fn is_passage_like(self) -> bool {
        matches!(
            self,
            DungeonCellClass::Passage | DungeonCellClass::PassageVariant
        )
    }
}

pub fn dungeon_cell_class(tile: u8) -> &'static str {
    match tile >> 4 {
        0x0 => "passage",
        0x1 => "up ladder",
        0x2 => "down ladder",
        0x3 => "two-way ladder",
        0x4 => "chest",
        0x5 => "fountain",
        0x6 => "pit/trap",
        0x7 => "passage variant",
        0x8 | 0x9 => "energy field",
        0xA => "room-helper state",
        0xB..=0xE => "wall",
        0xF => "heavy door/room trigger",
        _ => "unknown",
    }
}

pub fn dungeon_look_description(tile: u8) -> &'static str {
    let tile = if tile == 0x61 { 0x00 } else { tile };
    match tile {
        0x80 => "a sleep field",
        0x81 => "a poison gas field",
        0x82 => "a wall of fire",
        0x83 => "an electric field",
        _ => match tile >> 4 {
            0x0 => "passage",
            0x1 => "an up ladder",
            0x2 => "a down ladder",
            0x3 => "a two-way ladder",
            0x4 => "a wooden chest",
            0x5 => "a fountain",
            0x6 => "a pit or trap",
            0x7 => "passage",
            0x8 | 0x9 => "an energy field",
            0xA => "a cleared room trigger",
            0xB..=0xE => "a wall",
            0xF => "a heavy door or room trigger",
            _ => "unknown dungeon cell",
        },
    }
}

pub fn dungeon_search_description(tile: u8) -> &'static str {
    if let Some(field) = dungeon_field_effect(tile) {
        return field.label();
    }
    match tile >> 4 {
        0x0 => "nothing of note",
        0x1 => "an up ladder",
        0x2 => "a down ladder",
        0x3 => "a two-way ladder",
        0x4 => "a wooden chest",
        0x5 => "a fountain",
        0x6 => "a pit or trap",
        0x7 => "a passage",
        0xA => "a cleared room trigger",
        0xB..=0xE => "a wall",
        0xF => "a heavy door or room trigger",
        _ => "an unknown dungeon cell",
    }
}

pub fn render_class_byte(tile: u8) -> u8 {
    match tile {
        0 => b' ',
        1..=4 => b'~',
        5..=15 => b',',
        16..=23 => b'.',
        24..=63 => b'#',
        64..=95 => b'f',
        96..=103 => b'D',
        104..=127 => b'd',
        128..=159 => b's',
        160..=191 => b'v',
        192..=255 => b'n',
    }
}

pub fn waypoint_for_hour(schedule: &[u8; 16], hour: u8) -> usize {
    let t0 = schedule[12];
    let t1 = schedule[13];
    let t2 = schedule[14];
    let t3 = schedule[15];
    if in_wrapping_range(hour, t0, t1) {
        0
    } else if in_wrapping_range(hour, t2, t3) {
        2
    } else {
        1
    }
}

pub fn in_wrapping_range(hour: u8, start: u8, end: u8) -> bool {
    if start == end {
        return false;
    }
    if start < end {
        hour >= start && hour < end
    } else {
        hour >= start || hour < end
    }
}

pub fn names(slots: &[NpcSlot]) -> Vec<String> {
    slots
        .iter()
        .filter(|slot| slot.type_byte != 0)
        .filter_map(|slot| slot.name.clone())
        .collect()
}

pub fn contains_all(names: &[String], needles: &[&str]) -> bool {
    needles.iter().all(|needle| contains_any(names, &[*needle]))
}

pub fn contains_any(names: &[String], needles: &[&str]) -> bool {
    names.iter().any(|name| {
        needles.iter().any(|needle| {
            name.to_ascii_lowercase()
                .contains(&needle.to_ascii_lowercase())
        })
    })
}

pub fn sample_names(names: &[String]) -> String {
    let mut sample: Vec<_> = names.iter().take(8).cloned().collect();
    if names.len() > sample.len() {
        sample.push(format!("... +{} more", names.len() - sample.len()));
    }
    sample.join(", ")
}

pub fn hash_palette_indices(pixels: &[u8]) -> u64 {
    hash_bytes(pixels)
}

pub fn hash_bytes(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

pub fn compact(s: &str) -> String {
    s.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim_matches('"')
        .to_string()
}

pub fn manhattan(a: (usize, usize), b: (usize, usize)) -> usize {
    a.0.abs_diff(b.0) + a.1.abs_diff(b.1)
}

pub fn world_scroll_base(x: usize, y: usize) -> (usize, usize) {
    (world_scroll_base_axis(x), world_scroll_base_axis(y))
}

pub fn world_scroll_base_axis(position: usize) -> usize {
    let base = (position / CHUNK_SIDE) * CHUNK_SIDE;
    if position % CHUNK_SIDE < CHUNK_SIDE / 2 {
        (base + WORLD_SIDE - CHUNK_SIDE) % WORLD_SIDE
    } else {
        base
    }
}

pub fn world_scroll_neighborhood_contains(scroll_base: (usize, usize), x: usize, y: usize) -> bool {
    world_scroll_axis_offset(scroll_base.0, x) <= ACTIVE_OBJECT_NEIGHBORHOOD_RADIUS
        && world_scroll_axis_offset(scroll_base.1, y) <= ACTIVE_OBJECT_NEIGHBORHOOD_RADIUS
}

pub fn world_scroll_axis_offset(base: usize, coordinate: usize) -> usize {
    (coordinate + WORLD_SIDE - base) % WORLD_SIDE
}

pub fn u16_at(bytes: &[u8], off: usize) -> u16 {
    u16::from_le_bytes([bytes[off], bytes[off + 1]])
}

#[cfg(test)]
pub fn u32_at(bytes: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([bytes[off], bytes[off + 1], bytes[off + 2], bytes[off + 3]])
}

pub fn write_u16_at(bytes: &mut [u8], off: usize, value: u16) {
    bytes[off..off + 2].copy_from_slice(&value.to_le_bytes());
}
