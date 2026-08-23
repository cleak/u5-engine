//! Animation clock, active object, phase ticking, active-ship wind state.

use crate::*;

/// `animation.md §6` (spec HEAD `c00bf63`): the world-tick tile animator
/// owns **exactly five** tile families, and that is the complete list.
///
/// Earlier revisions of `animation.md §6` and `catalogs/tile-catalog.md
/// §4` headed the list with water, lava and torch/fire families plus
/// unnamed "special effect" and "alternate decorative" families. Both
/// documents now retract that: "**no water, lava, brazier or torch tile
/// animates through this pass at all.**" Nothing in this module may grow
/// a water or fire terrain family back.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum StaticTileAnimationFamily {
    /// `0xD4..0xD7`, four-frame cycle, ungated (advances every tick).
    Waterfall,
    /// `0xD8..0xDB`, four-frame cycle, ungated (advances every tick).
    Fountain,
    /// `0x80..0x83`, two-frame toggle in adjacent pairs, inside the
    /// bit-0 gate (half rate).
    Pendulum,
    /// `0xEC..0xEF`, four-frame cycle, inside the same bit-0 gate as the
    /// pendulum (half rate, **not** every tick).
    StandardOfBritannia,
    /// `0xFA..0xFD` — grandfather clock `0xFA..0xFB` and bellows
    /// `0xFC..0xFD` — two-frame toggles in adjacent pairs, inside the
    /// bit-1 gate nested within the bit-0 gate (quarter rate).
    ClockAndBellows,
}

/// One row of the `animation.md §6` family table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StaticTileAnimationFamilySpec {
    pub family: StaticTileAnimationFamily,
    /// First tile id owned by the family.
    pub first_id: u8,
    /// How many adjacent ids the family owns.
    pub id_count: u8,
    /// Frames per cycle group. The four-frame families own one group of
    /// four ids; the paired toggles own two adjacent groups of two.
    pub cycle: u8,
}

/// `animation.md §6` family table, spec HEAD `c00bf63`. The id ranges
/// are published by that section's table and corroborated by
/// `catalogs/tile-catalog.md §3.1` ("waterfall family `0xD4..0xD7`",
/// "the fountain band `0xD8..0xDB`", "grandfather clock `0xFA..0xFB`")
/// and by `catalogs/tile-catalog.md §4`'s animation table.
///
/// No-fallback policy: every range and cycle length below is published
/// spec text. Nothing here is inferred, and
/// [`crate::static_tile_animation_family`] deliberately has no catch-all
/// guess for ids the spec does not list.
pub const STATIC_TILE_ANIMATION_FAMILIES: [StaticTileAnimationFamilySpec; 5] = [
    StaticTileAnimationFamilySpec {
        family: StaticTileAnimationFamily::Waterfall,
        first_id: 0xD4,
        id_count: 4,
        cycle: 4,
    },
    StaticTileAnimationFamilySpec {
        family: StaticTileAnimationFamily::Fountain,
        first_id: 0xD8,
        id_count: 4,
        cycle: 4,
    },
    StaticTileAnimationFamilySpec {
        family: StaticTileAnimationFamily::Pendulum,
        first_id: 0x80,
        id_count: 4,
        cycle: 2,
    },
    StaticTileAnimationFamilySpec {
        family: StaticTileAnimationFamily::StandardOfBritannia,
        first_id: 0xEC,
        id_count: 4,
        cycle: 4,
    },
    StaticTileAnimationFamilySpec {
        family: StaticTileAnimationFamily::ClockAndBellows,
        first_id: 0xFA,
        id_count: 4,
        cycle: 2,
    },
];

impl StaticTileAnimationFamily {
    pub const fn spec(self) -> StaticTileAnimationFamilySpec {
        STATIC_TILE_ANIMATION_FAMILIES[self as usize]
    }

    pub const fn contains(self, tile: u8) -> bool {
        let spec = self.spec();
        tile >= spec.first_id && (tile - spec.first_id) < spec.id_count
    }

    /// First id of the cycle group `tile` belongs to. A four-frame
    /// family has a single group covering all four ids; a paired toggle
    /// has two groups of two, so `0x82` toggles inside `0x82..0x83` and
    /// never displays `0x80`.
    pub const fn cycle_group_base(self, tile: u8) -> u8 {
        let spec = self.spec();
        spec.first_id + ((tile - spec.first_id) / spec.cycle) * spec.cycle
    }
}

/// `animation.md §6` (spec HEAD `c00bf63`): the shared phase counter is
/// incremented once at the end of every pass. Eight ticks is the exact
/// period of the whole layer, so the counter may wrap there:
///
/// * waterfall / fountain advance 8 times in 8 ticks; `8 % 4 == 0`.
/// * pendulum / flag advance on the four odd phases; `4 % 2 == 0` and
///   `4 % 4 == 0`.
/// * clock / bellows advance on phases `3` and `7`; `2 % 2 == 0`.
///
/// This replaces the retracted `STATIC_TILE_ANIMATION_FRAME_WRAP = 12`,
/// which was justified as "the LCM of the three-frame water cycle and
/// the four-frame lava / fire / wind cycles". Both of those cycles are
/// withdrawn, so the justification and the value went with them.
pub const STATIC_TILE_ANIMATION_PERIOD_TICKS: u8 = 8;

/// Which families one run of the `animation.md §6` pass advances.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StaticTileAnimationPass {
    pub waterfall: bool,
    pub fountain: bool,
    pub pendulum: bool,
    pub standard_of_britannia: bool,
    pub clock_and_bellows: bool,
}

impl StaticTileAnimationPass {
    pub const fn advances(self, family: StaticTileAnimationFamily) -> bool {
        match family {
            StaticTileAnimationFamily::Waterfall => self.waterfall,
            StaticTileAnimationFamily::Fountain => self.fountain,
            StaticTileAnimationFamily::Pendulum => self.pendulum,
            StaticTileAnimationFamily::StandardOfBritannia => self.standard_of_britannia,
            StaticTileAnimationFamily::ClockAndBellows => self.clock_and_bellows,
        }
    }
}

/// One run of the `animation.md §6` pass for the tick whose shared
/// phase-counter value is `phase`, written in the spec's own order:
///
/// > "advance the waterfall family, advance the fountain family, then
/// > test bit 0 of the shared phase counter. If bit 0 is clear, the pass
/// > skips **everything** that follows — pendulum, flag and
/// > clock/bellows alike — and goes straight to incrementing the
/// > counter. If bit 0 is set, the pendulum and the flag advance, and
/// > only then is bit 1 tested; the clock/bellows family advances only
/// > when both bits are set."
///
/// The gates are **nested**, not independent: the quarter rate of
/// `0xFA..0xFD` falls out of `bit 0 AND bit 1`, not out of a lone bit-1
/// test. An earlier revision calling this pass "short and
/// unconditional" with independently gated families is withdrawn.
pub const fn static_tile_animation_pass(phase: u8) -> StaticTileAnimationPass {
    // Two families are ungated.
    let waterfall = true;
    let fountain = true;

    if phase & 0x01 == 0 {
        // Bit 0 clear: skip pendulum, flag AND clock/bellows, and go
        // straight to incrementing the counter.
        return StaticTileAnimationPass {
            waterfall,
            fountain,
            pendulum: false,
            standard_of_britannia: false,
            clock_and_bellows: false,
        };
    }

    // Bit 0 set: pendulum and flag advance, and only now is bit 1 read.
    StaticTileAnimationPass {
        waterfall,
        fountain,
        pendulum: true,
        standard_of_britannia: true,
        clock_and_bellows: phase & 0x02 != 0,
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AnimationClock {
    /// `animation.md §6` shared phase counter, wrapped at
    /// [`STATIC_TILE_ANIMATION_PERIOD_TICKS`].
    pub frame: u8,
    /// Moongate presence counter. **Not** a member of the §6 families:
    /// the tile-animation tick never advances it and it has no frame
    /// selector. See `catalogs/tile-catalog.md §4`, "Moongate graphics
    /// are **not** animated at all".
    pub moongate_frame: u8,
}

impl AnimationClock {
    /// A clock whose §6 pass has run `phase` times since phase zero.
    pub const fn at_static_tile_phase(phase: u8) -> Self {
        Self {
            frame: phase % STATIC_TILE_ANIMATION_PERIOD_TICKS,
            moongate_frame: 0,
        }
    }

    /// `animation.md §6`: run the pass once, then increment the shared
    /// phase counter — "whichever path was taken".
    ///
    /// This never touches [`Self::moongate_frame`].
    pub fn tick_static_tiles(&mut self) {
        self.frame = self.frame.wrapping_add(1) % STATIC_TILE_ANIMATION_PERIOD_TICKS;
    }

    pub fn tick_moongate(&mut self) {
        // `MOONGATE_ANIMATION_FRAMES` is 1 - `moons.md §3` withdrew the
        // moongate animator, so the single frame is the whole cycle and
        // the modulo pins the counter at zero. Keep the modulo rather
        // than hard-coding the result: it is the frame-count constant
        // that carries the contract, not this line.
        #[allow(clippy::modulo_one)]
        {
            self.moongate_frame = self.moongate_frame.wrapping_add(1) % MOONGATE_ANIMATION_FRAMES;
        }
    }

    /// How many times `family`'s selectors have advanced since phase
    /// zero, obtained by replaying the nested-gate pass across every
    /// phase the shared counter has already passed through.
    pub fn static_tile_advances(self, family: StaticTileAnimationFamily) -> u8 {
        (0..self.frame)
            .filter(|phase| static_tile_animation_pass(*phase).advances(family))
            .count() as u8
    }

    /// `animation.md §6`: "Each id inside a family owns its own selector
    /// byte, so the four ids of a four-frame family are permanently a
    /// quarter-cycle apart and a wall of waterfall cells does not
    /// flicker in lockstep."
    ///
    /// So an id keeps its own offset inside its cycle group and only the
    /// family's advance count is added on top. A previous revision of
    /// this method used one shared frame for the whole family — "every
    /// cell in the family displays the same frame at any given tick" —
    /// which is withdrawn.
    ///
    /// "These are render selectors, not map edits; the authored map byte
    /// remains the phase-zero tile id." Nothing here mutates a grid.
    pub fn resolve_static_tile(self, tile: u8) -> u8 {
        let Some(family) = static_tile_animation_family(tile) else {
            return tile;
        };
        let cycle = family.spec().cycle;
        let group_base = family.cycle_group_base(tile);
        let own_selector_offset = tile - group_base;
        group_base + (own_selector_offset + self.static_tile_advances(family)) % cycle
    }

    pub fn resolve_moongate_tile(self) -> u8 {
        MOONGATE_TILE_BASE + self.moongate_frame
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActiveObject {
    pub type_byte: u8,
    pub tile: u8,
    pub x: usize,
    pub y: usize,
    pub z: i8,
    pub phase: u8,
    pub aux1: u8,
    pub aux3: u8,
}

impl ActiveObject {
    pub fn empty() -> Self {
        Self {
            type_byte: 0,
            tile: 0,
            x: 0,
            y: 0,
            z: 0,
            phase: 0,
            aux1: 0,
            aux3: 0,
        }
    }

    pub fn moonstone_pickup(slot_index: usize, x: usize, y: usize, z: i8) -> Self {
        Self {
            type_byte: FIRST_PLAYABLE_MOONSTONE_PICKUP_TILE,
            tile: FIRST_PLAYABLE_MOONSTONE_PICKUP_TILE,
            x,
            y,
            z,
            phase: STEADY_PHASE,
            aux1: slot_index as u8,
            aux3: MOONSTONE_PICKUP_AUX3,
        }
    }

    pub fn fixed_hidden_treasure_pickup(record: usize, x: usize, y: usize, z: i8) -> Self {
        Self {
            type_byte: FIXED_HIDDEN_TREASURE_OBJECT_TILE,
            tile: FIXED_HIDDEN_TREASURE_OBJECT_TILE,
            x,
            y,
            z,
            phase: STEADY_PHASE,
            aux1: record as u8,
            aux3: FIXED_HIDDEN_TREASURE_OBJECT_AUX3,
        }
    }

    pub fn free(&mut self) {
        self.type_byte = 0;
    }

    pub fn clear_consumed_record_fields(&mut self) {
        self.type_byte = 0;
        self.tile = 0;
        self.x = 0;
        self.y = 0;
        self.z = 0;
        self.aux1 = 0;
    }

    pub fn tick_phase(&mut self) -> PhaseTick {
        let low = self.phase & 0x0f;
        if low == STEADY_PHASE {
            PhaseTick::Steady
        } else if low > 0 {
            self.phase = (self.phase & 0xf0) | (low - 1);
            PhaseTick::Countdown
        } else {
            PhaseTick::DecisionPoint
        }
    }

    pub fn is_player(self) -> bool {
        self.type_byte == PLAYER_TILE
    }

    pub fn is_player_phantom(self) -> bool {
        self.type_byte == PLAYER_NPC_SENTINEL_TYPE
    }

    pub fn is_empty(self) -> bool {
        self.type_byte == 0
    }

    pub fn moonstone_slot_index(self) -> Option<usize> {
        let slot_index = self.aux1 as usize;
        (self.type_byte == FIRST_PLAYABLE_MOONSTONE_PICKUP_TILE
            && self.tile == FIRST_PLAYABLE_MOONSTONE_PICKUP_TILE
            && self.aux3 == MOONSTONE_PICKUP_AUX3
            && slot_index < MOONSTONE_SLOT_COUNT)
            .then_some(slot_index)
    }

    pub fn fixed_hidden_treasure_record(self) -> Option<usize> {
        let record = self.aux1 as usize;
        (self.type_byte == FIXED_HIDDEN_TREASURE_OBJECT_TILE
            && self.tile == FIXED_HIDDEN_TREASURE_OBJECT_TILE
            && self.aux3 == FIXED_HIDDEN_TREASURE_OBJECT_AUX3
            && record < FIXED_HIDDEN_TREASURE_COUNT)
            .then_some(record)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhaseTick {
    Steady,
    Countdown,
    DecisionPoint,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActiveShipWind {
    None,
    Stalled,
    Drifted,
}
