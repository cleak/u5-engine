// Regressions for the Look/command exploratory-playtest findings (p5look,
// J02-J29).
//
// Each test names the published sentence it pins and the playtest id whose
// reproduction it captures.

/// `view.md §3` rows 5a-5c: each appender row prints the base `LOOK2.DAT`
/// description and then "Append[s]" the time, virtue word or dungeon name.
/// The shipped base strings for these rows carry their own trailing space,
/// so the appender adds no separator of its own, and the clock time is "a
/// colon, two-digit minutes, then an AM suffix" with no brackets (J02/J07).
#[test]
fn playtest_j02_look_appenders_add_no_separator_or_brackets() {
    let table = parse_look2_dat(&look2_bytes(&[
        (0xfa, "a grandfather clock, showing: "),
        (ETERNAL_FLAME_LOOK_TILE as usize, "the Flame of "),
        (
            WORD_OF_POWER_SEALED_TILE as usize,
            "the collapsed entrance to the dungeon ",
        ),
    ]))
    .unwrap();
    let mut state = test_state(open_grid(), 1, 1);

    state.clock = GameClock::new(15, 7).unwrap();
    assert_eq!(
        state.look_description(0xfa, Some(&table), 0),
        "a grandfather clock, showing: 3:07 PM"
    );
    assert_eq!(
        state.look_description(WORD_OF_POWER_SEALED_TILE, Some(&table), 240),
        "the collapsed entrance to the dungeon Deceit"
    );

    state.area = Area::Town {
        scene: Scene::new(SCENE_THE_LYCAEUM).unwrap(),
        floor: 2,
    };
    assert_eq!(
        state.look_description(ETERNAL_FLAME_LOOK_TILE, Some(&table), 15),
        "the Flame of Truth"
    );
}
