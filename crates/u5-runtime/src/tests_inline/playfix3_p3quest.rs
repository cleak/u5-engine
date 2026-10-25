// Regressions for the quest exploratory-playtest findings (G29-G32).
//
// Each test names the published sentence it pins and the playtest id whose
// reproduction it captures.

/// `catalogs/quest-graph.md §5`: the shards and the Amulet are placed "by
/// the outdoor setup pass that runs whenever the party is on a non-surface
/// outdoor plane". A Gate Travel warp to an Underworld moonstone slot is a
/// normal scene entry (`overworld.md §9`), so it places them too (G29).
#[test]
fn playtest_g29_gate_travel_into_the_underworld_places_the_shards_and_amulet() {
    let dir = debug_game_dir();
    let mut state = britannia_state(open_world_grid(), 5, 5);
    state.special_items = [0; SPECIAL_ITEM_COUNT];
    state.shadowlord_hideouts = [0; SHADOWLORD_COUNT];
    state.spell_charges[GATE_TRAVEL_SPELL_INDEX] = 1;
    state.party[0].mana = 9;
    state.party[0].level = 8;
    state.moonstone_slots[1] = MoonstoneGateSlot {
        scene: 0,
        x: 192,
        y: 81,
        z: 0xFF,
    };

    handle_play_key_input(&mut state, 'C', "1PRV2", &dir).unwrap();

    assert_eq!(
        state.area,
        Area::World {
            plane: WorldPlane::Underworld
        }
    );
    let at = |x: usize, y: usize| {
        state
            .active_objects
            .iter()
            .find(|object| object.x == x && object.y == y && !object.is_empty())
            .copied()
    };
    let shard = at(192, 80).expect("Shard of Falsehood placed at (192,80)");
    assert_eq!(shard.type_byte, INVENTORY_ADD_CLASS_SHADOWLORD_SHARD);
    assert!(at(130, 65).is_some());
    assert!(at(176, 184).is_some());
    let amulet = at(105, 225).expect("Amulet placed at (105,225)");
    assert_eq!(amulet.type_byte, INVENTORY_ADD_CLASS_AMULET_LORD_BRITISH);
    let _ = fs::remove_dir_all(dir);
}

/// `commands.md §11.1` step 1 counts a sealed `0xDF` neighbour as
/// qualifying, and step 3 says a valid word spoken at the wrong coordinate
/// "still produces the utterance feedback and presentation effect, but
/// changes nothing" - with no further no-effect line, which step 1 adds only
/// when no neighbour qualifies (G30).
#[test]
fn playtest_g30_word_of_power_beside_another_seal_prints_no_no_effect_line() {
    let mut world = world_state(open_world_grid(), 128, 128);
    world.area = Area::World {
        plane: WorldPlane::Underworld,
    };
    // The sealed Doom mouth sits east of the party; FALLAX belongs to Deceit.
    let idx = world_cell_index(129, 128);
    world.grid[idx] = WORD_OF_POWER_SEALED_TILE;
    world.refresh_world_live_chunks_for_current_area().unwrap();

    assert_eq!(
        handle_play_key_input(&mut world, 'Y', "FALLAX", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );

    assert_eq!(world.grid[idx], WORD_OF_POWER_SEALED_TILE);
    assert_eq!(world.word_of_power_seal_flags[0], 0);
    assert_eq!(
        world.message,
        crate::commands::WORD_OF_POWER_UTTERED_MESSAGE
    );
}

/// `town-mode.md §13`: "After the location is drawn, the same presentation
/// helper walks the three Shadowlord runtime slots and, for every Shadowlord
/// that is still alive, prints the corresponding \"air of\" virtue-opposition
/// line". An overworld `E`-Enter replaced the transcript the load-time pass
/// had printed into, so the rows never reached the window (G31).
#[test]
fn playtest_g31_entering_stonegate_prints_the_air_rows_for_living_shadowlords() {
    let dir = debug_game_dir();
    let scene = Scene::new(STONEGATE_SCENE_BYTE).unwrap();
    fs::write(dir.join("KEEP.DAT"), location_pages()).unwrap();
    fs::write(dir.join("KEEP.NPC"), vec![0; (scene.block + 1) * 576]).unwrap();
    fs::write(dir.join("KEEP.TLK"), [0, 0]).unwrap();
    fs::write(
        dir.join(WORLD_LOCATION_TABLE_FILE),
        "BRITANNIA 10 20 KEEP:4 7 0x12 KEEP\n",
    )
    .unwrap();
    let mut grid = open_world_grid();
    grid[world_cell_index(10, 20)] = 0x12;
    let mut state = britannia_state(grid, 10, 20);
    // Falsehood and cowardice alive, hatred vanquished.
    state.shadowlord_hideouts = [0, SHADOWLORD_VANQUISHED, 3];

    assert_eq!(
        state.enter_current_location(&dir).unwrap(),
        MoveOutcome::Transition(AreaTransition::EnteredLocation(scene))
    );

    let rows: Vec<&str> = state
        .message_entries()
        .iter()
        .map(|entry| entry.text.as_str())
        .collect();
    let keep = rows
        .iter()
        .rposition(|row| *row == "keep")
        .expect("entry narration row");
    assert_eq!(
        &rows[keep + 1..],
        &[
            "",
            "An air of",
            "cowardice doth",
            "surround thee...",
            "",
            "An air of",
            "falsehood doth",
            "surround thee...",
        ]
    );
    assert!(state.message_row_open_mid_line);
    assert!(!state.message.contains("hatred"));
    let _ = fs::remove_dir_all(dir);
}
