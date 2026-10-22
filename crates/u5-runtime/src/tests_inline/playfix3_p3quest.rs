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
