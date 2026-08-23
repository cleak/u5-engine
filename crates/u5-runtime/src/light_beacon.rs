//! `visibility.md §12.6`: the night-time rotating light beacon.
//!
//! This is **not** one of the disc-shaped local-light sources of
//! `visibility.md §12.1`-`§12.3`. It is a separate mechanism with its own
//! state and its own cadence — a rotating beam — that writes into the same
//! 32x32 local-light mask those sources build. It owns the small resident
//! coordinate/phase scratch block that earlier spec revisions attributed to
//! a "moongate animator"; that attribution is withdrawn in full
//! (`overworld.md §9`, `catalogs/tile-catalog.md §4`), and this pass never
//! draws a gate.
//!
//! Four rules, all from `visibility.md §12.6`:
//!
//! * **Sources** are harvested by whichever *map loader* is active, never by
//!   the light pass. Outdoors the chunk loader scans each freshly loaded
//!   32x32 window for the lighthouse tile and records the first hit as the
//!   single beacon position, or the "no beacon" sentinel when the window
//!   holds none; it never fills the second slot. Inside a location, map
//!   setup clears both positions and then records up to two hits on the
//!   bright-light tile. Combat entry switches the beacon off outright.
//! * **Light gate.** The pass first compares the ambient lighting value
//!   against the full-daylight value of fifty and runs only while it is
//!   *strictly below* it — from the first step of the dusk ramp to the last
//!   step of the dawn ramp. At or above fifty it clears its state, draws
//!   nothing, and the rotation restarts from its initial bearing next
//!   nightfall. This is a day/night test, **not** a distance threshold, and
//!   `lighting.md §7.2` names it the only read of the ambient value outside
//!   the `§5` visibility carve.
//! * **Beam shape.** Sixteen bearings evenly spaced around the compass:
//!   bearing one due north, five due east, nine due south, thirteen due
//!   west, four on the diagonals and eight halfway between. Each bearing is
//!   a fixed set of at most sixteen cell offsets reaching up to seven tiles
//!   — a stencil, not a computed sweep. `formats/tiles.md §5.1.1` publishes
//!   the stencil table's location, structure, per-record heading and every
//!   offset, so this module *anchors* to it rather than hunting for it.
//! * **Cadence.** Three adjacent bearings are lit at once (a cone a little
//!   under seventy degrees wide). Once per world turn the trailing bearing
//!   is cleared and the next leading bearing lit, so the cone advances one
//!   sixteenth of a revolution per turn and the counter wraps at sixteen.
//!
//! An implementation that omits the beacon loses only night-time
//! illumination around lighthouses and indoor lamps.

use std::io;
use std::path::Path;

use crate::play_state_impl::surface_local_light_mask_index;
use crate::*;

/// `visibility.md §12.6`: the outdoor beacon source tile — the lighthouse.
///
/// `catalogs/tile-catalog.md §5` fixes the id: "decimal `22`, `23`, and
/// `24` are the three dungeon-entrance variants named here, decimal `25`
/// and `26` are the shrine pair, and decimal `27` is a lighthouse" — the
/// named row that precedes it lists those entrance variants as `0x16`,
/// `0x17`, `0x18` and the shrine pair as `0x19`/`0x1A`, so decimal 27 is
/// `0x1B`. Confirmed against the shipped description table
/// (`formats/look2-dat.md`), whose terrain entry `0x1B` reads
/// "a lighthouse", and against the shipped surface map, which holds
/// exactly four `0x1B` cells, at the four coordinates
/// `catalogs/gazetteer.md §8.1` publishes.
pub const BEACON_LIGHTHOUSE_TILE: u8 = 0x1B;

/// `visibility.md §12.6`: the indoor beacon source tile — the bright light.
///
/// `systems/npc-schedules.md §9` places "roof, crystal sphere, bright
/// light, and hollow stump" in `0x27..=0x2B`, and
/// `catalogs/tile-catalog.md §2` fixes crystal sphere at `0x29` and hollow
/// stump at `0x2B`, which leaves the bright light unresolved between the
/// remaining ids. The shipped description table settles it:
/// `formats/look2-dat.md` terrain entry `0x2A` reads "a bright light"
/// (`0x27` and `0x28` both read "a roof"). The shipped location maps agree
/// with `§12.6`'s "up to two": no `0x2A` cell exists anywhere on either
/// outdoor map, and the only location floors carrying one are the four
/// lighthouse lantern rooms, three with one cell and one with two.
///
/// **The competing "player spawn marker" reading is withdrawn.**
/// `formats/location-dat.md §6` used to read this byte as "a player spawn
/// or stairway-up landing point" harvested into primary/secondary spawn
/// slots; that section now says in full that "`0x2A` is not a player spawn
/// marker. It is the night beacon's indoor light source", and settles it
/// without reference to any code: **`0x2A` appears in zero town, castle and
/// keep floors** — five cells across four dwelling-class floors, the four
/// lighthouse lantern rooms — and "a player town-entry spawn marker that
/// exists in no town is not a spawn marker". Reproduced against the shipped
/// `TOWNE.DAT`, `DWELLING.DAT`, `CASTLE.DAT` and `KEEP.DAT` by
/// `shipped_location_files_carry_the_published_beacon_source_layout`.
pub const BEACON_BRIGHT_LIGHT_TILE: u8 = 0x2A;

/// `visibility.md §12.6`: the beacon holds at most two source positions.
/// Outdoors only the first is ever filled.
pub const BEACON_SOURCE_SLOTS: usize = 2;

/// `visibility.md §12.6`: "sixteen bearings evenly spaced around the
/// compass"; the bearing counter wraps at sixteen.
pub const BEACON_BEARING_COUNT: u8 = 16;

/// `visibility.md §12.6`: "three adjacent bearings are lit at any moment —
/// a cone roughly three sixteenths of the compass wide".
pub const BEACON_CONE_BEARINGS: usize = 3;

/// `visibility.md §12.6`: "the beam is a cone of lit cells reaching up to
/// seven tiles from the source". Used as the per-axis bound on a stencil
/// offset when locating the bearing table.
pub const BEACON_BEAM_MAX_REACH: u8 = 7;

/// `visibility.md §12.6`: "each bearing is a fixed set of at most sixteen
/// cell offsets relative to the source".
pub const BEACON_STENCIL_MAX_OFFSETS: usize = 16;

/// One bearing record: [`BEACON_STENCIL_MAX_OFFSETS`] signed `(dx, dy)`
/// byte pairs (`formats/tiles.md §5.1.1` — "a table of thirty-two-byte
/// records made of sixteen signed byte pairs", walked "sixteen pairs at a
/// time" and indexed by a frame number reduced modulo sixteen).
pub const BEACON_STENCIL_RECORD_BYTES: usize = BEACON_STENCIL_MAX_OFFSETS * 2;

/// The whole bearing table: one record per bearing.
pub const BEACON_STENCIL_TABLE_BYTES: usize =
    BEACON_STENCIL_RECORD_BYTES * BEACON_BEARING_COUNT as usize;

/// `formats/tiles.md §5.1.1`: "it lives in the shared data overlay at file
/// offset `0x1F8E` and is **512 bytes**".
///
/// This offset is published contract, not a guess. Earlier revisions of
/// this module located the table by structural search because the offsets
/// were unpublished; `§5.1.1` now carries the table's location, structure,
/// per-record heading mapping and every offset, and the section adds that
/// "locating it structurally is unnecessary but safe" — the shipped image
/// holds exactly one region matching the structural rules and it is this
/// table. [`scan_beacon_bearing_stencil_offsets`] keeps the search alive as
/// a standing cross-check of that agreement; nothing in the load path uses
/// it.
pub const BEACON_STENCIL_TABLE_OFFSET: usize = 0x1F8E;

/// `formats/tiles.md §5.1.1`: "the four **cardinals** (records 1, 5, 9, 13)
/// light **fifteen** cells".
pub const BEACON_CARDINAL_CELLS: usize = 15;

/// `formats/tiles.md §5.1.1`: "the four **diagonals** (3, 7, 11, 15) light
/// **eleven**".
pub const BEACON_DIAGONAL_CELLS: usize = 11;

/// `formats/tiles.md §5.1.1`: "the eight **halfway** bearings light
/// **nine**".
pub const BEACON_HALFWAY_CELLS: usize = 9;

/// `formats/tiles.md §5.1.1`: "cell counts follow the heading class
/// exactly".
///
/// `index` is the record index, and record *r* carries the heading
/// `(r - 1) * 22.5` degrees clockwise from north — so the step count from
/// north is `(index - 1) mod 16`, a multiple of four on the cardinals and
/// an even non-multiple on the diagonals.
pub const fn beacon_record_cell_count(index: usize) -> usize {
    let steps = (index + BEACON_BEARING_COUNT as usize - 1) % BEACON_BEARING_COUNT as usize;
    match steps % 4 {
        0 => BEACON_CARDINAL_CELLS,
        2 => BEACON_DIAGONAL_CELLS,
        _ => BEACON_HALFWAY_CELLS,
    }
}

/// `visibility.md §12.6` does not name a numeric initial bearing; it says
/// only that the rotation "restarts from its initial bearing" whenever the
/// day/night gate closes, and that the shipped data image starts the whole
/// scratch block cleared. The observable contract is therefore "the same
/// bearing every nightfall", which the zero-cleared counter satisfies.
///
/// Index zero is spec bearing *sixteen* — see
/// [`BeaconBearingStencils::bearing`] for the index/bearing-number
/// relation.
pub const BEACON_INITIAL_BEARING: u8 = 0;

/// `visibility.md §12.6` light gate: the pass runs only while the ambient
/// lighting value is **strictly below** the full-daylight value of fifty.
///
/// This is a day/night flag, not a distance threshold
/// (`lighting.md §7.2`). Its polarity is the opposite of the withdrawn
/// "daylight threshold" precondition earlier revisions carried
/// (`overworld.md §9`: "the beacon that owns that gate runs only *after
/// dark*; nothing runs it by day"). Under the `lighting.md §3` clock
/// schedule, strictly-below-fifty is true from the first step of the dusk
/// ramp (hour 19, minute 0, value 49) through the last step of the dawn
/// ramp (hour 5, minute 50, value 49), and false across hours 6..=18.
pub const fn beacon_pass_runs(ambient: u8) -> bool {
    ambient < FULL_DAYLIGHT
}

/// `visibility.md §12.6`: "the cone advances one sixteenth of a revolution
/// per turn ... the bearing counter wraps at sixteen".
pub const fn beacon_next_bearing(bearing: u8) -> u8 {
    (bearing + 1) % BEACON_BEARING_COUNT
}

/// `visibility.md §12.6`: the three adjacent bearings lit at one moment.
///
/// KNOWN GAP: `§12.6` fixes the cone's width (three bearings) and its
/// cadence ("the trailing bearing is cleared and the next leading bearing
/// is lit") but never says which of the three the counter names. This
/// engine treats the counter as the trailing bearing, so the cone is
/// `b, b + 1, b + 2`. Any other choice differs from this one only by a
/// constant phase offset of at most two turns; nothing observable in the
/// published contract distinguishes them.
pub const fn beacon_cone_bearings(bearing: u8) -> [u8; BEACON_CONE_BEARINGS] {
    [
        bearing % BEACON_BEARING_COUNT,
        (bearing + 1) % BEACON_BEARING_COUNT,
        (bearing + 2) % BEACON_BEARING_COUNT,
    ]
}

/// `visibility.md §12.6` beacon state: up to two source positions plus the
/// beam's current bearing — the resident coordinate/phase scratch block
/// `formats/data-ovl.md §6.3` describes.
///
/// A source slot holding `None` is the spec's "no beacon" sentinel. The
/// shipped data image starts with both positions at that sentinel, so
/// nothing is lit until a loader finds a source.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LightBeaconState {
    pub sources: [Option<(u8, u8)>; BEACON_SOURCE_SLOTS],
    pub bearing: u8,
}

impl Default for LightBeaconState {
    fn default() -> Self {
        Self::new()
    }
}

impl LightBeaconState {
    /// Both positions at the "no beacon" sentinel, bearing at its initial
    /// value — the shipped data image's starting state.
    pub const fn new() -> Self {
        Self {
            sources: [None; BEACON_SOURCE_SLOTS],
            bearing: BEACON_INITIAL_BEARING,
        }
    }

    /// True while no source slot holds a position: nothing to light.
    pub fn is_off(&self) -> bool {
        self.sources.iter().all(Option::is_none)
    }

    /// `visibility.md §12.6`: "combat entry switches the beacon off
    /// outright". The source slots are the beacon's on/off state, so both
    /// return to the sentinel; the bearing returns to its initial value
    /// with them, exactly as the daylight clear leaves it. Nothing
    /// re-harvests on combat *exit* — `§12.6` gives the beacon no exit
    /// trigger (unlike the local-light mask of `§12.4`), so the beacon
    /// stays off until a map loader next runs.
    pub fn switch_off(&mut self) {
        *self = Self::new();
    }

    /// `visibility.md §12.6` daylight clear: "at or above fifty the pass
    /// clears its state and draws nothing, and the rotation restarts from
    /// its initial bearing the next time darkness falls". The harvested
    /// source positions belong to the map loader, not to the pass, so the
    /// clear resets the beam state only. Returns whether anything changed.
    pub fn clear_beam_state(&mut self) -> bool {
        let changed = self.bearing != BEACON_INITIAL_BEARING;
        self.bearing = BEACON_INITIAL_BEARING;
        changed
    }

    /// `visibility.md §12.6`: advance the cone one sixteenth of a
    /// revolution. Returns whether anything changed — the pass "sets the
    /// visibility-dirty flag when it changes anything", and a beacon with
    /// no source changes nothing.
    pub fn advance_bearing(&mut self) -> bool {
        if self.is_off() {
            return false;
        }
        self.bearing = beacon_next_bearing(self.bearing);
        true
    }
}

/// `visibility.md §12.6`: outdoor source harvest.
///
/// "The chunk loader scans each freshly loaded thirty-two-by-thirty-two
/// window for the lighthouse tile and records the first hit as the single
/// beacon position, or records a 'no beacon' sentinel when the window holds
/// none. It never fills the second position."
///
/// `origin` is the window's top-left world cell; `tile_at` is queried with
/// world coordinates already wrapped into `0..WORLD_SIDE`. The scan is
/// row-major, matching the `§12.1` mask scan; the shipped surface map does
/// not discriminate scan orders here, because its four lighthouses
/// (`catalogs/gazetteer.md §8.1`) are far enough apart that no 32x32
/// window can contain two.
pub fn harvest_outdoor_beacon_sources(
    origin: (usize, usize),
    tile_at: impl Fn(usize, usize) -> u8,
) -> [Option<(u8, u8)>; BEACON_SOURCE_SLOTS] {
    for row in 0..LOCAL_LIGHT_MASK_SIDE {
        for col in 0..LOCAL_LIGHT_MASK_SIDE {
            let x = (origin.0 + col) % WORLD_SIDE;
            let y = (origin.1 + row) % WORLD_SIDE;
            if tile_at(x, y) == BEACON_LIGHTHOUSE_TILE {
                return [Some((x as u8, y as u8)), None];
            }
        }
    }
    [None; BEACON_SOURCE_SLOTS]
}

/// `visibility.md §12.6`: indoor source harvest.
///
/// "Inside a location, the map setup clears both positions and then
/// records up to two hits on the bright-light tile." The grid is one 32x32
/// location floor.
///
/// Two published details this walk has to get right, both from
/// `formats/location-dat.md §6`:
///
/// * **Loader order** is column-major — "column 0 north-to-south, then
///   column 1, and so on". The beacon share the NPC start markers' single
///   grid walk ("one walk, two purposes"), so they share its order.
/// * **The slot rule is not first-then-second.** "The walk tests only
///   whether the *first* slot is still empty. So the **first** hit takes
///   slot one and, once slot one is filled, **every later hit overwrites
///   slot two** — meaning the **last** hit wins slot two, not the second."
///   No shipped floor carries three hits, so this is invisible in shipped
///   data and observable only in custom data.
///
/// The walk never stops early: with both slots filled it keeps overwriting
/// slot two, which is what makes the last hit win.
pub fn harvest_location_beacon_sources(grid: &[u8]) -> [Option<(u8, u8)>; BEACON_SOURCE_SLOTS] {
    let mut sources = [None; BEACON_SOURCE_SLOTS];
    for col in 0..LOCAL_LIGHT_MASK_SIDE {
        for row in 0..LOCAL_LIGHT_MASK_SIDE {
            let Some(&tile) = grid.get(row * LOCAL_LIGHT_MASK_SIDE + col) else {
                continue;
            };
            if tile != BEACON_BRIGHT_LIGHT_TILE {
                continue;
            }
            let position = Some((col as u8, row as u8));
            if sources[0].is_none() {
                sources[0] = position;
            } else {
                sources[1] = position;
            }
        }
    }
    sources
}

/// `visibility.md §12.6`: the sixteen bearing stencils.
///
/// Each bearing is "a fixed set of at most sixteen cell offsets relative to
/// the source", so a bearing is a stencil, not a computed sweep. The
/// offsets live in the shared data overlay as sixteen thirty-two-byte
/// records of sixteen signed byte pairs, at the published offset
/// [`BEACON_STENCIL_TABLE_OFFSET`] (`formats/tiles.md §5.1.1`). This engine
/// reads them out of the shipped `DATA.OVL` at load time rather than
/// carrying a copy: the table is published prose now, but the bytes stay
/// where the game ships them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BeaconBearingStencils {
    offsets: [[(i8, i8); BEACON_STENCIL_MAX_OFFSETS]; BEACON_BEARING_COUNT as usize],
    lengths: [u8; BEACON_BEARING_COUNT as usize],
}

impl BeaconBearingStencils {
    /// The whole record for one bearing — **all sixteen pairs**, padding
    /// included. This is what the stamp iterates.
    ///
    /// `formats/tiles.md §5.1.1`: "the stamp always runs **all sixteen
    /// iterations** of a record - there is no early exit on the `(0, 0)`
    /// padding, so a padded pair writes at the record's own origin cell,
    /// which is harmless because that cell is the source". Every shipped
    /// record has at least one padded pair (the longest lights fifteen
    /// cells), so the source cell is always among the cells a live beacon
    /// writes.
    ///
    /// `visibility.md §12.6` numbers bearings from one: "bearing one points
    /// due north, five due east, nine due south, thirteen due west".
    /// `formats/tiles.md §5.1.1` says the records are indexed by a frame
    /// number "reduced modulo sixteen", which puts bearing sixteen at index
    /// zero. Record index `r` therefore carries the heading
    /// `(r - 1) * 22.5` degrees clockwise from north, and this method takes
    /// the wrapped index — [`beacon_cone_bearings`] produces exactly that.
    pub fn bearing(&self, bearing: u8) -> &[(i8, i8); BEACON_STENCIL_MAX_OFFSETS] {
        &self.offsets[(bearing % BEACON_BEARING_COUNT) as usize]
    }

    /// The live cells of one bearing: the contiguous run of non-padding
    /// pairs at the head of its record.
    ///
    /// This is the published cell list — fifteen entries on a cardinal,
    /// eleven on a diagonal, nine on a halfway bearing
    /// ([`beacon_record_cell_count`]). It is the geometry to assert
    /// against; [`Self::bearing`] is what the stamp walks.
    pub fn cells(&self, bearing: u8) -> &[(i8, i8)] {
        let index = (bearing % BEACON_BEARING_COUNT) as usize;
        &self.offsets[index][..self.lengths[index] as usize]
    }
}

/// How far a stencil offset may sit from its bearing's nominal heading
/// before the candidate table is rejected, in degrees.
///
/// `visibility.md §12.6` fixes the sixteen headings (bearing one north,
/// five east, nine south, thirteen west, four diagonals, eight halfway
/// between), so every offset in a record must point broadly along its own
/// bearing. Forty-five degrees is a deliberately loose bound — an offset
/// must merely be nearer its own heading than the perpendicular bearings —
/// and the shipped `DATA.OVL` still yields exactly one candidate under it.
const BEACON_STENCIL_HEADING_TOLERANCE_DEGREES: f64 = 45.0;

/// Degrees between adjacent bearings: a full revolution over
/// [`BEACON_BEARING_COUNT`] bearings (`visibility.md §12.6`, "sixteen
/// bearings evenly spaced around the compass").
const BEACON_BEARING_STEP_DEGREES: f64 = 360.0 / BEACON_BEARING_COUNT as f64;

/// `visibility.md §12.6` nominal heading of one bearing record, in degrees
/// clockwise from north. Index zero is bearing sixteen, so the record index
/// runs one ahead of the step count.
fn beacon_record_heading_degrees(index: usize) -> f64 {
    let steps = (index + BEACON_BEARING_COUNT as usize - 1) % BEACON_BEARING_COUNT as usize;
    steps as f64 * BEACON_BEARING_STEP_DEGREES
}

/// Whether one signed offset points along `index`'s nominal heading, within
/// [`BEACON_STENCIL_HEADING_TOLERANCE_DEGREES`]. Screen coordinates: `+x`
/// east, `+y` south, so north is `-y`.
pub(crate) fn beacon_offset_matches_bearing(dx: i8, dy: i8, index: usize) -> bool {
    let angle = f64::from(dx)
        .atan2(-f64::from(dy))
        .to_degrees()
        .rem_euclid(360.0);
    let deviation = (angle - beacon_record_heading_degrees(index)).rem_euclid(360.0);
    let deviation = deviation.min(360.0 - deviation);
    deviation < BEACON_STENCIL_HEADING_TOLERANCE_DEGREES
}

/// Validate one thirty-two-byte bearing record and return its cell count.
///
/// Structure, all from `formats/tiles.md §5.1.1`: sixteen signed
/// `(dx, dy)` pairs; "live pairs are contiguous from the start of a record;
/// every remaining pair is exactly `(0, 0)` and means 'no cell'"; "no
/// component exceeds seven in magnitude, and no record repeats a pair"; the
/// cell count is fixed by the heading class ([`beacon_record_cell_count`]);
/// and every offset points along the record's own bearing.
fn beacon_record_offsets(record: &[u8], index: usize) -> Option<usize> {
    let mut cells = [(0i8, 0i8); BEACON_STENCIL_MAX_OFFSETS];
    let mut length = 0usize;
    let mut padding_seen = false;
    for pair in record.chunks_exact(2) {
        let dx = pair[0] as i8;
        let dy = pair[1] as i8;
        if (dx, dy) == (0, 0) {
            padding_seen = true;
            continue;
        }
        if padding_seen
            || dx.unsigned_abs() > BEACON_BEAM_MAX_REACH
            || dy.unsigned_abs() > BEACON_BEAM_MAX_REACH
            || !beacon_offset_matches_bearing(dx, dy, index)
            || cells[..length].contains(&(dx, dy))
        {
            return None;
        }
        cells[length] = (dx, dy);
        length += 1;
    }
    (length == beacon_record_cell_count(index)).then_some(length)
}

/// Parse a candidate [`BEACON_STENCIL_TABLE_BYTES`]-byte window as the
/// sixteen bearing stencils, or reject it.
pub fn parse_beacon_bearing_stencils(bytes: &[u8]) -> Option<BeaconBearingStencils> {
    if bytes.len() != BEACON_STENCIL_TABLE_BYTES {
        return None;
    }
    let mut table = BeaconBearingStencils {
        offsets: [[(0, 0); BEACON_STENCIL_MAX_OFFSETS]; BEACON_BEARING_COUNT as usize],
        lengths: [0; BEACON_BEARING_COUNT as usize],
    };
    for index in 0..BEACON_BEARING_COUNT as usize {
        let start = index * BEACON_STENCIL_RECORD_BYTES;
        let record = &bytes[start..start + BEACON_STENCIL_RECORD_BYTES];
        let length = beacon_record_offsets(record, index)?;
        for (slot, pair) in record.chunks_exact(2).take(length).enumerate() {
            table.offsets[index][slot] = (pair[0] as i8, pair[1] as i8);
        }
        table.lengths[index] = length as u8;
    }
    Some(table)
}

/// Read the sixteen bearing stencils out of a `DATA.OVL` image, at the
/// published offset.
///
/// `formats/tiles.md §5.1.1` puts the table "in the shared data overlay at
/// file offset `0x1F8E`", 512 bytes. There is nothing to search for and
/// nothing to guess: the bytes at that offset either match the published
/// structure or the image is not one this engine can read.
pub fn read_beacon_bearing_stencils(data: &[u8]) -> io::Result<BeaconBearingStencils> {
    let end = BEACON_STENCIL_TABLE_OFFSET + BEACON_STENCIL_TABLE_BYTES;
    let window = data.get(BEACON_STENCIL_TABLE_OFFSET..end).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{DATA_OVL_FILENAME} is {} bytes, too short to hold the beacon \
                 bearing-stencil table published at offset {BEACON_STENCIL_TABLE_OFFSET:#x} \
                 (formats/tiles.md §5.1.1)",
                data.len()
            ),
        )
    })?;
    parse_beacon_bearing_stencils(window)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, BEACON_STENCIL_MISMATCH_MESSAGE))
}

/// The failure `formats/tiles.md §5.1.1` demands be loud: "an
/// implementation ... should fail loudly on zero candidates rather than
/// silently lighting nothing".
const BEACON_STENCIL_MISMATCH_MESSAGE: &str = "DATA.OVL holds no beacon bearing-stencil table matching formats/tiles.md §5.1.1 \
     at the published offset";

/// Every offset in `data` at which a [`BEACON_STENCIL_TABLE_BYTES`]-byte
/// window matches the published structural rules.
///
/// **Nothing in the load path calls this.** `formats/tiles.md §5.1.1`
/// publishes the table's offset, so [`read_beacon_bearing_stencils`]
/// anchors to it; but the same section records that a structural search of
/// the shipped overlay "yields **exactly one** candidate, and it is this
/// table", reproduced independently twice. This function exists so that
/// claim is a standing assertion against the shipped image
/// (`the_structural_search_agrees_with_the_published_offset`) rather than a
/// one-off check — a published offset that stopped agreeing with the
/// structure would otherwise be silent.
pub fn scan_beacon_bearing_stencil_offsets(data: &[u8]) -> Vec<usize> {
    let mut found = Vec::new();
    if data.len() < BEACON_STENCIL_TABLE_BYTES {
        return found;
    }
    // Every byte of the table is a signed offset of at most seven cells,
    // so only runs of in-range bytes can hold it; restricting the window
    // scan to those runs keeps the cross-check cheap.
    let in_reach = |byte: u8| (byte as i8).unsigned_abs() <= BEACON_BEAM_MAX_REACH;
    let mut run_start = None;
    for position in 0..=data.len() {
        let inside = position < data.len() && in_reach(data[position]);
        match (inside, run_start) {
            (true, None) => run_start = Some(position),
            (false, Some(start)) => {
                run_start = None;
                if position - start < BEACON_STENCIL_TABLE_BYTES {
                    continue;
                }
                for offset in start..=position - BEACON_STENCIL_TABLE_BYTES {
                    let window = &data[offset..offset + BEACON_STENCIL_TABLE_BYTES];
                    if parse_beacon_bearing_stencils(window).is_some() {
                        found.push(offset);
                    }
                }
            }
            _ => {}
        }
    }
    found
}

/// Read the beacon bearing stencils out of a game directory's `DATA.OVL`.
///
/// **There is no quiet failure here.** An absent file, a short file, or an
/// image whose published offset does not carry the published structure are
/// all errors. This used to return `Ok(None)` for each of those and leave
/// the beacon lighting nothing, which `formats/tiles.md §5.1.1` names as
/// the wrong answer: "it should fail loudly on zero candidates rather than
/// silently lighting nothing". A dark beacon and a missing table are not
/// distinguishable to a player, which is exactly why the loader has to
/// distinguish them.
pub fn load_beacon_bearing_stencils(game_dir: &Path) -> io::Result<BeaconBearingStencils> {
    let path = game_dir.join(DATA_OVL_FILENAME);
    let data = read(&path)?;
    read_beacon_bearing_stencils(&data)
}

impl PlayState {
    /// `visibility.md §12.6` per-turn rotation, run by the same per-turn
    /// cleanup that recomputes ambient daylight.
    ///
    /// The light gate comes first and decides everything: at or above full
    /// daylight the pass clears its beam state and draws nothing; below it,
    /// the cone advances one bearing. Either way the pass "sets the
    /// visibility-dirty flag when it changes anything".
    pub fn advance_light_beacon(&mut self) {
        let changed = if beacon_pass_runs(self.ambient_light) {
            self.light_beacon.advance_bearing()
        } else {
            self.light_beacon.clear_beam_state()
        };
        if changed {
            self.mark_visibility_dirty();
        }
    }

    /// `visibility.md §12.6` outdoor source harvest, run by the chunk
    /// loader after it refreshes the live 32x32 window.
    pub fn harvest_outdoor_light_beacon(&mut self) {
        let Some(buffer) = &self.world_live_chunks else {
            self.light_beacon.sources = [None; BEACON_SOURCE_SLOTS];
            return;
        };
        let origin = buffer.scroll_base;
        self.light_beacon.sources =
            harvest_outdoor_beacon_sources(origin, |x, y| buffer.tile_at(x, y));
    }

    /// `visibility.md §12.6` indoor source harvest, run by location map
    /// setup: clear both positions, then record up to two bright-light hits
    /// on the freshly loaded floor.
    pub fn harvest_location_light_beacon(&mut self) {
        self.light_beacon.sources = harvest_location_beacon_sources(&self.grid);
    }

    /// `visibility.md §12.6` beam stamp: write the three lit bearings of
    /// every live source straight into the local-light mask.
    ///
    /// `visibility.md §12.4` fixes the order inside a non-combat redraw —
    /// "local-light refresh first, beacon stamps second, visibility carve
    /// third" — so this runs after the disc-shaped sources of `§12.1` have
    /// filled the same mask, and the producer then reads the union. Cells
    /// outside the 32x32 window are dropped.
    ///
    /// The inner loop walks the **whole** sixteen-pair record, padding
    /// included (`formats/tiles.md §5.1.1`: "the stamp always runs all
    /// sixteen iterations ... there is no early exit on the `(0, 0)`
    /// padding"). A padded pair resolves to `(0, 0)` and so writes the
    /// source cell itself.
    pub(crate) fn stamp_light_beacon(
        &self,
        mask: &mut [bool],
        origin_x: isize,
        origin_y: isize,
        wrap_world: bool,
    ) {
        if !beacon_pass_runs(self.ambient_light) {
            return;
        }
        let stencils = &self.beacon_bearing_stencils;
        for source in self.light_beacon.sources.iter().flatten() {
            for bearing in beacon_cone_bearings(self.light_beacon.bearing) {
                for &(dx, dy) in stencils.bearing(bearing) {
                    let x = isize::from(source.0) + isize::from(dx);
                    let y = isize::from(source.1) + isize::from(dy);
                    if let Some(index) =
                        surface_local_light_mask_index(origin_x, origin_y, x, y, wrap_world)
                    {
                        mask[index] = true;
                    }
                }
            }
        }
    }
}
