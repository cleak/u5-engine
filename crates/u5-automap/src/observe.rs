//! What the faithful views show, read without changing the game (design
//! 4.3.3, 10.3.3 "dungeon cells seen").
//!
//! [`dungeon_corridor_cells`] names the dungeon cells the first-person
//! corridor frame paints. It is a mirror of the band loop of the faithful
//! renderer (`draw_dungeon_corridor`), built only from `&PlayState`
//! readers: [`PlayState::dungeon_renderer_offset_cell`],
//! [`dungeon_forward_outcome`], [`DUNGEON_BANDS`], the facing deltas and
//! [`PlayState::has_personal_light`]. The renderer itself is never called:
//! it takes `&mut self` and spends a gameplay PRNG draw for the flavour-3
//! wall one band ahead (systems/dungeon-mode.md §6.4), so calling it from
//! an observer would change the game. A black-box cross-check (design 4.6)
//! pins the mirror to the renderer instead.

use u5_runtime::{
    Area, DUNGEON_BANDS, PlayState, dungeon_floor_wrap_coord, dungeon_forward_outcome,
};

/// Where a corridor cell sits relative to the party's line of sight.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CorridorLane {
    /// On the forward line (band 0 is the party's own cell).
    Forward,
    /// The cell to the left of the forward cell of its band.
    Left,
    /// The cell to the right of the forward cell of its band.
    Right,
}

/// One cell the corridor frame paints.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CorridorCell {
    /// Column on the 8x8 level, wrapped (systems/dungeon-mode.md §6.1).
    pub x: u8,
    /// Row on the 8x8 level, wrapped.
    pub y: u8,
    /// Depth band, 0 (the party's cell) to `DUNGEON_BANDS - 1`.
    pub band: u8,
    pub lane: CorridorLane,
}

/// The dungeon cells one corridor frame shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CorridorCells {
    /// Dungeon record, 0..8 (the scene's record).
    pub record: u8,
    /// Level within the record, 0..8.
    pub level: u8,
    /// The party has a personal light (torch or Light spell), so the frame
    /// paints the corridor at all.
    pub lit: bool,
    /// The cells, nearest band first and forward before left before right
    /// within a band. The party's own cell is always first: unlit, it is
    /// the only cell (the party stands on it, though the frame is black).
    pub cells: Vec<CorridorCell>,
}

impl CorridorCells {
    /// The `(x, y)` of every cell, in order.
    pub fn positions(&self) -> impl Iterator<Item = (u8, u8)> + '_ {
        self.cells.iter().map(|cell| (cell.x, cell.y))
    }

    /// Whether the frame shows `(x, y)`.
    pub fn contains(&self, x: u8, y: u8) -> bool {
        self.positions().any(|at| at == (x, y))
    }
}

/// The cells the faithful dungeon corridor frame paints for `state`, or
/// `None` outside a dungeon.
///
/// The mirror of the band loop (systems/dungeon-mode.md §6.5):
/// - unlit (no torch and no Light spell), the frame is black and only the
///   party's own cell is reported;
/// - band 0 through `DUNGEON_BANDS - 1`, the forward cell of each band is
///   shown, then, if the band's forward test is see-through, its left and
///   right cells;
/// - the sweep stops after the first band whose forward test is blocked,
///   so that band's side cells are not shown;
/// - the point-blank door rule: a heavy door (`0xE?`) in the party's own
///   cell is see-through but hides the band-0 side cells.
///
/// Gating on combat, overlays and the other screens that are not the
/// corridor belongs to the caller (design 4.3.3).
pub fn dungeon_corridor_cells(state: &PlayState) -> Option<CorridorCells> {
    let Area::Dungeon { scene, level } = state.area else {
        return None;
    };
    let wrap = |coord: usize, delta: isize| dungeon_floor_wrap_coord(coord as i16 + delta as i16);
    let party = CorridorCell {
        x: wrap(state.player.x, 0),
        y: wrap(state.player.y, 0),
        band: 0,
        lane: CorridorLane::Forward,
    };
    let lit = state.has_personal_light();
    let mut out = CorridorCells {
        record: scene.record as u8,
        level,
        lit,
        cells: vec![party],
    };
    let facing = state.player.facing;
    // The renderer draws nothing for a facing without a cardinal turn.
    let (Some(left), Some(right)) = (facing.turn_left_cardinal(), facing.turn_right_cardinal())
    else {
        return Some(out);
    };
    if !lit {
        return Some(out);
    }
    let (fdx, fdy) = facing.delta();
    let (ldx, ldy) = left.delta();
    let (rdx, rdy) = right.delta();
    let mut push = |dx: isize, dy: isize, band: usize, lane: CorridorLane| {
        let cell = CorridorCell {
            x: wrap(state.player.x, dx),
            y: wrap(state.player.y, dy),
            band: band as u8,
            lane,
        };
        if !out
            .cells
            .iter()
            .any(|seen| (seen.x, seen.y) == (cell.x, cell.y))
        {
            out.cells.push(cell);
        }
    };

    let mut point_blank = false;
    for band in 0..DUNGEON_BANDS {
        let step = band as isize;
        let (ahead_dx, ahead_dy) = (fdx * step, fdy * step);
        let ahead = state.dungeon_renderer_offset_cell(level, ahead_dx, ahead_dy);
        let outcome = dungeon_forward_outcome(ahead, band);
        point_blank |= outcome.point_blank;
        // The forward cell is always painted: a blocker image, or the
        // floor and its contents seen through it.
        push(ahead_dx, ahead_dy, band, CorridorLane::Forward);
        if outcome.see_through && !(band == 0 && point_blank) {
            push(ahead_dx + ldx, ahead_dy + ldy, band, CorridorLane::Left);
            push(ahead_dx + rdx, ahead_dy + rdy, band, CorridorLane::Right);
        }
        if !outcome.see_through {
            break;
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use u5_runtime::enhancements::digest_groups;
    use u5_runtime::test_fixtures::{
        dungeon_state, open_dungeon_record, synthetic_dungeon_billboard_banks,
    };
    use u5_runtime::{
        DUNGEON_RECORD_LEN, Direction, DungeonBillboardBanks, DungeonScene, TileGraphicsDepth,
        VIEWPORT_PLAYER_ROW, dungeon_cell_index,
    };

    const PASSAGE: u8 = 0x00;
    const WALL: u8 = 0xB0;
    const FACINGS: [Direction; 4] = [
        Direction::North,
        Direction::East,
        Direction::South,
        Direction::West,
    ];

    /// A small deterministic generator for the synthetic levels (not the
    /// game's PRNG).
    struct Lcg(u32);

    impl Lcg {
        fn next(&mut self) -> u32 {
            self.0 = self.0.wrapping_mul(1_103_515_245).wrapping_add(12_345);
            self.0 >> 16
        }
    }

    /// Every renderer class the forward and side tests tell apart, with
    /// passage the most common so corridors run deep.
    const CLASSES: [u8; 16] = [
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x20, 0x40, 0x50, 0x60, 0xA0, 0xB0, 0xC0, 0xE0,
        0xF0,
    ];

    fn lit(mut state: PlayState, on: bool) -> PlayState {
        state.torch_counter = u8::from(on) * 50;
        state.light_spell_counter = 0;
        state
    }

    fn state_on(record: u8, grid: Vec<u8>, level: u8, x: usize, y: usize) -> PlayState {
        let mut state = dungeon_state(grid, level, x, y);
        state.area = Area::Dungeon {
            scene: DungeonScene::from_record(record).unwrap(),
            level,
        };
        state
    }

    fn render(state: &PlayState, banks: &DungeonBillboardBanks) -> Vec<u8> {
        state
            .clone()
            .render_dungeon_viewport(
                VIEWPORT_PLAYER_ROW,
                TileGraphicsDepth::Ega16,
                Some(banks),
                None,
            )
            .unwrap()
            .pixels
    }

    /// The black-box visible set: a cell is visible exactly when the frame
    /// with it set to passage differs from the frame with it set to wall,
    /// every other cell held (design 4.6).
    fn frame_visible_cells(state: &PlayState, banks: &DungeonBillboardBanks) -> Vec<(u8, u8)> {
        let Area::Dungeon { level, .. } = state.area else {
            panic!("not a dungeon");
        };
        let mut visible = Vec::new();
        for y in 0..8u8 {
            for x in 0..8u8 {
                let at = dungeon_cell_index(level, usize::from(x), usize::from(y));
                let mut open = state.clone();
                open.grid[at] = PASSAGE;
                let mut shut = state.clone();
                shut.grid[at] = WALL;
                if render(&open, banks) != render(&shut, banks) {
                    visible.push((x, y));
                }
            }
        }
        visible
    }

    fn sorted(mut cells: Vec<(u8, u8)>) -> Vec<(u8, u8)> {
        cells.sort_unstable();
        cells
    }

    /// Design 4.6 "Corridor iterator, black-box": over random levels, a
    /// set of positions (each also with a heavy door underfoot), the four
    /// facings, both light states and the three presentation flavours, the iterator names exactly the cells
    /// the faithful frame paints (the party's cell alone when unlit).
    #[test]
    fn corridor_cells_match_the_faithful_frame_on_a_corpus() {
        let banks = synthetic_dungeon_billboard_banks(TileGraphicsDepth::Ega16);
        let mut rng = Lcg(0x5eed_0304);
        let mut checked = 0usize;
        let mut lengths = [0usize; 13];
        let mut point_blank = 0usize;
        // Records 1 (Normal), 5 (Mine) and 0 (flavour byte 3).
        for record in [1u8, 5, 0] {
            for round in 0..2 {
                let level = (round * 3 + usize::from(record)) as u8 % 8;
                let mut grid = open_dungeon_record();
                for cell in grid.iter_mut() {
                    *cell = CLASSES[rng.next() as usize % CLASSES.len()];
                }
                // Each position as the level has it, then with a heavy
                // door in the party's own cell (the point-blank rule).
                let positions = [(0, 0), (3, 4), (7, 7), (5, 1)];
                for (x, y, threshold) in positions
                    .iter()
                    .flat_map(|&(x, y)| [(x, y, None), (x, y, Some(0xE0u8))])
                {
                    for facing in FACINGS {
                        for on in [true, false] {
                            let mut state = lit(state_on(record, grid.clone(), level, x, y), on);
                            if let Some(byte) = threshold {
                                state.grid[dungeon_cell_index(level, x, y)] = byte;
                            }
                            state.player.facing = facing;
                            let mine = dungeon_corridor_cells(&state).unwrap();
                            assert_eq!((mine.record, mine.level, mine.lit), (record, level, on));
                            let frame = sorted(frame_visible_cells(&state, &banks));
                            let party = (x as u8, y as u8);
                            if on {
                                assert_eq!(
                                    sorted(mine.positions().collect()),
                                    frame,
                                    "record {record} level {level} at {party:?} facing {facing:?}: {mine:?}"
                                );
                            } else {
                                assert!(frame.is_empty(), "an unlit frame paints nothing");
                                assert_eq!(mine.positions().collect::<Vec<_>>(), vec![party]);
                            }
                            assert_eq!(mine.cells[0].band, 0);
                            assert_eq!(mine.positions().next(), Some(party));
                            lengths[mine.cells.len()] += 1;
                            if on && threshold.is_some() {
                                point_blank += usize::from(
                                    !mine
                                        .cells
                                        .iter()
                                        .any(|c| c.band == 0 && c.lane != CorridorLane::Forward),
                                );
                            }
                            checked += 1;
                        }
                    }
                }
            }
        }
        assert_eq!(checked, 3 * 2 * 8 * 4 * 2);
        assert_eq!(point_blank, 3 * 2 * 4 * 4);
        // The corpus reaches both the deepest corridor and a wall at the
        // party's own threshold.
        assert!(lengths[12] > 0, "{lengths:?}");
        assert!(lengths[1] > 48, "{lengths:?}");
    }

    /// The hand-built rules, each on an open level: a straight open
    /// corridor shows all four bands, a wall stops the sweep and hides its
    /// band's sides, and a heavy door at point-blank hides the band-0 sides
    /// but lets the sweep run on. Each case also passes the frame check.
    #[test]
    fn corridor_rules_on_hand_built_levels() {
        let banks = synthetic_dungeon_billboard_banks(TileGraphicsDepth::Ega16);
        let at = |x: usize, y: usize| dungeon_cell_index(0, x, y);
        let check = |state: &PlayState| {
            let mine = dungeon_corridor_cells(state).unwrap();
            assert_eq!(
                sorted(mine.positions().collect()),
                sorted(frame_visible_cells(state, &banks)),
                "{mine:?}"
            );
            mine
        };

        // Open floor, facing east from (2, 3): bands 0..=3 with both sides.
        let open = lit(state_on(1, open_dungeon_record(), 0, 2, 3), true);
        let cells = check(&open);
        assert_eq!(cells.cells.len(), 12);
        assert_eq!(
            cells
                .cells
                .iter()
                .filter(|c| c.lane == CorridorLane::Forward)
                .map(|c| (c.x, c.y, c.band))
                .collect::<Vec<_>>(),
            vec![(2, 3, 0), (3, 3, 1), (4, 3, 2), (5, 3, 3)]
        );
        // East-facing: left is north (y - 1), right is south (y + 1).
        assert!(
            cells
                .cells
                .iter()
                .any(|c| c.lane == CorridorLane::Left && (c.x, c.y, c.band) == (2, 2, 0))
        );
        assert!(
            cells
                .cells
                .iter()
                .any(|c| c.lane == CorridorLane::Right && (c.x, c.y, c.band) == (2, 4, 0))
        );

        // A wall two ahead: bands 0 and 1 with sides, then band 2's wall
        // alone.
        let mut walled = open.clone();
        walled.grid[at(4, 3)] = WALL;
        let cells = check(&walled);
        assert_eq!(cells.cells.len(), 7);
        assert_eq!(cells.cells.last().unwrap().band, 2);
        assert!(!cells.contains(4, 2) && !cells.contains(4, 4) && !cells.contains(5, 3));

        // A wall in the party's own cell: that cell alone.
        let mut boxed = open.clone();
        boxed.grid[at(2, 3)] = WALL;
        assert_eq!(check(&boxed).positions().collect::<Vec<_>>(), vec![(2, 3)]);

        // A plain door (0xA?) at point-blank blocks like a wall.
        let mut door = open.clone();
        door.grid[at(2, 3)] = 0xA0;
        assert_eq!(check(&door).cells.len(), 1);

        // A heavy door (0xE?) at point-blank: no band-0 sides, the sweep
        // runs on to band 3.
        let mut heavy = open.clone();
        heavy.grid[at(2, 3)] = 0xE0;
        let cells = check(&heavy);
        assert_eq!(cells.cells.len(), 10);
        assert!(!cells.contains(2, 2) && !cells.contains(2, 4));
        assert!(cells.contains(5, 3) && cells.contains(5, 2) && cells.contains(5, 4));

        // A heavy door one ahead blocks: band 1 is its image only.
        let mut ahead = open.clone();
        ahead.grid[at(3, 3)] = 0xE0;
        let cells = check(&ahead);
        assert_eq!(cells.cells.len(), 4);
        assert!(!cells.contains(3, 2) && !cells.contains(4, 3));

        // The floor wraps: facing west from x = 1 sees x = 0, 7, 6.
        let mut wrapped = lit(state_on(1, open_dungeon_record(), 0, 1, 0), true);
        wrapped.player.facing = Direction::West;
        let cells = check(&wrapped);
        assert!(cells.contains(6, 0) && cells.contains(7, 7) && cells.contains(6, 1));
    }

    /// Unlit, the frame is black and the iterator names the party's cell
    /// only, whatever the corridor; a Light spell lights it like a torch.
    #[test]
    fn unlit_corridor_is_the_party_cell_only() {
        let banks = synthetic_dungeon_billboard_banks(TileGraphicsDepth::Ega16);
        let dark = lit(state_on(1, open_dungeon_record(), 2, 4, 4), false);
        assert!(!dark.has_personal_light());
        let cells = dungeon_corridor_cells(&dark).unwrap();
        assert!(!cells.lit);
        assert_eq!(cells.positions().collect::<Vec<_>>(), vec![(4, 4)]);
        assert!(frame_visible_cells(&dark, &banks).is_empty());

        let mut spell = dark.clone();
        spell.light_spell_counter = 9;
        let cells = dungeon_corridor_cells(&spell).unwrap();
        assert!(cells.lit);
        assert_eq!(cells.cells.len(), 12);
    }

    /// Outside a dungeon there is no corridor.
    #[test]
    fn no_corridor_outside_a_dungeon() {
        let town =
            u5_runtime::test_fixtures::test_state(u5_runtime::test_fixtures::open_grid(), 5, 5);
        assert_eq!(dungeon_corridor_cells(&town), None);
    }

    /// Purity: the iterator reads only. On a flavour-3 dungeon with a
    /// `0xC?` wall one band ahead, the faithful render spends a gameplay
    /// PRNG draw; the iterator leaves `prng_state`, the flourish cache and
    /// every digest group as they were, and answers the same twice.
    #[test]
    fn corridor_cells_are_pure() {
        let banks = synthetic_dungeon_billboard_banks(TileGraphicsDepth::Ega16);
        let mut state = state_on(0, vec![0u8; DUNGEON_RECORD_LEN], 0, 2, 3);
        state.grid[dungeon_cell_index(0, 3, 3)] = 0xC0;
        let state = lit(state, true);
        let prng = state.prng_state;
        let flourish = state.dungeon_flourish_draw;
        let digest = digest_groups(&state);

        let first = dungeon_corridor_cells(&state).unwrap();
        let second = dungeon_corridor_cells(&state).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.cells.len(), 4);
        assert_eq!(state.prng_state, prng);
        assert_eq!(state.dungeon_flourish_draw, flourish);
        assert_eq!(digest_groups(&state), digest);

        // The reason the renderer is not called: it does spend the draw.
        let mut rendered = state.clone();
        rendered
            .render_dungeon_viewport(
                VIEWPORT_PLAYER_ROW,
                TileGraphicsDepth::Ega16,
                Some(&banks),
                None,
            )
            .unwrap();
        assert_ne!(rendered.prng_state, prng);
    }
}
