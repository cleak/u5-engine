//! In-game wall clock: year, month, day, hour, minute.

use std::io;

use crate::*;

/// `time.md §5` Britannian calendar wrap thresholds. Sixty minutes
/// per hour, twenty-four hours per day, twenty-eight days per month,
/// thirteen months per year. The cascade rolls over when minutes
/// reach `MINUTES_PER_HOUR`, hours reach `HOURS_PER_DAY`, days
/// exceed `DAYS_PER_MONTH` (one-based), and months exceed
/// `MONTHS_PER_YEAR` (one-based).
pub const MINUTES_PER_HOUR: u8 = 60;
pub const HOURS_PER_DAY: u8 = 24;
pub const DAYS_PER_MONTH: u8 = 28;
pub const MONTHS_PER_YEAR: u8 = 13;

/// `time.md §12` Britannian calendar year length: thirteen months
/// of twenty-eight days each = 364 days. Implementations must not
/// normalise the calendar to a twelve-month / 365-day year.
pub const DAYS_PER_YEAR: u16 = DAYS_PER_MONTH as u16 * MONTHS_PER_YEAR as u16;

/// `time.md §2`: convert the underlying 0..=23 hour to the 12-hour
/// display value used by status-row presentation. Hour 0 displays as
/// 12; 1..=12 display as themselves; 13..=23 display as `hour - 12`.
/// There is no AM/PM flag in the result; callers consult the raw
/// hour to pick the suffix.
pub const fn display_hour_12h(hour_24h: u8) -> u8 {
    match hour_24h {
        0 => 12,
        1..=12 => hour_24h,
        _ => hour_24h - 12,
    }
}

/// `moons.md §2.2`, the renderer's **scene gate** - the first test in
/// the routine and the only one that precedes the two cache writes:
/// "Scenes outside the surface/town family (combat, intro, and every
/// scene id at or above the location range, dungeons included) never
/// get past the renderer's scene gate. A caller may still call it -
/// several do - but it returns immediately: nothing is drawn, and,
/// because that gate precedes the two cache writes, nothing is cached
/// either."
///
/// Surface is scene `0`; the town family is `1..=32`.
///
/// This is the predicate the two cache-refresh callers use. Whether the
/// strip is actually *painted* once past this gate is
/// [`sky_strip_renders`], and the difference is exactly the erase arm
/// of [`sky_strip_erase_arm`].
pub const fn sky_strip_scene_gate_passes(scene_byte: u8) -> bool {
    scene_byte <= SKY_STRIP_LAST_TOWN_FAMILY_SCENE
}

/// `moons.md §2.2`: the last town-family scene byte. Above it the
/// renderer's scene gate rejects: "every scene id at or above the
/// location range, dungeons included".
pub const SKY_STRIP_LAST_TOWN_FAMILY_SCENE: u8 = 32;

/// `moons.md §2.2`: scene 25, "Ararat, the underworld-only keep". It
/// "reaches the marker painter but makes it paint the strip's footprint
/// flat instead of printing it", by the scene test rather than the
/// level test, "so it fires there whatever the party's floor byte
/// holds".
pub const ARARAT_SCENE_BYTE: u8 = 25;

/// `moons.md §2.2`, the marker painter's **erase arm**, resolved by
/// issue #190: it "is **live, and it fires on four distinct routes in
/// ordinary play**. Reproduce it."
///
/// Two independent tests reach it:
///
/// * the **scene** test - scene [`ARARAT_SCENE_BYTE`]; and
/// * the **below-surface map level** test - the party's saved Z with
///   its high bit set, which is the Underworld plane outdoors and a
///   below-entry (basement) floor inside a town-family location
///   (`formats/saved-gam.md §6`, party Z row). Four locations own such
///   a floor and are reached on it in ordinary play: Yew, both large
///   castles, and Serpent's Hold.
///
/// On the arm "the strip is not rendered at all, and the painter does
/// the same work it does for Ararat: it still caches both glyph bytes,
/// still selects and restores the text window and the runic font, and
/// then flat-fills the strip footprint and rules the scanline under it.
/// Nothing of the hour marker or of either moon is left on screen, and
/// both end-caps are erased with them."
///
/// So the arm suppresses *painting* and not *caching*: callers that
/// reach the renderer past its scene gate still write the pair
/// (`moons.md §3`).
pub const fn sky_strip_erase_arm(scene_byte: u8, below_surface: bool) -> bool {
    scene_byte == ARARAT_SCENE_BYTE || below_surface
}

/// `moons.md §2.2`: `true` when a refresh actually **paints** the
/// twelve-cell strip - the scene gate passes and the erase arm does
/// not fire. `below_surface` is the party Z's high bit: the Underworld
/// plane outdoors, or a below-entry floor inside a location.
///
/// *Corrected (issue #190).* This predicate previously took an
/// `underworld_plane` flag and returned `true` for Ararat and for every
/// below-entry town floor, because `moons.md §2.2` then recorded the
/// erase arm's reachability as unresolved. It is resolved and live, so
/// both now suppress the paint. The predicate is no longer the gate for
/// the cache writes either - that is
/// [`sky_strip_scene_gate_passes`], which the erase arm runs *after*.
pub const fn sky_strip_renders(scene_byte: u8, below_surface: bool) -> bool {
    sky_strip_scene_gate_passes(scene_byte) && !sky_strip_erase_arm(scene_byte, below_surface)
}

/// `time.md §11` (issue #190): `true` when the ambient-audio tick's
/// free-running sub-tick counter, read **on entry and before its own
/// advance**, is one of the two decrementing residues.
///
/// "The sub-tick counter is a single byte that cycles `0, 1, 2, 3, 4, 5,
/// 6, 7` and wraps back to `0`, and the decrement fires on the calls
/// where it holds **`0` or `4`** on entry. So the two residues are zero
/// and four of the eight-phase cycle - every fourth call, not two
/// adjacent calls out of eight."
///
/// The same predicate is the loud-envelope selector for the tick's other
/// branch: "The same two residues also pick the loud envelope in the
/// tick's own lava/shrine effect branch, so one counter drives both
/// behaviours and an implementation should not give them separate
/// phases." That branch is not modelled in this engine - it has no
/// lava/shrine ambient effect - but when it is, it must read this
/// predicate off the same counter rather than a second one.
pub const fn ambient_audio_sub_tick_decrements(sub_tick_on_entry: u8) -> bool {
    let residue = sub_tick_on_entry % AMBIENT_AUDIO_SUB_TICK_PERIOD;
    residue == AMBIENT_AUDIO_DECREMENT_RESIDUES[0] || residue == AMBIENT_AUDIO_DECREMENT_RESIDUES[1]
}

/// `time.md §11` (issue #190): the loud envelope of the ambient tick's
/// lava/shrine effect branch is picked by the *same* two residues off
/// the *same* counter. Alias rather than a second predicate, so the two
/// behaviours cannot drift into separate phases.
pub const fn ambient_audio_sub_tick_selects_loud_envelope(sub_tick_on_entry: u8) -> bool {
    ambient_audio_sub_tick_decrements(sub_tick_on_entry)
}

/// `time.md §5` provision-decrement hours: food is spent only at
/// 06:00, 12:00, and 18:00 (when the food counter is non-zero).
pub const PROVISION_DECREMENT_HOURS: [u8; 3] = [6, 12, 18];

/// `time.md §10` town-arrest surrender path constants. The ordinary
/// town arrest relocates the party to the Yew jail scene and then
/// advances time through repeated cleanup calls of
/// [`TOWN_ARREST_CLEANUP_INCREMENT_MINUTES`] minutes each until the
/// hour byte reaches [`TOWN_ARREST_RELEASE_HOUR`]. The loop does not
/// roll back partial time side effects if the start time is not
/// aligned to the target hour.
pub const TOWN_ARREST_CLEANUP_INCREMENT_MINUTES: u8 = 20;
pub const TOWN_ARREST_RELEASE_HOUR: u8 = 8;

/// `time.md §10`: returns `true` when the town-arrest release loop
/// has reached its hour-byte target and the relocation timing burst
/// can stop. The loop fires another 20-minute cleanup call whenever
/// this returns `false`.
pub const fn town_arrest_release_loop_done(hour: u8) -> bool {
    hour == TOWN_ARREST_RELEASE_HOUR
}

/// `time.md §5`: `true` when the given hour is one of the three
/// provision-decrement hours.
pub const fn is_provision_decrement_hour(hour: u8) -> bool {
    matches!(hour, 6 | 12 | 18)
}

/// `catalogs/quest-graph.md §5`: returns `true` when every
/// Shadowlord slot holds the vanquished sentinel, which is the
/// Doom-entrance gate. Caller supplies the three save-backed
/// slot bytes in slot order: Falsehood, Hatred, Cowardice.
pub const fn all_shadowlords_vanquished(slots: [u8; 3]) -> bool {
    let s0 = slots[0] == SHADOWLORD_HIDEOUT_VANQUISHED;
    let s1 = slots[1] == SHADOWLORD_HIDEOUT_VANQUISHED;
    let s2 = slots[2] == SHADOWLORD_HIDEOUT_VANQUISHED;
    s0 && s1 && s2
}

/// `catalogs/quest-graph.md §5` Shadowlord-name vocabulary in slot
/// order: Falsehood (0) = FAULINEI, Hatred (1) = ASTAROTH,
/// Cowardice (2) = NOSFENTOR. The Yell handler and the shard /
/// flame destruction path consume these case-insensitive strings.
pub const SHADOWLORD_NAME_FAULINEI: &str = "FAULINEI";
pub const SHADOWLORD_NAME_ASTAROTH: &str = "ASTAROTH";
pub const SHADOWLORD_NAME_NOSFENTOR: &str = "NOSFENTOR";

/// `catalogs/quest-graph.md §5`: returns the published Shadowlord
/// name for one of the three hideout slots (`0..=2`).
pub const fn shadowlord_name_for_slot(slot: usize) -> Option<&'static str> {
    Some(match slot {
        0 => SHADOWLORD_NAME_FAULINEI,
        1 => SHADOWLORD_NAME_ASTAROTH,
        2 => SHADOWLORD_NAME_NOSFENTOR,
        _ => return None,
    })
}

/// `catalogs/quest-graph.md §5`: returns the hideout-slot index
/// for a typed Shadowlord name. Caller should uppercase the input
/// (the Yell input pipeline already does this).
pub fn shadowlord_slot_for_name(name: &str) -> Option<usize> {
    match name {
        SHADOWLORD_NAME_FAULINEI => Some(0),
        SHADOWLORD_NAME_ASTAROTH => Some(1),
        SHADOWLORD_NAME_NOSFENTOR => Some(2),
        _ => None,
    }
}

/// `time.md §7` Shadowlord hideout slot range and vanquished
/// sentinel. The midnight pass picks a new hideout id in
/// `1..=8` for each living slot and treats `0xFF` as the
/// vanquished marker (the daily walker skips those slots).
pub const SHADOWLORD_HIDEOUT_FIRST: u8 = 1;
pub const SHADOWLORD_HIDEOUT_LAST: u8 = 8;
pub const SHADOWLORD_HIDEOUT_VANQUISHED: u8 = 0xFF;

/// `time.md §7`: returns `true` when a Shadowlord hideout slot
/// holds the vanquished sentinel; the daily walker skips it
/// without rerolling.
pub const fn shadowlord_hideout_is_vanquished(slot: u8) -> bool {
    slot == SHADOWLORD_HIDEOUT_VANQUISHED
}

/// `time.md §7`: returns `true` when a Shadowlord hideout slot
/// holds a live hideout id in the published `1..=8` range. The
/// midnight rotation only picks values in this band.
pub const fn shadowlord_hideout_is_live(slot: u8) -> bool {
    slot >= SHADOWLORD_HIDEOUT_FIRST && slot <= SHADOWLORD_HIDEOUT_LAST
}

/// `time.md §3` per-turn cleanup "recompute, do not advance"
/// argument. Mode-zero callers (scene transitions, daylight
/// re-evaluation) pass this value so the cleanup routine refreshes
/// daylight and the visible clock without advancing the minute
/// counter or light-source counters. The non-zero per-turn cadences
/// are [`MINUTES_PER_INDOOR_TURN`] (= 1) and [`MINUTES_PER_OUTDOOR_TURN`]
/// (= 2).
pub const CLEANUP_INCREMENT_RECOMPUTE: u8 = 0;

/// `time.md §4` per-turn cleanup state-tag modifier byte values. The
/// `Q` tag halves the minute increment with a 1-minute floor; the
/// `T` tag suppresses the minute-counter and light-counter writes
/// entirely. Other values pass through.
pub const TIMING_TAG_QUICKNESS: u8 = b'Q';
pub const TIMING_TAG_NEGATE_TIME: u8 = b'T';

/// `time.md §4`: apply the state-tag modifier to a caller-supplied
/// minute increment. Returns the adjusted increment to write into the
/// minute counter (or `None` when the `T` tag suppresses the write).
pub const fn apply_timing_tag_increment(increment: u8, tag_byte: u8) -> Option<u8> {
    if tag_byte == TIMING_TAG_NEGATE_TIME {
        return None;
    }
    if tag_byte == TIMING_TAG_QUICKNESS {
        let halved = increment / 2;
        if increment > 0 && halved == 0 {
            return Some(1);
        }
        return Some(halved);
    }
    Some(increment)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GameClock {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
}

impl GameClock {
    pub fn new(hour: u8, minute: u8) -> io::Result<Self> {
        Self::with_date(
            PLAY_START_YEAR,
            PLAY_START_MONTH,
            PLAY_START_DAY,
            hour,
            minute,
        )
    }

    pub fn with_date(year: u16, month: u8, day: u8, hour: u8, minute: u8) -> io::Result<Self> {
        if !(1..=13).contains(&month) || !(1..=28).contains(&day) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("invalid Britannian date year {year}, month {month}, day {day}"),
            ));
        }
        if hour > 23 || minute > 59 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("invalid clock time {hour:02}:{minute:02}"),
            ));
        }
        Ok(Self {
            year,
            month,
            day,
            hour,
            minute,
        })
    }

    pub fn advance_minutes(&mut self, minutes: u8) {
        let total = self.minute as u16 + minutes as u16;
        self.minute = (total % 60) as u8;
        for _ in 0..(total / 60) {
            self.advance_hour();
        }
    }

    pub fn display_hour(self) -> u8 {
        match self.hour {
            0 => 12,
            1..=12 => self.hour,
            _ => self.hour - 12,
        }
    }

    pub fn am_pm_suffix(self) -> &'static str {
        if self.hour < 12 { "A.M." } else { "P.M." }
    }

    pub fn advance_hour(&mut self) {
        self.hour += 1;
        if self.hour >= 24 {
            self.hour = 0;
            self.advance_day();
        }
    }

    pub fn advance_day(&mut self) {
        self.day += 1;
        if self.day > 28 {
            self.day = 1;
            self.month += 1;
            if self.month > 13 {
                self.month = 1;
                self.year = self.year.saturating_add(1);
            }
        }
    }
}

/// `time.md §10` standard per-turn cleanup minute increment supplied
/// by each gameplay mode loop. Indoor scenes (town, dungeon, combat)
/// pass [`MINUTES_PER_INDOOR_TURN`]; the overworld mode loop passes
/// [`MINUTES_PER_OUTDOOR_TURN`]. The cleanup routine can still
/// receive caller-supplied larger values (rest, town arrest, wait
/// commands); the timing-tag adjustments in
/// [`apply_timing_tag_increment`] still apply.
pub const MINUTES_PER_INDOOR_TURN: u8 = 1;
pub const MINUTES_PER_OUTDOOR_TURN: u8 = 2;

/// `time.md §10` per-turn cleanup minute increment for one
/// `MINUTES_PER_*_TURN` mode loop. Combat rounds also use
/// [`MINUTES_PER_INDOOR_TURN`], applied once when the round counter
/// wraps. Town-arrest surrender uses
/// [`TOWN_ARREST_CLEANUP_INCREMENT_MINUTES`]; the rest path drives
/// elapsed rest through repeated ten-minute calls.
pub const TOWN_REST_CLEANUP_INCREMENT_MINUTES: u8 = 10;

/// `time.md §8` per-character month-counter cap. The month rollover
/// increments each of the sixteen character records' one-byte counters
/// (the inn's guest-stay counter), capped at this value. The inn
/// pickup path treats a stored zero as one billable unit, so the cap
/// gates the maximum bill at lodging-rate × 25.
pub const CHARACTER_MONTH_COUNTER_CAP: u8 = 25;

/// `time.md §8`: age one character record's month counter by the
/// 28-day rollover. Increments the byte by one, clamped at
/// [`CHARACTER_MONTH_COUNTER_CAP`]. Apply to every character record
/// at the day-28 → day-1 rollover regardless of party/lodged state.
pub const fn age_character_month_counter(counter: u8) -> u8 {
    if counter >= CHARACTER_MONTH_COUNTER_CAP {
        CHARACTER_MONTH_COUNTER_CAP
    } else {
        counter + 1
    }
}

/// One of the three sky-strip markers per `moons.md` §2.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkyStripMarker {
    FixedHour,
    Trammel,
    Felucca,
}

/// Width of the sky/status strip per `moons.md` §2.
pub const SKY_STRIP_CELL_COUNT: u8 = 12;

/// `moons.md §2` plot order. The renderer attempts hour first, then
/// Trammel, then Felucca; later markers overwrite earlier ones when
/// they select the same cell.
pub const SKY_STRIP_RENDER_ORDER: [SkyStripMarker; 3] = [
    SkyStripMarker::FixedHour,
    SkyStripMarker::Trammel,
    SkyStripMarker::Felucca,
];

/// `shops.md §4.1` time-of-day partition boundaries for the `@`
/// substitution placeholder. Hours `0..MORNING_END` are morning,
/// `MORNING_END..AFTERNOON_END` are afternoon, and the remainder
/// `AFTERNOON_END..HOURS_PER_DAY` are evening. The morning band
/// ends at noon = HOURS_PER_DAY / 2; the afternoon band ends
/// three-quarters of the way through the day (18 = HOURS_PER_DAY
/// * 3 / 4). Anchored to the clock-day-length partitions.
pub const SHOP_TIME_OF_DAY_MORNING_END_HOUR: u8 = HOURS_PER_DAY / 2;
pub const SHOP_TIME_OF_DAY_AFTERNOON_END_HOUR: u8 = HOURS_PER_DAY * 3 / 4;

/// `shops.md` §4.1 substitution placeholder `@` (and any caller that wants
/// the same time-of-day word): returns `"morning"` for hours `0..12`,
/// `"afternoon"` for hours `12..18`, and `"evening"` for hours `18..24`.
pub const fn shop_time_of_day_word(hour: u8) -> &'static str {
    if hour < SHOP_TIME_OF_DAY_MORNING_END_HOUR {
        "morning"
    } else if hour < SHOP_TIME_OF_DAY_AFTERNOON_END_HOUR {
        "afternoon"
    } else {
        "evening"
    }
}

/// `moons.md §2`: compose the twelve-cell sky strip for the given
/// `hour` into a `[Option<SkyStripMarker>; 12]` array. Each cell
/// records the marker that the renderer would paint there, applying
/// the published plot order (hour → Trammel → Felucca, later
/// overwrites earlier when two markers select the same cell). Cells
/// the markers do not reach stay `None`, which the renderer paints
/// blank.
pub fn sky_strip_composed_cells(
    hour: u8,
) -> [Option<SkyStripMarker>; SKY_STRIP_CELL_COUNT as usize] {
    let mut cells: [Option<SkyStripMarker>; SKY_STRIP_CELL_COUNT as usize] =
        [None; SKY_STRIP_CELL_COUNT as usize];
    for marker in SKY_STRIP_RENDER_ORDER {
        if let Some(cell) = sky_strip_marker_position(hour, marker) {
            cells[usize::from(cell)] = Some(marker);
        }
    }
    cells
}

/// Per `moons.md` §2: compute the cell index `0..11` where the given marker
/// is visible at the given hour. Returns `None` when the marker is below the
/// strip's visible horizon.
/// `moons.md §2` published sky-strip marker hour bands and position
/// offsets. Each marker's cell position is computed as
/// `offset - hour`, only when the hour sits inside the corresponding
/// visible band. Promote the per-marker constants so the renderer's
/// cell-position math is auditable against the spec table at one
/// site instead of comparing against bare literals.
pub const SKY_STRIP_FIXED_HOUR_BAND_FIRST: u8 = 6;
pub const SKY_STRIP_FIXED_HOUR_BAND_LAST: u8 = 17;
/// `moons.md §2`: the fixed-hour marker's cell position is
/// `OFFSET - hour`. When `hour == BAND_LAST`, the marker sits in
/// cell `0`; when `hour == BAND_FIRST`, the marker sits in cell
/// `OFFSET - BAND_FIRST`. The offset therefore equals
/// [`SKY_STRIP_FIXED_HOUR_BAND_LAST`] by construction. Anchored
/// to the band-last hour so the cell-zero edge derives from the
/// last visible hour and the marker's offset and the band's
/// upper bound share one source of truth.
pub const SKY_STRIP_FIXED_HOUR_OFFSET: u8 = SKY_STRIP_FIXED_HOUR_BAND_LAST;
pub const SKY_STRIP_TRAMMEL_MORNING_BAND_FIRST: u8 = 0;
pub const SKY_STRIP_TRAMMEL_MORNING_BAND_LAST: u8 = 8;
/// `moons.md §2`: Trammel morning marker's cell position is
/// `OFFSET - hour`. At hour == BAND_LAST the marker sits in cell
/// 0, so the offset equals the band-last hour by construction.
/// Anchored to [`SKY_STRIP_TRAMMEL_MORNING_BAND_LAST`].
pub const SKY_STRIP_TRAMMEL_MORNING_OFFSET: u8 = SKY_STRIP_TRAMMEL_MORNING_BAND_LAST;
pub const SKY_STRIP_TRAMMEL_NIGHT_BAND_FIRST: u8 = 21;
/// `moons.md §2`: Trammel night-band runs through the last hour
/// of the day. Anchored to HOURS_PER_DAY - 1.
pub const SKY_STRIP_TRAMMEL_NIGHT_BAND_LAST: u8 = HOURS_PER_DAY - 1;
pub const SKY_STRIP_TRAMMEL_NIGHT_OFFSET: u8 = 32;
pub const SKY_STRIP_FELUCCA_MORNING_BAND_FIRST: u8 = 0;
pub const SKY_STRIP_FELUCCA_MORNING_BAND_LAST: u8 = 2;
/// `moons.md §2`: Felucca morning marker's cell position is
/// `OFFSET - hour`. At hour == BAND_LAST the marker sits in cell
/// 0, so the offset equals the band-last hour by construction.
/// Anchored to [`SKY_STRIP_FELUCCA_MORNING_BAND_LAST`].
pub const SKY_STRIP_FELUCCA_MORNING_OFFSET: u8 = SKY_STRIP_FELUCCA_MORNING_BAND_LAST;
pub const SKY_STRIP_FELUCCA_AFTERNOON_BAND_FIRST: u8 = 15;
/// `moons.md §2`: Felucca afternoon-band runs through the last
/// hour of the day. Anchored to HOURS_PER_DAY - 1.
pub const SKY_STRIP_FELUCCA_AFTERNOON_BAND_LAST: u8 = HOURS_PER_DAY - 1;
pub const SKY_STRIP_FELUCCA_AFTERNOON_OFFSET: u8 = 26;

pub fn sky_strip_marker_position(hour: u8, marker: SkyStripMarker) -> Option<u8> {
    // Both morning bands in the `moons.md §3` table start at hour 0, so
    // their lower-bound tests are trivially true for a `u8` hour. Keep
    // them written out anyway: the guards mirror the published band
    // table row for row, and dropping the halves that happen to be
    // vacuous today would hide which bound each row actually names.
    #[allow(clippy::absurd_extreme_comparisons)]
    let position = match marker {
        SkyStripMarker::FixedHour
            if hour >= SKY_STRIP_FIXED_HOUR_BAND_FIRST
                && hour <= SKY_STRIP_FIXED_HOUR_BAND_LAST =>
        {
            Some(SKY_STRIP_FIXED_HOUR_OFFSET.wrapping_sub(hour))
        }
        SkyStripMarker::Trammel
            if hour >= SKY_STRIP_TRAMMEL_MORNING_BAND_FIRST
                && hour <= SKY_STRIP_TRAMMEL_MORNING_BAND_LAST =>
        {
            Some(SKY_STRIP_TRAMMEL_MORNING_OFFSET.wrapping_sub(hour))
        }
        SkyStripMarker::Trammel
            if hour >= SKY_STRIP_TRAMMEL_NIGHT_BAND_FIRST
                && hour <= SKY_STRIP_TRAMMEL_NIGHT_BAND_LAST =>
        {
            Some(SKY_STRIP_TRAMMEL_NIGHT_OFFSET.wrapping_sub(hour))
        }
        SkyStripMarker::Felucca
            if hour >= SKY_STRIP_FELUCCA_MORNING_BAND_FIRST
                && hour <= SKY_STRIP_FELUCCA_MORNING_BAND_LAST =>
        {
            Some(SKY_STRIP_FELUCCA_MORNING_OFFSET.wrapping_sub(hour))
        }
        SkyStripMarker::Felucca
            if hour >= SKY_STRIP_FELUCCA_AFTERNOON_BAND_FIRST
                && hour <= SKY_STRIP_FELUCCA_AFTERNOON_BAND_LAST =>
        {
            Some(SKY_STRIP_FELUCCA_AFTERNOON_OFFSET.wrapping_sub(hour))
        }
        _ => None,
    };
    position.filter(|cell| *cell < SKY_STRIP_CELL_COUNT)
}

impl Default for GameClock {
    fn default() -> Self {
        Self {
            year: PLAY_START_YEAR,
            month: PLAY_START_MONTH,
            day: PLAY_START_DAY,
            hour: PLAY_START_HOUR,
            minute: 0,
        }
    }
}
