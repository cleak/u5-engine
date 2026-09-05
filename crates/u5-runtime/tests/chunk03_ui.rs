//! Batch `chunk03-ui` regression pins.
//!
//! Every assertion here traces to published spec text quoted at the test.

use u5_runtime::test_fixtures::{open_grid, test_state};
use u5_runtime::*;

/// `inventory.md §4.6`: "The stored literals are the bare words with their
/// punctuation - `Select:`, `Items:`, `Reagents`, `Spells`, `Armaments` [...]
/// When neither a picker nor a member selection is active, the panel's top
/// border carries no label."
///
/// `inventory.md §4.7` assigns those literals to the six Z-stats pages, giving
/// the attribute page and the equipment page no border label at all. The
/// runtime's border-label slot is [`PlayState::roster_box_label`], which the
/// stats-panel chrome reads; before this batch it never consulted the live
/// Z-stats page and returned `None` for all six.
#[test]
fn z_stats_pages_paint_their_published_border_labels() {
    let mut state = test_state(open_grid(), 5, 5);
    assert_eq!(state.roster_box_label(), None);

    assert_eq!(state.z_stats(), MoveOutcome::Observed);

    // Runtime observation: the two character-specific pages have no
    // *page* label in `§4.7`'s table, and a capture of the original
    // shows the selected member's own name framed there instead.
    let member = state.party_member_display_name(0);
    let expected = [
        (ZStatsPage::Stats, Some(member.as_str())),
        (ZStatsPage::Equipment, Some(member.as_str())),
        // The counters screen and its `Equipment` label, and a bare
        // `Items` on the items page (`cleak/u5-spec#202`).
        (ZStatsPage::Counters, Some("Equipment")),
        (ZStatsPage::Reagents, Some("Reagents")),
        (ZStatsPage::Spells, Some("Spells")),
        (ZStatsPage::SpecialUse, Some("Items")),
        (ZStatsPage::EquipmentStock, Some("Armaments")),
    ];

    for (index, (page, label)) in expected.into_iter().enumerate() {
        if index > 0 {
            assert!(state.step_active_z_stats('>', ""));
        }
        assert_eq!(
            state.active_z_stats.as_ref().map(|session| session.page),
            Some(page),
            "page sequence diverged at step {index}"
        );
        assert_eq!(
            state.roster_box_label().as_deref(),
            label,
            "border label wrong on {page:?}"
        );
    }

    // Escape closes the browser; with no picker and no member selection live
    // the border carries no label again.
    assert!(state.step_active_z_stats('\u{1b}', ""));
    assert!(state.active_z_stats.is_none());
    assert_eq!(state.roster_box_label(), None);
}

/// `inventory.md §4.7`: the two parenthesised empty-state placeholders are
/// `(None ready)` - "The equipment list has nothing readied." - and
/// `(None owned!)` - "An inventory page has no slot with a non-zero count."
/// "When no displayable row exists, the panel prints the none placeholder and
/// waits for a key before returning to the page loop."
///
/// The engine printed the invented `Nothing equipped.` and `None.` instead.
#[test]
fn empty_z_stats_pages_print_the_published_placeholders() {
    let mut state = test_state(open_grid(), 5, 5);
    for equipment in state.party_equipment.iter_mut() {
        *equipment = [EQUIPMENT_EMPTY; EQUIPMENT_SLOT_COUNT];
    }
    state.keys = 0;
    state.gems = 0;
    state.torches = 0;
    state.climbing_gear = 0;
    state.special_items = Default::default();
    state.scroll_stock = Default::default();
    state.potion_stock = Default::default();

    assert_eq!(state.z_stats(), MoveOutcome::Observed);

    // Page 2 is the equipment page.
    assert!(state.step_active_z_stats('>', ""));
    assert_eq!(
        state.active_z_stats.as_ref().map(|session| session.page),
        Some(ZStatsPage::Equipment)
    );
    // `inventory.md §4.7`: the page body is drawn over the panel.
    let panel = z_stats_panel_text(&state);
    assert!(
        panel.contains(Z_STATS_NONE_READY_PLACEHOLDER),
        "equipment page panel was {panel:?}"
    );
    assert!(!panel.contains("Nothing equipped."));

    // The Items page is the only zero-filtered inventory page whose scanner
    // can run out of displayable rows in this fixture.
    // Arms -> counters -> reagents -> spells -> items.
    for _ in 0..4 {
        assert!(state.step_active_z_stats('>', ""));
    }
    assert_eq!(
        state.active_z_stats.as_ref().map(|session| session.page),
        Some(ZStatsPage::SpecialUse)
    );
    // The items page always lists the eight Moonstones
    // (`inventory.md §4.5`, `cleak/u5-spec#202`), so it is the one
    // inventory page that never reaches the placeholder. Step once more
    // to the armaments page, which does.
    let panel = z_stats_panel_text(&state);
    assert!(
        panel.contains(u5_runtime::Z_STATS_MOONSTONE_LABEL),
        "items page panel was {panel:?}"
    );
    assert!(!panel.contains(Z_STATS_NONE_OWNED_PLACEHOLDER));

    assert!(state.step_active_z_stats('>', ""));
    assert_eq!(
        state.active_z_stats.as_ref().map(|session| session.page),
        Some(ZStatsPage::EquipmentStock)
    );
    let panel = z_stats_panel_text(&state);
    assert!(
        panel.contains(Z_STATS_NONE_OWNED_PLACEHOLDER),
        "armaments page panel was {panel:?}"
    );
    assert!(!panel.contains("None."));
}

/// `inventory.md §4.7`: a live Z-stats page draws its body over the stats
/// panel, so page content is read from there, not from the message window.
fn z_stats_panel_text(state: &PlayState) -> String {
    let session = state
        .active_z_stats
        .clone()
        .expect("a live Z-stats page to read");
    state.z_stats_panel_rows(&session).join("|")
}

/// `magic.md §5` Step 2: "The echo shown while typing is friendlier than the
/// stored token: each letter prints its associated rune word followed by a
/// space, but that echo is not a long-form input alias."
///
/// `magic.md §3` publishes the twenty-four-syllable vocabulary; the selector
/// keying is each syllable's own initial, which is why `§5` says "`J` and `O`
/// are ignored because no rune selector is keyed by those letters".
#[test]
fn cast_prompt_echoes_rune_words_but_still_parses_selectors() {
    let mut state = test_state(open_grid(), 5, 5);
    state.active_player = Some(0);
    assert_eq!(state.start_cast_spell_prompt(), MoveOutcome::Observed);

    // `PRV` for *Vas Rel Por* (Gate Travel) is one of the spec's own examples,
    // and it also shows that the echo follows the typed order rather than the
    // sorted token: "the parser sorts the typed letters before lookup".
    assert!(
        state
            .step_active_cast('P', "RV", std::path::Path::new(""))
            .unwrap()
            .is_none()
    );
    assert!(
        state.message.contains("Spell name:\n:Por Rel Vas "),
        "cast echo was {:?}",
        state.message
    );

    // The echo is presentation only: the parse path still holds the compact
    // letter-coded form.
    assert_eq!(
        state
            .active_cast
            .as_ref()
            .map(|session| session.buffer.clone()),
        Some("PRV".to_string())
    );
}
