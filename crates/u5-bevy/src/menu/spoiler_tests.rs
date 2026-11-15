//! The menu's no-spoiler suite (Phase B design 1.4.7, 3.2.2 to 3.2.4,
//! 3.6; build plan M4-14), with the Save and Load pages' fake host.
//!
//! What a slot, the title layer, a toast or a settings row shows must never
//! tell the player more than the game has shown them:
//!
//! - a thumbnail is the box-filtered faithful frame of the save moment,
//!   never the grow-left canvas, its ring, the menu or a toast;
//! - a slot's place is the published name of the place the party is in
//!   (D-M4.8) or plain words, never a scene key;
//! - a slot's details are the Avatar, the place, the game's date and time
//!   and the turn (and the cheats mark): no quest progress;
//! - no label, help line, toast or title line names a place, person or
//!   item, judged against the names the engine knows and, when
//!   `U5_TEST_ASSET_DIR` names a copy of the game, against the game's own
//!   text (hashed as it is read, nothing kept).
//!
//! The synthetic tests run in every `cargo test`. The real-file suite runs
//! the fresh save after character creation, scripted walks out of the
//! start, a dungeon and the Underworld, saving through the menu in each,
//! with the wider view on in a 16:9 window.

use std::collections::{BTreeSet, HashSet};

use bevy::window::WindowResolution;
use u5_automap::gem::{UNNAMED_DWELLING, slot_place_text};
use u5_runtime::test_fixtures::{
    configured_original_asset_dir, copy_asset_writable, unique_scratch_dir,
};
use u5_runtime::{
    ChargenStats, DungeonScene, GameClock, PlayOptions, PlayTarget, Scene as TownScene, WorldPlane,
    commit_chargen_save,
};

use super::*;
use crate::enh::present::PresentHost;
use crate::menu::frame::Ink;
use crate::menu::pages::{Row, page_rows};
use crate::menu::pages_settings_tests::{
    NOT_NAMES, engine_names, game_proper_names, says, word_hash, words,
};
use crate::menu::render::title_label;
use crate::menu::render_slots::detail_lines;
use crate::menu::thumb::box_filter;

// ---------------------------------------------------------------------------
// What the menu says.

/// Every string literal of the menu's wording table (`menu/text.rs`), read
/// from its source: every label, help line, toast, dialog and title line
/// the menu can show, including any added later.
fn wording_table() -> Vec<String> {
    let source = include_str!("text.rs");
    let mut texts = Vec::new();
    let mut chars = source.chars().peekable();
    let mut in_comment = false;
    while let Some(ch) = chars.next() {
        if in_comment {
            in_comment = ch != '\n';
            continue;
        }
        match ch {
            '/' if chars.peek() == Some(&'/') => in_comment = true,
            // A char literal such as '"' is not a string.
            '\'' => {
                let mut ahead = chars.clone();
                if let (Some(_), Some('\'')) = (ahead.next(), ahead.next()) {
                    chars.next();
                    chars.next();
                }
            }
            '"' => {
                let mut text = String::new();
                while let Some(ch) = chars.next() {
                    match ch {
                        '\\' => {
                            if let Some(next) = chars.next() {
                                // A line continuation joins with a space.
                                if next == '\n' {
                                    while chars.peek().is_some_and(|c| c.is_whitespace()) {
                                        chars.next();
                                    }
                                } else {
                                    text.push(next);
                                }
                            }
                        }
                        '"' => break,
                        _ => text.push(ch),
                    }
                }
                texts.push(text);
            }
            _ => {}
        }
    }
    texts
}

/// The words of the menu's own that name the game itself, not a place,
/// person or item of its world: the title layer's tab.
const THE_GAME_S_TITLE: &[&str] = &["ultima"];

/// Every word of `texts` that is one of the game's proper names (`names`,
/// hashed), or a name the engine knows, with the text it is in.
fn named(texts: &[String], names: Option<&HashSet<u64>>) -> BTreeSet<(String, String)> {
    let known = engine_names();
    let mut hits = BTreeSet::new();
    for text in texts {
        for name in &known {
            if says(text, name) {
                hits.insert((name.clone(), text.clone()));
            }
        }
        let Some(names) = names else {
            continue;
        };
        for word in words(text) {
            if word.len() >= 3
                && names.contains(&word_hash(&word))
                && !NOT_NAMES.contains(&word.as_str())
                && !THE_GAME_S_TITLE.contains(&word.as_str())
            {
                hits.insert((word, text.clone()));
            }
        }
    }
    hits
}

#[test]
fn the_wording_table_is_read_whole() {
    let table = wording_table();
    for text in [
        t::MENU_TITLE,
        t::SAVE_UNAVAILABLE,
        t::UNSAVED_SESSION,
        t::SLOTS_BUSY,
        t::CANNOT_RESTART,
        t::AUDIO_HELP,
        t::PRESET_HELP,
        t::CONTINUE_SLOT,
    ] {
        assert!(table.iter().any(|entry| entry == text), "{text:?}");
    }
    assert!(table.len() > 100, "{}", table.len());
}

/// No line of the wording table, toasts included, names a place or item
/// the engine knows (the title layer's "Ultima V" names the game).
#[test]
fn no_menu_text_names_a_place_or_item_the_engine_knows() {
    let hits = named(&wording_table(), None);
    assert!(hits.is_empty(), "{hits:?}");
}

// ---------------------------------------------------------------------------
// Slot places and details.

/// Whether `place` is a slot place as D-M4.8 allows: the plane's name, a
/// published name, "<dungeon> level N", or the plain words for a dwelling
/// without a name; never a scene key or a byte.
fn place_is_published_words(place: &str) -> bool {
    !place.is_empty()
        && !place.contains(':')
        && !place
            .split_whitespace()
            .any(|word| word.chars().all(|c| c.is_ascii_uppercase()) && word.len() > 1)
        && (place == UNNAMED_DWELLING || place.starts_with(|c: char| c.is_ascii_uppercase()))
        && place
            .split_whitespace()
            .filter(|word| word.chars().any(|c| c.is_ascii_digit()))
            .all(|number| place.contains(&format!("level {number}")))
}

/// Every scene a save can name: both planes, every town, castle, keep and
/// dwelling on two floors, and every dungeon on its eight levels.
#[test]
fn every_slot_place_is_a_published_name_or_plain_words() {
    let mut places = vec![
        slot_place_text(PlayTarget::World(WorldPlane::Britannia), 0),
        slot_place_text(PlayTarget::World(WorldPlane::Underworld), 0),
    ];
    for byte in 1..=32 {
        let town = PlayTarget::Town(TownScene::new(byte).unwrap());
        places.push(slot_place_text(town, 0));
        places.push(slot_place_text(town, -1));
    }
    for record in 0..8 {
        for level in 0..8 {
            let dungeon = PlayTarget::Dungeon(DungeonScene::from_record(record).unwrap());
            let place = slot_place_text(dungeon, level);
            assert!(place.ends_with(&format!(" level {}", level + 1)), "{place}");
            places.push(place);
        }
    }
    for place in &places {
        assert!(place_is_published_words(place), "{place:?}");
    }
    assert!(places.iter().any(|place| place == UNNAMED_DWELLING));
    for bad in ["TOWNE:1", "DWELLING", "0x21", "a 3"] {
        assert!(!place_is_published_words(bad), "{bad:?}");
    }
}

/// The fields a slot carries are the reviewed ones: a field added to the
/// store request or the listing fails to compile here until it is judged
/// against 1.4.7. The Load page's lines are exactly the Avatar, the place,
/// the date and the turn.
#[test]
fn slot_details_carry_no_quest_progress() {
    let StoreSlot {
        target: _,
        avatar: _,
        place: _,
        clock: _,
        turn: _,
        cheats_used: _,
        thumbnail_png: _,
    } = StoreSlot {
        target: SlotTarget::Quick,
        avatar: String::new(),
        place: String::new(),
        clock: [0; 5],
        turn: 0,
        cheats_used: Vec::new(),
        thumbnail_png: None,
    };
    let info = SlotInfo {
        id: SlotId("s0001".into()),
        name: "Before the shrine".into(),
        quick: false,
        imported: false,
        damaged: None,
        avatar: "Tester".into(),
        place: "Britannia".into(),
        clock: [139, 4, 7, 9, 15],
        turn: 812,
        saved_at_unix: 1,
        modified: false,
        thumbnail_png: None,
    };
    let SlotInfo {
        id: _,
        name: _,
        quick: _,
        imported: _,
        damaged: _,
        avatar: _,
        place: _,
        clock: _,
        turn: _,
        saved_at_unix: _,
        modified: _,
        thumbnail_png: _,
    } = info.clone();
    let entry = |info: SlotInfo| crate::menu::pages::SlotEntry { info, thumb: None };
    let lines: Vec<String> = detail_lines(&entry(info.clone()))
        .into_iter()
        .map(|(text, _)| text)
        .collect();
    assert_eq!(
        lines,
        [
            "Tester".to_string(),
            "Britannia".to_string(),
            t::slot_when(info.clock),
            "Turn 812".to_string()
        ]
    );
    let imported = detail_lines(&entry(SlotInfo {
        imported: true,
        ..info.clone()
    }));
    assert_eq!(imported.len(), 3, "an import has no turn count");
    let damaged = detail_lines(&entry(SlotInfo {
        damaged: Some("SAVED.GAM is missing".into()),
        ..info
    }));
    assert_eq!(damaged, [(t::DAMAGED.to_string(), Ink::Grey)]);
}

// ---------------------------------------------------------------------------
// The save, with its frames watched.

/// The box filter of the game's own frame after every frame of a run.
#[derive(Resource, Default)]
struct FaithfulThumbs(Vec<Vec<u8>>);

fn note_faithful_thumb(
    visual: Option<Res<crate::VisualState>>,
    images: Res<Assets<Image>>,
    mut thumbs: ResMut<FaithfulThumbs>,
) {
    let Some(visual) = visual else {
        return;
    };
    let Some(thumb) = images
        .get(&visual.image_handle)
        .and_then(|image| image.data.as_deref())
        .and_then(box_filter)
    else {
        return;
    };
    if thumbs.0.last() != Some(&thumb) {
        thumbs.0.push(thumb);
    }
}

/// Every toast the run showed.
#[derive(Resource, Default)]
struct SeenToasts(BTreeSet<String>);

fn note_toast(toasts: Option<Res<Toasts>>, mut seen: ResMut<SeenToasts>) {
    if let Some(text) = toasts.as_deref().and_then(Toasts::current)
        && !seen.0.contains(text)
    {
        seen.0.insert(text.to_string());
    }
}

/// The menu with named slots, the wider view on, in a 16:9 window: the
/// default launch's shell, its frames and toasts watched.
fn watched_game(state: PlayState, scene: &Scene, host: &Arc<FakeHost>, dir: &Path) -> Game {
    let mut launch = launch(host, dir);
    launch
        .enhancements
        .toggles
        .set(Toggle::DisplayGrowLeft, true);
    let exit = ShellExitSlot::default();
    let slot = exit.clone();
    let mut shell = Shell::new(state, scene, &launch, move |app| {
        app.insert_resource(slot)
            .insert_resource(MenuShell {
                intro_shell: true,
                slots: true,
                shipped: SHIPPED_PHASES,
            })
            .init_resource::<FaithfulThumbs>()
            .init_resource::<SeenToasts>()
            .add_systems(Last, (note_faithful_thumb, note_toast));
    });
    shell.app.world_mut().spawn(Window {
        resolution: WindowResolution::new(2560.0, 1440.0),
        ..default()
    });
    Game { shell, exit }
}

/// The last store's thumbnail, decoded to 80x50 RGBA.
fn stored_thumb(host: &FakeHost) -> Vec<u8> {
    let stores = host.stores();
    let png = stores
        .last()
        .and_then(|store| store.thumbnail_png.clone())
        .expect("the menu save stored a thumbnail");
    image::load_from_memory_with_format(&png, image::ImageFormat::Png)
        .unwrap()
        .to_rgba8()
        .into_raw()
}

/// The thumbnail of the last save is the box filter of a frame the game
/// itself drew during the run, and the wider view really drew its canvas
/// beside it, so the picture was not taken from there.
fn assert_faithful_thumb(game: &Game, host: &FakeHost, what: &str) {
    let thumb = stored_thumb(host);
    assert_eq!(
        thumb.len(),
        (THUMB_WIDTH * THUMB_HEIGHT * 4) as usize,
        "{what}"
    );
    let seen = &game.world().resource::<FaithfulThumbs>().0;
    assert!(
        seen.contains(&thumb),
        "{what}: the thumbnail is no frame the game drew ({} frames)",
        seen.len()
    );
    assert!(
        game.world()
            .get_resource::<PresentHost>()
            .is_some_and(|host| host.composes() > 0),
        "{what}: the canvas was never drawn"
    );
}

/// The synthetic town with the wider view: the thumbnail is the faithful
/// frame's.
#[test]
fn a_thumbnail_is_the_faithful_frame_never_the_canvas() {
    let dir = GameDir::new();
    let host = FakeHost::new(&dir.0);
    let mut game = watched_game(town(), &scene(&dir.0), &host, &dir.0);
    game.walk();
    game.save_new("Walk");
    assert_faithful_thumb(&game, &host, "the synthetic town");
    let store = host.stores().pop().unwrap();
    assert!(place_is_published_words(&store.place), "{}", store.place);
}

// ---------------------------------------------------------------------------
// Real game files.

/// Everything a run of the real game showed in the menu, its toasts and
/// the title layer: what [`named`] judges.
fn shown_in(game: &Game) -> Vec<String> {
    let world = game.world();
    let snap = world_snapshot(world);
    let mut texts: Vec<String> = world.resource::<SeenToasts>().0.iter().cloned().collect();
    let mut rows: Vec<Row> = snap.settings.iter().flatten().cloned().collect();
    rows.extend(page_rows(crate::menu::pages::Page::Top, &snap));
    let title = page_rows(crate::menu::pages::Page::Title, &snap);
    for row in &title {
        texts.push(title_label(row, &snap));
    }
    rows.extend(title);
    for row in rows {
        texts.push(row.label.to_string());
        texts.extend(row.help.map(str::to_string));
        texts.extend(row.reason.map(str::to_string));
        texts.extend(row.tag.map(str::to_string));
    }
    // The Load page's lines but the Avatar's name (the player's own) and
    // the place (judged by `place_is_published_words`).
    for slot in snap.slot_list.iter() {
        texts.push(slot.info.name.clone());
        for (text, _) in detail_lines(slot).into_iter().skip(2) {
            texts.push(text);
        }
    }
    texts
}

/// A writable copy of the game with a fresh save after character creation,
/// or `None` without one.
fn fresh_game() -> Option<PathBuf> {
    let dir = configured_original_asset_dir()?;
    if !dir.join("INIT.GAM").exists() {
        return None;
    }
    let game = unique_scratch_dir("menu-spoiler");
    for entry in fs::read_dir(&dir).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            copy_asset_writable(&entry.path(), &game.join(entry.file_name())).unwrap();
        }
    }
    commit_chargen_save(
        &game,
        b"Tester",
        true,
        ChargenStats {
            strength: 15,
            dexterity: 15,
            intelligence: 15,
        },
    )
    .unwrap();
    Some(game)
}

/// The fresh save, walks out of the start, a dungeon and the Underworld,
/// each saved through the menu with the wider view on: every thumbnail is
/// a faithful frame, every place the party's own in published words, every
/// detail free of quest progress, and nothing the menu, its toasts or the
/// title layer said names the game's people, places or things.
#[test]
fn a_fresh_save_walks_a_dungeon_and_the_underworld_reveal_nothing() {
    let Some(game_dir) = fresh_game() else {
        eprintln!("skipped: set U5_TEST_ASSET_DIR to a writable copy of the game");
        return;
    };
    let names = game_proper_names(&game_dir);
    assert!(names.len() > 100, "the game's proper names");
    let scene = Scene::from_game_dir(&game_dir).unwrap();
    let fresh = {
        let options = load_play_options_from_save(&game_dir).unwrap();
        let mut state = PlayState::load_scene(&game_dir, options).unwrap();
        state.prng_state = 0x1234;
        state
    };
    let load = |options: PlayOptions| {
        let mut state = PlayState::load_scene(&game_dir, options).unwrap();
        state.prng_state = 0x1234;
        state
    };
    let dungeon = load(PlayOptions {
        target: PlayTarget::Dungeon(DungeonScene::new(0x21).unwrap()),
        floor: 0,
        start: Some((1, 1)),
        ..load_play_options_from_save(&game_dir).unwrap()
    });
    let underworld = load(PlayOptions {
        target: PlayTarget::World(WorldPlane::Underworld),
        start: Some((100, 100)),
        clock: GameClock::new(12, 0).unwrap(),
        ..load_play_options_from_save(&game_dir).unwrap()
    });
    let out_of_the_door = [
        KeyCode::Space,
        KeyCode::ArrowDown,
        KeyCode::ArrowDown,
        KeyCode::ArrowRight,
        KeyCode::ArrowDown,
        KeyCode::ArrowDown,
        KeyCode::ArrowLeft,
        KeyCode::ArrowDown,
        KeyCode::ArrowDown,
    ];
    let about = [
        KeyCode::ArrowUp,
        KeyCode::ArrowLeft,
        KeyCode::ArrowDown,
        KeyCode::ArrowRight,
    ];
    let runs: [(&str, PlayState, &[KeyCode]); 4] = [
        ("the fresh save", fresh.clone(), &[]),
        ("out of the door", fresh, &out_of_the_door),
        ("a dungeon", dungeon, &about),
        ("the Underworld", underworld, &about),
    ];
    let mut hits = BTreeSet::new();
    for (what, start, keys) in runs {
        // Each run on its own copy of the working set.
        let dir = unique_scratch_dir("menu-spoiler-run");
        for entry in fs::read_dir(&game_dir).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_file() {
                fs::copy(entry.path(), dir.join(entry.file_name())).unwrap();
            }
        }
        let host = FakeHost::new(&dir);
        let scene = Scene {
            game_dir: dir.clone(),
            ..scene.clone()
        };
        let mut game = watched_game(start, &scene, &host, &dir);
        game.idle(30);
        for key in keys {
            game.tap(*key);
            game.idle(25);
        }
        // Two saves: the second slot is listed beside the first, and the
        // title layer's Continue names the newest.
        game.save_new("First");
        game.walk();
        game.save_new("");
        let stores = host.stores();
        assert_eq!(stores.len(), 2, "{what}: {stores:?}");
        assert_faithful_thumb(&game, &host, what);
        let state = &game.shell.visual().state;
        let options = load_play_options_from_save(&dir).unwrap();
        let expected_place = slot_place_text(options.target, options.floor);
        for store in &stores {
            assert!(place_is_published_words(&store.place), "{what}: {store:?}");
            assert_eq!(store.avatar, "Tester", "{what}");
            assert!(store.cheats_used.is_empty(), "{what}");
        }
        let last = stores.last().unwrap();
        assert_eq!(last.place, expected_place, "{what}: the party's own place");
        assert_eq!(last.turn, state.turn, "{what}");
        assert_eq!(
            last.clock[3..],
            [u16::from(state.clock.hour), u16::from(state.clock.minute)],
            "{what}"
        );
        let texts = shown_in(&game);
        assert!(
            texts.iter().any(|text| text.starts_with("Continue  (")),
            "{what}: the title layer's Continue names a slot: {texts:?}"
        );
        for (word, text) in named(&texts, Some(&names)) {
            hits.insert((what, word, text));
        }
        let _ = fs::remove_dir_all(&dir);
    }
    let _ = fs::remove_dir_all(&game_dir);
    assert!(hits.is_empty(), "names of the game in the menu: {hits:?}");
}
