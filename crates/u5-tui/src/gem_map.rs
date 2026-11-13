//! The enhanced gem map's terminal half (Phase B design 10.3.6; build plan
//! M3-08).
//!
//! The terminal draws no gem map, but it keeps the same record the Bevy
//! shell keeps, so the record and its file can be tested without a window:
//!
//! - `--gem-map` turns it on for a `--play` run (interactive or scripted)
//!   and for `--route-smoke`. Terminal modes read no `config.toml`
//!   (foundation 1.2.4); under the parity lock the flag is ignored and the
//!   foundation's lock line is the only thing printed;
//! - a probe is taken before each command and its events are folded after
//!   it ([`GemSession`]);
//! - in `--play`, the record is read from `gemmap.u5e` beside the save at
//!   the start, and written after each save the [`crate::TerminalSaveWatch`]
//!   detects, stamped with it. `--route-smoke` keeps it in memory only;
//! - `@gemmap` prints a count-only summary ([`GEM_MAP_TOKEN`]). With the
//!   feature off the token is not intercepted and runs as 1.0.1 runs it;
//! - `--gem-map-png <dir>` (with `--gem-map-canvas WxH`) writes the map a
//!   window would show as a PNG each time a gem view opens ([`GemPng`],
//!   M3-17). It is a test aid: the terminal still draws no map, and the
//!   game sees nothing of it.

use std::path::{Path, PathBuf};

use u5_automap::StaticMaps;
use u5_automap::gem::{
    AttachOutcome, DungeonInputs, GemCanvas, GemImage, GemKnowledge, GemRender, GemSession,
    GemViewState, GemWindow, MapArt, OutdoorLayout, TownInputs, gem_view_open, render_gem_map,
    town_world_plane,
};
use u5_runtime::enhancements::{EffectClass, GemMapToggles, SaveStamp, SidecarStore, Toggle};
use u5_runtime::{Area, PlayState, TileGraphicsDepth};

use crate::equivalence::TerminalFeature;

/// The gem map's play-script token: a count-only summary of the record.
pub const GEM_MAP_TOKEN: &str = "@gemmap";

/// Printed once when the record belongs to the same Avatar's later save
/// (design 1.3.10, `SameLineage`).
pub const GEM_STALE_NOTE: &str = "gem map: the discoveries are from another save of this game";
/// Printed when the record could not be written. The save itself is
/// already on disk and is never touched.
pub const GEM_WRITE_ERROR: &str = "gem map: the discoveries could not be saved";

/// The record of one terminal run.
#[derive(Debug, Default)]
pub struct TerminalGemMap {
    session: GemSession,
    /// The folder holding `SAVED.GAM` and its sidecar folder, when the
    /// record is kept on disk.
    save_dir: Option<PathBuf>,
    /// `--gem-map-png`: the pictures written on each open.
    png: Option<GemPng>,
}

impl TerminalGemMap {
    /// A record kept in memory only: nothing is read or written.
    pub fn in_memory() -> Self {
        Self::default()
    }

    /// A record kept beside the save in `save_dir`. Nothing is read until
    /// [`attach`](Self::attach).
    pub fn for_save_dir(save_dir: &Path) -> Self {
        Self {
            session: GemSession::with_store(SidecarStore::open(save_dir)),
            save_dir: Some(save_dir.to_path_buf()),
            png: None,
        }
    }

    /// The record with `--gem-map-png`'s writer.
    pub fn with_png(mut self, png: GemPng) -> Self {
        self.png = Some(png);
        self
    }

    /// The PNG writer, when `--gem-map-png` is on.
    pub fn png(&self) -> Option<&GemPng> {
        self.png.as_ref()
    }

    /// Reads `gemmap.u5e` against the save on disk, then learns where the
    /// party is. Prints the staleness note when the record is another
    /// save's of the same game.
    pub fn attach(&mut self, state: &PlayState) -> AttachOutcome {
        let now = self.save_dir.as_deref().and_then(SaveStamp::read);
        let outcome = self.session.attach(state, now.as_ref());
        if outcome.shows_stale_note() {
            eprintln!("{GEM_STALE_NOTE}");
        }
        outcome
    }

    /// Before a command.
    pub fn before(&mut self, state: &PlayState) {
        self.session.before(state);
    }

    /// After a command. With the PNG writer the record also takes the
    /// wider outdoor window its canvas shows, as a window does (10.3.3),
    /// and then the picture is written when a gem view has just opened.
    pub fn after(&mut self, state: &PlayState) {
        let window = self.png.as_ref().and_then(|png| png.drawn_window(state));
        self.session.after_drawn(state, 1, window);
        if let Some(png) = self.png.as_mut() {
            png.after(state, self.session.knowledge());
        }
    }

    /// After the save stamped `stamp` was detected: writes the record. An
    /// error is printed and never fails the run.
    pub fn saved(&mut self, stamp: &SaveStamp) {
        if let Err(error) = self.session.saved(stamp) {
            eprintln!("{GEM_WRITE_ERROR}: {error}");
        }
    }

    /// The record's session.
    pub fn session(&self) -> &GemSession {
        &self.session
    }

    /// The `@gemmap` line.
    pub fn summary(&self) -> String {
        self.session.summary()
    }

    /// Whether `command` is the gem map's token.
    pub fn owns(command: &str) -> bool {
        command.trim() == GEM_MAP_TOKEN
    }
}

impl TerminalFeature for TerminalGemMap {
    fn toggle(&self) -> Toggle {
        Toggle::GemMapEnhanced
    }

    fn class(&self) -> EffectClass {
        EffectClass::Presentation
    }

    fn owns_token(&self, token: &str) -> bool {
        Self::owns(token)
    }

    fn on_token(&mut self, _token: &str, _state: &PlayState) -> Vec<String> {
        println!("{}", self.summary());
        Vec::new()
    }

    fn before_command(&mut self, state: &PlayState) {
        self.before(state);
    }

    fn after_command(&mut self, state: &PlayState) {
        self.after(state);
    }
}

/// `--gem-map-png`: the enhanced gem map written as a PNG each time a gem
/// view opens (design 10.3.6, build plan M3-17), at one canvas. It draws
/// with the same renderer, art and inputs as the window, and reads the
/// game only; a failed write is printed and never fails the run.
pub struct GemPng {
    dir: PathBuf,
    canvas: GemCanvas,
    game_dir: PathBuf,
    depth: TileGraphicsDepth,
    toggles: GemMapToggles,
    display_map_brightness: u8,
    /// Built on the first open, from the player's own files.
    art: Option<MapArt>,
    statics: Option<StaticMaps>,
    /// The gem count when the open view was drawn: a new count is a new
    /// view, as in the window (`gem_render`).
    open_gems: Option<u8>,
    image: GemImage,
    written: Vec<PathBuf>,
}

impl std::fmt::Debug for GemPng {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GemPng")
            .field("dir", &self.dir)
            .field("canvas", &self.canvas)
            .field("open_gems", &self.open_gems)
            .field("written", &self.written.len())
            .finish_non_exhaustive()
    }
}

impl GemPng {
    /// A writer into `dir` at `canvas` for the game in `game_dir`, drawing
    /// with `toggles` (the main toggle is taken as on, since the run asked
    /// for the map) and `display.map_brightness` = `display_map_brightness`
    /// for `brightness_link`.
    pub fn new(
        dir: &Path,
        canvas: GemCanvas,
        game_dir: &Path,
        depth: TileGraphicsDepth,
        toggles: GemMapToggles,
        display_map_brightness: u8,
    ) -> Self {
        Self {
            dir: dir.to_path_buf(),
            canvas,
            game_dir: game_dir.to_path_buf(),
            depth,
            toggles: GemMapToggles {
                enhanced: true,
                ..toggles
            },
            display_map_brightness,
            art: None,
            statics: None,
            open_gems: None,
            image: GemImage::default(),
            written: Vec::new(),
        }
    }

    /// The writer of a terminal run: the catalogue's `[gem_map]` defaults,
    /// since terminal modes resolve no settings (foundation 1.2.4), and
    /// `display.map_brightness` off.
    pub fn terminal(
        dir: &Path,
        canvas: GemCanvas,
        game_dir: &Path,
        depth: TileGraphicsDepth,
    ) -> Self {
        Self::new(
            dir,
            canvas,
            game_dir,
            depth,
            GemMapToggles::default(),
            u5_runtime::enhancements::BRIGHTNESS_OFF,
        )
    }

    /// The writer drawing with `art` instead of the player's files (tests
    /// on a synthetic atlas).
    pub fn with_art(mut self, art: MapArt) -> Self {
        self.art = Some(art);
        self
    }

    /// The canvas the pictures are drawn at.
    pub fn canvas(&self) -> GemCanvas {
        self.canvas
    }

    /// The files written so far, oldest first.
    pub fn written(&self) -> &[PathBuf] {
        &self.written
    }

    /// The outdoor window this canvas shows while a gem view is up
    /// outdoors, for the record's side cells; `None` otherwise.
    pub fn drawn_window(&self, state: &PlayState) -> Option<GemWindow> {
        (gem_view_open(state) && matches!(state.area, Area::World { .. }))
            .then(|| OutdoorLayout::new(self.canvas).gem_window())
    }

    /// After a command: writes the picture when a gem view has just opened.
    pub fn after(&mut self, state: &PlayState, know: &GemKnowledge) {
        if !gem_view_open(state) {
            self.open_gems = None;
            return;
        }
        if self.open_gems == Some(state.gems) {
            return;
        }
        self.open_gems = Some(state.gems);
        match self.render(state, know) {
            Ok(Some(path)) => {
                eprintln!("gem map: wrote {}", path.display());
                self.written.push(path);
            }
            Ok(None) => eprintln!("gem map: this view has no enhanced map"),
            Err(error) => eprintln!("gem map: the picture could not be written: {error}"),
        }
    }

    /// Draws the open view and writes it; `None` when this view has no
    /// enhanced map (the window shows the 1.0.1 view then).
    fn render(
        &mut self,
        state: &PlayState,
        know: &GemKnowledge,
    ) -> std::io::Result<Option<PathBuf>> {
        if self.art.is_none() {
            let atlas = u5_runtime::load_tile_atlas(&self.game_dir, self.depth)?;
            let mut art = MapArt::build(&atlas);
            if let (Ok(text), Ok(runic)) = (
                u5_runtime::load_ibm_ch_font(&self.game_dir),
                u5_runtime::load_runes_ch_font(&self.game_dir),
            ) {
                art = art.with_fonts(text, runic);
            }
            self.art = Some(art);
        }
        let t = self.toggles;
        let surroundings = t.opts.town_surroundings;
        let game_dir = &self.game_dir;
        let world = match town_world_plane(state).filter(|_| surroundings) {
            Some(plane) => self
                .statics
                .get_or_insert_with(|| StaticMaps::new(game_dir))
                .world_map(plane)
                .ok(),
            None => None,
        };
        let view = GemViewState::open_all(
            state,
            self.canvas,
            self.display_map_brightness,
            TownInputs {
                know: Some(know),
                world,
                surroundings,
            },
            DungeonInputs {
                know: Some(know),
                glyphs: t.opts.glyphs,
            },
        );
        let art = self.art.as_ref().expect("built above");
        let drawn = render_gem_map(state, know, art, &t, &view, &mut self.image);
        let kind = match drawn {
            GemRender::Cleared => return Ok(None),
            GemRender::Outdoor => "outdoor",
            GemRender::Town => "town",
            GemRender::Dungeon => "dungeon",
        };
        std::fs::create_dir_all(&self.dir)?;
        let path = self.dir.join(format!(
            "gem-{:04}-{kind}-{}x{}.png",
            self.written.len() + 1,
            self.canvas.w,
            self.canvas.h
        ));
        image::save_buffer(
            &path,
            &self.image.rgba,
            self.image.w,
            self.image.h,
            image::ExtendedColorType::Rgba8,
        )
        .map_err(std::io::Error::other)?;
        Ok(Some(path))
    }
}
