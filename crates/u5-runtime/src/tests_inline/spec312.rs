// u5-spec#312 Q2: underfoot effects by transport (`systems/vehicles.md` §6,
// "Underfoot Effects By Transport"; `systems/overworld.md` §5;
// `systems/movement.md` §8.2; `systems/town-mode.md` §3). Q1 and Q3 are
// pinned beside the tests they amend (`playfix20_rows20.rs`,
// `playfix12_item12.rs`).

fn spec312_carpet() -> TransportState {
    TransportState::Carpet {
        type_byte: CARPET_MOUNTED,
        tile: CARPET_MOUNTED,
    }
}

fn spec312_horse() -> TransportState {
    TransportState::Horse {
        type_byte: HORSE_MOUNTED_FIRST,
        tile: HORSE_MOUNTED_FIRST,
    }
}

/// One world Pass on `tile` under `transport`; returns the party's message
/// rows and whether any member lost HP or became Poisoned.
fn spec312_world_pass(tile: u8, transport: TransportState) -> (Vec<String>, bool, bool) {
    let dir = debug_game_dir();
    let mut grid = open_world_grid();
    grid[world_cell_index(5, 5)] = tile;
    let mut state = world_state(grid, 5, 5);
    for member in &mut state.party {
        member.hp = 200;
        member.max_hp = 200;
        // Dexterity zero: the swamp save's `1..30` draw always fails.
        member.climb_stat = 0;
    }
    state.player.transport = transport;
    state.sync_player_object();
    handle_play_key_input(&mut state, ' ', "", &dir).unwrap();
    state.flush_message_slot();
    let rows = combat2b_rows_text(&state);
    let hurt = state.party.iter().any(|member| member.hp < 200);
    let poisoned = state.party.iter().any(|member| member.status == b'P');
    let _ = fs::remove_dir_all(dir);
    (rows, hurt, poisoned)
}

/// `overworld.md §5` (issue #312 Q2): "A party on the magic carpet, which
/// accepts lava, burns on every such turn exactly like a party on foot", and
/// a horse placed there burns too - the arm "tests only the tile, never the
/// transport". The horse case used to be exempt here.
#[test]
fn spec312_q2_world_lava_burns_a_carpet_and_a_horse_party() {
    for (label, transport) in [
        ("foot", TransportState::Foot),
        ("carpet", spec312_carpet()),
        ("horse", spec312_horse()),
    ] {
        let (rows, hurt, _) = spec312_world_pass(0x8f, transport);
        assert!(
            rows.iter().any(|row| row == WORLD_BURNING_LINE),
            "{label}: {rows:?}"
        );
        assert!(hurt, "{label}: the 1..8 burn lands");
    }
}

/// `movement.md §8.2` (issue #312 Q2): "Only a party exactly on foot enters
/// this poisoning check; any other transport marker skips it, drawing
/// nothing and printing nothing. The magic carpet accepts swamp and flies
/// over it, and it is still never poisoned there." The on-foot control on
/// the same cell is poisoned.
#[test]
fn spec312_q2_world_swamp_poisons_only_a_party_on_foot() {
    let (rows, _, poisoned) = spec312_world_pass(BRIT_SWAMP_TILE, TransportState::Foot);
    assert!(poisoned, "foot control: {rows:?}");
    assert!(rows.iter().any(|row| row == "Poisoned!"), "{rows:?}");
    for (label, transport) in [("carpet", spec312_carpet()), ("horse", spec312_horse())] {
        let (rows, _, poisoned) = spec312_world_pass(BRIT_SWAMP_TILE, transport);
        assert!(!poisoned, "{label}: {rows:?}");
        assert!(!rows.iter().any(|row| row == "Poisoned!"), "{label}: {rows:?}");
    }
}

/// `town-mode.md §3` (issue #312 Q2): "The exemption belongs to the carpet
/// alone: a party on a horse, which can enter the cell, takes the line, the
/// damage and the descent like a party on foot. The carpet test comes
/// before the Stonegate test". The carpet side is pinned by
/// `magic_carpet_suppresses_native_stonegate_trapdoor`; a horse over
/// Stonegate's ring runs the scripted death.
#[test]
fn spec312_q2_a_horse_party_takes_the_stonegate_trapdoor() {
    let dir = debug_game_dir();
    let scene = Scene::new(STONEGATE_SCENE_BYTE).unwrap();
    let mut grid = open_grid();
    grid[32 + 1] = TOWN_TRAPDOOR_LIVE_TILE;
    let mut state = test_state(grid, 1, 1);
    state.area = Area::Town { scene, floor: 0 };
    state.player.transport = spec312_horse();
    state.sync_player_object();
    let _ = state.pass_turn_with_game_dir(Some(&dir)).unwrap();
    assert!(
        state.pending_stonegate_trapdoor_playback.is_some(),
        "the horse does not float over the ring"
    );
    let _ = fs::remove_dir_all(dir);
}

/// `town-mode.md §3` / `vehicles.md §6` (issue #312 Q2): "the gas save
/// requires the party to be exactly on foot", so a carpet or horse party on
/// a town gas cell draws nothing and prints nothing.
#[test]
fn spec312_q2_town_gas_poisons_only_a_party_on_foot() {
    let gas = |transport: TransportState| {
        let mut state = test_state(open_grid(), 1, 1);
        for member in &mut state.party {
            member.climb_stat = 0;
        }
        state.player.transport = transport;
        state.sync_player_object();
        let Area::Town { scene, floor } = state.area else {
            panic!("test_state is a town");
        };
        let tile = TOWN_POISON_GAS_LIVE_TILE;
        state
            .town_poison_gas_at(Path::new(""), scene, floor, 1, 1, tile)
            .unwrap()
            .is_some()
    };
    assert!(gas(TransportState::Foot), "foot control");
    assert!(!gas(spec312_carpet()), "carpet");
    assert!(!gas(spec312_horse()), "horse");
}
