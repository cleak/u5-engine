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

/// §7 "The altar's own cell": tile `0xB2`, "the only cell of the record
/// holding that tile", at `(5, 5)` - one row above the walk's last cell.
pub const SHRINE_ALTAR_CELL: (usize, usize) = (SHRINE_WALK_COLUMN, 5);
pub const SHRINE_ALTAR_TILE: u8 = 0xB2;

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

/// How far into the approach walk the presentation has got.
///
/// The walk is measured in world steps rather than wall-clock: §7 counts
/// forty-five of them, each "paired with one request for a one-tick
/// hardware-timer delay". This engine does not model the nine two-part
/// stings' own cost, so it spreads the forty-five steps evenly across the
/// measured Approach-to-Kneel interval that
/// `crate::SHRINE_KNEEL_HOLD_BIOS_TICKS` carries - the two agree to within
/// the bracket that constant was measured at. `cleak/u5-engine#30` asks for
/// the constant to fall out of the animation once the sting cost is modelled;
/// until then the interval is the measurement and this is its subdivision.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct ShrineApproachWalk {
    started_at_secs: Option<f64>,
    step: u8,
    /// Set once the meditation handler has replaced the pose.
    kneeling: bool,
}

impl ShrineApproachWalk {
    pub fn new() -> Self {
        Self::default()
    }

    /// Advance to a monotonic clock reading, spreading the published
    /// forty-five world steps across `interval_secs`.
    pub fn advance_to(&mut self, now_secs: f64, interval_secs: f64) {
        let started = *self.started_at_secs.get_or_insert(now_secs);
        if interval_secs <= 0.0 {
            self.step = SHRINE_APPROACH_WALK_WORLD_STEPS;
            return;
        }
        let elapsed = (now_secs - started).max(0.0);
        let per_step = interval_secs / f64::from(SHRINE_APPROACH_WALK_WORLD_STEPS);
        let step = (elapsed / per_step).floor();
        self.step = if step >= f64::from(SHRINE_APPROACH_WALK_WORLD_STEPS) {
            SHRINE_APPROACH_WALK_WORLD_STEPS
        } else {
            step as u8
        };
    }

    /// The handler's first act, §7: "replace the walking pose with a
    /// **different** pose, the kneeling one, in the same cell".
    pub fn kneel(&mut self) {
        self.step = SHRINE_APPROACH_WALK_WORLD_STEPS;
        self.kneeling = true;
    }

    pub fn is_kneeling(&self) -> bool {
        self.kneeling
    }

    /// One-based animation frame, 1..=9.
    pub fn frame(&self) -> u8 {
        (self.step / SHRINE_APPROACH_WALK_STEPS_PER_FRAME + 1)
            .min(SHRINE_APPROACH_WALK_FRAMES)
    }

    pub fn pose(&self) -> ShrinePose {
        if self.kneeling {
            return ShrinePose::Kneeling {
                column: SHRINE_WALK_COLUMN,
                row: SHRINE_WALK_LAST_ROW,
            };
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
            row: SHRINE_WALK_FIRST_ROW.saturating_sub(climbed),
        }
    }

    pub fn pose_tile(&self) -> Option<(u16, usize, usize)> {
        match self.pose() {
            ShrinePose::None => None,
            ShrinePose::Walking { column, row } => Some((SHRINE_WALKING_POSE_TILE, column, row)),
            ShrinePose::Kneeling { column, row } => Some((SHRINE_KNEELING_POSE_TILE, column, row)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_walk_draws_nothing_for_its_first_four_frames() {
        let mut walk = ShrineApproachWalk::new();
        walk.advance_to(0.0, 4.0);
        for frame in 1..SHRINE_APPROACH_WALK_FIRST_POSE_FRAME {
            let step = (frame - 1) * SHRINE_APPROACH_WALK_STEPS_PER_FRAME;
            walk.advance_to(f64::from(step) * (4.0 / 45.0) + 0.001, 4.0);
            assert_eq!(walk.frame(), frame, "frame {frame}");
            assert_eq!(walk.pose(), ShrinePose::None, "frame {frame} draws nothing");
        }
    }

    /// §7: "Stamped at `(5, 10)` on frame 5, stepping to `(5, 9)`, `(5, 8)`,
    /// `(5, 7)` and `(5, 6)`."
    #[test]
    fn the_walk_steps_the_published_cells() {
        let mut walk = ShrineApproachWalk::new();
        walk.advance_to(0.0, 4.5);
        let mut seen = Vec::new();
        for step in 0..SHRINE_APPROACH_WALK_WORLD_STEPS {
            walk.advance_to(f64::from(step) * 0.1 + 0.001, 4.5);
            if let ShrinePose::Walking { column, row } = walk.pose() {
                if seen.last() != Some(&(column, row)) {
                    seen.push((column, row));
                }
            }
        }
        assert_eq!(seen, vec![(5, 10), (5, 9), (5, 8), (5, 7), (5, 6)]);
    }

    /// The walk ends "one row short of the altar tile, which sits in the same
    /// column one row further up".
    #[test]
    fn the_walk_ends_one_row_below_the_altar() {
        let (altar_column, altar_row) = SHRINE_ALTAR_CELL;
        assert_eq!(altar_column, SHRINE_WALK_COLUMN);
        assert_eq!(SHRINE_WALK_LAST_ROW, altar_row + 1);
    }

    #[test]
    fn kneeling_replaces_the_pose_in_the_same_cell() {
        let mut walk = ShrineApproachWalk::new();
        walk.advance_to(0.0, 1.0);
        walk.advance_to(10.0, 1.0);
        assert_eq!(
            walk.pose(),
            ShrinePose::Walking {
                column: SHRINE_WALK_COLUMN,
                row: SHRINE_WALK_LAST_ROW
            }
        );
        walk.kneel();
        assert_eq!(
            walk.pose(),
            ShrinePose::Kneeling {
                column: SHRINE_WALK_COLUMN,
                row: SHRINE_WALK_LAST_ROW
            }
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
