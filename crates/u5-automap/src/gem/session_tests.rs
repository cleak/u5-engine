//! M3-08 session tests: the probe and observe pair, the gem view's footprint
//! on the frame it opens, the write after a save and only then, the attach
//! of a written record, a new game, and the counts-only summary.

use std::fs;
use std::path::PathBuf;

use u5_runtime::enhancements::{SIDECAR_DIR, SaveStamp, SidecarStore, sidecar_file_name};
use u5_runtime::test_fixtures::{
    debug_game_dir, dungeon_state, open_dungeon_record, open_grid, open_world_grid, test_state,
    world_state,
};
use u5_runtime::{Area, Direction, PlayState, ViewOverlay, ViewOverlayKind, WorldPlane};

use super::*;
use crate::gem::knowledge::FeatureKey;
use crate::gem::payload::EmptyReason;

const NOW: SaveStamp = SaveStamp {
    fnv: 0x0123_4567_89ab_cdef,
    len: 4192,
    avatar: *b"AVATAR\0\0\0",
    date: [0x8B, 0x01, 4, 12, 9, 30],
};

fn town(x: usize, y: usize) -> PlayState {
    test_state(open_grid(), x, y)
}

fn britannia(x: usize, y: usize) -> PlayState {
    let mut state = world_state(open_world_grid(), x, y);
    state.area = Area::World {
        plane: WorldPlane::Britannia,
    };
    state
}

fn lit_dungeon(level: u8, x: usize, y: usize) -> PlayState {
    let mut state = dungeon_state(open_dungeon_record(), level, x, y);
    state.torch_counter = 50;
    state.player.facing = Direction::East;
    state
}

/// A scratch save folder, removed on drop.
struct SaveDir(PathBuf);

impl SaveDir {
    fn new(name: &str) -> Self {
        let dir = debug_game_dir().join(name);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("SAVED.GAM"), [0u8; 16]).unwrap();
        Self(dir)
    }

    fn store(&self) -> Option<SidecarStore> {
        SidecarStore::open(&self.0)
    }

    fn file(&self) -> PathBuf {
        self.0
            .join(SIDECAR_DIR)
            .join(sidecar_file_name(GEM_SIDECAR_TAG).unwrap())
    }
}

impl Drop for SaveDir {
    fn drop(&mut self) {
        if let Some(parent) = self.0.parent() {
            let _ = fs::remove_dir_all(parent);
        }
    }
}

#[test]
fn a_command_between_before_and_after_is_observed() {
    let mut session = GemSession::new();
    let outside = britannia(100, 100);
    session.before(&outside);
    assert!(session.attached(), "the first probe attaches in memory");
    assert!(session.knowledge().is_empty());
    let inside = town(5, 5);
    let Area::Town { scene, .. } = inside.area else {
        panic!("a town")
    };
    assert!(session.after(&inside, 1));
    assert!(
        session
            .knowledge()
            .is_known(&FeatureKey::Place { scene: scene.byte })
    );
    // No probe in flight: nothing more to fold.
    assert!(!session.after(&inside, 1));
}

#[test]
fn attaching_learns_where_the_party_is() {
    let mut session = GemSession::new();
    let state = town(5, 5);
    let outcome = session.attach(&state, None);
    assert_eq!(outcome, AttachOutcome::Empty(EmptyReason::Missing));
    assert_eq!(session.knowledge().entered().len(), 1);
}

#[test]
fn a_gem_view_footprint_is_taken_on_the_frame_it_opens() {
    let mut session = GemSession::new();
    let mut state = lit_dungeon(3, 4, 4);
    session.attach(&state, None);
    let corridor = session.knowledge().cells().total();
    state.active_view_overlay = Some(ViewOverlay {
        title: String::new(),
        text_map: String::new(),
        kind: ViewOverlayKind::Dungeon { level: 3 },
        mode: ViewOverlayMode::GemView,
    });
    let Some(u5_runtime::ViewOverlayFootprint::Dungeon { cells, .. }) =
        state.view_overlay_footprint()
    else {
        panic!("a dungeon footprint");
    };
    assert!(session.frame(&state, None), "the flood is new");
    assert!(session.knowledge().cells().total() >= cells.len() as u32);
    assert!(session.knowledge().cells().total() >= corridor);
    // Still open: nothing new, and the footprint is not taken again.
    assert!(!session.frame(&state, None));
}

#[test]
fn the_record_is_written_only_after_a_save_and_read_back() {
    let dir = SaveDir::new("write");
    let mut session = GemSession::with_store(dir.store());
    let state = town(5, 5);
    session.attach(&state, Some(&NOW));
    session.before(&britannia(10, 10));
    session.after(&lit_dungeon(1, 2, 2), 1);
    assert!(!dir.file().exists(), "nothing is written before a save");

    assert_eq!(
        session.saved(&NOW).unwrap(),
        Some(u5_runtime::enhancements::SidecarWrite::Written)
    );
    assert!(dir.file().exists());

    let mut again = GemSession::with_store(dir.store());
    assert_eq!(again.attach(&state, Some(&NOW)), AttachOutcome::Restored);
    assert_eq!(again.knowledge(), session.knowledge());
}

#[test]
fn no_store_writes_nothing_and_a_newer_record_is_never_overwritten() {
    let mut memory = GemSession::new();
    memory.attach(&town(5, 5), Some(&NOW));
    assert_eq!(memory.saved(&NOW).unwrap(), None);

    let dir = SaveDir::new("newer");
    let store = dir.store().unwrap();
    store
        .write(
            GEM_SIDECAR_TAG,
            &NOW,
            GEM_PAYLOAD_VERSION + 1,
            &crate::gem::encode_payload(&GemKnowledge::new()),
        )
        .unwrap();
    let before = fs::read(dir.file()).unwrap();
    let mut session = GemSession::with_store(Some(store));
    assert_eq!(
        session.attach(&town(5, 5), Some(&NOW)),
        AttachOutcome::ReadOnly { stale: false }
    );
    assert_eq!(session.saved(&NOW).unwrap(), None);
    assert_eq!(fs::read(dir.file()).unwrap(), before);
}

#[test]
fn a_new_game_forgets_the_record_and_removes_its_file() {
    let dir = SaveDir::new("new-game");
    let mut session = GemSession::with_store(dir.store());
    session.attach(&town(5, 5), Some(&NOW));
    session.saved(&NOW).unwrap();
    assert!(dir.file().exists());
    session.new_game().unwrap();
    assert!(!dir.file().exists());
    assert!(session.knowledge().is_empty());
    assert!(!session.attached());
}

#[test]
fn the_summary_is_counts_only() {
    let mut session = GemSession::new();
    session.attach(&town(5, 5), None);
    let line = session.summary();
    assert!(line.starts_with(GEM_SUMMARY_PREFIX));
    assert!(line.contains("places 1,"), "{line}");
    // No coordinate, scene byte or name: every number is a count.
    let Area::Town { scene, .. } = town(5, 5).area else {
        panic!("a town")
    };
    assert!(!line.contains(&format!("0x{:02x}", scene.byte)));
    assert!(line.is_ascii());
}

#[test]
fn the_session_never_changes_the_game() {
    let state = lit_dungeon(2, 1, 4);
    let copy = state.clone();
    let mut session = GemSession::new();
    for _ in 0..10 {
        session.before(&state);
        session.after(&state, 1);
        session.frame(&state, None);
    }
    assert_eq!(
        u5_runtime::enhancements::trajectory_digest(&state),
        u5_runtime::enhancements::trajectory_digest(&copy)
    );
    assert_eq!(state.prng_state, copy.prng_state);
}
