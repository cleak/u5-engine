//! The Teleport face: valid targets only, its spoiler labels and its form
//! (design 7.2.7, 1.4.7; build plan M8-05).

use u5_runtime::enhancements::Toggle;
use u5_runtime::test_fixtures::{debug_game_dir, open_grid, test_state};

use super::*;

/// A resolver that accepts floors -1..=1 of the castles and floor 0
/// elsewhere.
fn fake_places() -> Places {
    Places::build(|scene, floor| match scene.byte {
        17..=24 => (-1..=1).contains(&floor),
        14 => false,
        _ => floor == 0,
    })
}

#[test]
fn only_valid_targets_are_offered() {
    let places = fake_places();
    let bytes: Vec<u8> = places.locations.iter().map(|(s, _)| s.byte).collect();
    assert_eq!(bytes.len(), 31, "scene 14 has no floor, so is not offered");
    assert!(bytes.iter().all(|b| (1..=32).contains(b)));
    assert!(!bytes.contains(&14));
    for (scene, floors) in &places.locations {
        let want: Vec<i8> = if (17..=24).contains(&scene.byte) {
            vec![-1, 0, 1]
        } else {
            vec![0]
        };
        assert_eq!(floors, &want, "{scene:?}");
    }
    let dungeons: Vec<u8> = places.dungeons.iter().map(|d| d.byte).collect();
    assert_eq!(dungeons, (0x21..=0x28).collect::<Vec<u8>>());
    // Never the non-stock dungeon class, the intro values or the combat
    // marker.
    for never in [0x29u8, 0x40, 0x41, 0x42, 0x7F, 0xFF] {
        assert!(!bytes.contains(&never) && !dungeons.contains(&never));
    }
}

#[test]
fn the_floor_list_comes_from_the_engine_s_resolver() {
    let dir = debug_game_dir();
    let places = Places::of_game(&dir);
    assert!(!places.locations.is_empty());
    for (scene, floors) in &places.locations {
        for floor in floors {
            assert!(
                resolve_location_floor_page(&dir, *scene, *floor).is_ok(),
                "{scene:?} {floor}"
            );
        }
        for floor in FLOOR_CANDIDATES.filter(|f| !floors.contains(f)) {
            assert!(resolve_location_floor_page(&dir, *scene, floor).is_err());
        }
    }
    let _ = std::fs::remove_dir_all(dir);
}

fn form() -> TeleportForm {
    // The synthetic castle, floor 0, at 5,5.
    TeleportForm::new(fake_places(), &test_state(open_grid(), 5, 5))
}

#[test]
fn the_form_starts_where_the_party_is_and_stays_in_bounds() {
    let mut form = form();
    assert_eq!(form.kind(), Where::Location);
    let castle = Scene::new(17).unwrap();
    assert_eq!(
        form.target(),
        Some(TeleportTarget {
            area: TeleportArea::Location {
                scene: castle,
                floor: 0
            },
            x: 5,
            y: 5
        })
    );
    // X is clamped to the location's 0..31.
    form.move_field(3);
    assert_eq!(form.field(), TeleportField::X);
    for _ in 0..10 {
        form.step(8);
    }
    assert_eq!(form.target().unwrap().x, 31);
    // The floor stays in the resolved list.
    form.move_field(-1);
    assert_eq!(form.field(), TeleportField::Floor);
    for _ in 0..5 {
        form.step(1);
    }
    assert!(matches!(
        form.area(),
        Some(TeleportArea::Location { floor: 1, .. })
    ));
    // The radio wraps through the four areas; a dungeon clamps to 0..7.
    form.move_field(-5);
    form.step(1);
    assert_eq!(form.kind(), Where::Dungeon);
    assert_eq!(form.target().unwrap().x, 7);
    form.move_field(2);
    for _ in 0..12 {
        form.step(1);
    }
    assert!(matches!(
        form.area(),
        Some(TeleportArea::Dungeon { level: 7, .. })
    ));
    form.move_field(-5);
    form.step(1);
    assert_eq!(form.kind(), Where::Britannia);
    assert_eq!(form.field(), TeleportField::Where);
    form.move_field(1);
    assert_eq!(form.field(), TeleportField::X, "no place field on a plane");
    for _ in 0..40 {
        form.step(8);
    }
    assert_eq!(form.target().unwrap().x, 255);
}

#[test]
fn doom_and_other_locations_warn_of_skipped_story_steps() {
    let mut form = form();
    assert!(!form.warns(), "the location the party stands in");
    form.move_field(1);
    form.step(1);
    assert!(form.warns(), "another location");
    form.move_field(-1);
    form.step(1);
    assert_eq!(form.kind(), Where::Dungeon);
    assert!(!form.warns(), "Deceit");
    form.move_field(1);
    form.step(-1);
    assert_eq!(
        form.area()
            .map(|a| matches!(a, TeleportArea::Dungeon { scene, .. } if scene.byte == 0x28)),
        Some(true)
    );
    assert!(form.warns(), "Doom");
    form.move_field(-1);
    form.step(1);
    assert!(!form.warns(), "Britannia");
}

#[test]
fn the_face_says_spoilers_and_its_settings_row_does_too() {
    assert!(TELEPORT_TITLE.contains("SPOILERS"));
    assert!(
        Toggle::CheatsTeleport.spec().label.contains("Spoiler"),
        "the settings row names the spoiler"
    );
    let state = test_state(open_grid(), 5, 5);
    let runs = form().runs(&state);
    let texts: Vec<&str> = runs.iter().map(|r| r.text.as_str()).collect();
    assert_eq!(&texts[..2], &SPOILER_LINES[..]);
    assert!(texts.contains(&"(*) Location"));
    assert!(texts.contains(&"Lord British's Castle"));
    assert!(texts.contains(&"(of -1 0 +1)"));
    // Inside the box: columns 0..37 of the 38-cell box.
    for run in &runs {
        assert!(run.col + run.text.len() as u32 <= 37, "{run:?}");
    }
}

#[test]
fn a_load_failure_is_shown_as_its_line() {
    let refusal = TeleportRefusal::LoadFailed("missing".into());
    assert_eq!(refusal_text(&refusal), LOAD_FAILED);
    // No game files: the plan cannot load and Enter only says so.
    let state = test_state(open_grid(), 5, 5);
    let mut form = TeleportForm::new(fake_places(), &state);
    let none = std::path::Path::new("/nonexistent-u5-game");
    // Another area: Britannia, two places left of Location.
    form.step(-1);
    form.step(-1);
    assert_eq!(form.kind(), Where::Britannia);
    assert_eq!(
        teleport_key(&mut form, KeyCode::Enter, &state, none),
        TeleportOutcome::Say(LOAD_FAILED.to_string())
    );
}
