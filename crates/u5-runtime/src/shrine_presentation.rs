//! `karma.md §7` "Entry pacing": the shrine presentation's approach walk.
//!
//! The presentation - not the meditation handler - carries the avatar up to
//! the altar, and it is finished before the handler is entered. The handler's
//! own first act is to replace the walking pose with the kneeling pose and
//! repaint once.
//!
//! This was unimplemented while the two pose tiles were unpublished: the
//! engine drew the shrine backdrop and no avatar at all
//! (`cleak/u5-engine#30`). `cleak/u5-spec#273` published both, and §7's "The
//! two poses, exactly" table now gives their atlas indices and every cell of
//! the path.

/// §7: "The walk is **nine animation frames**".
pub const SHRINE_APPROACH_WALK_FRAMES: u8 = 9;

/// §7: each frame is "one world step, a two-part speaker sting, then four
/// more world steps", so the walk is forty-five world steps and "one
/// animation frame is five viewport repaints".
pub const SHRINE_APPROACH_WALK_STEPS_PER_FRAME: u8 = 5;
pub const SHRINE_APPROACH_WALK_WORLD_STEPS: u8 =
    SHRINE_APPROACH_WALK_FRAMES * SHRINE_APPROACH_WALK_STEPS_PER_FRAME;

/// §7: frames 1-4 show "no avatar and no other actor at all"; the pose
/// appears on frame 5.
pub const SHRINE_APPROACH_WALK_FIRST_POSE_FRAME: u8 = 5;

/// §7 "The two poses, exactly": the walking pose is the party's own on-foot
/// map sprite, "stamped at `(5, 10)` on frame 5, stepping to `(5, 9)`,
/// `(5, 8)`, `(5, 7)` and `(5, 6)`".
pub const SHRINE_WALKING_POSE_TILE: u16 = 0x11C;
pub const SHRINE_WALK_COLUMN: usize = 5;
pub const SHRINE_WALK_FIRST_ROW: usize = 10;
pub const SHRINE_WALK_LAST_ROW: usize = 6;

/// §7: the kneeling pose is "the base member of a four-frame family ... the
/// tile catalogue's beggar run, `0x16C..0x16F`", at `(5, 6)` - the walk's
/// last cell.
pub const SHRINE_KNEELING_POSE_TILE: u16 = 0x16C;
pub const SHRINE_KNEELING_POSE_FAMILY: [u16; 4] = [0x16C, 0x16D, 0x16E, 0x16F];

/// `animation.md §5`: "from the third the slot either returns to the base -
/// on a one-in-four roll - and holds it for six ticks, or rewinds to the
/// first and climbs again".
pub const SHRINE_KNEELING_BASE_HOLD_TICKS: u8 = 6;

/// §5's gate arithmetic, "exact rather than approximate: both rolls reduce a
/// fifteen-bit generator value modulo 256, so the coin is a true one-in-two
/// and the re-base a true one-in-four, with none of the modulo bias a
/// narrower reduction would introduce". `U5Prng::next_range_u16(0, 255)` is
/// that reduction exactly - it masks the state to fifteen bits and takes it
/// modulo the width.
///
/// Which bits carry each decision is not published, only that the
/// probabilities are exact; these take the low ones.
pub const SHRINE_KNEELING_ROLL_HIGH: u16 = 255;

/// §7 "The altar's own cell": tile `0xB2`, "the only cell of the record
/// holding that tile", at `(5, 5)` - one row above the walk's last cell.
pub const SHRINE_ALTAR_CELL: (usize, usize) = (SHRINE_WALK_COLUMN, 5);
pub const SHRINE_ALTAR_TILE: u8 = 0xB2;

/// `karma.md §8`: the Codex walk is "**twelve** animation frames, and
/// **sixty** world steps", stamped at `(5, 10)` on frame 5 and stepped up to
/// `(5, 3)`, "ending one row below the Codex at `(5, 2)`".
pub const CODEX_APPROACH_WALK_FRAMES: u8 = 12;
pub const CODEX_WALK_LAST_ROW: usize = 3;

/// §7 "Exit pacing": after the re-stamp step the presentation "runs four
/// more frames with nothing drawn, waits ten world steps".
pub const SHRINE_EXIT_BLANK_FRAMES: u8 = 4;
pub const SHRINE_EXIT_CLOSING_WAIT_STEPS: u16 = 10;

/// Which presentation arm is running: they share the frame shape and the
/// exit block, and differ in how far the avatar climbs (§7, §8).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PresentationArm {
    /// Nine frames, four rows climbed, then the meditation handler kneels.
    #[default]
    Shrine,
    /// Twelve frames, seven rows climbed; the reader never kneels.
    Codex,
}

impl PresentationArm {
    pub const fn walk_frames(self) -> u8 {
        match self {
            Self::Shrine => SHRINE_APPROACH_WALK_FRAMES,
            Self::Codex => CODEX_APPROACH_WALK_FRAMES,
        }
    }

    /// The approach's world steps: forty-five for a shrine, sixty for the
    /// Codex.
    pub const fn walk_world_steps(self) -> u16 {
        self.walk_frames() as u16 * SHRINE_APPROACH_WALK_STEPS_PER_FRAME as u16
    }

    /// The row the walk ends on, one row below the feature.
    pub const fn rest_row(self) -> usize {
        match self {
            Self::Shrine => SHRINE_WALK_LAST_ROW,
            Self::Codex => CODEX_WALK_LAST_ROW,
        }
    }

    /// §7: "one frame per grid row the approach climbed" - four for a
    /// shrine, seven for the Codex.
    pub const fn rows_climbed(self) -> u8 {
        (SHRINE_WALK_FIRST_ROW - self.rest_row()) as u8
    }

    /// §7 "Exit pacing", after the handler returns: one re-stamp step, the
    /// walk-back frames at five steps each, four blank frames and the
    /// ten-step closing wait - fifty-one steps for a shrine, sixty-six for
    /// the Codex.
    pub const fn exit_world_steps(self) -> u16 {
        1 + (self.rows_climbed() as u16 + SHRINE_EXIT_BLANK_FRAMES as u16)
            * SHRINE_APPROACH_WALK_STEPS_PER_FRAME as u16
            + SHRINE_EXIT_CLOSING_WAIT_STEPS
    }
}

/// What the shrine presentation is drawing over its backdrop right now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShrinePose {
    /// Frames 1-4: "twenty repaints of bare shrine backdrop".
    None,
    /// Frames 5-9, stepping up the centre column.
    Walking { column: usize, row: usize },
    /// The handler's own repaint, and every repaint until the meditation
    /// ends.
    Kneeling { column: usize, row: usize },
}

/// How far into the presentation's counted world steps the walk has got.
///
/// The walk is measured in world steps rather than wall-clock: §7 counts
/// them, and each is "paired with one request for a one-tick hardware-timer
/// delay". The presentation calls [`Self::on_world_step`] once per counted
/// step, and the pose is a pure function of how many have run - so a shell
/// that spreads the steps across a measured hold and one that runs them
/// straight through draw the same sequence.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct ShrineApproachWalk {
    arm: PresentationArm,
    /// Counted world steps since the presentation started, the opening step
    /// included.
    steps: u16,
    /// Set once the meditation handler has replaced the pose.
    kneeling: bool,
    /// §7 "What 'restore the standing Avatar pose' means": the ordaining arm
    /// re-stamps the walking pose in the altar cell.
    standing: bool,
    /// `Some(handler_steps)` once the handler has returned: how many of the
    /// following counted steps are still the handler's own closing wait.
    exit_handler_steps: Option<u16>,
    /// Counted steps since the exit began.
    exit_steps: u16,
    /// Which member of the kneeling family is displayed: 0 is the base.
    kneel_frame: u8,
    /// Ticks still owed to §5's six-tick hold after a return to the base.
    kneel_hold: u8,
    /// The animator's own generator. `animation.md §5` gives the gate
    /// arithmetic but not whether the animator shares the gameplay stream,
    /// and spending gameplay draws on a presentation would shift every roll
    /// the meditation makes afterwards. This is seeded from the gameplay
    /// state once, at the moment the presentation starts, and advanced
    /// independently - the published distribution without the side effect.
    kneel_prng: u16,
}

impl ShrineApproachWalk {
    pub fn new() -> Self {
        Self::default()
    }

    /// The Codex arm of the same presentation (§8).
    pub fn codex() -> Self {
        Self {
            arm: PresentationArm::Codex,
            ..Self::default()
        }
    }

    pub fn arm(&self) -> PresentationArm {
        self.arm
    }

    /// One counted world step of the presentation. The first is the
    /// presentation's own opening step; the approach walk's steps follow;
    /// on the shrine arm the step after the walk is the handler's kneeling
    /// repaint (§7: "that repaint is the forty-sixth world step of the
    /// interval").
    pub fn on_world_step(&mut self) {
        if self.exit_handler_steps.is_some() {
            self.exit_steps = self.exit_steps.saturating_add(1);
            return;
        }
        let walk_end = 1 + self.arm.walk_world_steps();
        if self.steps >= walk_end && self.arm == PresentationArm::Shrine && !self.standing {
            self.kneel();
        }
        self.steps = self.steps.saturating_add(1);
    }

    /// Counted steps run so far, the opening step included.
    pub fn steps(&self) -> u16 {
        self.steps
    }

    /// Whether the approach walk has finished drawing.
    pub fn approach_finished(&self) -> bool {
        self.steps > self.arm.walk_world_steps()
    }

    /// The handler's first act, §7: "replace the walking pose with a
    /// **different** pose, the kneeling one, in the same cell".
    pub fn kneel(&mut self) {
        if self.standing || self.exit_handler_steps.is_some() {
            return;
        }
        self.steps = self.steps.max(1 + self.arm.walk_world_steps());
        self.kneeling = true;
    }

    /// §7: on the ordaining arm "the walking pose ... stamped back into the
    /// slot while the figure is still in the altar cell `(5, 6)`", so the
    /// avatar "stands up in place" through both key waits and the handler's
    /// closing ten steps.
    pub fn stand(&mut self) {
        self.steps = self.steps.max(1 + self.arm.walk_world_steps());
        self.kneeling = false;
        self.standing = true;
    }

    /// §7 "Exit pacing": the handler has returned. `handler_steps` is how
    /// many of the following counted steps are still the handler's own
    /// closing wait (ten on the completion arms, none on a refusal), after
    /// which the presentation re-stamps the walking pose, walks back down,
    /// clears the slot and waits.
    pub fn begin_exit(&mut self, handler_steps: u16) {
        self.steps = self.steps.max(1 + self.arm.walk_world_steps());
        if self.exit_handler_steps.is_none() {
            self.exit_handler_steps = Some(handler_steps);
            self.exit_steps = 0;
        }
    }

    pub fn is_exiting(&self) -> bool {
        self.exit_handler_steps.is_some()
    }

    /// Seed the animator's generator. Called once, when the presentation
    /// starts, from the gameplay state.
    pub fn seed_animator(&mut self, seed: u16) {
        self.kneel_prng = seed;
    }

    /// One animation tick of `animation.md §5`'s kneeling-family cycle.
    ///
    /// "The successors are emitted in ascending order, and from the third
    /// the slot either returns to the base - on a one-in-four roll - and
    /// holds it for six ticks, or rewinds to the first and climbs again.
    /// Each decision point is additionally behind an exact fair coin, so a
    /// frame changes on roughly half the eligible passes."
    ///
    /// Only the displayed frame moves; §7 is explicit that "its type value
    /// reads the base for the whole meditation", which is why `pose` still
    /// reports the kneeling pose and this only chooses the family member.
    pub fn tick_kneeling_frame(&mut self) {
        if !matches!(self.pose(), ShrinePose::Kneeling { .. }) {
            return;
        }
        if self.kneel_hold > 0 {
            self.kneel_hold -= 1;
            return;
        }
        let mut prng = crate::prng::U5Prng::new(self.kneel_prng);
        let coin = prng.next_range_u16(0, SHRINE_KNEELING_ROLL_HIGH);
        if !crate::animation_eligibility_coin_passes(coin as u8) {
            // The fair coin decides whether this pass advances at all.
            self.kneel_prng = prng.state();
            return;
        }
        let last = (SHRINE_KNEELING_POSE_FAMILY.len() - 1) as u8;
        if self.kneel_frame == last {
            let rebase = prng.next_range_u16(0, SHRINE_KNEELING_ROLL_HIGH);
            if crate::animation_snap_taken(rebase as u8) {
                self.kneel_frame = 0;
                self.kneel_hold = SHRINE_KNEELING_BASE_HOLD_TICKS;
            } else {
                // "rewinds to the first and climbs again" - the first
                // successor, not the base. §5: "the base was never entered
                // from anywhere but the third successor".
                self.kneel_frame = 1;
            }
        } else {
            self.kneel_frame += 1;
        }
        self.kneel_prng = prng.state();
    }

    pub fn kneeling_frame(&self) -> u8 {
        self.kneel_frame
    }

    pub fn is_kneeling(&self) -> bool {
        self.kneeling
    }

    pub fn is_standing(&self) -> bool {
        self.standing
    }

    /// One-based approach animation frame, 1..=9 (1..=12 for the Codex).
    /// The presentation's opening step counts as frame 1.
    pub fn frame(&self) -> u8 {
        let walked = self.steps.saturating_sub(2);
        let frame = walked / u16::from(SHRINE_APPROACH_WALK_STEPS_PER_FRAME) + 1;
        frame.min(u16::from(self.arm.walk_frames())) as u8
    }

    pub fn pose(&self) -> ShrinePose {
        let rest_row = self.arm.rest_row();
        if let Some(handler_steps) = self.exit_handler_steps
            && self.exit_steps > handler_steps
        {
            // §7 "Exit pacing": "re-stamps the walking pose ... **in the cell
            // the figure knelt in**, and spends one world step there", then
            // "walks the avatar back down to the bottom grid row one row per
            // animation frame", then clears the slot for the four blank
            // frames and the closing wait.
            let exit = self.exit_steps - handler_steps;
            if exit == 1 {
                return ShrinePose::Walking {
                    column: SHRINE_WALK_COLUMN,
                    row: rest_row,
                };
            }
            let walk_back = exit - 2;
            let frame = walk_back / u16::from(SHRINE_APPROACH_WALK_STEPS_PER_FRAME);
            if frame < u16::from(self.arm.rows_climbed()) {
                return ShrinePose::Walking {
                    column: SHRINE_WALK_COLUMN,
                    row: rest_row + 1 + usize::from(frame),
                };
            }
            return ShrinePose::None;
        }
        if self.standing {
            return ShrinePose::Walking {
                column: SHRINE_WALK_COLUMN,
                row: rest_row,
            };
        }
        if self.kneeling {
            return ShrinePose::Kneeling {
                column: SHRINE_WALK_COLUMN,
                row: rest_row,
            };
        }
        if self.steps <= 1 {
            // The presentation's own opening step: bare backdrop.
            return ShrinePose::None;
        }
        let frame = self.frame();
        if frame < SHRINE_APPROACH_WALK_FIRST_POSE_FRAME {
            return ShrinePose::None;
        }
        // Frame 5 is the bottom row and each later frame is one row up,
        // "ending one row short of the altar tile".
        let climbed = usize::from(frame - SHRINE_APPROACH_WALK_FIRST_POSE_FRAME);
        ShrinePose::Walking {
            column: SHRINE_WALK_COLUMN,
            row: SHRINE_WALK_FIRST_ROW.saturating_sub(climbed).max(rest_row),
        }
    }

    pub fn pose_tile(&self) -> Option<(u16, usize, usize)> {
        match self.pose() {
            ShrinePose::None => None,
            ShrinePose::Walking { column, row } => Some((SHRINE_WALKING_POSE_TILE, column, row)),
            ShrinePose::Kneeling { column, row } => Some((
                SHRINE_KNEELING_POSE_FAMILY[usize::from(self.kneel_frame)
                    .min(SHRINE_KNEELING_POSE_FAMILY.len() - 1)],
                column,
                row,
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn run(walk: &mut ShrineApproachWalk, steps: u16) -> Vec<ShrinePose> {
        (0..steps)
            .map(|_| {
                walk.on_world_step();
                walk.pose()
            })
            .collect()
    }

    /// The distinct cells a run of poses visits, in order.
    fn cells(poses: &[ShrinePose]) -> Vec<(usize, usize)> {
        let mut seen = Vec::new();
        for pose in poses {
            if let ShrinePose::Walking { column, row } = *pose
                && seen.last() != Some(&(column, row))
            {
                seen.push((column, row));
            }
        }
        seen
    }

    /// §7: frames 1-4 draw nothing - "twenty repaints of bare shrine
    /// backdrop, twenty-one counting the presentation's own opening world
    /// step".
    #[test]
    fn the_walk_draws_nothing_for_its_first_four_frames() {
        let mut walk = ShrineApproachWalk::new();
        let poses = run(&mut walk, 21);
        assert!(poses.iter().all(|pose| *pose == ShrinePose::None));
        assert_eq!(walk.frame(), 4);
        walk.on_world_step();
        assert_eq!(walk.frame(), 5);
        assert_eq!(
            walk.pose(),
            ShrinePose::Walking {
                column: SHRINE_WALK_COLUMN,
                row: SHRINE_WALK_FIRST_ROW
            }
        );
    }

    /// §7: "Stamped at `(5, 10)` on frame 5, stepping to `(5, 9)`, `(5, 8)`,
    /// `(5, 7)` and `(5, 6)`."
    #[test]
    fn the_walk_steps_the_published_cells() {
        let mut walk = ShrineApproachWalk::new();
        let poses = run(&mut walk, 1 + SHRINE_APPROACH_WALK_WORLD_STEPS as u16);
        assert_eq!(cells(&poses), vec![(5, 10), (5, 9), (5, 8), (5, 7), (5, 6)]);
        // Five repaints on each painted cell.
        let painted = poses
            .iter()
            .filter(|pose| matches!(pose, ShrinePose::Walking { .. }))
            .count();
        assert_eq!(painted, 25);
    }

    /// The walk ends "one row short of the altar tile, which sits in the same
    /// column one row further up".
    #[test]
    fn the_walk_ends_one_row_below_the_altar() {
        let (altar_column, altar_row) = SHRINE_ALTAR_CELL;
        assert_eq!(altar_column, SHRINE_WALK_COLUMN);
        assert_eq!(SHRINE_WALK_LAST_ROW, altar_row + 1);
    }

    /// §7: the handler's kneeling repaint "is the forty-sixth world step of
    /// the interval".
    #[test]
    fn kneeling_replaces_the_pose_in_the_same_cell() {
        let mut walk = ShrineApproachWalk::new();
        run(&mut walk, 1 + 45);
        assert_eq!(
            walk.pose(),
            ShrinePose::Walking {
                column: SHRINE_WALK_COLUMN,
                row: SHRINE_WALK_LAST_ROW
            }
        );
        walk.on_world_step();
        assert_eq!(
            walk.pose(),
            ShrinePose::Kneeling {
                column: SHRINE_WALK_COLUMN,
                row: SHRINE_WALK_LAST_ROW
            }
        );
    }

    /// §8: the Codex walk stamps `(5, 10)` and steps up to `(5, 3)` - "eight
    /// painted cells at five repaints each" - and never kneels.
    #[test]
    fn the_codex_walk_climbs_seven_rows_and_never_kneels() {
        let mut walk = ShrineApproachWalk::codex();
        let poses = run(&mut walk, 1 + 60 + 5);
        assert_eq!(
            cells(&poses),
            (3..=10).rev().map(|row| (5, row)).collect::<Vec<_>>()
        );
        assert!(poses.iter().all(|pose| !matches!(pose, ShrinePose::Kneeling { .. })));
        assert_eq!(
            poses
                .iter()
                .filter(|pose| matches!(pose, ShrinePose::Walking { .. }))
                .count(),
            40 + 5
        );
    }

    /// §7 "Exit pacing": one re-stamp repaint in the kneel cell, the walk
    /// back down rows 7..10 at five repaints each, then thirty repaints of
    /// bare backdrop - fifty-one steps after the handler's own wait.
    #[test]
    fn the_shrine_exit_walks_back_down_and_clears() {
        assert_eq!(PresentationArm::Shrine.exit_world_steps(), 51);
        let mut walk = ShrineApproachWalk::new();
        run(&mut walk, 1 + 45 + 1 + 10);
        walk.begin_exit(10);
        let poses = run(&mut walk, 10 + 51);
        assert!(
            poses[..10]
                .iter()
                .all(|pose| matches!(pose, ShrinePose::Kneeling { row: 6, .. })),
            "the handler's closing wait still kneels on this arm"
        );
        let exit = &poses[10..];
        assert_eq!(exit[0], ShrinePose::Walking { column: 5, row: 6 });
        assert_eq!(cells(exit), vec![(5, 6), (5, 7), (5, 8), (5, 9), (5, 10)]);
        assert_eq!(
            exit.iter()
                .filter(|pose| matches!(pose, ShrinePose::Walking { .. }))
                .count(),
            1 + 20
        );
        assert!(exit[21..].iter().all(|pose| *pose == ShrinePose::None));
        assert_eq!(exit[21..].len(), 30);
    }

    /// §8: the Codex exit "is the shrine's - the same block": re-stamp at
    /// `(5, 3)`, rows 4 through 10, four blank frames and the closing wait.
    #[test]
    fn the_codex_exit_walks_back_seven_rows() {
        assert_eq!(PresentationArm::Codex.exit_world_steps(), 66);
        let mut walk = ShrineApproachWalk::codex();
        run(&mut walk, 61);
        walk.begin_exit(0);
        let poses = run(&mut walk, 66);
        assert_eq!(
            cells(&poses),
            (3..=10).map(|row| (5, row)).collect::<Vec<_>>()
        );
        assert!(poses[36..].iter().all(|pose| *pose == ShrinePose::None));
    }

    /// §7: the ordaining arm re-stamps the walking pose in the altar cell
    /// and holds it through the key waits and the closing ten steps.
    #[test]
    fn standing_restores_the_walking_pose_in_the_altar_cell() {
        let mut walk = ShrineApproachWalk::new();
        run(&mut walk, 1 + 45 + 1 + 10);
        assert!(walk.is_kneeling());
        walk.stand();
        assert_eq!(walk.pose(), ShrinePose::Walking { column: 5, row: 6 });
        walk.begin_exit(10);
        let poses = run(&mut walk, 11);
        assert!(
            poses
                .iter()
                .all(|pose| *pose == ShrinePose::Walking { column: 5, row: 6 }),
            "ten standing repaints plus the exit's re-stamp make eleven"
        );
    }

    /// `animation.md §5`, driven the way it says it drove the original:
    /// "ten thousand ticks", checking the three claims it makes about the
    /// result - the four values, the complete transition set, and that "the
    /// base was never entered from anywhere but the third successor".
    #[test]
    fn the_kneeling_family_cycles_the_published_transition_set() {
        let mut walk = ShrineApproachWalk::new();
        walk.seed_animator(0x4321);
        walk.kneel();
        let mut seen = std::collections::BTreeSet::new();
        let mut transitions = std::collections::BTreeSet::new();
        let mut changes = 0usize;
        let mut previous = walk.kneeling_frame();
        seen.insert(previous);
        for _ in 0..10_000 {
            walk.tick_kneeling_frame();
            let frame = walk.kneeling_frame();
            seen.insert(frame);
            if frame != previous {
                transitions.insert((previous, frame));
                changes += 1;
            }
            previous = frame;
        }
        assert_eq!(
            seen,
            BTreeSet::from([0, 1, 2, 3]),
            "the displayed frame takes only those four values"
        );
        assert_eq!(
            transitions,
            BTreeSet::from([(0, 1), (1, 2), (2, 3), (3, 0), (3, 1)]),
            "base to first, first to second, second to third, third to base              and third to first"
        );
        assert!(
            transitions.iter().all(|(from, to)| *to != 0 || *from == 3),
            "the base is only ever entered from the third successor"
        );
        // "a frame changes on roughly half the eligible passes" - the fair
        // coin, blunted by the six-tick hold after each re-base.
        assert!(
            (2_000..6_000).contains(&changes),
            "frame changed {changes} times in ten thousand ticks"
        );
    }

    /// §5: from the third successor the slot "returns to the base - on a
    /// one-in-four roll - and holds it for six ticks".
    #[test]
    fn a_return_to_the_base_holds_it() {
        let mut walk = ShrineApproachWalk::new();
        walk.seed_animator(0x1111);
        walk.kneel();
        let mut held = 0usize;
        let mut run = 0usize;
        let mut previous = walk.kneeling_frame();
        for _ in 0..10_000 {
            walk.tick_kneeling_frame();
            let frame = walk.kneeling_frame();
            if previous == 3 && frame == 0 {
                run = 1;
            } else if run > 0 && frame == 0 {
                run += 1;
            } else if run > 0 {
                held = held.max(run);
                run = 0;
            }
            previous = frame;
        }
        assert!(
            held >= usize::from(SHRINE_KNEELING_BASE_HOLD_TICKS),
            "the base was held for {held} ticks, fewer than the published six"
        );
    }

    /// §7 names the family as the beggar run `0x16C..0x16F`, base first.
    #[test]
    fn the_kneeling_family_is_the_published_run() {
        assert_eq!(SHRINE_KNEELING_POSE_FAMILY[0], SHRINE_KNEELING_POSE_TILE);
        for (index, tile) in SHRINE_KNEELING_POSE_FAMILY.iter().enumerate() {
            assert_eq!(*tile, SHRINE_KNEELING_POSE_TILE + index as u16);
        }
    }
}
