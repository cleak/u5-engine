//! The save watcher: every save path gives exactly one stamp of the bytes
//! written, and nothing else gives one (design 1.3.9, P7).

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use super::*;
use crate::enhancements::TOGGLES;
use crate::test_fixtures::{
    debug_game_dir, open_grid, open_world_grid, saved_game_seed_bytes, test_state, world_state,
    write_empty_ool_mirrors,
};
use crate::{
    Area, GATE_TRAVEL_SPELL_INDEX, MoonstoneGateSlot, PlayInputDisposition, SAVED_GAM_LEN,
    SAVED_OOL_FILENAME, SAVED_OOL_LEN, WORLD_LOCATION_TABLE_FILE, WorldPlane,
    handle_play_key_input, world_cell_index,
};

/// A writable save folder holding a seed save, with the seed's
/// modification time set well in the past, so a save written in the same
/// clock tick as the seed still changes it.
fn save_dir() -> PathBuf {
    let dir = debug_game_dir();
    let mut seed = saved_game_seed_bytes(0, 0, 15, 15);
    seed[SAVE_AVATAR_NAME_OFFSET..SAVE_AVATAR_NAME_OFFSET + 4].copy_from_slice(b"Ava\0");
    fs::write(dir.join(SAVED_GAM_FILENAME), &seed).unwrap();
    fs::write(dir.join(SAVED_OOL_FILENAME), vec![0; SAVED_OOL_LEN]).unwrap();
    write_empty_ool_mirrors(&dir);
    backdate(&dir);
    dir
}

fn backdate(dir: &Path) {
    let file = fs::File::options()
        .write(true)
        .open(dir.join(SAVED_GAM_FILENAME))
        .unwrap();
    file.set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000))
        .unwrap();
}

fn saved_bytes(dir: &Path) -> Vec<u8> {
    fs::read(dir.join(SAVED_GAM_FILENAME)).unwrap()
}

fn key(state: &mut PlayState, dir: &Path, key: char, suffix: &str) {
    assert_eq!(
        handle_play_key_input(state, key, suffix, dir).unwrap(),
        PlayInputDisposition::Continue
    );
}

/// Polls after every key, as both frontends do, and collects the stamps.
struct Run<'a> {
    dir: &'a Path,
    watcher: SaveWatcher,
    stamps: Vec<SaveStamp>,
}

impl<'a> Run<'a> {
    fn new(dir: &'a Path) -> Self {
        Self {
            dir,
            watcher: SaveWatcher::new(dir),
            stamps: Vec::new(),
        }
    }

    fn key(&mut self, state: &mut PlayState, key_char: char, suffix: &str) {
        key(state, self.dir, key_char, suffix);
        self.poll(state);
    }

    fn poll(&mut self, state: &PlayState) {
        self.stamps.extend(self.watcher.poll(state, self.dir));
    }

    /// Exactly one stamp so far, and it is the stamp of the bytes on disk.
    fn assert_one_stamp_of_the_save(&self, label: &str) {
        let bytes = saved_bytes(self.dir);
        assert_eq!(
            self.stamps,
            [SaveStamp::of_image(&bytes)],
            "{label}: one stamp of the bytes written"
        );
        assert_eq!(self.watcher.last_stamp(), Some(self.stamps[0]), "{label}");
    }
}

#[test]
fn fnv1a64_is_the_published_fnv_1a() {
    assert_eq!(fnv1a64(b""), 0xcbf2_9ce4_8422_2325);
    assert_eq!(fnv1a64(b"a"), 0xaf63_dc4c_8601_ec8c);
    assert_eq!(fnv1a64(b"foobar"), 0x8594_4171_f739_67e8);
    const AT_COMPILE_TIME: u64 = fnv1a64(b"a");
    assert_eq!(AT_COMPILE_TIME, fnv1a64(b"a"));
}

#[test]
fn a_stamp_carries_the_hash_length_avatar_and_calendar() {
    let mut image = saved_game_seed_bytes(0, 0, 15, 15);
    image[SAVE_AVATAR_NAME_OFFSET..SAVE_AVATAR_NAME_OFFSET + 9].copy_from_slice(b"Shamino\0\0");
    image[SAVE_YEAR_OFFSET..SAVE_YEAR_OFFSET + 2].copy_from_slice(&0x0182_u16.to_le_bytes());
    image[SAVE_MONTH_OFFSET] = 4;
    image[SAVE_DAY_OFFSET] = 21;
    image[SAVE_HOUR_OFFSET] = 9;
    image[SAVE_MINUTE_OFFSET] = 33;
    let stamp = SaveStamp::of_image(&image);
    assert_eq!(stamp.fnv, fnv1a64(&image));
    assert_eq!(stamp.len as usize, SAVED_GAM_LEN);
    assert_eq!(&stamp.avatar, b"Shamino\0\0");
    assert_eq!(stamp.date, [0x82, 0x01, 4, 21, 9, 33]);

    let dir = debug_game_dir();
    assert_eq!(SaveStamp::read(&dir), None, "no SAVED.GAM, no stamp");
    fs::write(dir.join(SAVED_GAM_FILENAME), [0; 10]).unwrap();
    assert_eq!(SaveStamp::read(&dir), None, "not a save image");
    fs::write(dir.join(SAVED_GAM_FILENAME), &image).unwrap();
    assert_eq!(SaveStamp::read(&dir), Some(stamp));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn no_stamp_without_a_save() {
    let dir = save_dir();
    let mut state = test_state(open_grid(), 3, 4);
    let mut run = Run::new(&dir);
    assert_eq!(
        run.watcher.last_stamp(),
        SaveStamp::read(&dir),
        "the save on disk at start is known"
    );
    for _ in 0..3 {
        run.poll(&state);
    }
    // Steps, a pass, and a save prompt answered No.
    run.key(&mut state, 'd', "");
    run.key(&mut state, ' ', "");
    run.key(&mut state, 'Q', "");
    assert!(save_prompt_open(&state), "Q opens the save prompt");
    run.key(&mut state, 'N', "");
    assert!(!save_prompt_open(&state));
    run.key(&mut state, 'Q', "N");
    assert!(run.stamps.is_empty(), "{:?}", run.stamps);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn a_q_y_save_through_the_prompt_gives_one_stamp() {
    let dir = save_dir();
    let mut state = test_state(open_grid(), 3, 4);
    let mut run = Run::new(&dir);
    run.key(&mut state, 'd', "");
    run.key(&mut state, 'Q', "");
    assert!(save_prompt_open(&state));
    assert!(run.stamps.is_empty(), "an open prompt is not a save");
    run.key(&mut state, 'Y', "");
    run.assert_one_stamp_of_the_save("Q, Y");
    for _ in 0..3 {
        run.poll(&state);
    }
    run.key(&mut state, 'a', "");
    run.assert_one_stamp_of_the_save("later polls");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn the_prompt_closing_alone_finds_a_save_the_metadata_hides() {
    // A filesystem whose timestamps did not move: the closing prompt is
    // what triggers the hash.
    let dir = save_dir();
    let mut state = test_state(open_grid(), 3, 4);
    let mut run = Run::new(&dir);
    run.key(&mut state, 'Q', "");
    key(&mut state, &dir, 'Y', "");
    backdate(&dir);
    run.poll(&state);
    run.assert_one_stamp_of_the_save("prompt closed, metadata unchanged");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn a_buffered_q_y_inside_one_frame_gives_one_stamp() {
    let dir = save_dir();
    let mut state = test_state(open_grid(), 3, 4);
    let mut run = Run::new(&dir);
    run.key(&mut state, 'd', "");
    // Q and Y read in one frame: the poll never sees the prompt open.
    run.key(&mut state, 'Q', "Y");
    assert!(!save_prompt_open(&state));
    run.assert_one_stamp_of_the_save("buffered Q, Y");
    run.poll(&state);
    run.assert_one_stamp_of_the_save("later polls");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn two_saves_give_two_stamps_and_an_unchanged_resave_none() {
    let dir = save_dir();
    let mut state = test_state(open_grid(), 3, 4);
    let mut run = Run::new(&dir);
    run.key(&mut state, 'Q', "Y");
    run.key(&mut state, 'd', "");
    run.key(&mut state, 'Q', "");
    run.key(&mut state, 'Y', "");
    assert_eq!(run.stamps.len(), 2);
    assert_ne!(run.stamps[0].fnv, run.stamps[1].fnv);
    assert_eq!(run.stamps[1], SaveStamp::of_image(&saved_bytes(&dir)));

    // The same bytes written again are the same save.
    let bytes = saved_bytes(&dir);
    fs::write(dir.join(SAVED_GAM_FILENAME), &bytes).unwrap();
    backdate(&dir);
    run.poll(&state);
    assert_eq!(run.stamps.len(), 2, "unchanged bytes give no stamp");
    let _ = fs::remove_dir_all(dir);
}

/// Britannia at `(x, y)` on an open grid.
fn britannia(grid: Vec<u8>, x: usize, y: usize) -> PlayState {
    let mut state = world_state(grid, x, y);
    state.area = Area::World {
        plane: WorldPlane::Britannia,
    };
    state.active_objects[0].z = WorldPlane::Britannia.save_floor();
    state
        .rebuild_world_live_chunks_from_grid(WorldPlane::Britannia)
        .unwrap();
    state
}

/// The party on a location's entrance tile at Britannia (10, 20).
fn at_entrance(dir: &Path, row: &str, tile: u8) -> PlayState {
    fs::write(dir.join(WORLD_LOCATION_TABLE_FILE), row).unwrap();
    let mut grid = open_world_grid();
    grid[world_cell_index(10, 20)] = tile;
    britannia(grid, 10, 20)
}

#[test]
fn a_save_after_town_entry_gives_one_stamp() {
    let dir = save_dir();
    let mut state = at_entrance(&dir, "BRITANNIA 10 20 CASTLE:0 7 0x15 CASTLE\n", 0x15);
    let mut run = Run::new(&dir);
    state.enter_current_location(&dir).unwrap();
    run.poll(&state);
    assert!(matches!(state.area, Area::Town { .. }));
    assert!(run.stamps.is_empty(), "entering a town is not a save");
    run.key(&mut state, 'Q', "");
    run.key(&mut state, 'Y', "");
    run.assert_one_stamp_of_the_save("town entry, then Q, Y");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn a_save_after_dungeon_entry_gives_one_stamp() {
    let dir = save_dir();
    let mut state = at_entrance(&dir, "BRITANNIA,10,20,DUNGEON:0,0x18,DUNGEON\n", 0x18);
    let mut run = Run::new(&dir);
    state.enter_current_location(&dir).unwrap();
    run.poll(&state);
    assert!(matches!(state.area, Area::Dungeon { .. }));
    assert!(run.stamps.is_empty(), "entering a dungeon is not a save");
    state.save_game_command(&dir, None).unwrap();
    run.poll(&state);
    state.save_game_command(&dir, Some(true)).unwrap();
    run.poll(&state);
    run.assert_one_stamp_of_the_save("dungeon entry, then a save");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn a_save_after_gate_travel_gives_one_stamp() {
    let dir = save_dir();
    let mut state = britannia(open_world_grid(), 5, 5);
    state.spell_charges[GATE_TRAVEL_SPELL_INDEX] = 1;
    state.party[0].mana = 9;
    state.party[0].level = 8;
    state.moonstone_slots[1] = MoonstoneGateSlot {
        scene: 0,
        x: 6,
        y: 7,
        z: 0,
    };
    let mut run = Run::new(&dir);
    run.key(&mut state, 'C', "1PRV2");
    assert_eq!(
        (state.player.x, state.player.y),
        (6, 7),
        "the gate moved us"
    );
    assert!(run.stamps.is_empty(), "gate travel is not a save");
    run.key(&mut state, 'Q', "Y");
    run.assert_one_stamp_of_the_save("gate travel, then Q, Y");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn a_slot_load_gives_one_stamp() {
    let dir = save_dir();
    let state = test_state(open_grid(), 3, 4);
    let mut run = Run::new(&dir);
    // A slot restore writes another save over SAVED.GAM.
    let mut slot = saved_game_seed_bytes(0, 0, 40, 41);
    slot[SAVE_AVATAR_NAME_OFFSET] = b'S';
    fs::write(dir.join(SAVED_GAM_FILENAME), &slot).unwrap();
    run.stamps.extend(run.watcher.poll_rewritten(&dir));
    assert_eq!(run.stamps, [SaveStamp::of_image(&slot)]);
    run.poll(&state);
    run.poll(&state);
    assert_eq!(run.stamps.len(), 1, "the frame after the load adds nothing");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn a_new_game_after_a_reset_gives_one_stamp_even_with_the_same_bytes() {
    let dir = save_dir();
    let state = test_state(open_grid(), 3, 4);
    let mut watcher = SaveWatcher::new(&dir);
    let before = watcher.last_stamp();
    assert!(before.is_some());
    // Without the reset, the same bytes are no new save.
    assert_eq!(watcher.poll_rewritten(&dir), None);
    watcher.reset();
    assert_eq!(watcher.last_stamp(), None);
    assert_eq!(watcher.poll_rewritten(&dir), before);
    assert_eq!(watcher.poll(&state, &dir), None);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn the_save_bytes_are_the_same_with_and_without_the_watcher() {
    fn play(dir: &Path, watch: bool) -> Vec<u8> {
        let mut state = test_state(open_grid(), 3, 4);
        let mut watcher = watch.then(|| SaveWatcher::new(dir));
        for (key_char, suffix) in [('d', ""), ('s', ""), ('Q', ""), ('Y', ""), ('a', "")] {
            key(&mut state, dir, key_char, suffix);
            if let Some(watcher) = watcher.as_mut() {
                watcher.poll(&state, dir);
            }
        }
        key(&mut state, dir, 'Q', "Y");
        if let Some(watcher) = watcher.as_mut() {
            watcher.poll(&state, dir);
        }
        saved_bytes(dir)
    }
    let watched = save_dir();
    let unwatched = save_dir();
    assert_eq!(play(&watched, true), play(&unwatched, false));
    let _ = fs::remove_dir_all(watched);
    let _ = fs::remove_dir_all(unwatched);
}

#[test]
fn only_the_gem_map_toggle_keeps_a_sidecar_so_far() {
    assert!(!sidecar_feature_on(FeatureToggles::CLASSIC));
    // Chapter 10's `gem_map.enhanced` is the first sidecar feature with a
    // catalogue row (M3-01); chapters 4, 7 and 8 bring the others.
    let mut others = FeatureToggles::CLASSIC;
    for spec in TOGGLES.iter().filter(|spec| spec.key != "gem_map.enhanced") {
        others.set(spec.toggle, true);
    }
    assert!(!sidecar_feature_on(others));
    let mut gem = FeatureToggles::CLASSIC;
    gem.set(Toggle::GemMapEnhanced, true);
    assert!(sidecar_feature_on(gem));
    let resolved: Vec<&str> = SIDECAR_KEYS
        .iter()
        .copied()
        .filter(|key| Toggle::from_key(key).is_some())
        .collect();
    assert_eq!(resolved, ["gem_map.enhanced"]);
    for key in SIDECAR_KEYS {
        let (section, name) = key.split_once('.').unwrap();
        assert!(!section.is_empty() && !name.is_empty() && !name.contains('.'));
    }
}
