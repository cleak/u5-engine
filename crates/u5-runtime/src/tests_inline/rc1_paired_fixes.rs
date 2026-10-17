// Regressions for the rc1 full paired sweep of 2026-10-17. Each test names
// the stock capture it reproduces and the published sentence it rests on.

fn blackthorn_audience_fixture_map() -> MiscmapsCutsceneMap {
    let mut tiles = vec![0x44; MISCMAPS_CUTSCENE_ROWS * MISCMAPS_CUTSCENE_VISIBLE_COLUMNS];
    // The audience grid's manacles under the two party actors' start cells.
    for (x, y) in [(3usize, 5usize), (7, 5), (0, 1)] {
        tiles[y * MISCMAPS_CUTSCENE_VISIBLE_COLUMNS + x] = 0x85;
    }
    MiscmapsCutsceneMap {
        record_index: BLACKTHORN_AUDIENCE_CUTSCENE_MAP_RECORD,
        tiles,
    }
}

/// `bt-correct` (artifact bt-correct-20261017-004820): on every audience
/// beat the stock game draws the Avatar at `(3,5)` and the second member at
/// `(7,5)` as the manacled prisoner. `blackthorn.md §6.3` puts slot 0 at
/// `(3,5)` and §5 the second-member actor at `(7,5)`; both cells are the
/// manacles `0x85`, and `visibility.md §8` stamps "one of `0x64..0x67`" for a
/// terrain-aware actor there. The world's slot-zero refresh must not drag
/// the Avatar actor off the cutscene grid to the party's town position.
#[test]
fn rc1_blackthorn_audience_party_actors_draw_as_manacled_prisoners() {
    let mut state = test_state(open_grid(), 12, 20);
    state.party.push(audit_g5_companion(1));
    state.party.push(audit_g5_companion(2));
    state.blackthorn_audience_map = Some(blackthorn_audience_fixture_map());
    state.install_blackthorn_audience_actors();

    state.sync_player_object();
    assert_eq!(
        (state.active_objects[0].x, state.active_objects[0].y),
        (3, 5),
        "the cutscene owns slot zero while the audience runs"
    );

    let prisoner = 0x164..=0x167usize;
    for slot in [0usize, 1] {
        let tile = state.blackthorn_audience_actor_atlas_tile(slot).unwrap();
        assert!(prisoner.contains(&tile), "slot {slot} drew {tile:#x}");
    }
    // A guard on cobble is stamped unchanged.
    assert_eq!(
        state.blackthorn_audience_actor_atlas_tile(6),
        Some(0x100 + usize::from(BLACKTHORN_GUARD_ACTOR_BYTE))
    );
    // The suppressed tableau draws nothing.
    assert_eq!(state.blackthorn_audience_actor_atlas_tile(8), None);

    // `visibility.md §8.1`: one fresh draw per selecting actor per pass,
    // and the frame follows the drawn variant.
    state.blackthorn_audience_actor_variants = [0; 9];
    let mut seen = std::collections::BTreeSet::new();
    for _ in 0..64 {
        state.run_blackthorn_audience_composite_pass();
        seen.insert(state.blackthorn_audience_actor_atlas_tile(0).unwrap());
    }
    assert!(seen.len() > 1, "the prisoner re-rolls its frame: {seen:?}");
    assert!(seen.iter().all(|tile| prisoner.contains(tile)));
}

/// Outside the audience the slot-zero refresh still runs.
#[test]
fn rc1_slot_zero_refresh_still_runs_outside_the_audience() {
    let mut state = test_state(open_grid(), 12, 20);
    state.party.push(audit_g5_companion(1));
    state.party.push(audit_g5_companion(2));
    state.install_blackthorn_audience_actors();
    state.sync_player_object();
    assert_eq!(
        (state.active_objects[0].x, state.active_objects[0].y),
        (12, 20)
    );
}

/// Play-test P22, `shops.md §8.C`: the fee question `Wilt thou\npay?" `
/// "accept[s] Y/N only and echo[es] bare `Yes` or `No`" - the declined `No`
/// lands after `pay?" ` on the prompt's own row, like `Yes`, not on a row of
/// its own; the continuation question follows.
#[test]
fn rc1_healer_declined_fee_echoes_no_on_the_prompt_row() {
    use crate::shop_runtime::*;
    use crate::shop_session::ActiveShopSession;
    use crate::shops::Healer;

    let mut state = test_state(open_grid(), 1, 1);
    state.active_shop = Some(ActiveShopSession::Healer(
        HealerShopState::Confirm {
            service: HealerService::Cure,
            slot: 0,
            cost: 10,
        },
        Healer::TheHealersMission,
    ));
    state.emit_message_line("\n\"I can cure thy poisoned body for 10 gold.\n\nWilt thou\npay?\" ");
    state.message_row_open_mid_line = true;
    handle_play_key_input(&mut state, 'N', "", Path::new("")).unwrap();
    state.flush_message_slot();
    let rows: Vec<&str> = state
        .message_transcript
        .iter()
        .map(|entry| entry.text.as_str())
        .collect();
    let at = rows
        .iter()
        .position(|row| *row == "pay?\" No")
        .unwrap_or_else(|| panic!("`No` is not on the prompt row: {rows:#?}"));
    assert!(!rows.contains(&"No"), "{rows:#?}");
    assert_eq!(
        rows[at + 1],
        "",
        "one blank row before the continuation: {rows:#?}"
    );
    assert_eq!(
        rows[at + 2],
        "\"Is there any other way in which I may",
        "{rows:#?}"
    );
    assert!(
        state.message.ends_with("aid thee?\" "),
        "{:?}",
        state.message
    );
}

/// `audio.md §8.9`, drowning: `Ship sunk!`, the panel refresh, **then** the
/// long descent, then `DROWNING!!!`, then the death loop, whose passes each
/// play their own damage rumble. Stock capture (`sail-heading-echo`, beat
/// `sailing`, 2026-10-17): about four seconds after the sinking the window
/// still ends on `Ship sunk!` with the whole party listed, and the hit points
/// fall over the following samples. The engine printed `DROWNING!!!` and the
/// dead party at once.
#[test]
fn rc1_drowning_holds_ship_sunk_through_the_descent_then_paces_the_loop() {
    let mut state = world_state(open_world_grid(), 5, 5);
    state.party = six_member_party(40);
    state.special_items[SPECIAL_ITEM_MAGIC_CARPET_INDEX] = 0;
    aboard_frigate(&mut state, 1, 0);
    let full_row = crate::stats_panel::stats_panel_snapshot_of(&state).rows[0].clone();
    let OutdoorImpactAbsorption::ShipDestroyed { drowning, .. } = state.apply_outdoor_impact()
    else {
        panic!("hull one is always destroyed");
    };
    assert!(drowning.len() > 1, "a hardy party survives several passes");
    let last_row = |state: &PlayState| {
        state
            .message_entries()
            .last()
            .map(|entry| entry.text.clone())
            .unwrap_or_default()
    };
    // Through the descent: `Ship sunk!` is the last row and the panel still
    // shows the party whole, although the live roster is already dead.
    let descent = SoundEffect::LongDescent
        .program(&mut crate::audio::RumbleJitter::default())
        .duration(true)
        .as_secs_f64();
    assert!(descent > 6.0);
    assert!(state.advance_staged_narration_to(100.0));
    assert!(state.advance_staged_narration_to(100.0 + descent - 0.1));
    assert_eq!(last_row(&state), SHIP_SUNK_MESSAGE);
    assert_eq!(state.displayed_stats_panel().rows[0], full_row);
    assert!(state.party.iter().all(|member| member.hp == 0));

    // Then `DROWNING!!!`, and the passes repaint the panel one at a time.
    assert!(state.advance_staged_narration_to(100.0 + descent + 0.01));
    assert_eq!(last_row(&state), DROWNING_MESSAGE);
    let mut shown = vec![state.displayed_stats_panel().rows[0].clone()];
    let mut now = 100.0 + descent + 0.01;
    while state.staged_narration_active() {
        now += 0.05;
        state.advance_staged_narration_to(now);
        let row = state.displayed_stats_panel().rows[0].clone();
        if shown.last() != Some(&row) {
            shown.push(row);
        }
    }
    assert!(
        shown.len() > 2,
        "the hit points fall pass by pass: {shown:?}"
    );
    assert!(now - (100.0 + descent) > 0.5, "the loop takes time");
    assert_eq!(state.displayed_stats_panel(), &state.stats_panel);
}

fn picker_row(quantity: Option<u8>, name: &str) -> crate::stats_panel::PanelPickerRow {
    crate::stats_panel::PanelPickerRow {
        quantity,
        selector: crate::stats_panel::PANEL_PICKER_SELECTOR_BLANK,
        selector_runic: false,
        name: name.to_string(),
        zero_padded: false,
    }
}

fn picker_panel_text(panel: &crate::stats_panel::PickerPanelCells, row: usize) -> String {
    (0..crate::stats_panel::PICKER_PANEL_COLUMNS)
        .map(|column| panel.get(column, row).map_or('.', |cell| cell.code as char))
        .collect()
}

/// `inventory.md §4.4`: the picker repaints its list into the same
/// rectangle without clearing it, so a shorter row, an unpadded wrapped
/// first line and the two cells past a wrap keep the previous pass's text.
/// Stock `use-specials` (2026-10-17) shows exactly this as the highlight
/// moves from the Sceptre to a counted Shard of Falsehood: ` 1 ` over
/// ` 1 Sceptre`, a stray `S` in column 0, `ed` in columns 14-15 and the
/// bottom border gone but for its corner.
#[test]
fn rc1_use_picker_passes_leave_the_previous_pass_as_residue() {
    use crate::stats_panel::{PanelPickerView, PickerPanelCells};
    let rows = vec![
        picker_row(Some(3), "Aa"),
        picker_row(Some(3), "Bb"),
        picker_row(Some(3), "White"),
        picker_row(Some(1), "Magic Crpt"),
        picker_row(Some(1), "Amulet"),
        picker_row(Some(1), "Crown"),
        picker_row(Some(1), "Sceptre"),
        picker_row(Some(1), "Shard/Falsehd"),
        picker_row(Some(1), "Shard/Hatred"),
        picker_row(None, "Spyglass"),
        picker_row(None, "Sextant"),
    ];
    let view = |selected| PanelPickerView {
        label: String::new(),
        rows: rows.clone(),
        selected,
        ornamental_frame: true,
    };
    let mut panel = PickerPanelCells::with_frame(None);
    let mut inverse = false;
    panel.run_pass(&view(6), &mut inverse);
    assert!(!inverse, "a one-line highlight toggles on and off");
    assert_eq!(picker_panel_text(&panel, 6), "Shard/Falsehd \u{17}.");
    assert_eq!(picker_panel_text(&panel, 7), "\u{17} 1 Shard/Hatred");

    panel.run_pass(&view(7), &mut inverse);
    assert!(inverse, "a two-line highlight leaks the inverse bit (§4.4)");
    let row4 = picker_panel_text(&panel, 4);
    assert!(row4.starts_with("\u{17} 1 Sceptre"), "{row4:?}");
    assert!(
        panel.get(1, 4).unwrap().inverse,
        "the highlight's first line"
    );
    // The stale `Sceptre` was the previous pass's highlight and stays inverted.
    assert!(
        panel.get(5, 4).unwrap().inverse,
        "stale pixels keep their style"
    );
    assert_eq!(picker_panel_text(&panel, 6).chars().next(), Some('S'));
    assert_eq!(&picker_panel_text(&panel, 7)[14..], "ed");
    let bottom = picker_panel_text(&panel, 8);
    assert_eq!(
        bottom.as_bytes()[14],
        crate::stats_panel::PANEL_PICKER_FRAME_BOTTOM_RIGHT
    );
    assert!(bottom[..14].chars().all(|ch| ch == ' '), "{bottom:?}");

    // The next session's frame build clears columns 0-14 only.
    let next = PickerPanelCells::with_frame(Some(&panel));
    assert_eq!(next.get(15, 7).map(|cell| cell.code), Some(b'd'));
    assert_eq!(next.get(5, 4), None);
}

/// With the picker's inverse leak set, the divider band's timed-effect tag is
/// drawn dark on a light cell (stock `use-specials`, beat `sceptre`); the
/// cell is reported opaque so a frontend does not let the band show through
/// the glyph. Without the leak, or without a tag, nothing is reported.
#[test]
fn rc1_leaked_inverse_reports_the_effect_tag_cell_opaque() {
    let mut state = test_state(open_grid(), 5, 5);
    state.active_effect_tag = Some(PROTECTION_ACTIVE_EFFECT_TAG);
    state.active_effect_counter = 10;
    state.repaint_stats_panel();
    assert_eq!(
        crate::stats_panel::stats_panel_opaque_effect_cell(&state),
        None
    );
    state.stats_panel_window_inverse = true;
    assert_eq!(
        crate::stats_panel::stats_panel_opaque_effect_cell(&state),
        Some((31, 7))
    );
    // Column-15 residue alone is reported once the picker has closed.
    let mut panel = crate::stats_panel::PickerPanelCells::with_frame(None);
    let view = crate::stats_panel::PanelPickerView {
        label: String::new(),
        rows: vec![picker_row(Some(1), "Shard/Hatred")],
        selected: 0,
        ornamental_frame: true,
    };
    let mut inverse = false;
    panel.run_pass(&view, &mut inverse);
    state.use_picker_panel = Some(panel);
    assert_eq!(
        crate::stats_panel::picker_panel_opaque_cells(&state),
        vec![(39, 2)]
    );
}
