//! Tile/glyph rendering, NPC tile helpers, transport conversion, direction phase helpers, hashing, world scroll math, byte readers.

use std::io;

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
    !matches!(tile >> 4, 0x0b..=0x0d)
}

pub const fn dungeon_back_step_rejected(tile: u8) -> bool {
    matches!(tile >> 4, 0x0a | 0x0f)
}

pub fn dungeon_minimap_expands(tile: u8) -> bool {
    !matches!(tile >> 4, 0x0b..=0x0d)
}

/// `doors-and-z-transitions.md §10` dungeon fall-trap bytes. Walking
/// onto either of these cells fires the pit/fall transition: print
/// the messages, increment Z by one, land at the same X/Y on the
/// next level, and update marker bits on the affected cells. If the
/// destination is another fall-trap byte, the chain repeats.
pub const DUNGEON_PIT_FALL_TRAP_VISIBLE: u8 = 0x61;
/// `doors-and-z-transitions.md §10` second dungeon fall-trap byte
/// (hidden variant — exact `0x69`).
pub const DUNGEON_PIT_FALL_TRAP_HIDDEN: u8 = 0x69;

/// `doors-and-z-transitions.md §10` dungeon bomb-trap bytes. These
/// share the `0x6?` pit family with the fall traps but do not change
/// Z. Search resolution narrates them separately from fall pits.
pub const DUNGEON_PIT_BOMB_TRAP_VISIBLE: u8 = 0x62;
/// `doors-and-z-transitions.md §10` hidden bomb-trap byte (`0x6A`).
pub const DUNGEON_PIT_BOMB_TRAP_HIDDEN: u8 = 0x6A;

pub fn is_dungeon_fall_trap(tile: u8) -> bool {
    matches!(
        tile,
        DUNGEON_PIT_FALL_TRAP_VISIBLE | DUNGEON_PIT_FALL_TRAP_HIDDEN
    )
}

pub fn is_dungeon_bomb_trap(tile: u8) -> bool {
    matches!(
        tile,
        DUNGEON_PIT_BOMB_TRAP_VISIBLE | DUNGEON_PIT_BOMB_TRAP_HIDDEN
    )
}

/// `dungeon-mode.md §8` published energy-field base bytes. Magic
/// field placement preserves the dungeon visit-marker bit when it
/// writes into the live dungeon image, so each field has a paired
/// marker variant at `base | DUNGEON_VISIT_MARKER_BIT`.
pub const DUNGEON_FIELD_SLEEP_BASE: u8 = 0x80;
pub const DUNGEON_FIELD_POISON_GAS_BASE: u8 = 0x81;
pub const DUNGEON_FIELD_FIRE_BASE: u8 = 0x82;
pub const DUNGEON_FIELD_ELECTRIC_BASE: u8 = 0x83;
pub const DUNGEON_FIELD_STATUS_ROLL_LOW: u8 = 1;
pub const DUNGEON_FIELD_STATUS_ROLL_HIGH: u8 = 30;

/// `dungeon-mode.md §8`: sleep/poison status applies when the inclusive
/// `1..30` roll is equal to or greater than current Dexterity. Dexterity is
/// not clamped, so values above 30 always save.
pub const fn dungeon_field_status_applies(dexterity: u8, roll: u8) -> bool {
    roll >= dexterity
}

pub fn dungeon_field_effect(tile: u8) -> Option<DungeonFieldEffect> {
    match tile {
        0x80 | 0x88 => Some(DungeonFieldEffect::Sleep),
        0x81 | 0x89 => Some(DungeonFieldEffect::PoisonGas),
        0x82 | 0x8a => Some(DungeonFieldEffect::Fire),
        0x83 | 0x8b => Some(DungeonFieldEffect::Electric),
        0x84..=0x8f => Some(DungeonFieldEffect::Energy),
        _ => None,
    }
}

/// `dungeon-mode.md §8`: returns the energy-field base byte for one
/// effect family. Used by both the look-text path (which keys off
/// the base) and the placement path (which writes
/// `base | DUNGEON_VISIT_MARKER_BIT` to preserve the visit-marker
/// bit). Returns `None` for the generic catch-all `Energy` band.
pub const fn dungeon_field_base_byte(effect: DungeonFieldEffect) -> Option<u8> {
    Some(match effect {
        DungeonFieldEffect::Sleep => DUNGEON_FIELD_SLEEP_BASE,
        DungeonFieldEffect::PoisonGas => DUNGEON_FIELD_POISON_GAS_BASE,
        DungeonFieldEffect::Fire => DUNGEON_FIELD_FIRE_BASE,
        DungeonFieldEffect::Electric => DUNGEON_FIELD_ELECTRIC_BASE,
        DungeonFieldEffect::Energy => return None,
    })
}

pub fn is_dungeon_room_trigger(tile: u8) -> bool {
    tile >> 4 == 0x0f
}

pub fn is_dungeon_room_helper_state(tile: u8) -> bool {
    tile >> 4 == 0x0a
}

/// `dungeon-mode.md §5` per-visit room-trigger promotion. When a
/// party walks onto a `0xF?` room-trigger cell, the room-entry
/// helper patches the *loaded* dungeon image by rewriting the cell
/// to the matching `0xA?` room-helper-state value (low nibble
/// preserved so the helper still maps the cell back to the same
/// arena slot). The on-disk `DUNGEON.DAT` source byte is unchanged.
/// Returns `None` for cells outside the `0xF?` trigger family.
pub const fn dungeon_room_trigger_promoted_visit_byte(tile: u8) -> Option<u8> {
    if tile >> 4 == 0x0f {
        Some((tile & 0x0f) | 0xa0)
    } else {
        None
    }
}

/// `formats/dungeon-dat.md §4`: mask isolating the room-trigger
/// low-nibble slot index (`0..=15`) inside an `0xF?` cell byte.
pub const DUNGEON_ROOM_SLOT_MASK: u8 = 0x0F;
/// `formats/dungeon-dat.md §4`: number of room-arena slots per
/// dungeon bank in `DUNGEON.CBT`. Each room-bearing dungeon
/// contributes one 16-slot bank, indexed by the low-nibble slot.
pub const DUNGEON_ROOM_SLOTS_PER_BANK: usize = 16;
/// `formats/dungeon-dat.md §4`: highest dungeon record that shares
/// arena bank `0` with Deceit. The shipped Despise record carries no
/// `0xF?` room-trigger cells, so records `0..=DESPISE_RECORD` map to
/// the same arena bank to keep the arithmetic dense.
pub const DUNGEON_ARENA_BANK_SHARED_RECORD_MAX: usize = 1;

pub fn dungeon_room_slot(tile: u8) -> u8 {
    tile & DUNGEON_ROOM_SLOT_MASK
}

/// `dungeon-mode.md §14`: collapse a raw dungeon record into the
/// `DUNGEON.CBT` arena bank.
///
/// ```text
/// arena_bank = 0 if dungeon_record <= 1 else dungeon_record - 1
/// ```
///
/// "Despise shares the bank-zero arithmetic path, but the stock
/// Despise dungeon record has no `0xF?` room-trigger cells", which is
/// why records `0..=1` collapse onto one bank and every later record
/// shifts down by one. The seven resulting banks are Deceit `0`,
/// Destard `1`, Wrong `2`, Covetous `3`, Shame `4`, Hythloth `5`, and
/// Doom `6`.
pub const fn dungeon_arena_bank(record: usize) -> usize {
    if record <= DUNGEON_ARENA_BANK_SHARED_RECORD_MAX {
        0
    } else {
        record - 1
    }
}

/// `dungeon-mode.md §5,§14` number of `DUNGEON.CBT` arena banks. §14's
/// bank listing runs Deceit `0..15` through Doom `96..111`, and §5
/// makes the same count explicit for the save-image bitmap: "one bit
/// per dungeon-room arena record - one hundred twelve bits, which
/// occupy the first fourteen bytes of the sixteen-byte save-image
/// field ... (the two trailing bytes are never addressed)".
pub const DUNGEON_ARENA_BANK_COUNT: usize = 7;

/// `dungeon-mode.md §5` addressed width of the room-clear bitmap:
/// `DUNGEON_ARENA_BANK_COUNT * DUNGEON_ROOM_SLOTS_PER_BANK / 8` = 14
/// bytes. The save field stays sixteen bytes wide
/// ([`SAVE_DUNGEON_ROOM_CLEAR_BITMAP_LEN`]); the final two bytes are
/// never addressed by either the reader or the writer.
pub const DUNGEON_ROOM_CLEAR_ADDRESSED_BYTES: usize =
    DUNGEON_ARENA_BANK_COUNT * DUNGEON_ROOM_SLOTS_PER_BANK / 8;

/// `dungeon-mode.md §14` raw dungeon record for Wrong (arena bank
/// `2`, arena records `32..47`).
pub const WRONG_DUNGEON_RECORD: u8 = 3;
/// `dungeon-mode.md §14` raw dungeon record for Covetous (arena bank
/// `3`, arena records `48..63`).
pub const COVETOUS_DUNGEON_RECORD: u8 = 4;

/// `dungeon-mode.md §5` resident room-clear writer deny-list: "The
/// list holds six `(dungeon, room)` pairs; when the room being
/// resolved matches one of them, the writer returns without setting
/// anything. In shipped data the deny-listed rooms are rooms one, six,
/// eleven, and twelve of the Wrong bank and rooms zero and eleven of
/// the Covetous bank ... Those six rooms therefore never persist as
/// cleared and re-arm on every visit."
///
/// The key is deliberately the **raw dungeon record**, not the
/// collapsed [`dungeon_arena_bank`] the bit index uses: §5 states "the
/// deny-list is keyed by the raw dungeon record number, while the bit
/// index uses the collapsed arena bank, so an implementation must not
/// reuse one for the other."
pub const DUNGEON_ROOM_CLEAR_DENY_LIST: [(u8, u8); 6] = [
    (WRONG_DUNGEON_RECORD, 1),
    (WRONG_DUNGEON_RECORD, 6),
    (WRONG_DUNGEON_RECORD, 11),
    (WRONG_DUNGEON_RECORD, 12),
    (COVETOUS_DUNGEON_RECORD, 0),
    (COVETOUS_DUNGEON_RECORD, 11),
];

/// `dungeon-mode.md §5`: whether the room-clear bitmap **writer** must
/// return without setting anything for this `(raw dungeon record,
/// room id)` pair. The **reader** applies no deny-list, so it simply
/// always reports these rooms as not cleared.
pub const fn dungeon_room_clear_is_denied(record: u8, room_id: u8) -> bool {
    let mut index = 0;
    while index < DUNGEON_ROOM_CLEAR_DENY_LIST.len() {
        let (denied_record, denied_room) = DUNGEON_ROOM_CLEAR_DENY_LIST[index];
        if denied_record == record && denied_room == room_id {
            return true;
        }
        index += 1;
    }
    false
}

pub fn dungeon_room_arena_index(scene: DungeonScene, tile: u8) -> usize {
    dungeon_arena_bank(scene.record) * DUNGEON_ROOM_SLOTS_PER_BANK
        + dungeon_room_slot(tile) as usize
}

/// `dungeon-mode.md §5`: locate the room-clear bit for one
/// `(scene, room id)` pair. "The bit index is the same
/// `arena_bank * 16 + room_id` value used to select the `DUNGEON.CBT`
/// record (§ 14)", so the position is derived from the collapsed
/// [`dungeon_arena_bank`] and never from the raw record.
fn dungeon_room_clear_bit_slot(scene: DungeonScene, room_id: u8) -> Option<(usize, u8)> {
    let bank = dungeon_arena_bank(scene.record);
    if bank >= DUNGEON_ARENA_BANK_COUNT {
        return None;
    }
    dungeon_room_clear_bit_position(bank as u8, room_id)
}

pub fn dungeon_room_clear_bit_is_set(
    bitmap: &[u8; SAVE_DUNGEON_ROOM_CLEAR_BITMAP_LEN],
    scene: DungeonScene,
    room_id: u8,
) -> bool {
    // `dungeon-mode.md §5`: "The bitmap **reader** applies no
    // deny-list, so it simply always reports those rooms as not
    // cleared" — the writer's guard is what keeps them clear.
    dungeon_room_clear_bit_slot(scene, room_id).is_some_and(|(byte, mask)| bitmap[byte] & mask != 0)
}

pub fn set_dungeon_room_clear_bit(
    bitmap: &mut [u8; SAVE_DUNGEON_ROOM_CLEAR_BITMAP_LEN],
    scene: DungeonScene,
    room_id: u8,
) -> bool {
    // `dungeon-mode.md §5`: "The bitmap **writer** consults a small
    // resident deny-list before setting a bit ... when the room being
    // resolved matches one of them, the writer returns without setting
    // anything." The guard lives in the helper rather than at the
    // single post-combat call site so any future writer is covered.
    if dungeon_room_clear_is_denied(scene.record as u8, room_id) {
        return false;
    }
    let Some((byte, mask)) = dungeon_room_clear_bit_slot(scene, room_id) else {
        return false;
    };
    let was_clear = bitmap[byte] & mask == 0;
    bitmap[byte] |= mask;
    was_clear
}

pub fn apply_dungeon_room_clear_bitmap(
    grid: &mut [u8],
    scene: DungeonScene,
    bitmap: &[u8; SAVE_DUNGEON_ROOM_CLEAR_BITMAP_LEN],
) {
    for cell in grid.iter_mut().take(DUNGEON_RECORD_LEN) {
        if is_dungeon_room_trigger(*cell)
            && dungeon_room_clear_bit_is_set(bitmap, scene, dungeon_room_slot(*cell))
        {
            *cell = 0xA0 | dungeon_room_slot(*cell);
        }
    }
}

pub const TOWN_KLIMB_ASCEND_TILE: u8 = 0xc8;
pub const TOWN_KLIMB_DESCEND_TILE: u8 = 0xc9;
pub const TOWN_KLIMB_DESCEND_GRATE_TILE: u8 = 0x86;
pub const TOWN_KLIMB_ROCKS_TILE: u8 = 0x4c;
pub const TOWN_KLIMB_FENCE_FIRST: u8 = 0xca;
pub const TOWN_KLIMB_FENCE_LAST: u8 = 0xcb;
pub const TOWN_TRAPDOOR_LIVE_TILE: u8 = 0x8c;

/// `town-mode.md §7`: classify the exact live town tile under the
/// party for K-Klimb. Town links are directional; unlike dungeon
/// ladders, no town tile offers both directions.
pub const fn town_klimb_underfoot_intent(tile: u8) -> Option<ClimbIntent> {
    match tile {
        TOWN_KLIMB_ASCEND_TILE => Some(ClimbIntent::Up),
        TOWN_KLIMB_DESCEND_TILE | TOWN_KLIMB_DESCEND_GRATE_TILE => Some(ClimbIntent::Down),
        _ => None,
    }
}

/// Compatibility name retained for callers that need the signed floor
/// delta of an exact town K-Klimb link.
pub const fn stair_delta(tile: u8, intent: ClimbIntent) -> Option<i8> {
    match (town_klimb_underfoot_intent(tile), intent) {
        (Some(actual), requested) if actual as u8 == requested as u8 => {
            Some(town_climb_delta(intent))
        }
        _ => None,
    }
}

/// `town-mode.md §7`: exact adjacent cells the town K-Klimb direction
/// prompt permits the party to climb over without changing floors.
pub const fn town_klimb_over_target(tile: u8) -> bool {
    matches!(
        tile,
        TOWN_KLIMB_ROCKS_TILE | TOWN_KLIMB_FENCE_FIRST..=TOWN_KLIMB_FENCE_LAST
    )
}

/// `town-mode.md §10`: the exact live tile that runs the town trapdoor
/// underfoot reaction after a consumed action.
pub const fn is_town_trapdoor_live_tile(tile: u8) -> bool {
    tile == TOWN_TRAPDOOR_LIVE_TILE
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

pub const fn town_climb_delta(intent: ClimbIntent) -> i8 {
    match intent {
        ClimbIntent::Up => 1,
        ClimbIntent::Down => -1,
    }
}

/// `dungeon-mode.md §13.1`: the level delta K-Klimb applies for the
/// underfoot cell and the chosen direction, or `None` when that cell
/// offers nothing in that direction.
///
/// The dispatcher masks the underfoot byte to its high nibble before
/// any comparison, so the whole pit family `0x6?` - not just the exact
/// byte `0x60`, and including the marked and fired variants - enables
/// the down arm and steps one level exactly as a down ladder does.
pub fn dungeon_ladder_delta(tile: u8, intent: ClimbIntent) -> Option<i8> {
    match (tile >> 4, intent) {
        (0x1, ClimbIntent::Up) => Some(-1),
        (0x2, ClimbIntent::Down) => Some(1),
        (0x3, ClimbIntent::Up) => Some(-1),
        (0x3, ClimbIntent::Down) => Some(1),
        (0x6, ClimbIntent::Down) => Some(1),
        _ => None,
    }
}

/// `dungeon-mode.md §13.1`: the destination test belonging to the
/// **level-change spells** (Up and Down, `catalogs/spell-list.md` ids
/// 21 and 22), which refuse a destination cell in the base `0x0`
/// class or in the wall and door-presentation families `0xB?` through
/// `0xE?`.
///
/// This test is *not* part of K-Klimb: a climb never inspects the cell
/// it lands on, and the ladder or pit underfoot is treated as proof
/// enough that the destination is reachable. An earlier spec revision
/// applied this test to the climb route; that claim is withdrawn.
pub const fn dungeon_level_change_spell_destination_allowed(tile: u8) -> bool {
    !matches!(tile >> 4, 0x0 | 0x0b..=0x0e)
}

pub fn render_glyph(tile: u8) -> char {
    // The moon-gate tile has a dedicated glyph. `overworld.md §9` (spec
    // HEAD c00bf63) retracts the frame ring an earlier revision assumed,
    // so this is one tile id, not a range.
    if tile == MOONGATE_TILE_BASE {
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
        96..=103 => '~',
        104..=127 => '*',
        128..=159 => '^',
        160..=191 => 'v',
        192..=255 => 'n',
    }
}

pub fn surface_view_class(tile: u8) -> u8 {
    crate::view_classes::tile_view_class(tile)
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

/// `formats/npc.md §6`: a roster slot's type byte "supplies the NPC's
/// sprite/tile classifier", and "the runtime sprite tile is derived by
/// adding the byte to the NPC sprite page". The NPC sprite page is the
/// actor bank base - `catalogs/tile-catalog.md §3.1`: "An actor's
/// stored byte is a value in `0..255` and the renderer adds **256** to
/// it before indexing this catalogue" - which this engine applies in
/// [`actor_tile_for_byte`]. So an ordinary roster tag *is* the actor
/// byte and needs no remapping at all.
///
/// `catalogs/npc-roster.md §4` confirms the identity from the art side:
/// tag `B8` is `a gargoyle`, tag `D8` `a daemon`, tag `90` `a rodent of
/// unusual size`, tag `94` `a bat`, tag `FC` `a shadow lord`
/// ("`catalogs/monster-bestiary.md`, class 47"). Those are bestiary
/// classes 30, 38, 20, 21 and 47, and `class * 4 + 0x40` reproduces
/// every one of the five tags exactly - the same relation
/// [`crate::combat_class_sprite_byte`] implements for combat classes.
/// The roster tag therefore lives in the same actor-byte domain, and
/// the person classes below `0x80` (`40` wizard, `44`/`5C` minstrel,
/// `48` fighter, `50` villager, `54` merchant, `58` jester, `68`
/// child, `6C` beggar, `70` guard, `78` Blackthorn) resolve the same
/// way.
///
/// Tag `0x01` is the one published exception: "the sprite-link helper
/// forces the standard person tile instead of using the tag as a direct
/// sprite class" (`catalogs/npc-roster.md §4`, `formats/npc.md §6`
/// row `1`). See [`NPC_DEFAULT_PERSON_SPRITE_TILE`].
///
/// The withdrawn implementation clamped every byte outside `192..=255`
/// to the literal `192`, which is bestiary class 32 in the actor bank -
/// so all three hundred and twenty-two ordinary roster slots in the
/// shipped `.NPC` files drew one single monster sprite.
pub fn npc_tile(type_byte: u8) -> u8 {
    match npc_type_byte_class(type_byte) {
        NpcTypeByteClass::DefaultHumanSprite => NPC_DEFAULT_PERSON_SPRITE_TILE,
        _ => type_byte,
    }
}

/// `npc-schedules.md §11`: the world-mutation primitive fills a freshly
/// allocated active-object slot "with the NPC's tile, type, and new
/// coordinates" - the roster type byte and the derived sprite tile are
/// two separate fields, and they differ for the tag-`0x01` sentinel.
pub fn npc_active_object(type_byte: u8, x: usize, y: usize, z: u8) -> ActiveObject {
    let tile = npc_tile(type_byte);
    ActiveObject {
        type_byte,
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
    object.type_byte == npc.type_byte
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
        96..=103 => "river",
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
    if let Some(family) = boardable_family(type_byte) {
        return match family {
            BoardableFamily::Horse => {
                let marker = mount_horse_marker(type_byte)?;
                let tile = transport_visual_tile_for_marker(marker)?;
                Some(TransportState::Horse {
                    type_byte: marker,
                    tile,
                })
            }
            BoardableFamily::MagicCarpet => {
                let marker = CARPET_MOUNTED;
                let tile = transport_visual_tile_for_marker(marker)?;
                Some(TransportState::Carpet {
                    type_byte: marker,
                    tile,
                })
            }
            BoardableFamily::Ship => {
                let tile = transport_visual_tile_for_marker(type_byte)?;
                Some(TransportState::Ship {
                    type_byte,
                    tile,
                    sails_hoisted: false,
                    hull: aux1,
                    skiffs: aux3,
                })
            }
            BoardableFamily::Skiff => {
                let tile = transport_visual_tile_for_marker(type_byte)?;
                Some(TransportState::Skiff { type_byte, tile })
            }
        };
    }

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
    // vehicles.md §2 lists marker `0x00` as "Party sprite suppressed", but
    // decoding it here would be wrong: the shipped chargen template also
    // leaves this byte zero before the first overworld entry, and §2 is
    // explicit that `0x1C` is "[t]he clean seed and default state". The
    // suppressed marker is therefore produced only by the loss-of-ship
    // ladder at runtime, and a save that carries it decodes as foot. See
    // the note on the asymmetry in `TransportState::SpriteSuppressed`.
    let Some(family) = transport_family(marker) else {
        return transport_from_vehicle_object(marker, marker, 0, 0).unwrap_or_default();
    };
    match family {
        TransportFamily::MountedHorse => TransportState::Horse {
            type_byte: marker,
            tile: transport_visual_tile_for_marker(marker).unwrap_or(FIRST_PLAYABLE_HORSE_TILE),
        },
        TransportFamily::MagicCarpet => TransportState::Carpet {
            type_byte: marker,
            tile: transport_visual_tile_for_marker(marker)
                .unwrap_or(FIRST_PLAYABLE_MAGIC_CARPET_TILE),
        },
        TransportFamily::Foot => TransportState::Foot,
        TransportFamily::ShipHoisted | TransportFamily::ShipFurled => TransportState::Ship {
            type_byte: marker,
            tile: transport_visual_tile_for_marker(marker).unwrap_or(FIRST_PLAYABLE_FRIGATE_TILE),
            sails_hoisted: matches!(family, TransportFamily::ShipHoisted),
            hull: 0,
            skiffs: 0,
        },
        TransportFamily::Skiff => TransportState::Skiff {
            type_byte: marker,
            tile: transport_visual_tile_for_marker(marker).unwrap_or(FIRST_PLAYABLE_SKIFF_TILE),
        },
    }
}

pub const fn transport_visual_tile_for_marker(marker: u8) -> Option<u8> {
    Some(match transport_family(marker) {
        Some(TransportFamily::MountedHorse) => {
            FIRST_PLAYABLE_HORSE_TILE + (marker - HORSE_TRANSPORT_FIRST)
        }
        Some(TransportFamily::MagicCarpet) => {
            FIRST_PLAYABLE_MAGIC_CARPET_TILE + (marker & TRANSPORT_FACING_MASK)
        }
        Some(TransportFamily::ShipHoisted) | Some(TransportFamily::ShipFurled) => {
            FIRST_PLAYABLE_FRIGATE_TILE + (marker & TRANSPORT_FACING_MASK)
        }
        Some(TransportFamily::Skiff) => {
            FIRST_PLAYABLE_SKIFF_TILE + (marker & TRANSPORT_FACING_MASK)
        }
        Some(TransportFamily::Foot) => PLAYER_TILE,
        None => return None,
    })
}

pub const fn transport_marker_for_vehicle_bytes(
    type_byte: u8,
    tile: u8,
    sails_hoisted: bool,
) -> Option<u8> {
    if let Some(family) = transport_family(type_byte) {
        let facing = type_byte & TRANSPORT_FACING_MASK;
        return Some(match family {
            TransportFamily::MountedHorse => {
                HORSE_TRANSPORT_FIRST + ((type_byte - HORSE_TRANSPORT_FIRST) & 0x01)
            }
            TransportFamily::MagicCarpet => TRANSPORT_MARKER_MAGIC_CARPET_FIRST + facing,
            TransportFamily::Foot => TRANSPORT_MARKER_FOOT_FIRST + facing,
            TransportFamily::ShipHoisted | TransportFamily::ShipFurled => {
                if sails_hoisted {
                    TRANSPORT_MARKER_SHIP_HOISTED_FIRST + facing
                } else {
                    TRANSPORT_MARKER_SHIP_FURLED_FIRST + facing
                }
            }
            TransportFamily::Skiff => TRANSPORT_MARKER_SKIFF_FIRST + facing,
        });
    }
    transport_marker_for_visual_tile(tile, sails_hoisted)
}

pub const fn transport_marker_for_visual_tile(tile: u8, sails_hoisted: bool) -> Option<u8> {
    Some(match tile {
        160..=167 => HORSE_TRANSPORT_FIRST + ((tile - FIRST_PLAYABLE_HORSE_TILE) & 0x01),
        168..=175 => {
            let facing = (tile - FIRST_PLAYABLE_FRIGATE_TILE) & TRANSPORT_FACING_MASK;
            if sails_hoisted {
                TRANSPORT_MARKER_SHIP_HOISTED_FIRST + facing
            } else {
                TRANSPORT_MARKER_SHIP_FURLED_FIRST + facing
            }
        }
        176..=183 => {
            TRANSPORT_MARKER_SKIFF_FIRST
                + ((tile - FIRST_PLAYABLE_SKIFF_TILE) & TRANSPORT_FACING_MASK)
        }
        184..=187 => {
            TRANSPORT_MARKER_MAGIC_CARPET_FIRST
                + ((tile - FIRST_PLAYABLE_MAGIC_CARPET_TILE) & TRANSPORT_FACING_MASK)
        }
        _ => return None,
    })
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

/// `dungeon-mode.md §8` fountain sub-type classifier. The low nibble
/// of a `0x5?` fountain cell drives the drink effect on the selected
/// party member; the high nibble is the fountain class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DungeonFountainEffect {
    /// `0x50` — Cure: status flips to Good. "Cured!".
    Cure,
    /// `0x51` — Heal: current HP rises to maximum. "Healed!".
    Heal,
    /// `0x52` — Poison: status flips to Poisoned. "Poisoned!".
    Poison,
    /// `0x53..=0x5F` — Bad taste: roll `0..=7` HP damage. "Bad taste.".
    BadTaste,
}

/// `dungeon-mode.md §8` Bad-taste fountain damage upper bound. The
/// shared helper rolls in the inclusive `0..=DUNGEON_FOUNTAIN_BAD_TASTE_DAMAGE_MAX`
/// range; the spec's "random HP-damage roll in the inclusive range
/// 0..7" exclusive-7 phrasing matches `seed % 8 = 0..=7`.
pub const DUNGEON_FOUNTAIN_BAD_TASTE_DAMAGE_MAX: u8 = 7;

/// `dungeon-mode.md §8`: classify a fountain cell byte by its low
/// nibble. Returns `None` for cells outside the `0x5?` fountain
/// class.
pub const fn dungeon_fountain_effect(tile: u8) -> Option<DungeonFountainEffect> {
    if tile >> 4 != 0x5 {
        return None;
    }
    Some(match tile & 0x0F {
        0 => DungeonFountainEffect::Cure,
        1 => DungeonFountainEffect::Heal,
        2 => DungeonFountainEffect::Poison,
        _ => DungeonFountainEffect::BadTaste,
    })
}

/// `dungeon-mode.md §6` first-person renderer wall-class predicate.
/// The renderer paints a wall cue when the high nibble identifies a
/// wall or door-presentation class (`0xB..=0xE`) or the `0xF?`
/// room-trigger threshold.
/// Open passages and the other low-nibble classes paint as void/floor
/// instead. `0xF?` cells remain walkable gameplay triggers; the wall
/// cue is a first-person presentation choice, not a movement blocker.
pub const fn dungeon_renderer_paints_wall_cue(tile: u8) -> bool {
    matches!(tile >> 4, 0xB..=0xE | 0xF)
}

/// `dungeon-mode.md §13` K-Klimb apply-path outcome for the underfoot
/// dungeon cell. The handler reads only the high nibble to decide
/// whether to change Z, prompt the player, or refuse the climb. No
/// outcome reaches the surface-reset helper directly: the shared exit
/// contract of §13.2 runs only when the level step reports an edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DungeonKlimbApply {
    /// `0x1?` up ladder — decrement Z. Refuses when already on
    /// surface (`Z == 0`).
    UpLadder,
    /// `0x2?` down ladder — increment Z. Refuses when already on the
    /// deepest level (`Z == DUNGEON_DEEPEST_LEVEL`).
    DownLadder,
    /// `0x3?` two-way ladder — prompt the player for up or down,
    /// then dispatch.
    TwoWayPrompt,
    /// Pit family `0x6?` - offered as a climb-*down*. The dispatcher
    /// masks to the high nibble, so the marked and fired variants
    /// behave the same as the plain byte `0x60`, and the arm calls the
    /// same level-step helper a down ladder uses. An earlier spec
    /// revision claimed exact `0x60` bypassed the ordinary apply path
    /// and invoked the surface-reset helper directly; that claim is
    /// withdrawn, and it is contradicted by shipped data - Destard
    /// level zero carries `0x60` at (7, 3) and (1, 7) and Deceit level
    /// zero at (1, 3), and klimbing there descends to level one.
    PitDescent,
    /// Any other underfoot byte — K-Klimb returns without a level
    /// change.
    NoLevelChange,
}

/// `dungeon-mode.md §13`: classify the underfoot dungeon byte into
/// the K-Klimb apply-path outcome. Every comparison is made on the
/// high nibble alone, so the whole pit family shares one outcome.
pub const fn dungeon_klimb_apply(tile: u8) -> DungeonKlimbApply {
    match tile >> 4 {
        0x1 => DungeonKlimbApply::UpLadder,
        0x2 => DungeonKlimbApply::DownLadder,
        0x3 => DungeonKlimbApply::TwoWayPrompt,
        0x6 => DungeonKlimbApply::PitDescent,
        _ => DungeonKlimbApply::NoLevelChange,
    }
}

/// `dungeon-mode.md §8`: Search rewrites `0x61` (secret-door reveal)
/// to `0x60` for the current visit and marks the same X/Y cell one
/// level below with the visit bit `0x08` (when not already on the
/// deepest level). The deepest dungeon level index is one less than
/// the per-record level count. Anchored to
/// [`crate::DUNGEON_LEVELS_PER_RECORD`] - 1 so the deepest-level
/// index derives from the dungeon record layout.
pub const DUNGEON_DEEPEST_LEVEL: u8 = crate::DUNGEON_LEVELS_PER_RECORD as u8 - 1;
pub const DUNGEON_VISIT_MARKER_BIT: u8 = 0x08;

/// `dungeon-mode.md §8`: stepping into an automatic fall trap
/// (`0x61`/`0x69`) lands the party at the same X/Y on the next level
/// and marks bit `0x08` in the destination cell *only when* it is
/// below the wall/door band (`< 0x90`).
pub const DUNGEON_WALL_DOOR_BAND_FIRST: u8 = 0x90;
pub const fn dungeon_fall_destination_marks_visit(destination_byte: u8) -> bool {
    destination_byte < DUNGEON_WALL_DOOR_BAND_FIRST
}

/// `dungeon-mode.md §6.1`: renderer-facing cell-read normaliser. For
/// cell bytes below [`DUNGEON_WALL_DOOR_BAND_FIRST`] (`0x90`), bit
/// [`DUNGEON_VISIT_MARKER_BIT`] (`0x08`) is ignored by clearing it
/// before the renderer's class interpretation. For classes `0x9?`
/// and higher the bit remains meaningful as a render-side
/// overlay/extra-glyph flag. The static dungeon record is not
/// modified; this is a read-side transform.
pub const fn dungeon_render_cell_byte(raw_byte: u8) -> u8 {
    if raw_byte < DUNGEON_WALL_DOOR_BAND_FIRST {
        raw_byte & !DUNGEON_VISIT_MARKER_BIT
    } else {
        raw_byte
    }
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
    match tile & DUNGEON_CELL_LOW_NIBBLE_MASK {
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

/// `formats/dungeon-dat.md §3` low-nibble mask for dungeon cell bytes.
/// The cell byte's upper nibble selects the broad dispatch class; its
/// lower nibble carries the class-specific attribute/subtype/sentinel.
/// Promote the mask so subtype helpers (fountain effect, energy-field
/// sub-type, etc.) share one source of truth instead of repeating
/// `& 0x0F`.
pub const DUNGEON_CELL_LOW_NIBBLE_MASK: u8 = 0x0F;

/// `formats/dungeon-dat.md §3` high-nibble right shift for dungeon
/// cell bytes. Shifting the cell byte right by this amount yields
/// the broad cell-class index in `0..=15` consumed by
/// `dungeon_cell_class_of` and the renderer's wall/door/passage
/// dispatch.
pub const DUNGEON_CELL_HIGH_NIBBLE_SHIFT: u32 = 4;

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
    HeavyDoorVariant,
    RoomTrigger,
}

/// `dungeon-mode.md §13` Z-axis floor bounds. Dungeon levels are
/// indexed `0..=7` with zero at the top and seven at the deepest
/// floor. K-Klimb refuses to step above level zero or below level
/// seven; the pit-chain off-bottom path is the only Z mutation that
/// can leave the level byte at the incremented value above seven.
pub const DUNGEON_LEVEL_TOP: u8 = 0;
pub const DUNGEON_LEVEL_BOTTOM: u8 = 7;

/// `dungeon-mode.md §13` K-Klimb requested Z direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KlimbDirection {
    /// Up ladder step — decrement Z toward the surface.
    Up,
    /// Down ladder step — increment Z toward the deepest floor.
    Down,
}

/// `dungeon-mode.md §13`: apply a K-Klimb ladder step. Returns the
/// new Z when the step is accepted, or `None` when the apply path
/// refuses (already at level zero for Up, already at level seven
/// for Down). Caller should still test the destination cell with
/// the obstruction check before honouring the new Z.
pub const fn dungeon_klimb_z_step(z: u8, direction: KlimbDirection) -> Option<u8> {
    match direction {
        KlimbDirection::Up => {
            if z > DUNGEON_LEVEL_TOP {
                Some(z - 1)
            } else {
                None
            }
        }
        KlimbDirection::Down => {
            if z < DUNGEON_LEVEL_BOTTOM {
                Some(z + 1)
            } else {
                None
            }
        }
    }
}

/// `dungeon-mode.md §12` V-View minimap flood expansion rule. The
/// per-cell painter returns "expand" for most classes after painting
/// the glyph; only the wall presentation classes `0xB?`, `0xC?`, and
/// `0xD?` stop the flood walker. Room-helper / wall-variant /
/// room-trigger families (`0xA?`, `0xE?`, `0xF?`) still expand even
/// though they paint a door-like glyph.
pub const fn dungeon_minimap_flood_expands(tile: u8) -> bool {
    !matches!(tile >> 4, 0xB | 0xC | 0xD)
}

/// `dungeon-mode.md §12.3`: which of the engine's two fixed
/// eight-by-eight one-bit fonts a minimap class selects. Each font holds
/// one hundred twenty-eight glyphs of eight bytes, one byte per row, most
/// significant bit leftmost. Most classes select the runic font; exactly
/// four select the text font — three directional arrows the runic font
/// does not have, and the solid block whose runic slot at that index is
/// blank, "which is precisely why the bedrock class does not switch
/// fonts".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DungeonMinimapFont {
    /// The text font (`IBM.CH` family).
    Text,
    /// The runic font (`RUNES.CH` family).
    Runic,
}

/// `dungeon-mode.md §12.4`: what the V-View minimap painter draws for one
/// dungeon cell. §12.3 says two classes "are not font characters at all
/// but small vector drawings", so this is a sum type rather than a bare
/// glyph index — the fountain and the energy field have no font index to
/// return, and giving them one is what made class `0x5?` collide with
/// exact byte `0x68`'s published up-and-down arrow.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DungeonMinimapGlyph {
    /// A character from one of the two fixed fonts.
    Font {
        /// Index into the selected font, `0..=127`.
        index: u8,
        /// Which font the class selects.
        font: DungeonMinimapFont,
    },
    /// `dungeon-mode.md §12.5` fountain vector drawing — a basin in the
    /// bright foreground pen plus a jet and spray in a brighter blue,
    /// inside one eight-by-eight cell.
    Fountain,
    /// `dungeon-mode.md §12.5` energy-field vector drawing — eight
    /// full-width horizontal runs covering all eight rows in four
    /// two-row colour bands. It reads no sub-type, so all four field
    /// flavours look identical on the map.
    EnergyField,
}

impl DungeonMinimapGlyph {
    /// A glyph taken from the text font.
    pub const fn text(index: u8) -> Self {
        Self::Font {
            index,
            font: DungeonMinimapFont::Text,
        }
    }

    /// A glyph taken from the runic font.
    pub const fn runic(index: u8) -> Self {
        Self::Font {
            index,
            font: DungeonMinimapFont::Runic,
        }
    }

    /// The font index, or `None` for the two vector drawings.
    pub const fn font_index(self) -> Option<u8> {
        match self {
            Self::Font { index, .. } => Some(index),
            Self::Fountain | Self::EnergyField => None,
        }
    }
}

/// `dungeon-mode.md §12.4` party marker: "Arrowhead glyph `0x60`, drawn
/// unconditionally at the centre cell `(11,11)`", from the runic font.
pub const DUNGEON_MINIMAP_PARTY_GLYPH: DungeonMinimapGlyph = DungeonMinimapGlyph::runic(0x60);

/// `dungeon-mode.md §12.4` V-View minimap output for one dungeon cell
/// byte, or `None` for the classes the painter intentionally leaves black
/// (`0x0?` without bit `0x08`, `0x7?`, and `0x9?`).
///
/// *Corrected:* this returned a bare `Option<u8>` glyph code and mapped
/// two classes to the wrong output. Class `0x5?` returned `0x12`, the
/// published glyph of *exact byte* `0x68`'s up-and-down arrow, so two
/// distinct classes painted one glyph; §12.4 gives `0x5?` a vector
/// fountain drawing instead. Class `0x8?` returned `0x18`, the `0x0?`
/// up-arrow, where §12.4 gives it the vector energy-field drawing. Both
/// vector drawings are published in §12.5.
pub const fn dungeon_minimap_glyph(tile: u8) -> Option<DungeonMinimapGlyph> {
    // Exact-byte cases inside 0x6? must be tested before the band.
    match tile {
        // Exact 0x60 — down-arrow, text font.
        0x60 => return Some(DungeonMinimapGlyph::text(0x19)),
        // Exact 0x61 / 0x69 — hidden/fall-pit, runic.
        0x61 | 0x69 => return Some(DungeonMinimapGlyph::runic(0x71)),
        // Exact 0x68 — up-and-down arrow, text font. This is the only
        // published owner of glyph 0x12.
        0x68 => return Some(DungeonMinimapGlyph::text(0x12)),
        _ => {}
    }
    Some(match tile >> 4 {
        0x0 => {
            if tile & DUNGEON_VISIT_MARKER_BIT != 0 {
                // Up-arrow, text font.
                DungeonMinimapGlyph::text(0x18)
            } else {
                return None;
            }
        }
        0x1 => DungeonMinimapGlyph::runic(0x2E),
        0x2 => DungeonMinimapGlyph::runic(0x2D),
        0x3 => DungeonMinimapGlyph::runic(0x2F),
        0x4 => DungeonMinimapGlyph::runic(0x70),
        0x5 => DungeonMinimapGlyph::Fountain,
        0x6 => DungeonMinimapGlyph::runic(0x72),
        0x7 => return None,
        0x8 => DungeonMinimapGlyph::EnergyField,
        0x9 => return None,
        0xA | 0xF => DungeonMinimapGlyph::runic(0x73),
        0xB => {
            if tile == 0xB0 {
                // Bedrock keeps the text font: the runic slot at 0x7F is
                // blank, which is exactly why this class does not switch.
                DungeonMinimapGlyph::text(0x7F)
            } else {
                DungeonMinimapGlyph::runic(0x74)
            }
        }
        0xC => DungeonMinimapGlyph::runic(0x75),
        0xD => DungeonMinimapGlyph::runic(0x76),
        0xE => DungeonMinimapGlyph::runic(0x77),
        _ => return None,
    })
}

/// `dungeon-mode.md §10` post-combat Z-axis intent the dungeon
/// A-Attack handler honours after combat returns: "result code five
/// moves the party one level **up** — it decrements the level byte
/// ... result code six moves the party one level **down** — it
/// increments the level byte". Every other code keeps the party on
/// the current level. This is the same polarity as K-Klimb in §13: a
/// smaller level byte is nearer the surface. When the step reaches a
/// level edge the surface-exit path of §13.2 runs — off the top onto
/// Britannia, off the bottom into the Underworld.
pub const fn dungeon_attack_post_combat_z_intent(result_code: u8) -> Option<i8> {
    match result_code {
        5 => Some(-1),
        6 => Some(1),
        _ => None,
    }
}

/// `dungeon-mode.md §8` Search-on-wall rewrite outcome. The Search
/// command can convert flavour-wall and hidden-wall cells into the
/// matching revealed sub-class for the current dungeon visit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DungeonSearchWallRewrite {
    /// Flavour class `0xC?` low nibbles `1` and `2` only narrate the
    /// inspected feature; the cell is not rewritten.
    NarrateOnly,
    /// Other flavour `0xC?` cells convert to `0xB0` or `0xB8`,
    /// preserving the visit-marker bit.
    ToFlavourFind(u8),
    /// Hidden-wall `0xD?` cells convert to `0xE0` or `0xE8`,
    /// preserving the visit-marker bit.
    ToHiddenWallReveal(u8),
}

/// `dungeon-mode.md §8`: classify the Search outcome on a flavour-
/// or hidden-wall cell. Returns `None` for any byte outside the
/// `0xC?` and `0xD?` classes; those classes have no Search-specific
/// rewrite.
pub const fn dungeon_search_wall_rewrite(tile: u8) -> Option<DungeonSearchWallRewrite> {
    let marker = tile & DUNGEON_RUNTIME_VARIANT_BIT;
    match tile >> 4 {
        0xC => match tile & 0x0F {
            // Marker-form variants of "1" and "2" (0x09, 0x0A) also
            // narrate only — the spec rewrite excludes those values.
            0x01 | 0x02 | 0x09 | 0x0A => Some(DungeonSearchWallRewrite::NarrateOnly),
            _ => Some(DungeonSearchWallRewrite::ToFlavourFind(0xB0 | marker)),
        },
        0xD => Some(DungeonSearchWallRewrite::ToHiddenWallReveal(0xE0 | marker)),
        _ => None,
    }
}

/// `dungeon-mode.md §8`: dungeon chest `Get` consumes the open chest
/// by clearing its chest class in the loaded dungeon image. The
/// visit-marker bit (`0x08`) is preserved so a follow-up Search /
/// renderer pass still sees the cell as visited; every other bit is
/// reset to passage. Returns `None` for any byte that is not in the
/// `0x4?` chest class.
pub const fn dungeon_chest_post_get_byte(tile: u8) -> Option<u8> {
    if tile >> 4 != 0x4 {
        return None;
    }
    Some(tile & DUNGEON_RUNTIME_VARIANT_BIT)
}

/// `dungeon-mode.md §6.1` runtime-variant bit (`0x08`). For dungeon
/// cells below `0x90` the renderer clears this bit before class
/// interpretation; for classes `0x9?` and higher the bit remains
/// meaningful as an extra-glyph / active-object overlay flag.
pub const DUNGEON_RUNTIME_VARIANT_BIT: u8 = 0x08;

/// `dungeon-mode.md §6.1`: returns the cell byte the first-person
/// renderer's class-interpretation pass should see. Bytes below
/// `0x90` strip the `0x08` runtime-variant bit; bytes from `0x90`
/// onward are returned unchanged so the renderer can read the bit
/// as an extra-glyph / active-object overlay.
pub const fn dungeon_renderer_cell_byte(tile: u8) -> u8 {
    if tile < 0x90 {
        tile & !DUNGEON_RUNTIME_VARIANT_BIT
    } else {
        tile
    }
}

/// `dungeon-mode.md §6.1`: every renderer-facing cell read wraps X
/// and Y independently to the range `0..=7`. The 8-by-8 floor torus
/// uses a simple low-three-bit mask for the wrap.
pub const fn dungeon_floor_wrap_coord(coord: i16) -> u8 {
    (coord.rem_euclid(8) & 7) as u8
}

/// `dungeon-mode.md §8` Search reveal of pit-class `0x61`. When the
/// searched cell is the unmarked secret-pit byte, the handler reports
/// "a found secret door", rewrites the searched cell to plain pit
/// `0x60` for the rest of the visit, and — unless the party is already
/// on the deepest level — stamps the runtime-variant visit bit on the
/// same X/Y cell one level below. Other pit-family bytes do not take
/// this branch. Returns the destination-level stamp instruction so
/// the caller knows whether to write the visit bit on the cell below
/// (or skip it on the deepest level).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DungeonSearchSecretPitReveal {
    /// Searched cell rewrites to `0x60` and the cell at the same X/Y
    /// on level `z + 1` gets the runtime-variant visit bit stamped.
    RewriteAndStampLevelBelow,
    /// Same rewrite, but the party is already on the deepest level so
    /// no cell-below stamp is performed.
    RewriteOnly,
}

/// `dungeon-mode.md §8`: returns `Some(reveal)` when Search on a
/// secret pit `0x61` should fire, and `None` for any other byte.
/// `current_z` is the dungeon level the party is on (`0..=7`).
pub const fn dungeon_search_secret_pit_reveal(
    searched_byte: u8,
    current_z: u8,
) -> Option<DungeonSearchSecretPitReveal> {
    if searched_byte != 0x61 {
        return None;
    }
    if current_z >= DUNGEON_LEVEL_BOTTOM {
        Some(DungeonSearchSecretPitReveal::RewriteOnly)
    } else {
        Some(DungeonSearchSecretPitReveal::RewriteAndStampLevelBelow)
    }
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

/// `dungeon-mode.md §4.1`: maximum random placement attempts the
/// dungeon active-object setup helper makes on the current 8x8 level
/// before clearing the active-object coordinates and sprite marker.
pub const DUNGEON_ACTIVE_OBJECT_PLACEMENT_ATTEMPTS: u8 = 8;

/// `dungeon-mode.md §4.1`: first tile id in the dungeon active-object
/// spawn family. The placement helper accepts only cells whose byte
/// falls in the pit (`0x6?`) or corridor (`0x7?`) classes.
pub const DUNGEON_ACTIVE_OBJECT_SPAWN_TILE_FIRST: u8 = 0x60;
/// `dungeon-mode.md §4.1`: last tile id in the dungeon active-object
/// spawn family.
pub const DUNGEON_ACTIVE_OBJECT_SPAWN_TILE_LAST: u8 = 0x7F;

/// `dungeon-mode.md §4.1`: returns `true` when `tile` is a legal cell
/// for dungeon active-object placement (pit/corridor classes only).
/// The party's current cell must still be rejected by the caller.
pub const fn dungeon_active_object_spawn_accepts(tile: u8) -> bool {
    tile >= DUNGEON_ACTIVE_OBJECT_SPAWN_TILE_FIRST && tile <= DUNGEON_ACTIVE_OBJECT_SPAWN_TILE_LAST
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
    match tile >> DUNGEON_CELL_HIGH_NIBBLE_SHIFT {
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
        0xB..=0xD => DungeonCellClass::Wall,
        0xE => DungeonCellClass::HeavyDoorVariant,
        // 0xF and any (impossible) higher value
        _ => DungeonCellClass::RoomTrigger,
    }
}

impl DungeonCellClass {
    /// `dungeon-mode.md §3/§8`: solid-blocker wall classes. `0xE?`
    /// remains a separate door-presentation variant for rendering and
    /// minimap glyphs, but ordinary movement treats it as pass-through.
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
        0xB..=0xD => "wall",
        0xE => "heavy-door variant",
        0xF => "room trigger",
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
            0xB..=0xD => "a wall",
            0xE => "a heavy-door variant",
            0xF => "a room trigger",
            _ => "unknown dungeon cell",
        },
    }
}

/// `dungeon-mode.md` Section 8.1, "Post-action underfoot consequences": the
/// exact line an underfoot energy field prints, **before** its per-member
/// rolls, so the line appears even when nobody is affected.
///
/// Electric contact is deliberately absent: it is a movement-time
/// consequence with its own two-line pair, printed before the
/// destination-class test. `Energy` - the generic `0x9?`/`0x84..0x8F`
/// collapse - is the table's "Any other underfoot byte: nothing" row.
pub const fn dungeon_field_consequence_line(field: DungeonFieldEffect) -> Option<&'static str> {
    match field {
        DungeonFieldEffect::Sleep => Some(crate::DUNGEON_SLEEP_FIELD_LINE),
        DungeonFieldEffect::PoisonGas => Some(crate::DUNGEON_POISON_FIELD_LINE),
        DungeonFieldEffect::Fire => Some(crate::DUNGEON_FIRE_FIELD_LINE),
        DungeonFieldEffect::Electric | DungeonFieldEffect::Energy => None,
    }
}

/// `dungeon-mode.md` Section 8.1, "Search outcomes": the one outcome line a
/// class prints after the unconditional `You find:` preamble.
///
/// The pit family, the chest class and the two rewriting wall branches are
/// **not** here - they have their own arms with state changes attached, and
/// they select from `DUNGEON_SEARCH_A_PIT`, the four trap-tier lines,
/// `DUNGEON_SEARCH_HIDDEN_DOOR` and the skeleton pair. `None` means "no
/// published outcome literal for this class".
///
/// One hedge, marked rather than hidden: Section 8.1 names both flavour
/// lines - `Nothing on the stalactite.` and `Nothing in the caved in
/// passage.` - and Section 8 names the two narrate-only flavour sub-values
/// `1` and `2`, but the spec never joins one to the other. The pairing below
/// is this engine's assignment, not a published fact.
pub const fn dungeon_search_outcome_line(tile: u8) -> Option<&'static str> {
    match tile >> 4 {
        0x0 => Some(crate::DUNGEON_SEARCH_NOTHING_OF_NOTE),
        0x1 | 0x2 | 0x3 => Some(crate::DUNGEON_SEARCH_NOTHING_ON_LADDER),
        0x5 => Some(crate::DUNGEON_SEARCH_NOTHING_ON_FOUNTAIN),
        0x7 => Some(crate::DUNGEON_SEARCH_TREASURE),
        0x9 => Some(crate::DUNGEON_SEARCH_IMPOSSIBLE_TILE),
        // "for the heavy-door class and for both door-presentation/room
        // classes".
        0xA | 0xE | 0xF => Some(crate::DUNGEON_SEARCH_NOTHING_ON_DOOR),
        0xB => Some(crate::DUNGEON_SEARCH_NOTHING_ON_WALL),
        0xC => match tile & 0x07 {
            0x01 => Some(crate::DUNGEON_SEARCH_NOTHING_ON_STALACTITE),
            0x02 => Some(crate::DUNGEON_SEARCH_NOTHING_IN_CAVED_IN_PASSAGE),
            _ => None,
        },
        _ => None,
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
        0xB..=0xD => "a wall",
        0xE => "a heavy-door variant",
        0xF => "a room trigger",
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
        96..=103 => b'R',
        104..=127 => b'd',
        128..=159 => b's',
        160..=191 => b'v',
        192..=255 => b'n',
    }
}

pub fn waypoint_for_hour(schedule: &[u8; NPC_SCHEDULE_RECORD_LEN], hour: u8) -> usize {
    let t0 = schedule[NPC_SCHEDULE_TIME_OFFSET];
    let t1 = schedule[NPC_SCHEDULE_TIME_OFFSET + 1];
    let t2 = schedule[NPC_SCHEDULE_TIME_OFFSET + 2];
    let t3 = schedule[NPC_SCHEDULE_TIME_OFFSET + 3];
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

/// `active-objects.md §8.1`: `true` when `(x, y)` is inside the
/// loaded overworld window, measured **forward** from `scroll_base`
/// (the window's origin corner) on both axes independently. The
/// complement of [`crate::active_object_should_prune`], which owns the
/// contract and the unsigned eight-bit arithmetic; this wrapper only
/// adapts the `usize` world-coordinate calling convention.
pub fn world_scroll_neighborhood_contains(scroll_base: (usize, usize), x: usize, y: usize) -> bool {
    !crate::active_object_should_prune(
        (x % WORLD_SIDE) as u8,
        (y % WORLD_SIDE) as u8,
        (scroll_base.0 % WORLD_SIDE) as u8,
        (scroll_base.1 % WORLD_SIDE) as u8,
    )
}

pub fn world_scroll_axis_offset(base: usize, coordinate: usize) -> usize {
    (coordinate + WORLD_SIDE - base) % WORLD_SIDE
}

pub fn u16_at(bytes: &[u8], off: usize) -> u16 {
    u16::from_le_bytes([bytes[off], bytes[off + 1]])
}

pub fn u32_at(bytes: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([bytes[off], bytes[off + 1], bytes[off + 2], bytes[off + 3]])
}

pub fn write_u16_at(bytes: &mut [u8], off: usize, value: u16) {
    bytes[off..off + 2].copy_from_slice(&value.to_le_bytes());
}

/// `systems/active-objects.md §12` / `catalogs/tile-catalog.md §3.1`:
/// active-object records do **not** carry a tile id.
///
/// Placing an active object writes its actor byte into the companion
/// band for that cell and sets the viewport grid cell to zero. A
/// non-zero grid cell draws the terrain tile it names; a zero grid
/// cell reads the companion byte and draws tile `byte + 256`. Actor
/// bytes therefore index the upper, actor half of the 512-tile space,
/// and a terrain byte and an actor byte with the same value are
/// different sprites - which is why actor byte `0x44` is the Bard
/// (tile 324) and not the floor tile whose terrain index is also
/// `0x44`.
///
/// `cleak/u5-spec#82` withdrew the "carry the tile id directly" note
/// and the `320..335` / `368..383` range names that went with it; a
/// confirmed id beats a range name.
pub const ACTOR_TILE_BANK_BASE: usize = 256;

/// `catalogs/tile-catalog.md §3.1`: the one reserved actor byte -
/// draw nothing, the transparent cell.
pub const ACTOR_TILE_TRANSPARENT_BYTE: u8 = 0x16;

/// Resolve an actor byte to the tile index the scene compositor draws,
/// or `None` for the reserved transparent value.
pub const fn actor_tile_for_byte(actor_byte: u8) -> Option<usize> {
    if actor_byte == ACTOR_TILE_TRANSPARENT_BYTE {
        return None;
    }
    Some(ACTOR_TILE_BANK_BASE + actor_byte as usize)
}

#[cfg(test)]
mod dungeon_room_clear_tests {
    use super::*;

    fn scene_for_record(record: u8) -> DungeonScene {
        DungeonScene::from_record(record).unwrap()
    }

    #[test]
    fn arena_bank_collapse_matches_the_published_bank_listing() {
        // `dungeon-mode.md §14`: "arena_bank = 0 if dungeon_record <= 1
        // else dungeon_record - 1", giving "Deceit records 0..15,
        // Destard 16..31, Wrong 32..47, Covetous 48..63, Shame 64..79,
        // Hythloth 80..95, and Doom 96..111".
        assert_eq!(dungeon_arena_bank(0), 0);
        assert_eq!(dungeon_arena_bank(1), 0);
        assert_eq!(dungeon_arena_bank(2), 1);
        assert_eq!(dungeon_arena_bank(WRONG_DUNGEON_RECORD as usize), 2);
        assert_eq!(dungeon_arena_bank(COVETOUS_DUNGEON_RECORD as usize), 3);
        assert_eq!(dungeon_arena_bank(7), 6);
        assert_eq!(DUNGEON_ARENA_BANK_COUNT, 7);
        assert_eq!(
            DUNGEON_ARENA_BANK_COUNT * DUNGEON_ROOM_SLOTS_PER_BANK,
            112,
            "§5: one hundred twelve bits"
        );
        assert_eq!(DUNGEON_ROOM_CLEAR_ADDRESSED_BYTES, 14);
        assert!(DUNGEON_ROOM_CLEAR_ADDRESSED_BYTES < SAVE_DUNGEON_ROOM_CLEAR_BITMAP_LEN);
    }

    #[test]
    fn clear_bit_index_is_the_same_value_as_the_arena_index() {
        // `dungeon-mode.md §5`: "the bit index is the same
        // `arena_bank * 16 + room_id` value used to select the
        // DUNGEON.CBT record (§ 14)". Reading the bit index back out of
        // (byte, mask) must reproduce `dungeon_room_arena_index`.
        for record in 0u8..=7 {
            let scene = scene_for_record(record);
            for room in 0u8..16 {
                let mut bitmap = [0u8; SAVE_DUNGEON_ROOM_CLEAR_BITMAP_LEN];
                if dungeon_room_clear_is_denied(record, room) {
                    continue;
                }
                assert!(set_dungeon_room_clear_bit(&mut bitmap, scene, room));
                let set_bits: Vec<usize> = (0..SAVE_DUNGEON_ROOM_CLEAR_BITMAP_LEN * 8)
                    .filter(|bit| bitmap[bit / 8] & (1 << (bit % 8)) != 0)
                    .collect();
                let arena_index = dungeon_room_arena_index(scene, 0xF0 | room);
                assert_eq!(
                    set_bits,
                    vec![arena_index],
                    "record {record} room {room} must set bit {arena_index}"
                );
                assert!(arena_index < DUNGEON_ARENA_BANK_COUNT * DUNGEON_ROOM_SLOTS_PER_BANK);
                // The two trailing bytes of the sixteen-byte field are
                // never addressed.
                assert_eq!(bitmap[DUNGEON_ROOM_CLEAR_ADDRESSED_BYTES..], [0, 0]);
            }
        }
    }

    #[test]
    fn clear_bit_uses_the_collapsed_bank_not_the_raw_record() {
        // Destard is raw record 2 but arena bank 1, so its room 0 bit
        // is bit 16 (byte 2), not bit 32 (byte 4).
        let destard = scene_for_record(2);
        let mut bitmap = [0u8; SAVE_DUNGEON_ROOM_CLEAR_BITMAP_LEN];
        assert!(set_dungeon_room_clear_bit(&mut bitmap, destard, 0));
        assert_eq!(bitmap[2], 0x01);
        assert_eq!(bitmap[4], 0x00);
        // Deceit (record 0) and Despise (record 1) share bank zero.
        let mut shared = [0u8; SAVE_DUNGEON_ROOM_CLEAR_BITMAP_LEN];
        assert!(set_dungeon_room_clear_bit(
            &mut shared,
            scene_for_record(0),
            3
        ));
        assert!(dungeon_room_clear_bit_is_set(
            &shared,
            scene_for_record(1),
            3
        ));
        // Doom (record 7 / bank 6) lands in the last addressed byte
        // pair, never in the two trailing bytes.
        let mut doom = [0u8; SAVE_DUNGEON_ROOM_CLEAR_BITMAP_LEN];
        assert!(set_dungeon_room_clear_bit(
            &mut doom,
            scene_for_record(7),
            15
        ));
        assert_eq!(doom[13], 0x80);
        assert_eq!(doom[14], 0x00);
        assert_eq!(doom[15], 0x00);
    }

    #[test]
    fn writer_deny_list_holds_the_six_published_pairs() {
        // `dungeon-mode.md §5`: "the deny-listed rooms are rooms one,
        // six, eleven, and twelve of the Wrong bank and rooms zero and
        // eleven of the Covetous bank".
        assert_eq!(DUNGEON_ROOM_CLEAR_DENY_LIST.len(), 6);
        for room in [1u8, 6, 11, 12] {
            assert!(dungeon_room_clear_is_denied(WRONG_DUNGEON_RECORD, room));
        }
        for room in [0u8, 11] {
            assert!(dungeon_room_clear_is_denied(COVETOUS_DUNGEON_RECORD, room));
        }
        // Neighbouring rooms in the same banks are not denied.
        assert!(!dungeon_room_clear_is_denied(WRONG_DUNGEON_RECORD, 0));
        assert!(!dungeon_room_clear_is_denied(COVETOUS_DUNGEON_RECORD, 1));
        // The key is the RAW record, never the collapsed bank: Wrong's
        // bank number is 2, which is Destard's raw record.
        assert!(!dungeon_room_clear_is_denied(2, 1));
    }

    #[test]
    fn denied_rooms_never_persist_and_the_reader_applies_no_deny_list() {
        // `dungeon-mode.md §5`: denied rooms "never persist as cleared
        // and re-arm on every visit", while "the bitmap reader applies
        // no deny-list, so it simply always reports those rooms as not
        // cleared".
        let wrong = scene_for_record(WRONG_DUNGEON_RECORD);
        let mut bitmap = [0u8; SAVE_DUNGEON_ROOM_CLEAR_BITMAP_LEN];
        assert!(!set_dungeon_room_clear_bit(&mut bitmap, wrong, 6));
        assert_eq!(bitmap, [0u8; SAVE_DUNGEON_ROOM_CLEAR_BITMAP_LEN]);
        assert!(!dungeon_room_clear_bit_is_set(&bitmap, wrong, 6));

        // A denied room's trigger cell is therefore never demoted on
        // reload; an allowed room in the same bank still is.
        assert!(set_dungeon_room_clear_bit(&mut bitmap, wrong, 7));
        let mut grid = vec![0u8; DUNGEON_RECORD_LEN];
        grid[0] = 0xF6;
        grid[1] = 0xF7;
        apply_dungeon_room_clear_bitmap(&mut grid, wrong, &bitmap);
        assert_eq!(grid[0], 0xF6);
        assert_eq!(grid[1], 0xA7);
    }
}

#[cfg(test)]
mod npc_sprite_tests {
    use super::*;
    use crate::{
        NPC_DEFAULT_PERSON_SPRITE_TILE, NPC_TYPE_DEFAULT_HUMAN_SPRITE, NPC_TYPE_EMPTY,
        NPC_TYPE_SHADOWLORD_ACTOR, combat_class_sprite_byte,
    };

    /// `catalogs/npc-roster.md §4` publishes the whole shipped tag
    /// alphabet; every one of these appears on at least one occupied
    /// roster slot in the four `.NPC` files.
    const SHIPPED_ROSTER_TAGS: [u8; 25] = [
        0x01, 0x0E, 0x10, 0x11, 0x1B, 0x1E, 0x28, 0x40, 0x44, 0x48, 0x50, 0x54, 0x58, 0x5C, 0x68,
        0x6C, 0x70, 0x78, 0x90, 0x94, 0xB5, 0xB6, 0xB8, 0xD8, 0xFC,
    ];

    #[test]
    fn ordinary_roster_tag_is_the_actor_byte_itself() {
        // `formats/npc.md §6`: "the runtime sprite tile is derived by
        // adding the byte to the NPC sprite page"; the sprite page is
        // the actor bank base, applied by `actor_tile_for_byte`.
        for tag in SHIPPED_ROSTER_TAGS {
            if tag == NPC_TYPE_DEFAULT_HUMAN_SPRITE {
                continue;
            }
            assert_eq!(npc_tile(tag), tag, "roster tag {tag:#04x}");
            assert_eq!(
                actor_tile_for_byte(npc_tile(tag)),
                Some(ACTOR_TILE_BANK_BASE + tag as usize),
                "roster tag {tag:#04x}"
            );
        }
    }

    #[test]
    fn creature_roster_tags_match_the_published_class_sprite_relation() {
        // `catalogs/npc-roster.md §4` names these tags, and
        // `catalogs/tile-catalog.md §7` + `catalogs/monster-bestiary.md`
        // put the same creatures at `class * 4 + 0x40`.
        for (tag, class) in [
            (0x90u8, 20u8), // a rodent of unusual size
            (0x94, 21),     // a bat
            (0xB8, 30),     // a gargoyle
            (0xD8, 38),     // a daemon
            (0xFC, 47),     // a shadow lord
        ] {
            assert_eq!(combat_class_sprite_byte(class), tag);
            assert_eq!(npc_tile(tag), tag);
        }
        assert_eq!(
            npc_tile(NPC_TYPE_SHADOWLORD_ACTOR),
            NPC_TYPE_SHADOWLORD_ACTOR
        );
    }

    #[test]
    fn default_person_sentinel_forces_the_person_tile() {
        // `catalogs/npc-roster.md §4` row `01`: "the sprite-link helper
        // forces the standard person tile instead of using the tag as a
        // direct sprite class".
        assert_eq!(
            npc_tile(NPC_TYPE_DEFAULT_HUMAN_SPRITE),
            NPC_DEFAULT_PERSON_SPRITE_TILE
        );
        assert_ne!(
            NPC_DEFAULT_PERSON_SPRITE_TILE,
            NPC_TYPE_DEFAULT_HUMAN_SPRITE
        );
    }

    #[test]
    fn shipped_roster_tags_do_not_collapse_onto_one_sprite() {
        // Regression for the withdrawn clamp, which mapped every tag
        // outside `192..=255` to the single tile `192`.
        let mut tiles: Vec<u8> = SHIPPED_ROSTER_TAGS.iter().map(|&t| npc_tile(t)).collect();
        tiles.sort_unstable();
        tiles.dedup();
        assert_eq!(
            tiles.len(),
            SHIPPED_ROSTER_TAGS.len() - 1,
            "tag 01 shares the villager tile; every other tag is distinct"
        );
        assert!(!SHIPPED_ROSTER_TAGS.iter().any(|&t| npc_tile(t) == 192));
    }

    #[test]
    fn active_object_keeps_the_roster_type_and_derives_the_tile() {
        // `npc-schedules.md §11`: the slot is filled "with the NPC's
        // tile, type, and new coordinates" - two separate fields.
        let object = npc_active_object(NPC_TYPE_DEFAULT_HUMAN_SPRITE, 4, 5, 0);
        assert_eq!(object.type_byte, NPC_TYPE_DEFAULT_HUMAN_SPRITE);
        assert_eq!(object.tile, NPC_DEFAULT_PERSON_SPRITE_TILE);

        let guard = npc_active_object(0x70, 4, 5, 0);
        assert_eq!(guard.type_byte, 0x70);
        assert_eq!(guard.tile, 0x70);
        assert_eq!(npc_tile(NPC_TYPE_EMPTY), NPC_TYPE_EMPTY);
    }

    #[test]
    fn hidden_sprite_tile_is_the_reserved_transparent_actor_byte() {
        // `npc-schedules.md §11` suppresses only presentation, and
        // `catalogs/tile-catalog.md §3.1` names `0x16` as "the sole
        // reserved actor byte ... draw nothing". Actor byte `0` is a
        // drawable tile, so it never suppressed anything.
        assert_eq!(crate::NPC_HIDDEN_SPRITE_TILE, ACTOR_TILE_TRANSPARENT_BYTE);
        assert_eq!(actor_tile_for_byte(crate::NPC_HIDDEN_SPRITE_TILE), None);
    }
}
