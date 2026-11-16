//! The Teleport face of the Cheats page (Phase B design 7.2.7; build plan
//! M8-05).
//!
//! **Spoilers.** Teleport is the one page of the chapter that names places
//! the player may not have found, and it says so: its title tab reads
//! `SPOILERS` and the line under it warns. The main page offers `[T]` only
//! while `teleport` is effectively on (1.4.7).
//!
//! **Targets.** Only valid targets are offered: Britannia and the
//! Underworld (X, Y 0..255), the locations 1..32 on the floors the engine's
//! floor-page resolver accepts for the player's own files, asked once when
//! the face opens ([`Places::of_game`]), and the eight stock dungeons,
//! levels 1..8 shown (0..7 stored), X, Y 0..7. Nothing else can be picked.
//!
//! **Preview.** Each change asks the runtime's read-only `plan_teleport`
//! and shows what it said: open ground and how the party arrives, the
//! nudged cell, or why it is refused. "This may skip story steps." is shown
//! for Doom and for any location other than the one the party stands in;
//! the engine keeps no record of places entered, so every other location
//! warns (a warning only). Enter queues the move for `cheat_apply`, which
//! plans again and commits; a load failure is shown and changes nothing.

use std::path::Path;

use bevy::prelude::*;
use u5_runtime::cheats::{
    TeleportArea, TeleportPlan, TeleportRefusal, TeleportTarget, plan_teleport,
};
use u5_runtime::{
    Area, DungeonScene, PlayState, Scene, TransportState, WorldPlane, resolve_location_floor_page,
    town_resident_name,
};

use super::edit::{CURSOR, EditRun, Ink};

// ---------------------------------------------------------------------------
// Engine text (7.2.7, 7.2.10).

pub const TELEPORT_TITLE: &str = "Teleport  SPOILERS";
pub const TELEPORT_FOOTER: &str = "Enter: go  Esc: back";
/// The main page's entry, shown only while `teleport` is on.
pub const TELEPORT_ENTRY: &str = "[T] Teleport";
pub const SPOILER_LINES: [&str; 2] = [
    "Lists every place, including ones",
    "you have not found yet.",
];
pub const STORY_WARNING: &str = "This may skip story steps.";
pub const LOAD_FAILED: &str = "Could not load that place.";
pub const TELEPORTED: &str = "Teleported.";
pub const TELEPORT_HELP: &str = "Up/Down field  Left/Right PgUp/PgDn";
const NOW: &str = "Now:";
const WHERE: &str = "Where:";
const PLACE: &str = "Place:";
const FLOOR: &str = "Floor:";
const LEVEL: &str = "Level:";
const CELL_X: &str = "X:";
const CELL_Y: &str = "Y:";
const FLOOR_WORD: &str = "floor";
const LEVEL_WORD: &str = "level";
const DUNGEON_WORD: &str = "Dungeon";
const LOCATION_WORD: &str = "Location";
const OF: &str = "of";
const OPEN: &str = "Open ground.";
const ARRIVES: &str = "Arrives";
const BLOCKED: &str = "Blocked; lands at";
const INSTEAD: &str = "instead.";
const NO_PLACES: &str = "No places found in the game files.";
const NOT_IDLE: &str = "Only at the command prompt.";
const IN_COMBAT: &str = "Not during combat.";
const ABOARD: &str = "Leave the ship first.";
const BAD_FLOOR: &str = "No such floor.";
const OUT_OF_BOUNDS: &str = "Outside the map.";
const NO_OPEN: &str = "No open cell near there.";

/// The area radio's choices, in their drawn order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Where {
    Britannia,
    Underworld,
    Location,
    Dungeon,
}

impl Where {
    pub const ALL: [Where; 4] = [
        Where::Britannia,
        Where::Underworld,
        Where::Location,
        Where::Dungeon,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Where::Britannia => "Britannia",
            Where::Underworld => "Underworld",
            Where::Location => LOCATION_WORD,
            Where::Dungeon => DUNGEON_WORD,
        }
    }

    fn index(self) -> usize {
        Self::ALL.iter().position(|w| *w == self).unwrap_or(0)
    }
}

/// The refusal as the preview and the result line say it.
pub fn refusal_text(refusal: &TeleportRefusal) -> &'static str {
    match refusal {
        TeleportRefusal::NotAtIdlePrompt => NOT_IDLE,
        TeleportRefusal::InCombat => IN_COMBAT,
        TeleportRefusal::AboardVessel => ABOARD,
        TeleportRefusal::BadFloor => BAD_FLOOR,
        TeleportRefusal::OutOfBounds => OUT_OF_BOUNDS,
        TeleportRefusal::NoOpenCell => NO_OPEN,
        TeleportRefusal::LoadFailed(_) => LOAD_FAILED,
    }
}

// ---------------------------------------------------------------------------
// The places offered.

/// The candidate floors asked about for each location: every page offset
/// a location file could hold.
const FLOOR_CANDIDATES: std::ops::RangeInclusive<i8> = -15..=15;
/// The stock dungeon records, scenes `0x21..0x28`.
const DUNGEON_RECORDS: std::ops::Range<u8> = 0..8;
/// The levels of a stock dungeon.
pub const DUNGEON_LEVELS: u8 = 8;

/// Every location and dungeon the face offers, built when it opens.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Places {
    /// Each location with at least one floor, and its floors in order.
    pub locations: Vec<(Scene, Vec<i8>)>,
    pub dungeons: Vec<DungeonScene>,
}

impl Places {
    /// The places `resolves` accepts: scenes 1..32 with the floors it
    /// accepts, and the eight stock dungeons.
    pub fn build(resolves: impl Fn(Scene, i8) -> bool) -> Self {
        let locations = (1..=32u8)
            .filter_map(|byte| Scene::new(byte).ok())
            .map(|scene| {
                let floors: Vec<i8> = FLOOR_CANDIDATES
                    .filter(|floor| resolves(scene, *floor))
                    .collect();
                (scene, floors)
            })
            .filter(|(_, floors)| !floors.is_empty())
            .collect();
        let dungeons = DUNGEON_RECORDS
            .filter_map(|record| DungeonScene::from_record(record).ok())
            .collect();
        Self {
            locations,
            dungeons,
        }
    }

    /// The places of the player's own files, through the engine's
    /// floor-page resolver.
    pub fn of_game(game_dir: &Path) -> Self {
        Self::build(|scene, floor| resolve_location_floor_page(game_dir, scene, floor).is_ok())
    }
}

/// A location's name (the gazetteer's), or its family and number.
pub fn location_name(scene: Scene) -> String {
    town_resident_name(scene.byte)
        .map_or_else(|| format!("{LOCATION_WORD} {}", scene.byte), str::to_string)
}

fn signed(floor: i8) -> String {
    if floor > 0 {
        format!("+{floor}")
    } else {
        floor.to_string()
    }
}

/// Where the party is, in words.
pub fn area_name(area: Area) -> String {
    match area {
        Area::World { plane } => match plane {
            WorldPlane::Britannia => Where::Britannia.label().to_string(),
            WorldPlane::Underworld => Where::Underworld.label().to_string(),
        },
        Area::Town { scene, floor } => {
            format!("{} {FLOOR_WORD} {}", location_name(scene), signed(floor))
        }
        Area::Dungeon { scene, level } => {
            format!("{} {LEVEL_WORD} {}", scene.name(), u32::from(level) + 1)
        }
    }
}

fn transport_words(transport: TransportState) -> &'static str {
    match transport {
        TransportState::Horse { .. } => "on horseback",
        TransportState::Carpet { .. } => "on the carpet",
        _ => "on foot",
    }
}

// ---------------------------------------------------------------------------
// The form.

/// The face's fields, in Up/Down order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TeleportField {
    Where,
    Place,
    Floor,
    X,
    Y,
}

/// The Teleport face's choices and its preview.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TeleportForm {
    places: Places,
    kind: Where,
    location: usize,
    floor: usize,
    dungeon: usize,
    level: u8,
    x: usize,
    y: usize,
    field: TeleportField,
    here: Area,
    preview: Option<TeleportPlan>,
}

impl TeleportForm {
    /// The form on `places`, starting where the party stands.
    pub fn new(places: Places, state: &PlayState) -> Self {
        let mut form = Self {
            places,
            kind: Where::Britannia,
            location: 0,
            floor: 0,
            dungeon: 0,
            level: 0,
            x: state.player.x,
            y: state.player.y,
            field: TeleportField::Where,
            here: state.area,
            preview: None,
        };
        match state.area {
            Area::World { plane } => {
                form.kind = match plane {
                    WorldPlane::Britannia => Where::Britannia,
                    WorldPlane::Underworld => Where::Underworld,
                };
            }
            Area::Town { scene, floor } => {
                form.kind = Where::Location;
                if let Some(i) = form.places.locations.iter().position(|(s, _)| *s == scene) {
                    form.location = i;
                    form.floor = form.places.locations[i]
                        .1
                        .iter()
                        .position(|f| *f == floor)
                        .unwrap_or(0);
                }
            }
            Area::Dungeon { scene, level } => {
                form.kind = Where::Dungeon;
                form.dungeon = form
                    .places
                    .dungeons
                    .iter()
                    .position(|d| *d == scene)
                    .unwrap_or(0);
                form.level = level.min(DUNGEON_LEVELS - 1);
            }
        }
        form.clamp_cell();
        form
    }

    /// The form on the places of the player's own files, planned at once.
    pub fn open(state: &PlayState, game_dir: &Path) -> Self {
        let mut form = Self::new(Places::of_game(game_dir), state);
        form.replan(state, game_dir);
        form
    }

    pub fn places(&self) -> &Places {
        &self.places
    }

    pub fn kind(&self) -> Where {
        self.kind
    }

    pub fn field(&self) -> TeleportField {
        self.field
    }

    pub fn preview(&self) -> Option<&TeleportPlan> {
        self.preview.as_ref()
    }

    /// The fields the current kind shows.
    fn fields(&self) -> &'static [TeleportField] {
        use TeleportField as F;
        match self.kind {
            Where::Britannia | Where::Underworld => &[F::Where, F::X, F::Y],
            _ => &[F::Where, F::Place, F::Floor, F::X, F::Y],
        }
    }

    fn side(&self) -> usize {
        self.area().map_or(1, TeleportArea::side)
    }

    fn clamp_cell(&mut self) {
        let max = self.side() - 1;
        self.x = self.x.min(max);
        self.y = self.y.min(max);
    }

    /// The area picked, if the kind has any place to pick.
    pub fn area(&self) -> Option<TeleportArea> {
        match self.kind {
            Where::Britannia => Some(TeleportArea::World(WorldPlane::Britannia)),
            Where::Underworld => Some(TeleportArea::World(WorldPlane::Underworld)),
            Where::Location => {
                let (scene, floors) = self.places.locations.get(self.location)?;
                Some(TeleportArea::Location {
                    scene: *scene,
                    floor: *floors.get(self.floor)?,
                })
            }
            Where::Dungeon => Some(TeleportArea::Dungeon {
                scene: *self.places.dungeons.get(self.dungeon)?,
                level: self.level,
            }),
        }
    }

    /// The target picked.
    pub fn target(&self) -> Option<TeleportTarget> {
        Some(TeleportTarget {
            area: self.area()?,
            x: self.x,
            y: self.y,
        })
    }

    /// Up/Down.
    pub fn move_field(&mut self, delta: i32) {
        let fields = self.fields();
        let at = fields.iter().position(|f| *f == self.field).unwrap_or(0) as i32;
        let next = (at + delta).clamp(0, fields.len() as i32 - 1);
        self.field = fields[next as usize];
    }

    /// Left/Right (one) and PgUp/PgDn (eight) on the selected field.
    pub fn step(&mut self, delta: i32) {
        let wrap = |at: usize, len: usize| -> usize {
            if len == 0 {
                0
            } else {
                (at as i64 + i64::from(delta.signum())).rem_euclid(len as i64) as usize
            }
        };
        match self.field {
            TeleportField::Where => {
                self.kind = Where::ALL[wrap(self.kind.index(), Where::ALL.len())];
            }
            TeleportField::Place => match self.kind {
                Where::Location => {
                    self.location = wrap(self.location, self.places.locations.len());
                    self.floor = 0;
                }
                Where::Dungeon => self.dungeon = wrap(self.dungeon, self.places.dungeons.len()),
                _ => {}
            },
            TeleportField::Floor => match self.kind {
                Where::Location => {
                    let len = self
                        .places
                        .locations
                        .get(self.location)
                        .map_or(0, |(_, floors)| floors.len());
                    self.floor = (self.floor as i64 + i64::from(delta))
                        .clamp(0, len.saturating_sub(1) as i64)
                        as usize;
                }
                Where::Dungeon => {
                    self.level = (i32::from(self.level) + delta)
                        .clamp(0, i32::from(DUNGEON_LEVELS) - 1)
                        as u8;
                }
                _ => {}
            },
            TeleportField::X | TeleportField::Y => {
                let max = self.side() as i64 - 1;
                let cell = if self.field == TeleportField::X {
                    &mut self.x
                } else {
                    &mut self.y
                };
                *cell = (*cell as i64 + i64::from(delta)).clamp(0, max) as usize;
            }
        }
        self.clamp_cell();
    }

    /// Asks the runtime about the target picked.
    pub fn replan(&mut self, state: &PlayState, game_dir: &Path) {
        self.here = state.area;
        self.preview = self
            .target()
            .map(|target| plan_teleport(state, game_dir, target));
    }

    /// Whether the preview warns of skipped story steps: Doom, or a
    /// location other than the one the party stands in.
    pub fn warns(&self) -> bool {
        match self.area() {
            Some(TeleportArea::Dungeon { scene, .. }) => scene.record == 7,
            Some(TeleportArea::Location { scene, .. }) => {
                !matches!(self.here, Area::Town { scene: here, .. } if here == scene)
            }
            _ => false,
        }
    }

    /// The preview's lines.
    fn preview_lines(&self, state: &PlayState) -> Vec<String> {
        let arrives = match self.kind {
            Where::Dungeon => transport_words(TransportState::Foot),
            _ => transport_words(state.player.transport),
        };
        match &self.preview {
            None => vec![NO_PLACES.to_string()],
            Some(TeleportPlan::Ready { target, nudged }) => {
                let first = if *nudged {
                    format!("{BLOCKED} {},{} {INSTEAD}", target.x, target.y)
                } else {
                    OPEN.to_string()
                };
                vec![first, format!("{ARRIVES} {arrives}.")]
            }
            Some(TeleportPlan::Refused(refusal)) => vec![refusal_text(refusal).to_string()],
        }
    }

    /// Every run of the face except the result line. Column 0 is the box's
    /// first cell, where the `>` cursor sits.
    pub fn runs(&self, state: &PlayState) -> Vec<EditRun> {
        let mut runs = Vec::new();
        let mut put = |col: u32, row: u32, text: String, ink: Ink| {
            runs.push(EditRun {
                col,
                row,
                text,
                ink,
            });
        };
        for (i, line) in SPOILER_LINES.iter().enumerate() {
            put(1, 1 + i as u32, line.to_string(), Ink::Text);
        }
        let now = format!(
            "{NOW} {} {},{}",
            area_name(state.area),
            state.player.x,
            state.player.y
        );
        put(
            1,
            ROW_NOW,
            now.chars().take(TEXT_WIDTH).collect(),
            Ink::Help,
        );
        let cursor = |field: TeleportField| self.field == field;
        let label = |row: u32,
                     field: TeleportField,
                     text: &str,
                     put: &mut dyn FnMut(u32, u32, String, Ink)| {
            if cursor(field) {
                put(0, row, CURSOR.to_string(), Ink::Text);
            }
            put(1, row, text.to_string(), Ink::Help);
        };
        label(ROW_WHERE, TeleportField::Where, WHERE, &mut put);
        for (i, kind) in Where::ALL.iter().enumerate() {
            let mark = if *kind == self.kind { '*' } else { ' ' };
            let (row, col) = (ROW_WHERE + (i / 2) as u32, VALUE_COL + (i % 2) as u32 * 15);
            let ink = if *kind == self.kind {
                Ink::Accent
            } else {
                Ink::Text
            };
            put(col, row, format!("({mark}) {}", kind.label()), ink);
        }
        let side = self.side();
        let mut row = ROW_PLACE;
        match self.kind {
            Where::Location => {
                label(row, TeleportField::Place, PLACE, &mut put);
                let (name, floors) = self
                    .places
                    .locations
                    .get(self.location)
                    .map_or((String::new(), &[][..]), |(scene, floors)| {
                        (location_name(*scene), floors.as_slice())
                    });
                put(VALUE_COL, row, name, Ink::Text);
                row += 1;
                label(row, TeleportField::Floor, FLOOR, &mut put);
                if let Some(floor) = floors.get(self.floor) {
                    let all: Vec<String> = floors.iter().map(|f| signed(*f)).collect();
                    put(VALUE_COL, row, signed(*floor), Ink::Text);
                    put(
                        VALUE_COL + 4,
                        row,
                        format!("({OF} {})", all.join(" ")),
                        Ink::Help,
                    );
                }
                row += 1;
            }
            Where::Dungeon => {
                label(row, TeleportField::Place, PLACE, &mut put);
                let name = self
                    .places
                    .dungeons
                    .get(self.dungeon)
                    .map_or("", |d| d.name());
                put(VALUE_COL, row, name.to_string(), Ink::Text);
                row += 1;
                label(row, TeleportField::Floor, LEVEL, &mut put);
                put(
                    VALUE_COL,
                    row,
                    (u32::from(self.level) + 1).to_string(),
                    Ink::Text,
                );
                put(
                    VALUE_COL + 4,
                    row,
                    format!("({OF} 1-{DUNGEON_LEVELS})"),
                    Ink::Help,
                );
                row += 1;
            }
            _ => {}
        }
        for (field, text, value) in [
            (TeleportField::X, CELL_X, self.x),
            (TeleportField::Y, CELL_Y, self.y),
        ] {
            label(row, field, text, &mut put);
            put(VALUE_COL, row, value.to_string(), Ink::Text);
            put(VALUE_COL + 4, row, format!("(0-{})", side - 1), Ink::Help);
            row += 1;
        }
        for (i, line) in self.preview_lines(state).into_iter().enumerate() {
            put(1, ROW_PREVIEW + i as u32, line, Ink::Text);
        }
        if self.warns() {
            put(1, ROW_WARNING, STORY_WARNING.to_string(), Ink::Accent);
        }
        put(1, ROW_HELP, TELEPORT_HELP.to_string(), Ink::Help);
        runs
    }
}

/// The face's rows, relative to the panel's top: the spoiler warning above
/// the first divider, the form, the help, then the result line.
pub const TELEPORT_DIVIDERS: [u32; 3] = [3, 17, 19];
const ROW_NOW: u32 = 4;
const ROW_WHERE: u32 = 6;
const ROW_PLACE: u32 = 8;
const ROW_PREVIEW: u32 = 13;
const ROW_WARNING: u32 = 15;
const ROW_HELP: u32 = 18;
pub const TELEPORT_RESULT_ROW: u32 = 20;
/// The values' column.
const VALUE_COL: u32 = 8;
/// The text a box row holds from column 1.
const TEXT_WIDTH: usize = 36;

/// One key on the face.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TeleportOutcome {
    Quiet,
    Say(String),
    Go(TeleportTarget),
    Back,
}

/// Applies `key` to `form`, planning again after any change.
pub fn teleport_key(
    form: &mut TeleportForm,
    key: KeyCode,
    state: &PlayState,
    game_dir: &Path,
) -> TeleportOutcome {
    match key {
        KeyCode::Escape => return TeleportOutcome::Back,
        KeyCode::Enter | KeyCode::NumpadEnter => {
            form.replan(state, game_dir);
            return match form.preview() {
                Some(TeleportPlan::Ready { .. }) => form
                    .target()
                    .map_or(TeleportOutcome::Quiet, TeleportOutcome::Go),
                Some(TeleportPlan::Refused(refusal)) => {
                    TeleportOutcome::Say(refusal_text(refusal).to_string())
                }
                None => TeleportOutcome::Say(NO_PLACES.to_string()),
            };
        }
        KeyCode::ArrowUp => form.move_field(-1),
        KeyCode::ArrowDown => form.move_field(1),
        KeyCode::ArrowLeft => form.step(-1),
        KeyCode::ArrowRight => form.step(1),
        KeyCode::PageUp => form.step(-8),
        KeyCode::PageDown => form.step(8),
        _ => return TeleportOutcome::Quiet,
    }
    if !form.fields().contains(&form.field) {
        form.field = TeleportField::Where;
    }
    form.replan(state, game_dir);
    TeleportOutcome::Quiet
}

#[cfg(test)]
#[path = "teleport_tests.rs"]
mod tests;
