//! The gem map's observer, part 2: Look and the dungeon events (Phase B
//! design 10.3.3; build plan M3-06).
//!
//! Like [`super::probe`], everything here reads `&PlayState` only: `pub`
//! fields and `&self` helpers. Nothing takes `&mut PlayState`, calls a
//! `&mut self` method or draws a random number; the source-scan test of
//! the observer covers this file too.
//!
//! | Event | Evidence |
//! |---|---|
//! | sign read | one new `Look-<direction>` echo followed by the `Thou dost see` preamble row, the party where it was, no blocking object on the target before or after, and the target's live tile passes `sign_or_wanted_poster_object_class` |
//! | entrance named | the same Look outdoors, no object and no sign on the target, and the target (after the look-redirect chain) is `0xDF` at one of the eight X columns whose name Look appends (`view.md §3` row 5c, D10.7) |
//! | dungeon ladder | same X and Y, the level changed by one, and the departure cell is a ladder that goes that way (`0x1?` up, `0x2?` down, `0x3?` both); both ends are marked when the landing is a ladder too. A Klimb off the top (or bottom) edge marks the departure ladder |
//! | pit climbed down | same X and Y, one level down, from a `0x6?` cell: the pit is found |
//! | fall | the level grew after a step onto a fall trap (`0x61`, `0x69`): every trap cell of the chain is found and every cell of it is remembered |
//! | cell rewrite | the probe's level image against the state, at the party cell (before and after) or a Search/Look focus cell only: `0x61` to `0x60` and the bomb `0x62`/`0x6A` masked to `0x08` (trap found), `0xD?` to `0xE?` (secret door found), `0x4?` to `0x7?` (chest opened) |
//! | fountain drunk | the probe held the dungeon fountain's Yes/No prompt, it closed, and the transcript gained the `Yes.  Gulp!` row |
//!
//! **What records nothing** (10.3.3, 10.6): a Look whose preamble never
//! printed (the visibility preflight failed, or it was cancelled), a Look
//! at a person or creature standing on a sign, a fountain answered No, a
//! rewrite away from the party and its focus cells, and any transcript
//! diff that cannot anchor. The fountain's inline harness form (`L` with
//! the answer typed in the same command) opens no prompt and is not
//! recorded: a "?" too many, never a name too early.
//!
//! With more than one command between probe and observation, only the
//! rewrites at a cell the party stood on are kept (the focus cells, Look,
//! the fountain, ladders and falls depend on where the party was when).

use u5_runtime::{
    Area, DUNGEON_FOUNTAIN_ACCEPTED, Direction, DungeonLookFocus, LOOK_REDIRECT_EAST_TILE,
    LOOK_REDIRECT_NORTH_TILE, LOOK_REDIRECT_WEST_TILE, LOOK_RESULT_PREFIX, MessageEntry, PlayState,
    SCENE_OVERWORLD, TOWN_GRID_SIDE, WORD_OF_POWER_SEALED_TILE, WORLD_SIDE, YesNoPromptKind,
    is_dungeon_bomb_trap, is_dungeon_fall_trap, sign_or_wanted_poster_object_class,
    world_cell_index,
};

use super::knowledge::{DungeonMark, KnowEvent, MarkKind, SignCell};
use super::probe::{KnowEvents, KnowProbe};

/// `view.md §3` row 5c: the map X columns at which Look appends a dungeon
/// name to the `0xDF` description, and the dungeon record each names
/// (records in `DungeonScene` order). A test pins this table to the
/// runtime's own `look_description`.
pub const ENTRANCE_NAME_COLUMNS: [(u8, u8); 8] = [
    (58, 5),  // Shame
    (72, 2),  // Destard
    (91, 1),  // Despise
    (126, 3), // Wrong
    (128, 7), // Doom
    (156, 4), // Covetous
    (239, 6), // Hythloth
    (240, 0), // Deceit
];

/// The dungeon record whose name Look appends at map column `x`, if any.
pub fn entrance_record_for_column(x: u8) -> Option<u8> {
    ENTRANCE_NAME_COLUMNS
        .iter()
        .find(|(column, _)| *column == x)
        .map(|(_, record)| *record)
}

/// Dungeon level side, in cells.
const LEVEL_SIDE: i16 = 8;

/// Whether a blocking object stands on cell `(x, y)` of the current town
/// floor or (wrapped) world plane. Off the town floor there is none.
pub(super) fn object_at(state: &PlayState, x: isize, y: isize) -> bool {
    match state.area {
        Area::Town { .. } => {
            let side = TOWN_GRID_SIDE as isize;
            (0..side).contains(&x)
                && (0..side).contains(&y)
                && state.blocking_object_at(x as usize, y as usize).is_some()
        }
        Area::World { .. } => state
            .world_object_at(wrap_world(x), wrap_world(y))
            .is_some(),
        Area::Dungeon { .. } => false,
    }
}

fn wrap_world(v: isize) -> usize {
    v.rem_euclid(WORLD_SIDE as isize) as usize
}

/// The entries the command added to the transcript, or `None` when the
/// diff cannot anchor (cleared, capped, ambiguous or rebuilt).
fn new_entries<'a>(before: &KnowProbe, after: &'a PlayState) -> Option<&'a [MessageEntry]> {
    before
        .transcript
        .diff_state(after)
        .new_entries(after.message_entries())
}

/// The direction of a `Look-<direction>` command echo.
fn look_echo_direction(entry: &MessageEntry) -> Option<Direction> {
    if !entry.is_command_echo {
        return None;
    }
    let text = entry.text.trim();
    let word = text
        .strip_prefix('>')
        .unwrap_or(text)
        .strip_prefix("Look-")?;
    const ALL: [Direction; 8] = [
        Direction::North,
        Direction::South,
        Direction::East,
        Direction::West,
        Direction::NorthEast,
        Direction::NorthWest,
        Direction::SouthEast,
        Direction::SouthWest,
    ];
    ALL.into_iter().find(|d| d.name() == word.trim())
}

/// The direction of the one completed top-down Look in `entries`, when its
/// preamble row followed it. Two Looks, a Look with no direction word
/// (cancelled) or no preamble (the preflight failed) give `None`.
fn completed_look(entries: &[MessageEntry]) -> Option<Direction> {
    let mut looks = entries
        .iter()
        .enumerate()
        .filter_map(|(i, entry)| look_echo_direction(entry).map(|d| (i, d)));
    let (Some((at, direction)), None) = (looks.next(), looks.next()) else {
        return None;
    };
    let preamble = LOOK_RESULT_PREFIX.trim();
    entries[at + 1..]
        .iter()
        .any(|entry| entry.text.trim() == preamble)
        .then_some(direction)
}

/// Sign read and entrance named (10.3.3, D10.7).
pub(super) fn look_events(before: &KnowProbe, after: &PlayState, events: &mut KnowEvents) {
    if !matches!(before.area, Area::Town { .. } | Area::World { .. }) || after.area != before.area {
        return;
    }
    let (bx, by) = (before.at.0 as usize, before.at.1 as usize);
    if (after.player.x, after.player.y) != (bx, by) {
        return;
    }
    let Some(entries) = new_entries(before, after) else {
        return;
    };
    let Some(direction) = completed_look(entries) else {
        return;
    };
    let (dx, dy) = direction.delta();
    // The probe's neighbourhood index of the target.
    let near = ((dy + 1) * 3 + (dx + 1)) as u32;
    let (tx, ty) = (bx as isize + dx, by as isize + dy);
    if before.objects_near & (1 << near) != 0 || object_at(after, tx, ty) {
        // `view.md §3` row 4: the object case wins over the sign case.
        return;
    }
    match after.area {
        Area::Town { scene, floor } => {
            let side = TOWN_GRID_SIDE as isize;
            if !(0..side).contains(&tx) || !(0..side).contains(&ty) {
                return;
            }
            let (x, y) = (tx as usize, ty as usize);
            if sign_or_wanted_poster_object_class(after.grid[y * TOWN_GRID_SIDE + x]) {
                events.push(KnowEvent::SignRead(SignCell {
                    scene: scene.byte,
                    floor,
                    x: x as u8,
                    y: y as u8,
                }));
            }
        }
        Area::World { plane } => {
            let (x, y) = (wrap_world(tx), wrap_world(ty));
            if sign_or_wanted_poster_object_class(after.grid[world_cell_index(x, y)]) {
                events.push(KnowEvent::SignRead(SignCell {
                    scene: SCENE_OVERWORLD,
                    floor: plane.save_floor(),
                    x: x as u8,
                    y: y as u8,
                }));
                return;
            }
            let (x, y) = resolve_world_redirects(after, x, y);
            if after.grid[world_cell_index(x, y)] == WORD_OF_POWER_SEALED_TILE
                && let Some(record) = entrance_record_for_column(x as u8)
            {
                events.push(KnowEvent::EntranceNamed { record });
            }
        }
        Area::Dungeon { .. } => {}
    }
}

/// `view.md §3` terrain row 1 on the world: `0xE0`, `0xE1` and `0xE2`
/// move the Look target one cell north, east and west, and chain. A cycle
/// stops where it is (no shipped map holds one).
fn resolve_world_redirects(state: &PlayState, x: usize, y: usize) -> (usize, usize) {
    let (mut x, mut y) = (x, y);
    for _ in 0..WORLD_SIDE {
        let (dx, dy): (isize, isize) = match state.grid[world_cell_index(x, y)] {
            LOOK_REDIRECT_NORTH_TILE => (0, -1),
            LOOK_REDIRECT_EAST_TILE => (1, 0),
            LOOK_REDIRECT_WEST_TILE => (-1, 0),
            _ => break,
        };
        x = wrap_world(x as isize + dx);
        y = wrap_world(y as isize + dy);
    }
    (x, y)
}

/// A dungeon cell `(dx, dy)` from `(x, y)`, wrapped onto the 8 x 8 level.
fn level_offset(x: u8, y: u8, dx: isize, dy: isize) -> (u8, u8) {
    let wrap = |v: u8, d: isize| (v as i16 + d as i16).rem_euclid(LEVEL_SIDE) as u8;
    (wrap(x, dx), wrap(y, dy))
}

/// The cell a dungeon Look or Search focus names, from the party's cell
/// and facing (the mirror of `dungeon_look_focus_coord`).
fn focus_cell(x: u8, y: u8, facing: Direction, focus: DungeonLookFocus) -> (u8, u8) {
    let direction = match focus {
        DungeonLookFocus::Ahead => Some(facing),
        DungeonLookFocus::Right => facing.turn_right_cardinal(),
        DungeonLookFocus::Left => facing.turn_left_cardinal(),
        DungeonLookFocus::Here => None,
    };
    let (dx, dy) = direction.map(Direction::delta).unwrap_or((0, 0));
    level_offset(x, y, dx, dy)
}

fn mark(record: u8, level: u8, (x, y): (u8, u8), kind: MarkKind) -> KnowEvent {
    KnowEvent::DungeonMarked(DungeonMark {
        record,
        level,
        x,
        y,
        kind,
    })
}

fn remembered(record: u8, level: u8, (x, y): (u8, u8)) -> KnowEvent {
    let mut rows = [0u8; 8];
    rows[y as usize & 7] = 1 << (x & 7);
    KnowEvent::DungeonCellsSeen {
        record,
        level,
        rows,
    }
}

/// Fountain drunk: the dungeon fountain's prompt closed with Yes.
pub(super) fn fountain_events(before: &KnowProbe, after: &PlayState, events: &mut KnowEvents) {
    let Some(YesNoPromptKind::DungeonFountainDrink { focus, .. }) = before.yes_no else {
        return;
    };
    let (Area::Dungeon { scene, level }, Some(image)) = (before.area, before.level_image) else {
        return;
    };
    if after.area != before.area || after.active_yes_no_prompt.is_some() {
        return;
    }
    let Some(entries) = new_entries(before, after) else {
        return;
    };
    let accepted = DUNGEON_FOUNTAIN_ACCEPTED.trim_end();
    if !entries.iter().any(|entry| entry.text.contains(accepted)) {
        return;
    }
    let (x, y, facing) = before.at;
    let cell = focus_cell(x, y, facing, focus);
    if image[cell.1 as usize * 8 + cell.0 as usize] >> 4 == 0x5 {
        events.push(mark(scene.record as u8, level, cell, MarkKind::Fountain));
    }
}

/// What a before/after byte pair at one cell discovered (10.3.3 table).
fn rewrite_kind(before: u8, after: u8) -> Option<MarkKind> {
    if before == after {
        return None;
    }
    if before == 0x61 && after == 0x60 {
        return Some(MarkKind::Trap);
    }
    if is_dungeon_bomb_trap(before) && after & !0x08 == 0 {
        return Some(MarkKind::Trap);
    }
    match (before >> 4, after >> 4) {
        (0xD, 0xE) => Some(MarkKind::SecretDoor),
        (0x4, 0x7) => Some(MarkKind::Chest),
        _ => None,
    }
}

/// The dungeon events: rewrites, ladders, pits and falls.
pub(super) fn dungeon_events(
    before: &KnowProbe,
    after: &PlayState,
    single: bool,
    events: &mut KnowEvents,
) {
    let (Area::Dungeon { scene, level }, Some(image)) = (before.area, before.level_image) else {
        return;
    };
    let record = scene.record as u8;
    let (bx, by, facing) = before.at;
    let party = level_offset(bx, by, 0, 0);
    let departure = image[party.1 as usize * 8 + party.0 as usize];
    match after.area {
        Area::Dungeon {
            scene: now,
            level: now_level,
        } if now == scene => {
            let landed = level_offset(after.player.x as u8, after.player.y as u8, 0, 0);
            // Cell rewrites on the probe's level.
            let mut cells = vec![party];
            if now_level == level {
                cells.push(landed);
            }
            if single {
                for focus in [
                    DungeonLookFocus::Ahead,
                    DungeonLookFocus::Left,
                    DungeonLookFocus::Right,
                ] {
                    cells.push(focus_cell(party.0, party.1, facing, focus));
                }
            }
            let mut done: Vec<(u8, u8)> = Vec::new();
            for cell in cells {
                if done.contains(&cell) {
                    continue;
                }
                done.push(cell);
                let was = image[cell.1 as usize * 8 + cell.0 as usize];
                let is = after.dungeon_cell(level, cell.0 as usize, cell.1 as usize);
                if let Some(kind) = rewrite_kind(was, is) {
                    events.push(mark(record, level, cell, kind));
                }
            }
            if single && now_level != level {
                level_change_events(
                    record, level, now_level, party, landed, departure, image, after, events,
                );
            }
        }
        Area::World { .. } if single => {
            // A Klimb off the top (or bottom) edge: the departure ladder.
            let class = departure >> 4;
            let up = matches!(class, 0x1 | 0x3);
            let down = matches!(class, 0x2 | 0x3);
            if (level == 0 && up) || (level == 7 && down) {
                events.push(mark(record, level, party, MarkKind::Ladder));
            } else if level == 7 && class == 0x6 {
                events.push(mark(record, level, party, MarkKind::Trap));
            }
        }
        _ => {}
    }
}

#[allow(clippy::too_many_arguments)]
fn level_change_events(
    record: u8,
    level: u8,
    now_level: u8,
    party: (u8, u8),
    landed: (u8, u8),
    departure: u8,
    image: [u8; 64],
    after: &PlayState,
    events: &mut KnowEvents,
) {
    let class = departure >> 4;
    let delta = now_level as i16 - level as i16;
    if landed == party && delta.abs() == 1 {
        let ladder_way = match class {
            0x1 => delta == -1,
            0x2 => delta == 1,
            0x3 => true,
            _ => false,
        };
        if ladder_way {
            events.push(mark(record, level, party, MarkKind::Ladder));
            let other = after.dungeon_cell(now_level, landed.0 as usize, landed.1 as usize);
            if matches!(other >> 4, 0x1..=0x3) {
                events.push(mark(record, now_level, landed, MarkKind::Ladder));
            }
            events.push(remembered(record, now_level, landed));
            return;
        }
        if class == 0x6 && delta == 1 {
            // A pit climbed down (or a fall trap underfoot): found.
            events.push(mark(record, level, party, MarkKind::Trap));
            events.push(remembered(record, now_level, landed));
            return;
        }
    }
    // A fall: one step (or none) onto a fall trap, then down the chain.
    let dx = (landed.0 as i16 - party.0 as i16).rem_euclid(LEVEL_SIDE);
    let dy = (landed.1 as i16 - party.1 as i16).rem_euclid(LEVEL_SIDE);
    let near = |d: i16| d <= 1 || d == LEVEL_SIDE - 1;
    if delta <= 0 || !near(dx) || !near(dy) {
        return;
    }
    if !is_dungeon_fall_trap(image[landed.1 as usize * 8 + landed.0 as usize]) {
        return;
    }
    events.push(mark(record, level, landed, MarkKind::Trap));
    events.push(remembered(record, level, landed));
    for between in level + 1..now_level {
        let byte = after.dungeon_cell(between, landed.0 as usize, landed.1 as usize);
        if is_dungeon_fall_trap(byte) {
            events.push(mark(record, between, landed, MarkKind::Trap));
        }
        events.push(remembered(record, between, landed));
    }
    events.push(remembered(record, now_level, landed));
}

#[cfg(test)]
#[path = "probe_more_tests.rs"]
mod tests;
