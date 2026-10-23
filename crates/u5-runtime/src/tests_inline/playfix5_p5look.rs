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

/// `formats/signs-dat.md §5`: "The active scene indexes the `SIGNS.DAT`
/// scene directory", that one block is scanned, and "[e]very matching
/// record is printed". Shipped blocks are back to back with no sentinel
/// between them; a scene-k record must still be collected once, not once
/// per non-empty directory slot at or before k (J04).
#[test]
fn playtest_j04_sign_scene_blocks_stop_at_the_next_block() {
    let mut bytes = vec![0u8; SIGNS_DAT_SCENE_DIRECTORY_BYTES];
    let first = SIGNS_DAT_SCENE_DIRECTORY_BYTES as u16;
    let second = first + 4 + 3 + 1;
    let third = second + 4 + 3 + 1;
    bytes[2 * 2..2 * 2 + 2].copy_from_slice(&first.to_le_bytes());
    bytes[3 * 2..3 * 2 + 2].copy_from_slice(&second.to_le_bytes());
    bytes[4 * 2..4 * 2 + 2].copy_from_slice(&third.to_le_bytes());
    bytes.extend_from_slice(&[2, 0, 15, 28]);
    bytes.extend_from_slice(b"Two\0");
    bytes.extend_from_slice(&[3, 0, 15, 28]);
    bytes.extend_from_slice(b"Thr\0");
    bytes.extend_from_slice(&[4, 0, 17, 17]);
    bytes.extend_from_slice(b"Fou\0");
    bytes.push(0);

    let records = parse_sign_records(&bytes).unwrap();
    assert_eq!(records.len(), 3);
    assert_eq!(crate::signs_io::matching_sign_bodies(&records, 3, 0, 28, 15), vec!["Thr"]);
    assert_eq!(crate::signs_io::matching_sign_bodies(&records, 4, 0, 17, 17), vec!["Fou"]);
}
