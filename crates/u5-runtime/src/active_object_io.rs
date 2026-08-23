//! Active-object encode/decode for SAVED.OOL mirroring + write helpers.

use std::io;
use std::path::Path;

use crate::*;

/// `active-objects.md §4` typed slot-index role. The 32-slot active-
/// object table is partitioned into three disjoint roles by index:
/// slot 0 is the canonical player slot, slots 1..=23 are walked by
/// the ordinary world/town acquisition allocator, and slots 24..=31
/// are reserved for setup paths outside the allocator (combat
/// placement, the player-as-NPC mirror helper, etc.).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActiveObjectSlotRole {
    /// Slot 0 — refreshed every frame from world-state globals.
    Player,
    /// Slots 1..=23 — ordinary world/town acquisition range. The
    /// allocator's lowest-up scan walks this range.
    OrdinaryAcquisition,
    /// Slots 24..=31 — reserved for setup paths outside the
    /// allocator (combat setup, player-as-NPC mirror, etc.).
    Reserved,
}

/// `active-objects.md §4`: classify a slot index `0..=31` into its
/// allocator role. Returns `None` for indices outside the 32-slot
/// table.
pub const fn active_object_slot_role(slot: usize) -> Option<ActiveObjectSlotRole> {
    if slot >= OOL_SLOTS {
        return None;
    }
    Some(if slot == ACTIVE_OBJECT_PLAYER_SLOT {
        ActiveObjectSlotRole::Player
    } else if slot >= ACTIVE_OBJECT_ORDINARY_FIRST && slot <= ACTIVE_OBJECT_ORDINARY_LAST {
        ActiveObjectSlotRole::OrdinaryAcquisition
    } else {
        ActiveObjectSlotRole::Reserved
    })
}

/// `active-objects.md §3` field offsets within the eight-byte record.
/// `formats/saved-gam.md §11` per-record active-object field
/// offsets. The eight fields pack contiguously from offset 0
/// through 7 (TYPE / TILE / X / Y / Z / DEP1 / PHASE / DEP3).
/// Anchor each successor to the chain so adding or reordering a
/// field only happens in one place.
pub const ACTIVE_OBJECT_FIELD_TYPE: usize = 0;
pub const ACTIVE_OBJECT_FIELD_TILE: usize = ACTIVE_OBJECT_FIELD_TYPE + 1;
pub const ACTIVE_OBJECT_FIELD_X: usize = ACTIVE_OBJECT_FIELD_TILE + 1;
pub const ACTIVE_OBJECT_FIELD_Y: usize = ACTIVE_OBJECT_FIELD_X + 1;
pub const ACTIVE_OBJECT_FIELD_Z: usize = ACTIVE_OBJECT_FIELD_Y + 1;
pub const ACTIVE_OBJECT_FIELD_DEP1: usize = ACTIVE_OBJECT_FIELD_Z + 1;
pub const ACTIVE_OBJECT_FIELD_PHASE: usize = ACTIVE_OBJECT_FIELD_DEP1 + 1;
pub const ACTIVE_OBJECT_FIELD_DEP3: usize = ACTIVE_OBJECT_FIELD_PHASE + 1;

/// `active-objects.md §8` outdoor step-committer destination-tile
/// chance gate. After ordinary terrain/occupancy validation accepts
/// a candidate cell, the step committer can still refuse the move
/// based on the destination tile family.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutdoorMovementChanceGate {
    /// Tile ids `0x04`, `0x06..=0x08`, `0x1E..=0x1F` — accept on a
    /// one-in-two roll.
    OneInTwo,
    /// Tile ids `0x09..=0x0F` — accept on a one-in-three roll.
    OneInThree,
    /// Tile id `0x05`, `0x10..=0x1D`, and ids outside `0x04..=0x1F`
    /// — accept immediately.
    Immediate,
}

/// `active-objects.md §8`: classify a destination tile id into its
/// post-validation chance gate. The four monster first-frame values
/// `0x94` Bat, `0xD8` Daemon, `0xDC` Dragon, and `0xF0` Mongbat —
/// plus ship-like water-creature frames `0x2C..=0x2F` — bypass the
/// low-terrain gate at the caller; this helper classifies the
/// destination tile only.
pub const fn outdoor_movement_chance_gate(destination_tile: u8) -> OutdoorMovementChanceGate {
    match destination_tile {
        0x04 | 0x06..=0x08 | 0x1E..=0x1F => OutdoorMovementChanceGate::OneInTwo,
        0x09..=0x0F => OutdoorMovementChanceGate::OneInThree,
        _ => OutdoorMovementChanceGate::Immediate,
    }
}

/// `active-objects.md §8` step-committer auto-clear tile. When a
/// committed step lands on destination tile id `0xDC` (the moon-gate
/// / local-light terrain family), the moving slot is cleared.
pub const OUTDOOR_STEP_CLEAR_DESTINATION_TILE: u8 = 0xDC;

/// `active-objects.md §8`: returns `true` when an outdoor active-
/// object's committed step lands on the auto-clear terrain tile
/// (`0xDC`). The caller frees the moving slot rather than placing
/// it on the moongate cell.
pub const fn outdoor_step_clears_on_destination(destination_tile: u8) -> bool {
    destination_tile == OUTDOOR_STEP_CLEAR_DESTINATION_TILE
}

/// `active-objects.md §8` `0xFC` proximity-mask age cap. Listed
/// proximity cells increment the slot's first auxiliary byte as an
/// age counter; while the counter remains below twenty, the slot
/// requests a directed step toward the player.
pub const FC_PROXIMITY_AGE_CAP: u8 = 20;

/// `active-objects.md §8` outdoor sea-serpent / dragon adjacency
/// trigger. The outdoor per-turn walker rolls a one-in-seven gate
/// for first-frame Sea Serpent and Dragon hostile classes near the
/// player; on the gate's hit and a clear directed probe the same
/// per-turn finishers as other outdoor encounter effects run.
pub const OUTDOOR_SERPENT_DRAGON_TRIGGER_DENOMINATOR: u8 = 7;

/// `active-objects.md §8`: returns `true` when the per-turn walker's
/// `0..6` PRNG roll triggers the outdoor sea-serpent / dragon
/// engagement on this tick.
pub const fn outdoor_serpent_dragon_triggers(roll_0_to_6: u8) -> bool {
    roll_0_to_6 == 0
}

/// `active-objects.md §8` ship-like water-creature and pirate
/// adjacency window. The outdoor per-turn walker prints the attack
/// message and runs the water-creature step path when the slot is
/// aligned with the player within this many cells.
pub const OUTDOOR_WATER_CREATURE_ADJACENCY_RADIUS: i32 = 3;

/// `active-objects.md §8`: returns `true` when a ship-like
/// water-creature or pirate slot is orthogonally aligned with the
/// player (sharing the same row or column) and the wrapped distance
/// along the shared axis is within
/// [`OUTDOOR_WATER_CREATURE_ADJACENCY_RADIUS`]. Diagonal offsets
/// never trigger the attack-message / water-creature step path,
/// regardless of distance.
pub const fn outdoor_water_creature_attack_aligned(wrapped_dx: i32, wrapped_dy: i32) -> bool {
    let abs_dx = if wrapped_dx < 0 {
        -wrapped_dx
    } else {
        wrapped_dx
    };
    let abs_dy = if wrapped_dy < 0 {
        -wrapped_dy
    } else {
        wrapped_dy
    };
    if abs_dx != 0 && abs_dy != 0 {
        return false;
    }
    if abs_dx == 0 && abs_dy == 0 {
        return false;
    }
    let along_axis = if abs_dx == 0 { abs_dy } else { abs_dx };
    along_axis <= OUTDOOR_WATER_CREATURE_ADJACENCY_RADIUS
}

/// `active-objects.md §2,§7` (whirlpool emergence) coordinates the
/// outdoor whirlpool transition writes when the party is moved to
/// the underworld plane.
pub const WHIRLPOOL_EMERGENCE_X: u8 = 34;
pub const WHIRLPOOL_EMERGENCE_Y: u8 = 18;

/// `active-objects.md §4` table-class byte the eviction cascade
/// excludes from every phase past phase 1 (the empty-slot phase).
pub const ACTIVE_OBJECT_EVICTION_PROTECTED_TYPE: u8 = 0xB5;

/// `active-objects.md §4` phase 2/6 low-priority scenery class range
/// (`0x01..=0x0F`). Phase 2 also requires the off-screen viewport
/// gate; phase 6 reuses the same byte range without that gate.
pub const ACTIVE_OBJECT_EVICTION_SCENERY_FIRST: u8 = 0x01;
pub const ACTIVE_OBJECT_EVICTION_SCENERY_LAST: u8 = 0x0F;

/// `active-objects.md §4` phase 3/7 dynamic-actor class lower bound
/// (`0x80..=0xFF` minus [`ACTIVE_OBJECT_EVICTION_PROTECTED_TYPE`]).
pub const ACTIVE_OBJECT_EVICTION_DYNAMIC_FIRST: u8 = 0x80;

/// `active-objects.md §4` phase 4/8 door/fixture-like low-class pair
/// (`0x10` and `0x11`). The pair begins immediately after the
/// scenery band; anchor to [`ACTIVE_OBJECT_EVICTION_SCENERY_LAST`]
/// + 1 so the band adjacency has one source of truth.
pub const ACTIVE_OBJECT_EVICTION_DOOR_FIXTURE_FIRST: u8 = ACTIVE_OBJECT_EVICTION_SCENERY_LAST + 1;
pub const ACTIVE_OBJECT_EVICTION_DOOR_FIXTURE_LAST: u8 =
    ACTIVE_OBJECT_EVICTION_DOOR_FIXTURE_FIRST + 1;

/// `active-objects.md §4` phase 5/9 midrange object class range
/// (`0x30..=0x7F`). The midrange band ends one byte below the
/// dynamic-actor lower bound at 0x80; anchor the upper bound to
/// [`ACTIVE_OBJECT_EVICTION_DYNAMIC_FIRST`] - 1.
pub const ACTIVE_OBJECT_EVICTION_MIDRANGE_FIRST: u8 = 0x30;
pub const ACTIVE_OBJECT_EVICTION_MIDRANGE_LAST: u8 = ACTIVE_OBJECT_EVICTION_DYNAMIC_FIRST - 1;

/// `active-objects.md §4`: the last (least selective) eviction phase.
/// Phases run `1..=10`; phase 10 is the last-resort pass that accepts
/// any type byte except the universally protected `0xB5`.
pub const ACTIVE_OBJECT_EVICTION_LAST_PHASE: u8 = 10;

/// `active-objects.md §4`: returns `true` when an active-object
/// type byte is acceptable as a candidate for eviction phase
/// 2..=5 (the off-screen phases) or 6..=9 (the same classes,
/// visible allowed). Phase 1 accepts only the empty-slot byte
/// (`0x00`); phase 10 is the last-resort eviction and accepts any
/// type byte except `0xB5`.
///
/// `phase` is the one-based eviction phase index `1..=10`.
pub const fn active_object_eviction_byte_accepted(byte: u8, phase: u8) -> bool {
    match phase {
        1 => byte == 0x00,
        2 | 6 => {
            byte >= ACTIVE_OBJECT_EVICTION_SCENERY_FIRST
                && byte <= ACTIVE_OBJECT_EVICTION_SCENERY_LAST
        }
        3 | 7 => {
            byte >= ACTIVE_OBJECT_EVICTION_DYNAMIC_FIRST
                && byte != ACTIVE_OBJECT_EVICTION_PROTECTED_TYPE
        }
        4 | 8 => {
            byte == ACTIVE_OBJECT_EVICTION_DOOR_FIXTURE_FIRST
                || byte == ACTIVE_OBJECT_EVICTION_DOOR_FIXTURE_LAST
        }
        5 | 9 => {
            byte >= ACTIVE_OBJECT_EVICTION_MIDRANGE_FIRST
                && byte <= ACTIVE_OBJECT_EVICTION_MIDRANGE_LAST
        }
        10 => byte != ACTIVE_OBJECT_EVICTION_PROTECTED_TYPE,
        _ => false,
    }
}

/// `active-objects.md §4`: returns `true` when the eviction phase
/// requires the off-screen viewport gate. Phases 2..=5 are
/// off-screen-only; phases 1, 6..=10 do not consult the gate.
pub const fn active_object_eviction_phase_is_off_screen(phase: u8) -> bool {
    matches!(phase, 2 | 3 | 4 | 5)
}

/// `active-objects.md §4` on-screen **window** half-extent for the
/// eviction cascade's off-screen phases (2..=5). Not a radius: the
/// spec states the gate per axis -- "a candidate more than roughly
/// five cells from the player **in either axis** is considered
/// eligible for the off-screen phases" -- so the two axes are tested
/// separately and independently against this bound. There is no
/// distance computation, no hypotenuse and no disc; a disc would
/// treat the corners of the square window as off-screen when the
/// original keeps them.
///
/// The window is centred on the player, so this is its half-extent:
/// the largest per-axis separation that still counts as on-screen. It
/// matches the visible 11x11 viewport half-width.
///
/// This bound belongs to **eviction only**. The overworld prune pass
/// of `§8.1` has its own, unrelated window bound
/// ([`ACTIVE_OBJECT_PRUNE_WINDOW_EXTENT`]) measured from a different
/// origin on a different trigger; `§8.1` warns that "a single shared
/// distance constant serving both is a sign the two have been
/// conflated", so the two names are kept apart deliberately even
/// though the mechanisms are neighbours.
pub const ACTIVE_OBJECT_EVICTION_ONSCREEN_HALF_WINDOW: u8 = 5;

/// `active-objects.md §4`: returns `true` when an active-object slot
/// at `(slot_x, slot_y)` falls outside the square on-screen window of
/// half-extent [`ACTIVE_OBJECT_EVICTION_ONSCREEN_HALF_WINDOW`] centred
/// on the player, qualifying it for the off-screen eviction phases.
/// Each axis is tested separately; failing either axis is off-screen.
///
/// Coordinates are the record's stored X/Y bytes (`§3`) and the
/// separation is formed in **unsigned eight-bit arithmetic**, so it
/// wraps naturally with the 256-cell coordinate space instead of
/// needing a map-seam special case. Signed or wider arithmetic
/// mis-handles a candidate one cell across the seam from the player,
/// reporting it ~255 cells away and evicting it early.
///
/// Spec gap: `§4` states the five-cell window but not its
/// arithmetic. The unsigned-byte form is carried over from the
/// prune pass, whose arithmetic `§8.1` does state, on the grounds
/// that both tests compare stored coordinate bytes on the same
/// 256-cell torus. If `§4` is ever given an explicit arithmetic that
/// differs, this is the line to change.
pub const fn active_object_eviction_off_screen(
    slot_x: u8,
    slot_y: u8,
    player_x: u8,
    player_y: u8,
) -> bool {
    !window_half_extent_contains(
        slot_x,
        player_x,
        ACTIVE_OBJECT_EVICTION_ONSCREEN_HALF_WINDOW,
    ) || !window_half_extent_contains(
        slot_y,
        player_y,
        ACTIVE_OBJECT_EVICTION_ONSCREEN_HALF_WINDOW,
    )
}

/// One axis of a square window **centred** on `centre` with the given
/// half-extent, in unsigned eight-bit arithmetic on the 256-cell
/// coordinate space. `true` when `coordinate` is within
/// `half_extent` cells of `centre` on either side of the wrap.
///
/// Distinct from [`window_extent_contains`], which measures forward
/// from a corner rather than out from a centre.
pub const fn window_half_extent_contains(coordinate: u8, centre: u8, half_extent: u8) -> bool {
    let forward = coordinate.wrapping_sub(centre);
    let backward = centre.wrapping_sub(coordinate);
    forward <= half_extent || backward <= half_extent
}

/// One axis of a square window whose **origin corner** is `base`, in
/// unsigned eight-bit arithmetic on the 256-cell coordinate space.
/// `true` when the unsigned difference `coordinate - base` falls
/// within `extent`.
///
/// `active-objects.md §8.1`: "The difference is formed in unsigned
/// eight-bit arithmetic against the scroll base, so it wraps
/// naturally with the 256-cell coordinate space rather than needing a
/// special case at the map seam."
pub const fn window_extent_contains(coordinate: u8, base: u8, extent: u8) -> bool {
    coordinate.wrapping_sub(base) <= extent
}

/// `active-objects.md §2` per-pass iteration order. The renderer
/// walks slots from `OOL_SLOTS - 1` down to `0` so lower-indexed
/// slots paint on top — guaranteeing the player (slot zero) draws
/// over every other entity in the same cell. The per-tick animator
/// walks slots from `0` up to `OOL_SLOTS - 1`; iteration order there
/// affects only deterministic tie-breaking, not correctness.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActiveObjectPassOrder {
    /// Renderer pass — high index to low (`31..=0`).
    RendererHighToLow,
    /// Per-tick animator pass — low index to high (`0..=31`).
    AnimatorLowToHigh,
}

impl ActiveObjectPassOrder {
    /// Returns the (start, end_inclusive, step_descending) tuple for
    /// the requested pass. `step_descending == true` means iterate
    /// from `start` down to `end_inclusive`.
    pub const fn iteration(self) -> (usize, usize, bool) {
        match self {
            Self::RendererHighToLow => (OOL_SLOTS - 1, 0, true),
            Self::AnimatorLowToHigh => (0, OOL_SLOTS - 1, false),
        }
    }
}

/// `active-objects.md §8.1`: the overworld per-turn prune-pass
/// position test. Returns `true` when a slot's stored `(slot_x,
/// slot_y)` falls outside the loaded window and the slot must be
/// released.
///
/// The pass "compares the slot's stored X and Y against the current
/// **scroll base** - the top-left corner of the loaded window - and
/// keeps the slot only when **both** differences fall within
/// thirty-two. Failing either axis releases the slot."
///
/// Three properties are contract and are each a place implementations
/// predictably go wrong:
///
/// * **Square window, not a radius.** The two axes are tested
///   separately and independently against
///   [`ACTIVE_OBJECT_PRUNE_WINDOW_EXTENT`]. No distance, no
///   hypotenuse, no disc -- a disc prunes the window corners that the
///   original keeps.
/// * **Measured from the scroll base**, the window's origin *corner*,
///   not the player's cell and not the viewport centre. The window
///   therefore extends forward from the base; this is
///   [`window_extent_contains`], not a centred band.
/// * **Unsigned eight-bit arithmetic**, so the difference wraps with
///   the 256-cell coordinate space and needs no map-seam special
///   case. Signed or wider arithmetic mis-handles objects across the
///   wrap.
///
/// Two further contract points live at the call site rather than here,
/// because they are about *which* slots reach this test: slot zero is
/// never prunable, and a slot whose type byte does not classify as a
/// prunable kind is skipped **before** the position test runs. See
/// `PlayState::prune_far_overworld_objects`.
pub const fn active_object_should_prune(
    slot_x: u8,
    slot_y: u8,
    scroll_base_x: u8,
    scroll_base_y: u8,
) -> bool {
    !window_extent_contains(slot_x, scroll_base_x, ACTIVE_OBJECT_PRUNE_WINDOW_EXTENT)
        || !window_extent_contains(slot_y, scroll_base_y, ACTIVE_OBJECT_PRUNE_WINDOW_EXTENT)
}

/// `active-objects.md §11` save-image active-object region length.
pub const ACTIVE_OBJECT_SAVE_BYTES: usize = 256;

/// `active-objects.md §8`: animator outcome for one slot's phase
/// counter (low nibble of byte 6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnimationPhaseStep {
    /// All-ones nibble — slot is steady; the animator skips it.
    Steady,
    /// Mid-cycle. The animator decrements the nibble and writes back
    /// the new value (always `>= 0`); the renderer combines this with
    /// the tile class to pick a frame.
    Decrement(u8),
    /// Cycle ended. The slot is eligible for an AI tick this pass.
    AiEligible,
}

/// All-ones nibble in byte 6 marks "steady, do not animate" per
/// `active-objects.md §8`. Anchored to
/// [`ACTIVE_OBJECT_PHASE_LOW_NIBBLE_MASK`] so the "steady"
/// sentinel and the low-nibble mask share one source of truth —
/// both name the same all-ones-nibble value.
pub const ANIMATION_PHASE_STEADY_NIBBLE: u8 = ACTIVE_OBJECT_PHASE_LOW_NIBBLE_MASK;

/// `active-objects.md §3` / `formats/ool.md §4` packed phase-byte
/// (byte 6) low-nibble mask. The low nibble holds the animation-phase
/// countdown; the high nibble holds the direction-step counter used
/// by AI movement. Promote the mask so both decoders share one
/// source of truth.
pub const ACTIVE_OBJECT_PHASE_LOW_NIBBLE_MASK: u8 = 0x0F;

/// `active-objects.md §3` / `formats/ool.md §4` packed phase-byte
/// high-nibble shift. The direction-step counter sits in bits 4..=7
/// of byte 6; right-shifting the byte by this amount yields the
/// counter as a value in `0..=15`.
pub const ACTIVE_OBJECT_PHASE_DIRECTION_NIBBLE_SHIFT: u32 = 4;

/// `active-objects.md §3`: extract the direction-step counter from
/// a packed phase byte (byte 6). The high nibble carries the AI
/// movement direction-step counter in `0..=15`.
pub const fn active_object_direction_step(phase_byte: u8) -> u8 {
    phase_byte >> ACTIVE_OBJECT_PHASE_DIRECTION_NIBBLE_SHIFT
}

/// `active-objects.md §8`: classify the low nibble of an active-object
/// phase byte (byte 6) into the animator's per-tick outcome. Higher
/// bits of the input are masked off; callers may pass either the raw
/// byte or just the nibble.
pub const fn animation_phase_step(phase_byte: u8) -> AnimationPhaseStep {
    let nibble = phase_byte & ACTIVE_OBJECT_PHASE_LOW_NIBBLE_MASK;
    if nibble == ANIMATION_PHASE_STEADY_NIBBLE {
        AnimationPhaseStep::Steady
    } else if nibble == 0 {
        AnimationPhaseStep::AiEligible
    } else {
        AnimationPhaseStep::Decrement(nibble - 1)
    }
}

/// `active-objects.md §4` eviction cascade phase bounds. The cascade is
/// ten one-based phases run in order; phase 1 is the empty-slot path and
/// phase 10 is the last-resort eviction.
pub const ACTIVE_OBJECT_EVICTION_PHASE_FIRST: u8 = 1;
pub const ACTIVE_OBJECT_EVICTION_PHASE_LAST: u8 = 10;

/// `active-objects.md §4`: deterministic eviction phase a candidate
/// qualifies for, or `None` if the byte-0 / on-screen combination is not
/// a victim in any phase. Phases 1..=5 are the off-screen passes (with
/// phase 1 being the empty-slot path); phases 6..=10 are the
/// any-on-screen passes. Byte 0x00 (empty slot) returns `Some(1)`. Byte
/// 0xB5 is universally protected and returns `None` regardless.
///
/// Derived from [`active_object_eviction_byte_accepted`] and
/// [`active_object_eviction_phase_is_off_screen`] rather than
/// re-tabulating the cascade, so this summary view and the allocator's
/// phase loop cannot drift apart.
pub const fn active_object_eviction_phase(byte0: u8, off_screen: bool) -> Option<u8> {
    let mut phase = ACTIVE_OBJECT_EVICTION_PHASE_FIRST;
    while phase <= ACTIVE_OBJECT_EVICTION_PHASE_LAST {
        if active_object_eviction_byte_accepted(byte0, phase)
            && (off_screen || !active_object_eviction_phase_is_off_screen(phase))
        {
            return Some(phase);
        }
        phase += 1;
    }
    None
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SavedOolMirrorWriteCounts {
    pub brit_ool: usize,
    pub under_ool: usize,
}

pub fn refresh_saved_ool_mirrors_for_load(
    game_dir: &Path,
    needs_underworld_disk_swap: bool,
) -> io::Result<SavedOolMirrorWriteCounts> {
    let bytes = read_saved_ool_bytes(game_dir)?;
    write_saved_ool_mirrors_for_load(game_dir, &bytes, needs_underworld_disk_swap)
}

pub fn write_saved_ool_mirrors_for_load(
    game_dir: &Path,
    bytes: &[u8],
    needs_underworld_disk_swap: bool,
) -> io::Result<SavedOolMirrorWriteCounts> {
    write_saved_ool_mirrors(game_dir, bytes)?;
    let mut counts = SavedOolMirrorWriteCounts {
        brit_ool: 1,
        under_ool: 1,
    };
    if needs_underworld_disk_swap {
        write_disk_file(&game_dir.join(UNDER_OOL_FILENAME), &bytes[OOL_PLANE_LEN..])?;
        counts.under_ool += 1;
    }
    Ok(counts)
}

pub fn write_saved_ool_mirrors(game_dir: &Path, bytes: &[u8]) -> io::Result<()> {
    if bytes.len() != SAVED_OOL_LEN {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "SAVED.OOL mirror payload must be {SAVED_OOL_LEN} bytes, got {}",
                bytes.len()
            ),
        ));
    }
    write_disk_file(&game_dir.join(BRIT_OOL_FILENAME), &bytes[..OOL_PLANE_LEN])?;
    write_disk_file(&game_dir.join(UNDER_OOL_FILENAME), &bytes[OOL_PLANE_LEN..])?;
    Ok(())
}

pub fn write_saved_ool_mirrors_for_save(
    game_dir: &Path,
    bytes: &[u8],
    entry_disk_prompt_mode: u8,
) -> io::Result<SavedOolMirrorWriteCounts> {
    write_saved_ool_mirrors(game_dir, bytes)?;
    let mut counts = SavedOolMirrorWriteCounts {
        brit_ool: 1,
        under_ool: 1,
    };
    if save_flow_double_writes_underworld(entry_disk_prompt_mode) {
        write_disk_file(&game_dir.join(UNDER_OOL_FILENAME), &bytes[OOL_PLANE_LEN..])?;
        counts.under_ool += 1;
    }
    Ok(counts)
}

pub fn read_saved_ool_bytes(game_dir: &Path) -> io::Result<Vec<u8>> {
    let path = game_dir.join(SAVED_OOL_FILENAME);
    let bytes = read_disk_file(&path)?;
    if bytes.len() != SAVED_OOL_LEN {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "SAVED.OOL must be {SAVED_OOL_LEN} bytes, got {}",
                bytes.len()
            ),
        ));
    }
    Ok(bytes)
}

pub fn encode_active_object_table(objects: &[ActiveObject]) -> io::Result<Vec<u8>> {
    if objects.len() > OOL_SLOTS {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "active-object table has {} slots, expected at most {OOL_SLOTS}",
                objects.len()
            ),
        ));
    }
    let mut bytes = vec![0; OOL_PLANE_LEN];
    for (slot, object) in objects.iter().copied().enumerate() {
        write_active_object_record(&mut bytes, slot, object)?;
    }
    Ok(bytes)
}

pub fn encode_ool_plane_objects(objects: &[ActiveObject]) -> io::Result<Vec<u8>> {
    if objects.len() > OOL_SLOTS - 1 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "world overlay has {} non-player slots, expected at most {}",
                objects.len(),
                OOL_SLOTS - 1
            ),
        ));
    }
    let mut bytes = vec![0; OOL_PLANE_LEN];
    for (index, object) in objects.iter().copied().enumerate() {
        write_active_object_record(&mut bytes, index + 1, object)?;
    }
    Ok(bytes)
}

pub fn write_active_object_record(
    bytes: &mut [u8],
    slot: usize,
    object: ActiveObject,
) -> io::Result<()> {
    if slot >= OOL_SLOTS {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("active-object slot {slot} is outside 0..{}", OOL_SLOTS - 1),
        ));
    }
    let x = u8::try_from(object.x).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "active-object slot {slot} x coordinate {} is outside 0..255",
                object.x
            ),
        )
    })?;
    let y = u8::try_from(object.y).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "active-object slot {slot} y coordinate {} is outside 0..255",
                object.y
            ),
        )
    })?;
    let offset = slot * OOL_RECORD_LEN;
    bytes[offset] = object.type_byte;
    bytes[offset + 1] = object.tile;
    bytes[offset + 2] = x;
    bytes[offset + 3] = y;
    bytes[offset + 4] = object.z as u8;
    bytes[offset + 5] = object.aux1;
    bytes[offset + 6] = object.phase;
    bytes[offset + 7] = object.aux3;
    Ok(())
}

pub fn decode_ool_plane_objects(bytes: &[u8]) -> io::Result<Vec<ActiveObject>> {
    decode_active_object_table(bytes, "OOL plane table")
}

pub fn decode_saved_active_objects(bytes: &[u8]) -> io::Result<Vec<ActiveObject>> {
    let end = SAVE_ACTIVE_OBJECTS_OFFSET + OOL_PLANE_LEN;
    let table = bytes
        .get(SAVE_ACTIVE_OBJECTS_OFFSET..end)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "SAVED.GAM is too short"))?;
    decode_active_object_table(table, "SAVED.GAM active-object table")
}

pub fn decode_active_object_table(bytes: &[u8], label: &str) -> io::Result<Vec<ActiveObject>> {
    if bytes.len() != OOL_PLANE_LEN {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{label} must be {OOL_PLANE_LEN} bytes, got {}", bytes.len()),
        ));
    }

    let mut objects = Vec::with_capacity(OOL_SLOTS - 1);
    for (slot, record) in bytes.chunks_exact(OOL_RECORD_LEN).enumerate() {
        let type_byte = record[0];
        if slot == 0 {
            continue;
        }
        objects.push(ActiveObject {
            type_byte,
            tile: record[1],
            x: record[2] as usize,
            y: record[3] as usize,
            z: record[4] as i8,
            phase: record[6],
            aux1: record[5],
            aux3: record[7],
        });
    }
    Ok(objects)
}
