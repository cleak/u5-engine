// rc14: the cleak/u5-spec#302 answers the pass-11 branches left open (Q2 and
// Q3), and the inline `C` intercept's dungeon loop-head minute.

/// A one-member party of `class_byte` at level 2, for the attribute page.
fn rc14_z_stats_level_row(class_byte: u8) -> String {
    let mut state = test_state(open_grid(), 5, 5);
    state.party = vec![PartyMember {
        slot: 0,
        class_byte,
        status: b'G',
        climb_stat: 11,
        mana: 7,
        hp: 5,
        max_hp: 300,
        level: 2,
    }];
    state.z_stats_panel_rows(&ZStatsSession::new(0))[0].clone()
}

/// `inventory.md §4.7` (issue #302 Q2, `RETRACTIONS.md` R622): "The
/// level-line indent, in spaces, by class: Avatar 1, Mage 2, Bard 2,
/// Fighter 1, Druid 1, Tinker 1, Paladin 1, Ranger 1, Shepherd 0. It comes
/// from a per-class table in the shipped data, not from the name length."
/// Centring the line gave the twelve-cell Druid line two spaces.
#[test]
fn spec302_q2_level_line_takes_the_per_class_indent() {
    let glyph = char::from(SAVE_GENDER_MALE_BYTE);
    for (class_byte, indent, name) in [
        (b'A', 1, "Avatar"),
        (b'M', 2, "Mage"),
        (b'B', 2, "Bard"),
        (b'F', 1, "Fighter"),
        (b'D', 1, "Druid"),
        (b'T', 1, "Tinker"),
        (b'P', 1, "Paladin"),
        (b'R', 1, "Ranger"),
        (b'S', 0, "Shepherd"),
    ] {
        assert_eq!(
            rc14_z_stats_level_row(class_byte),
            format!("{}{glyph} Lv-2 {name}", " ".repeat(indent)),
            "class {}",
            char::from(class_byte)
        );
    }
}

/// `inventory.md §4.7` attribute-page rows (issue #302 Q2, R622): row 2 is
/// the status name alone, centred in columns 24 to 38, with no `Status:`
/// label; `__HP:` is current and `__HM:` maximum hit points in four cells;
/// `____Magic:` is followed by current magic points in two cells. The
/// executed example: `Str=16  HP: 240` / `Int=12  HM: 300` / `Dex=15  Ex: 123`
/// / blank / `    Magic: 7` (settles pass-10 R03 and pass-11 S04).
#[test]
fn spec302_q2_attribute_page_rows_match_the_executed_example() {
    let mut state = test_state(open_grid(), 5, 5);
    state.party = vec![PartyMember {
        slot: 0,
        class_byte: b'A',
        status: b'G',
        climb_stat: 15,
        mana: 7,
        hp: 240,
        max_hp: 300,
        level: 2,
    }];
    state.party_strengths = vec![16];
    state.party_intelligence = vec![12];
    state.party_experience = vec![123];
    let rows = state.z_stats_panel_rows(&ZStatsSession::new(0));
    assert_eq!(rows[1], "  Good Health");
    assert!(!rows.iter().any(|row| row.contains("Status")));
    assert_eq!(rows[2], "");
    assert_eq!(rows[3], "Str=16  HP: 240");
    assert_eq!(rows[4], "Int=12  HM: 300");
    assert_eq!(rows[5], "Dex=15  Ex: 123");
    assert_eq!(rows[6], "");
    assert_eq!(rows[7], "    Magic: 7");
    state.party[0].mana = 25;
    let rows = state.z_stats_panel_rows(&ZStatsSession::new(0));
    assert_eq!(rows[7], "    Magic:25");
}
