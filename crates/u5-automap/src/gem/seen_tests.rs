//! M3-06 seen-cell tests: the corridor's dungeon cells each frame, a gem
//! view's flood and window when it opens, the side windows of tag 11, the
//! 4.3.3 gates, and the rule that the grow-left ring never marks a side
//! cell.

use std::collections::BTreeSet;

use u5_runtime::enhancements::trajectory_digest;
use u5_runtime::test_fixtures::{
    dungeon_state, open_dungeon_record, open_grid, open_world_grid, test_state, world_state,
};
use u5_runtime::{
    Area, Direction, PlayState, PlayTarget, VISIBILITY_CLEAR, VISIBILITY_GRID_LEN,
    VISIBILITY_HIDDEN, ViewOverlay, ViewOverlayKind, ViewOverlayMode, WorldPlane,
    published_world_location_entries, visibility_grid_active_index,
};

use super::*;
use crate::gem::knowledge::{GemKnowledge, SIDE_WINDOW_BYTES};
use crate::observe::dungeon_corridor_cells;

fn britannia(x: usize, y: usize) -> PlayState {
    let mut state = world_state(open_world_grid(), x, y);
    state.area = Area::World {
        plane: WorldPlane::Britannia,
    };
    ready(&mut state);
    state
}

/// A finished visibility grid: every cell of the 11 x 11 window shown.
fn ready(state: &mut PlayState) {
    state.visibility_dirty = false;
    state.visibility_buffers_ready = true;
    state.visibility_sweep = None;
    state.visibility_grid = [VISIBILITY_CLEAR; VISIBILITY_GRID_LEN];
}

fn first_window() -> SideWindow {
    *side_windows()
        .iter()
        .find(|w| w.plane == WorldPlane::Britannia)
        .expect("a Britannia town")
}

fn side_cells(events: &KnowEvents, scene: u8) -> BTreeSet<u16> {
    events
        .iter()
        .filter_map(|event| match *event {
            KnowEvent::SideCellSeen { scene: s, index } if s == scene => Some(index),
            _ => None,
        })
        .collect()
}

fn gem_overlay(kind: ViewOverlayKind, mode: ViewOverlayMode) -> Option<ViewOverlay> {
    Some(ViewOverlay {
        title: String::new(),
        text_map: String::new(),
        kind,
        mode,
    })
}

// ---- the side window -----------------------------------------------------

#[test]
fn every_town_family_entrance_gets_one_window() {
    let windows = side_windows();
    let entries = published_world_location_entries();
    let towns: BTreeSet<u8> = entries
        .iter()
        .filter_map(|e| match e.target {
            PlayTarget::Town(scene) => Some(scene.byte),
            _ => None,
        })
        .collect();
    assert!(!windows.is_empty());
    let scenes: BTreeSet<u8> = windows.iter().map(|w| w.scene).collect();
    assert_eq!(scenes.len(), windows.len(), "one window per scene");
    assert_eq!(scenes, towns, "every town-family scene has its window");
    for window in windows {
        assert_eq!(side_window(window.scene), Some(window));
    }
}

#[test]
fn the_window_index_round_trips_and_skips_the_entrance_column() {
    for window in [
        first_window(),
        SideWindow {
            scene: 1,
            plane: WorldPlane::Britannia,
            entrance: (3, 250),
        },
    ] {
        let mut seen = BTreeSet::new();
        for index in 0..(SIDE_WINDOW_BYTES * 8) as u16 {
            let (x, y) = window.cell_of(index).unwrap();
            assert_eq!(window.index_of(x, y), Some(index), "{index}");
            assert!(seen.insert((x, y)));
            assert_ne!(x, window.entrance.0, "the entrance column is cut out");
        }
        assert_eq!(window.cell_of((SIDE_WINDOW_BYTES * 8) as u16), None);
        let (ex, ey) = window.entrance;
        // Column 23 touches the floor's west edge, column 24 its east edge.
        assert_eq!(window.index_of(ex.wrapping_sub(1), ey), Some(18 * 48 + 23));
        assert_eq!(window.index_of(ex.wrapping_add(1), ey), Some(18 * 48 + 24));
        assert_eq!(window.index_of(ex, ey), None);
        assert_eq!(window.index_of(ex.wrapping_add(25), ey), None);
        assert_eq!(window.index_of(ex.wrapping_sub(25), ey), None);
        assert_eq!(
            window.index_of(ex.wrapping_add(1), ey.wrapping_sub(18)),
            Some(24)
        );
        assert_eq!(
            window.index_of(ex.wrapping_add(1), ey.wrapping_sub(19)),
            None
        );
        assert_eq!(
            window.index_of(ex.wrapping_add(1), ey.wrapping_add(18)),
            None
        );
    }
}

// ---- the faithful window, per frame ---------------------------------------

#[test]
fn the_faithful_window_marks_side_cells_it_shows() {
    let window = first_window();
    let (ex, ey) = window.entrance;
    // Ten cells west of the entrance: the 11 x 11 window lies in the side.
    let (px, py) = (ex.wrapping_sub(10), ey);
    let mut state = britannia(px as usize, py as usize);
    // Hide every cell but the party's row.
    for row in 0..11 {
        for col in 0..11 {
            let at = visibility_grid_active_index(row, col).unwrap();
            if row != 5 {
                state.visibility_grid[at] = VISIBILITY_HIDDEN;
            }
        }
    }
    let events = frame_seen_events(&state);
    let expected: BTreeSet<u16> = (-5..=5i16)
        .filter_map(|dx| window.index_of((px as i16 + dx).rem_euclid(256) as u8, py))
        .collect();
    assert_eq!(expected.len(), 11);
    assert_eq!(side_cells(&events, window.scene), expected);
    let mut know = GemKnowledge::new();
    for event in &events {
        know.apply(event);
    }
    for index in &expected {
        assert!(know.side_seen(window.scene, *index));
    }
}

#[test]
fn a_side_cell_is_never_set_from_the_grow_left_ring() {
    // Every cell the record gets lies in the faithful 11 x 11 window: the
    // widened view's ring (6 or more cells out) never counts, even with
    // the whole window shown and a side window all round.
    let window = first_window();
    let (ex, ey) = window.entrance;
    for (dx, dy) in [(-10i16, 0i16), (12, -9), (-20, 14), (3, 3)] {
        let px = (ex as i16 + dx).rem_euclid(256) as u8;
        let py = (ey as i16 + dy).rem_euclid(256) as u8;
        let state = britannia(px as usize, py as usize);
        let events = frame_seen_events(&state);
        for index in side_cells(&events, window.scene) {
            let (x, y) = window.cell_of(index).unwrap();
            let ddx = (x as i16 - px as i16 + 128).rem_euclid(256) - 128;
            let ddy = (y as i16 - py as i16 + 128).rem_euclid(256) - 128;
            assert!(ddx.abs() <= 5 && ddy.abs() <= 5, "({x}, {y})");
        }
    }
    // Six cells past the window edge in a side window: unseen.
    let px = ex.wrapping_sub(12);
    let state = britannia(px as usize, ey as usize);
    let events = frame_seen_events(&state);
    let far = window.index_of(px.wrapping_sub(6), ey).unwrap();
    assert!(!side_cells(&events, window.scene).contains(&far));
    assert!(
        side_cells(&events, window.scene)
            .contains(&window.index_of(px.wrapping_sub(5), ey).unwrap())
    );
}

#[test]
fn nothing_is_seen_while_the_faithful_view_is_not_the_map() {
    let window = first_window();
    let (ex, ey) = window.entrance;
    let base = britannia(ex.wrapping_sub(8) as usize, ey as usize);
    assert!(!frame_seen_events(&base).is_empty());
    let cases: [fn(&mut PlayState); 7] = [
        |s| s.combat_active = true,
        |s| s.visibility_dirty = true,
        |s| s.visibility_buffers_ready = false,
        |s| s.codex_presentation_steps = 1,
        |s| s.active_view_overlay = gem_overlay(ViewOverlayKind::Surface, ViewOverlayMode::GemView),
        |s| {
            s.active_shrine = Some(u5_runtime::ShrineSession::entering(
                u5_runtime::ShrineVirtue::Honor,
            ))
        },
        |s| {
            s.visibility_sweep = Some(u5_runtime::VisibilitySweep {
                frames_remaining: 3,
                pause_bios_ticks_per_frame: 1,
                center_x: 0,
                center_y: 0,
                visible_cells: [true; 121],
            })
        },
    ];
    for (i, case) in cases.iter().enumerate() {
        let mut state = base.clone();
        case(&mut state);
        assert!(!observation_open(&state), "case {i}");
        assert!(frame_seen_events(&state).is_empty(), "case {i}");
    }
}

#[test]
fn the_underworld_and_towns_mark_no_side_cell() {
    let window = first_window();
    let (ex, ey) = window.entrance;
    let mut under = world_state(open_world_grid(), ex.wrapping_sub(8) as usize, ey as usize);
    ready(&mut under);
    assert!(matches!(
        under.area,
        Area::World {
            plane: WorldPlane::Underworld
        }
    ));
    // Cells seen in the Underworld only reach windows on that plane.
    let under_scenes: BTreeSet<u8> = side_windows()
        .iter()
        .filter(|w| w.plane == WorldPlane::Underworld)
        .map(|w| w.scene)
        .collect();
    for event in frame_seen_events(&under) {
        let KnowEvent::SideCellSeen { scene, .. } = event else {
            panic!("{event:?}");
        };
        assert!(under_scenes.contains(&scene));
    }
    assert!(side_cells(&frame_seen_events(&under), window.scene).is_empty());
    let mut town = test_state(open_grid(), 5, 5);
    ready(&mut town);
    assert!(frame_seen_events(&town).is_empty());
}

// ---- dungeon cells -------------------------------------------------------

fn lit_dungeon(level: u8, x: usize, y: usize) -> PlayState {
    let mut state = dungeon_state(open_dungeon_record(), level, x, y);
    state.torch_counter = 50;
    state.player.facing = Direction::East;
    state
}

#[test]
fn the_corridor_cells_are_remembered_each_frame() {
    let state = lit_dungeon(2, 1, 4);
    let corridor = dungeon_corridor_cells(&state).unwrap();
    let events = frame_seen_events(&state);
    assert_eq!(events.len(), 1);
    let mut know = GemKnowledge::new();
    know.apply(&events[0]);
    for y in 0..8u8 {
        for x in 0..8u8 {
            assert_eq!(
                know.remembered(corridor.record, 2, x, y),
                corridor.contains(x, y),
                "({x}, {y})"
            );
        }
    }
    assert_eq!(know.cells().total(), corridor.cells.len() as u32);
    // Unlit: only the party's cell.
    let mut dark = state.clone();
    dark.torch_counter = 0;
    dark.light_spell_counter = 0;
    let mut know = GemKnowledge::new();
    for event in frame_seen_events(&dark) {
        know.apply(&event);
    }
    assert_eq!(know.cells().total(), 1);
    assert!(know.remembered(corridor.record, 2, 1, 4));
    // Combat or an open view: nothing.
    let mut fight = state.clone();
    fight.combat_active = true;
    assert!(frame_seen_events(&fight).is_empty());
}

#[test]
fn a_dungeon_gem_remembers_its_flood_cells() {
    let mut state = lit_dungeon(3, 4, 4);
    state.active_view_overlay = gem_overlay(
        ViewOverlayKind::Dungeon { level: 3 },
        ViewOverlayMode::GemView,
    );
    let Some(u5_runtime::ViewOverlayFootprint::Dungeon { cells, .. }) =
        state.view_overlay_footprint()
    else {
        panic!("a dungeon footprint");
    };
    let events = gem_view_events(&state, None);
    let mut know = GemKnowledge::new();
    for event in &events {
        know.apply(event);
    }
    let record = dungeon_corridor_cells(&state).unwrap().record;
    assert_eq!(know.cells().total(), cells.len() as u32);
    for (x, y) in cells {
        assert!(know.remembered(record, 3, x, y));
    }
    // Peer and X-Ray are not the gem.
    for mode in [ViewOverlayMode::PeerSpell, ViewOverlayMode::XRaySpell] {
        let mut other = state.clone();
        other.active_view_overlay = gem_overlay(ViewOverlayKind::Dungeon { level: 3 }, mode);
        assert!(gem_view_events(&other, None).is_empty());
    }
    // No overlay: nothing.
    let mut closed = state.clone();
    closed.active_view_overlay = None;
    assert!(gem_view_events(&closed, None).is_empty());
}

#[test]
fn an_outdoor_gem_marks_its_drawn_window_in_the_side_windows() {
    let window = first_window();
    let (ex, ey) = window.entrance;
    let (px, py) = (ex.wrapping_sub(40), ey);
    let mut state = britannia(px as usize, py as usize);
    state.active_view_overlay = gem_overlay(ViewOverlayKind::Surface, ViewOverlayMode::GemView);
    // The faithful 32 x 32 block alone reaches 16 east of the party: not
    // into this side window (it starts 24 west of the entrance).
    let faithful = side_cells(&gem_view_events(&state, None), window.scene);
    assert!(faithful.is_empty(), "{}", faithful.len());
    // The 16:9 canvas (128 x 60 cells at 4 px) reaches 64 east.
    let wide = side_cells(
        &gem_view_events(
            &state,
            Some(GemWindow {
                half_cols: 64,
                half_rows: 30,
            }),
        ),
        window.scene,
    );
    let mut expected = BTreeSet::new();
    for dy in -30..=30i16 {
        for dx in -64..=64i16 {
            let x = (px as i16 + dx).rem_euclid(256) as u8;
            let y = (py as i16 + dy).rem_euclid(256) as u8;
            if let Some(index) = window.index_of(x, y) {
                expected.insert(index);
            }
        }
    }
    assert!(!expected.is_empty());
    assert_eq!(wide, expected);
    // Peer outdoors is not the gem.
    state.active_view_overlay = gem_overlay(ViewOverlayKind::Surface, ViewOverlayMode::PeerSpell);
    assert!(
        gem_view_events(
            &state,
            Some(GemWindow {
                half_cols: 64,
                half_rows: 30
            })
        )
        .is_empty()
    );
}

#[test]
fn seen_cells_are_pure_reads() {
    let window = first_window();
    let mut world = britannia(
        window.entrance.0.wrapping_sub(8) as usize,
        window.entrance.1 as usize,
    );
    let mut cave = lit_dungeon(3, 4, 4);
    cave.active_view_overlay = gem_overlay(
        ViewOverlayKind::Dungeon { level: 3 },
        ViewOverlayMode::GemView,
    );
    for (i, state) in [&mut world, &mut cave].into_iter().enumerate() {
        state.prng_state = 0x0bad + i as u16;
        let digest = trajectory_digest(state);
        let _ = frame_seen_events(state);
        let _ = gem_view_events(
            state,
            Some(GemWindow {
                half_cols: 64,
                half_rows: 30,
            }),
        );
        assert_eq!(trajectory_digest(state), digest);
        assert_eq!(state.prng_state, 0x0bad + i as u16);
    }
}
