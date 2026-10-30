// Regressions for the pass-20 exploratory-playtest findings (misc20, P20-xx).
//
// Each test names the published sentence it pins and the playtest id whose
// reproduction it captures.

/// P20-01: `combat.md §11` (u5-spec#308 Q3): "The combat field-spell
/// placement maps Poison Field to `0xE8`, Sleep Field to `0xE9`, Energy Field
/// to `0xEB`, and Fire Field ... to `0xEA`". `tile-catalog.md` rows 488-491
/// are those ids' field art; the `0x33..0x36` dispatch byte drew as the
/// seated-figure actor row instead.
#[test]
fn p20_cast_field_marker_carries_the_e8_field_id() {
    let cases = [
        (CombatArenaFieldKind::Poison, 0xE8u8),
        (CombatArenaFieldKind::Sleep, 0xE9),
        (CombatArenaFieldKind::Fire, 0xEA),
        (CombatArenaFieldKind::Energy, 0xEB),
    ];
    for (field, marker_id) in cases {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.combat_active = true;
        state
            .active_objects
            .resize(COMBAT_ACTOR_SLOTS, ActiveObject::empty());

        let placed = state
            .apply_combat_arena_field_placement(field, 3, 4, true)
            .expect("the field is placed");
        let object = state.active_objects[placed.active_object_slot];
        assert_eq!(object.type_byte, marker_id, "{field:?}");
        assert_eq!(object.tile, marker_id, "{field:?}");
        assert_eq!((object.x, object.y), (3, 4));
        assert_eq!(
            CombatArenaFieldKind::from_kind_byte(object.type_byte),
            Some(field),
            "the placed marker still reads back as its field"
        );
    }
}

/// P20-04: `vehicles.md §5`: a carpet's "X-it requires either a passable tile
/// under the party or at least one nearby landing-support cell", a support
/// cell being one whose "visible terrain byte ... is passable for an on-foot
/// avatar". Molten lava `0x8F` is foot-passable (`tile-catalog.md §5/§6`);
/// its burning is an underfoot reaction, not a passability rule.
#[test]
fn p20_carpet_xit_lands_on_foot_passable_lava() {
    let mut state = world_state(vec![0x8F; WORLD_CELLS], 5, 5);
    let carpet = TransportState::Carpet {
        type_byte: 184,
        tile: 184,
    };
    state.player.transport = carpet;
    state.sync_player_object();
    let game_dir = Path::new("");

    assert!(state
        .vehicle_exit_foot_passable_at(Some(game_dir), 5, 5)
        .unwrap());
    assert!(state
        .vehicle_exit_has_nearby_support(Some(game_dir))
        .unwrap());

    crate::input_dispatch::handle_play_key_input(&mut state, 'X', "", game_dir).unwrap();

    assert_eq!(state.player.transport, TransportState::Foot);
    assert!(
        !state.message.contains("No land nearby!"),
        "{:?}",
        state.message
    );
}
