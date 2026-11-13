//! The gem map's observer: a probe before a command, the events after it
//! (Phase B design 10.3.3; build plan M3-05).
//!
//! [`probe`] takes a small `Copy` snapshot of what a command can change;
//! [`observe_command`] compares it with the state after the command and
//! names what the player discovered. Both read `&PlayState` only: `pub`
//! fields and `&self` helpers. Nothing here takes `&mut PlayState`, calls
//! a `&mut self` method, draws a random number or renders; a source-scan
//! test enforces it.
//!
//! The events of this file (M3-05, the place events):
//!
//! | Event | Evidence |
//! |---|---|
//! | place entered | `area` becomes a town or dungeon scene it was not; also at attach |
//! | Underworld visited | `area` becomes the Underworld surface; also at attach |
//! | shrine entered | `active_shrine` None to Some |
//! | Codex entered | `codex_presentation_steps` 0 to non-zero |
//! | shop opened | `active_shop` None to Some in a town, or a tavern turned to its sage; for Tavern and Sage, the one cell a keeper of that trade can be addressed from the party |
//! | moongate used | one live `0xDC` gate within a step of the party, which jumped or changed plane (and, on the same plane, the gate cell is no longer live) |
//! | town ladder or stairs | the floor changed, the party moved at most one step, and its cell on the old floor was `0xC4..=0xC9`; the direction is the floor change (L5) |
//! | town exit | town to world from an edge cell, or world to town onto one or one step inside one beside a passable edge cell |
//!
//! The events of [`super::probe_more`] (M3-06: Look, the dungeon events)
//! run from [`observe_commands`] too; the seen cells are in
//! [`super::seen`].
//!
//! **Ambiguity records nothing.** When the shell knows that more than one
//! command ran between the probe and the observation
//! ([`observe_commands`]), only the events whose evidence does not depend
//! on where the party stood are kept: a place, the Underworld, a shrine,
//! the Codex and a shop's trade. The keeper cell, a moongate, a ladder and
//! an exit are dropped.

use u5_runtime::shop_session::{ActiveShopSession, shop_session_for_talk_context};
use u5_runtime::{
    Area, Direction, DirectionPromptKind, NATURAL_MOONGATE_TERRAIN_TILE, PlayState, TOWN_GRID_SIDE,
    WORLD_SIDE, WorldPlane, YesNoPromptKind, enhancements::TranscriptCursor, world_cell_index,
};

use super::knowledge::{EdgeSide, ExitRun, GateCell, KnowEvent, LinkDir, TownLink, Trade};

/// The town ladder and stair tiles (`0xC4..=0xC7` ladders, `0xC8..=0xC9`
/// stairs; 10.2.7).
pub const TOWN_LINK_TILES: std::ops::RangeInclusive<u8> = 0xC4..=0xC9;

/// The eight directions, for the keeper search.
const DIRECTIONS: [Direction; 8] = [
    Direction::NorthWest,
    Direction::North,
    Direction::NorthEast,
    Direction::West,
    Direction::East,
    Direction::SouthWest,
    Direction::South,
    Direction::SouthEast,
];

/// What a command can change, taken just before it runs (10.3.2).
///
/// The 3 x 3 neighbourhoods are indexed `(dy + 1) * 3 + (dx + 1)`, so
/// index 4 is the party's own cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KnowProbe {
    pub area: Area,
    /// The party's cell and facing.
    pub at: (u8, u8, Direction),
    /// The current dungeon level's 64 cells, in a dungeon.
    pub level_image: Option<[u8; 64]>,
    pub direction_prompt: Option<DirectionPromptKind>,
    pub yes_no: Option<YesNoPromptKind>,
    pub shop_open: bool,
    /// The open shop's trade. A tavern hands over to its sage without
    /// closing, so the trade, not only `shop_open`, is compared.
    pub shop_trade: Option<Trade>,
    pub shrine_open: bool,
    pub codex_steps: u8,
    /// The party's own cell holds a live moongate.
    pub on_live_gate: bool,
    /// Bit `i` set: neighbourhood cell `i` holds a live moongate (world
    /// only).
    pub live_gates: u16,
    /// The town floor's terrain around the party (town only; `0` off the
    /// floor).
    pub town_tiles: [u8; 9],
    /// Bit `i` set: a blocking object (a person, a creature, a ship...)
    /// stood on neighbourhood cell `i` (town and world). A Look at such a
    /// cell prints the object, never the sign under it (10.6).
    pub objects_near: u16,
    /// The message transcript as it stood (chapter 8's cursor).
    pub transcript: TranscriptCursor,
}

/// Events of one observation. A plain `Vec`: the crate is std only.
pub type KnowEvents = Vec<KnowEvent>;

/// Takes the probe. Reads only.
pub fn probe(state: &PlayState) -> KnowProbe {
    let (px, py) = (state.player.x, state.player.y);
    let level_image = match state.area {
        Area::Dungeon { level, .. } => {
            let mut image = [0u8; 64];
            for (i, cell) in image.iter_mut().enumerate() {
                *cell = state.dungeon_cell(level, i % 8, i / 8);
            }
            Some(image)
        }
        _ => None,
    };
    let mut live_gates = 0u16;
    let mut objects_near = 0u16;
    let mut town_tiles = [0u8; 9];
    for (i, town_tile) in town_tiles.iter_mut().enumerate() {
        let (dx, dy) = (i as isize % 3 - 1, i as isize / 3 - 1);
        let (x, y) = (px as isize + dx, py as isize + dy);
        match state.area {
            Area::World { .. } => {
                if live_gate_at(state, x, y) {
                    live_gates |= 1 << i;
                }
            }
            Area::Town { .. } => *town_tile = state.gem_sample_tile(x, y, true),
            Area::Dungeon { .. } => {}
        }
        if super::probe_more::object_at(state, x, y) {
            objects_near |= 1 << i;
        }
    }
    KnowProbe {
        area: state.area,
        at: (px as u8, py as u8, state.player.facing),
        level_image,
        direction_prompt: state.active_direction_prompt.as_ref().map(|p| p.kind),
        yes_no: state.active_yes_no_prompt.as_ref().map(|p| p.kind),
        shop_open: state.active_shop.is_some(),
        shop_trade: state.active_shop.as_ref().map(Trade::of_session),
        shrine_open: state.active_shrine.is_some(),
        codex_steps: state.codex_presentation_steps,
        on_live_gate: live_gates & (1 << 4) != 0,
        live_gates,
        town_tiles,
        objects_near,
        transcript: TranscriptCursor::of(state),
    }
}

/// The events between `before` and `after`, for one command (10.3.3).
pub fn observe_command(before: &KnowProbe, after: &PlayState) -> KnowEvents {
    observe_commands(before, after, 1)
}

/// The events between `before` and `after` when `commands` commands ran
/// in between (a frame's `typed_keys` can rarely carry two). With more
/// than one, only the unambiguous events are kept (10.3.3).
pub fn observe_commands(before: &KnowProbe, after: &PlayState, commands: usize) -> KnowEvents {
    let single = commands <= 1;
    let mut events = KnowEvents::new();

    // Place entered.
    if let Some(scene) = place_scene(after.area)
        && place_scene(before.area) != Some(scene)
    {
        events.push(KnowEvent::PlaceEntered { scene });
    }
    // Underworld visited.
    if is_underworld(after.area) && !is_underworld(before.area) {
        events.push(KnowEvent::UnderworldVisited);
    }
    // Shrine entered.
    if !before.shrine_open
        && let Some(shrine) = after.active_shrine.as_ref()
    {
        events.push(KnowEvent::ShrineEntered {
            virtue: shrine.virtue as u8,
        });
    }
    // Codex entered.
    if before.codex_steps == 0 && after.codex_presentation_steps != 0 {
        events.push(KnowEvent::CodexEntered);
    }
    // Shop opened, or a tavern turned to its sage.
    if let Some(trade) = after.active_shop.as_ref().map(Trade::of_session)
        && before.shop_trade != Some(trade)
        && let Area::Town { scene, floor } = after.area
    {
        let keeper = if single && trade.is_unsigned() {
            keeper_cell(after, trade).map(|(x, y)| (floor, x, y))
        } else {
            None
        };
        events.push(KnowEvent::ShopOpened {
            scene: scene.byte,
            trade,
            keeper,
        });
    }
    if single {
        moongate_events(before, after, &mut events);
        town_link_events(before, after, &mut events);
        town_exit_events(before, after, &mut events);
        super::probe_more::look_events(before, after, &mut events);
        super::probe_more::fountain_events(before, after, &mut events);
    }
    super::probe_more::dungeon_events(before, after, single, &mut events);
    events
}

/// The events a record learns when it is attached to a running game:
/// "inside" uses the scene byte, so a save loaded inside a town counts as
/// having been there (10.2.8).
pub fn attach_events(state: &PlayState) -> KnowEvents {
    let mut events = KnowEvents::new();
    if let Some(scene) = place_scene(state.area) {
        events.push(KnowEvent::PlaceEntered { scene });
    }
    if is_underworld(state.area) {
        events.push(KnowEvent::UnderworldVisited);
    }
    events
}

fn place_scene(area: Area) -> Option<u8> {
    match area {
        Area::Town { scene, .. } => Some(scene.byte),
        Area::Dungeon { scene, .. } => Some(scene.byte),
        Area::World { .. } => None,
    }
}

fn is_underworld(area: Area) -> bool {
    matches!(
        area,
        Area::World {
            plane: WorldPlane::Underworld
        }
    )
}

fn plane_code(plane: WorldPlane) -> u8 {
    match plane {
        WorldPlane::Britannia => 0,
        WorldPlane::Underworld => 1,
    }
}

fn wrap_world(v: isize) -> usize {
    v.rem_euclid(WORLD_SIDE as isize) as usize
}

/// Whether world cell `(x, y)` (wrapped) holds a live natural moongate:
/// the gate tile in the grid and the cell in the live list.
fn live_gate_at(state: &PlayState, x: isize, y: isize) -> bool {
    let idx = world_cell_index(wrap_world(x), wrap_world(y));
    state.grid.get(idx).copied() == Some(NATURAL_MOONGATE_TERRAIN_TILE)
        && state.natural_moongate_live_cells.contains(&idx)
}

/// The one cell from which the party can address a keeper of `trade`, or
/// `None` when there is none or more than one. Uses the game's own Talk
/// target lookup in every direction; the roster is never read for a cell
/// the player did not just talk to (L4). A sage is reached through the
/// tavern keeper's menu, so its keeper is the tavern's.
fn keeper_cell(after: &PlayState, trade: Trade) -> Option<(u8, u8)> {
    let opened_by = match trade {
        Trade::Sage => Trade::Tavern,
        other => other,
    };
    let mut found: Option<(usize, usize)> = None;
    for direction in DIRECTIONS {
        let Some((dialog_id, x, y)) = after.talk_target_in_direction(direction) else {
            continue;
        };
        // Without a scene the mapping is the dialog's trade alone; the shop
        // that opened already passed the scene's own checks.
        let same_trade = shop_session_for_talk_context(dialog_id, None)
            .is_some_and(|session: ActiveShopSession| Trade::of_session(&session) == opened_by);
        if !same_trade {
            continue;
        }
        match found {
            None => found = Some((x, y)),
            Some(cell) if cell == (x, y) => {}
            Some(_) => return None,
        }
    }
    found.map(|(x, y)| (x as u8, y as u8))
}

/// Moongate used: the departure gate next to (or under) the party and the
/// arrival cell.
fn moongate_events(before: &KnowProbe, after: &PlayState, events: &mut KnowEvents) {
    let (Area::World { plane: from }, Area::World { plane: to }) = (before.area, after.area) else {
        return;
    };
    let (bx, by) = (before.at.0 as isize, before.at.1 as isize);
    let (ax, ay) = (after.player.x as isize, after.player.y as isize);
    let jumped = from != to || world_step_distance(bx, by, ax, ay) > 1;
    if !jumped || before.live_gates.count_ones() != 1 {
        return;
    }
    let i = before.live_gates.trailing_zeros() as isize;
    let (gx, gy) = (bx + i % 3 - 1, by + i / 3 - 1);
    if from == to && live_gate_at(after, gx, gy) {
        // The transit rewrites the gate it used; a gate still standing was
        // not the way the party went.
        return;
    }
    events.push(KnowEvent::GateUsed(GateCell {
        plane: plane_code(from),
        x: wrap_world(gx) as u8,
        y: wrap_world(gy) as u8,
    }));
    events.push(KnowEvent::GateUsed(GateCell {
        plane: plane_code(to),
        x: wrap_world(ax) as u8,
        y: wrap_world(ay) as u8,
    }));
}

/// The larger axis distance between two world cells, across the wrap.
fn world_step_distance(ax: isize, ay: isize, bx: isize, by: isize) -> usize {
    let side = WORLD_SIDE as isize;
    let axis = |a: isize, b: isize| {
        let d = (a - b).rem_euclid(side);
        d.min(side - d) as usize
    };
    axis(ax, bx).max(axis(ay, by))
}

/// Town ladder or stairs: the floor changed and the party left from a
/// ladder or stair cell.
fn town_link_events(before: &KnowProbe, after: &PlayState, events: &mut KnowEvents) {
    let (
        Area::Town {
            scene: from_scene,
            floor: from,
        },
        Area::Town {
            scene: to_scene,
            floor: to,
        },
    ) = (before.area, after.area)
    else {
        return;
    };
    if from_scene.byte != to_scene.byte || from == to {
        return;
    }
    let (bx, by) = (before.at.0 as isize, before.at.1 as isize);
    let (ax, ay) = (after.player.x as isize, after.player.y as isize);
    let (dx, dy) = (ax - bx, ay - by);
    if dx.abs() > 1 || dy.abs() > 1 {
        return;
    }
    let departure = before.town_tiles[((dy + 1) * 3 + (dx + 1)) as usize];
    if !TOWN_LINK_TILES.contains(&departure) {
        // A trapdoor or anything else that changes the floor.
        return;
    }
    let dir = if to > from {
        LinkDir::Up
    } else {
        LinkDir::Down
    };
    let scene = from_scene.byte;
    events.push(KnowEvent::TownLinkUsed(TownLink {
        scene,
        floor: from,
        x: ax as u8,
        y: ay as u8,
        dir,
    }));
    if TOWN_LINK_TILES.contains(&after.gem_sample_tile(ax, ay, true)) {
        events.push(KnowEvent::TownLinkUsed(TownLink {
            scene,
            floor: to,
            x: ax as u8,
            y: ay as u8,
            dir: dir.opposite(),
        }));
    }
}

/// The side a town floor cell is on and its place along it, or `None` off
/// the edge and on a corner (which belongs to two sides).
pub fn edge_side(x: u8, y: u8) -> Option<(EdgeSide, u8)> {
    let last = (TOWN_GRID_SIDE - 1) as u8;
    let sides = [
        (y == 0, EdgeSide::North, x),
        (x == last, EdgeSide::East, y),
        (y == last, EdgeSide::South, x),
        (x == 0, EdgeSide::West, y),
    ];
    let mut on = sides.iter().filter(|(hit, ..)| *hit);
    match (on.next(), on.next()) {
        (Some((_, side, at)), None) if x <= last && y <= last => Some((*side, *at)),
        _ => None,
    }
}

/// The gate a party arriving one step inside a town's edge came in by
/// (10.2.8 "entered the town through that run"): the town's start cell is
/// one step in from its gate (Britain's `(15, 30)` beside `(15, 31)`), so
/// the edge cell beside it is the run it came through. Only when exactly
/// one of the four neighbours is an edge cell and its drawn terrain is
/// foot-passable, so the cell is in a run the map draws as an exit; a
/// corner start or a wall beside it records nothing.
fn arrival_gate(after: &PlayState, x: u8, y: u8) -> Option<(EdgeSide, u8)> {
    let (x, y) = (i16::from(x), i16::from(y));
    let mut gates = [(0, 1), (0, -1), (1, 0), (-1, 0)]
        .into_iter()
        .filter_map(|(dx, dy)| {
            let (nx, ny) = (x + dx, y + dy);
            let side = TOWN_GRID_SIDE as i16;
            if !(0..side).contains(&nx) || !(0..side).contains(&ny) {
                return None;
            }
            edge_side(nx as u8, ny as u8).map(|gate| (gate, nx, ny))
        });
    let (gate, nx, ny) = gates.next()?;
    if gates.next().is_some() {
        return None;
    }
    let tile = after.gem_sample_tile(isize::from(nx), isize::from(ny), true);
    u5_runtime::predicates::foot_terrain_accepts(tile).then_some(gate)
}

/// Town exit: town to world from an edge cell, or world to town onto one
/// or one step inside one ([`arrival_gate`]).
fn town_exit_events(before: &KnowProbe, after: &PlayState, events: &mut KnowEvents) {
    let crossing = match (before.area, after.area) {
        (Area::Town { scene, .. }, Area::World { .. }) => {
            Some((scene.byte, before.at.0, before.at.1))
        }
        (Area::World { .. }, Area::Town { scene, .. }) => {
            Some((scene.byte, after.player.x as u8, after.player.y as u8))
        }
        _ => None,
    };
    let arriving = matches!(after.area, Area::Town { .. });
    if let Some((scene, x, y)) = crossing
        && let Some((side, at)) =
            edge_side(x, y).or_else(|| arriving.then(|| arrival_gate(after, x, y)).flatten())
    {
        events.push(KnowEvent::TownExit(ExitRun { scene, side, at }));
    }
}

#[cfg(test)]
#[path = "probe_tests.rs"]
mod tests;
