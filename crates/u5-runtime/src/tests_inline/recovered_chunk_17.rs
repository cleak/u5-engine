// Recovered regression tests. Every assertion below is derived from a
// sentence in the published clean specification, quoted in the doc comment
// above each test. Nothing here is derived by reading what the engine
// currently happens to return.

/// Builds the published Falsehood destruction row: The Lycaeum, floor `2`,
/// party at `(15, 9)`, carrying the Shard of Falsehood, with Faulinei's slot
/// alive.
///
/// `catalogs/quest-graph.md §5` destruction table, row one:
/// "Falsehood / Faulinei | The Lycaeum | 2 | 15 | 9 | `(15, 8)`, same floor |
/// Active Faulinei encounter immediately north".
fn recovered_lycaeum_falsehood_row() -> PlayState {
    let mut state = test_state(open_grid(), 15, 9);
    state.area = Area::Town {
        scene: Scene::new(SCENE_THE_LYCAEUM).unwrap(),
        floor: 2,
    };
    state.special_items[SPECIAL_ITEM_SHARD_FALSEHOOD_INDEX] = 1;
    state.shadowlord_hideouts[SHADOWLORD_FALSEHOOD_INDEX] = 1;
    state
}

/// Places a Shadow Lord actor into the active-object layer at `(x, y)` on the
/// party's current floor.
///
/// `catalogs/quest-graph.md §5` gate 2: "The handler queries the
/// *active-object* layer - not the terrain - at the cell one row north of the
/// party, and requires it to return the Shadow Lord actor tile (`0xFC` ...)."
fn recovered_place_shadowlord_actor(state: &mut PlayState, index: usize, x: usize, y: usize) {
    let z = state.current_floor().unwrap();
    let object = state
        .shadowlord_name_encounter_object(index, x, y, z)
        .unwrap();
    assert_eq!(
        object.type_byte, SHADOWLORD_ACTOR_TILE,
        "quest-graph.md §5 gate 2 names the Shadow Lord actor tile 0xFC"
    );
    state.active_objects.push(object);
}

/// `systems/time.md §7`, "Shadowlord hideout maintenance":
///
/// "For each slot whose high bit is clear, the midnight pass draws a candidate
/// id uniformly from `1..8` inclusive and rejects it when either of these
/// holds, then draws again: the candidate equals the party's current scene
/// byte, or the candidate equals the value currently stored in **any** of the
/// three slots, including the slot being rerolled and any slot already
/// rewritten earlier in the same pass."
///
/// and "The party-scene exclusion only bites when the party is standing inside
/// one of the eight towns at midnight; outdoors, in a dungeon, or in any other
/// interior the party's scene byte is outside `1..8` and the exclusion never
/// fires."
///
/// The town scene bytes are published in the same section: "`1` Moonglow, `2`
/// Britain, `3` Jhelom, `4` Yew, `5` Minoc, `6` Trinsic, `7` Skara Brae, `8`
/// New Magincia."
///
/// **Scope.** The per-slot rejection invariants themselves are already pinned,
/// over sixty-four seeds, by
/// `shadowlord_midnight_reroll_rejects_the_party_scene_and_every_stored_slot`
/// in `tests_inline/spec_conformance_chunk_07.rs`, and the vanquished-slot
/// rule by its sibling `shadowlord_midnight_reroll_skips_only_high_bit_slots`.
/// This test deliberately does **not** restate them. Every one of those tests
/// hands the exclusion in by hand through
/// `reroll_shadowlord_hideouts_excluding`, so what none of them covers is the
/// half this one owns: that the midnight entry point *derives* its exclusion
/// from the party's actual scene, and that the exclusion correctly disappears
/// when the party is not in a town.
#[test]
fn shadowlord_midnight_reroll_derives_its_exclusion_from_the_party_scene() {
    const PASSES: usize = 400;

    // Party standing inside Britain. Nothing tells the walker so except the
    // party's own area, which is the wiring under test.
    let mut state = test_state(open_grid(), 5, 5);
    state.area = Area::Town {
        scene: Scene::new(SCENE_BRITAIN).unwrap(),
        floor: 0,
    };
    state.shadowlord_hideouts = [1, 3, SHADOWLORD_VANQUISHED];

    let mut observed_in_town = std::collections::BTreeSet::new();
    for _ in 0..PASSES {
        state.reroll_shadowlord_hideouts();
        observed_in_town.insert(state.shadowlord_hideouts[SHADOWLORD_FALSEHOOD_INDEX]);
        observed_in_town.insert(state.shadowlord_hideouts[SHADOWLORD_HATRED_INDEX]);
    }

    // Across 800 accepted draws every id in `1..8` except Britain's own scene
    // byte must have been used at least once. That pins the exclusion as
    // removing exactly the party's town - not nothing, and not a wider slice.
    let expected_in_town: std::collections::BTreeSet<u8> = (SHADOWLORD_HIDEOUT_MIN
        ..=SHADOWLORD_HIDEOUT_MAX)
        .filter(|id| *id != SCENE_BRITAIN)
        .collect();
    assert_eq!(
        observed_in_town, expected_in_town,
        "the accepted range is `1..8` minus the party's current scene byte"
    );

    // Outdoors the party's scene byte is outside `1..8`, so "the exclusion
    // never fires" and Britain becomes drawable again.
    let mut outdoors = world_state(open_world_grid(), 40, 40);
    outdoors.shadowlord_hideouts = [1, 3, SHADOWLORD_VANQUISHED];

    let mut observed_outdoors = std::collections::BTreeSet::new();
    for _ in 0..PASSES {
        outdoors.reroll_shadowlord_hideouts();
        observed_outdoors.insert(outdoors.shadowlord_hideouts[SHADOWLORD_FALSEHOOD_INDEX]);
        observed_outdoors.insert(outdoors.shadowlord_hideouts[SHADOWLORD_HATRED_INDEX]);
    }
    let full_range: std::collections::BTreeSet<u8> =
        (SHADOWLORD_HIDEOUT_MIN..=SHADOWLORD_HIDEOUT_MAX).collect();
    assert_eq!(
        observed_outdoors, full_range,
        "with no town scene byte to exclude, all eight ids stay drawable"
    );
}

/// `catalogs/quest-graph.md §5`, "Presentation order":
///
/// Phase 1: "The handler first prints a heading naming the shard family and a
/// line describing the party holding the evil shard aloft, completed by the
/// shard's own virtue word (Falsehood, Hatred, or Cowardice). This happens
/// before any gate is evaluated."
///
/// Phase 3: "Only the **position** gate produces the shared no-effect result.
/// If the party's cell, floor, or scene is wrong, the handler prints that
/// result and returns with no state change."
///
/// Phase 4 must not have run: "Once the position matches, it pauses, prints a
/// line describing the shard being cast into the Eternal Flame completed by the
/// opposed principle's word (Truth, Love, or Courage)".
///
/// And: "a refused attempt (wrong cell, wrong floor, wrong scene, no active
/// Shadowlord, or a mismatched active Shadowlord index) leaves the shard in the
/// party's possession."
///
/// The divergence the spec names explicitly: "evaluating the gates before any
/// output (in the original, a wrong-position shard use still produces the
/// heading and the sound before refusing)."
///
///
/// The turn assertions come from `systems/inventory.md §7`: "The item handler
/// does not decide the turn cost by writing the clock itself" - the outer
/// command layer owns the action cost, so no path through this handler may
/// advance the clock on a refusal.
/// The identity of "the shared no-effect result" is the published `No effect!`
/// literal shared with `systems/magic.md §5` ("a nonempty selector that does not
/// match any of the forty-eight tokens prints `No effect!`") and
/// `catalogs/item-list.md §7` row 7 ("In Stonegate and Doom, prints `No
/// effect!`").
#[test]
fn use_shadowlord_shard_prints_the_prologue_then_the_shared_no_effect_result_off_position() {
    // Each case leaves every other gate satisfiable: the shard is carried,
    // Faulinei's slot is alive, Faulinei is the active named encounter, and a
    // Shadow Lord actor sits one cell north of the party. Only the published
    // position row is wrong, so any refusal is attributable to the position
    // gate alone.
    let cases: [(&str, u8, i8, usize, usize); 3] = [
        // Right scene and floor, one cell south of the published row.
        ("wrong cell", SCENE_THE_LYCAEUM, 2, 15, 10),
        // Right scene and cell, wrong floor.
        ("wrong floor", SCENE_THE_LYCAEUM, 1, 15, 9),
        // Falsehood's row is The Lycaeum, never Empath Abbey.
        ("wrong scene", SCENE_EMPATH_ABBEY, 2, 15, 9),
    ];

    let mut messages = Vec::new();
    for (label, scene_byte, floor, x, y) in cases {
        let mut state = test_state(open_grid(), x, y);
        state.area = Area::Town {
            scene: Scene::new(scene_byte).unwrap(),
            floor,
        };
        state.special_items[SPECIAL_ITEM_SHARD_FALSEHOOD_INDEX] = 1;
        state.shadowlord_hideouts[SHADOWLORD_FALSEHOOD_INDEX] = 1;
        state.summoned_shadowlord = Some(SHADOWLORD_FALSEHOOD_INDEX);
        recovered_place_shadowlord_actor(&mut state, SHADOWLORD_FALSEHOOD_INDEX, x, y - 1);

        let outcome = state
            .use_shadowlord_shard(SHADOWLORD_FALSEHOOD_INDEX, None)
            .unwrap();

        assert_eq!(outcome, MoveOutcome::Blocked, "{label}: the attempt refuses");
        assert_eq!(
            state.special_items[SPECIAL_ITEM_SHARD_FALSEHOOD_INDEX], 1,
            "{label}: `a refused attempt ... leaves the shard in the party's possession`"
        );
        assert!(
            !state.shadowlord_vanquished(SHADOWLORD_FALSEHOOD_INDEX),
            "{label}: the handler `returns with no state change`"
        );
        assert_eq!(
            state.shadowlord_hideouts[SHADOWLORD_FALSEHOOD_INDEX], 1,
            "{label}: the hideout slot is untouched by a refusal"
        );
        assert_eq!(state.turn, 0, "{label}: no turn is consumed by a refusal");

        let message = state.message.clone();

        // Phase 1 is unconditional and completes with the shard's own virtue
        // word, even though the position gate is about to refuse.
        let virtue_at = message.find("Falsehood").unwrap_or_else(|| {
            panic!(
                "{label}: phase 1 prints the aloft line `completed by the shard's own virtue word`, got {message:?}"
            )
        });

        // Phase 3 is the only gate that speaks.
        let no_effect_at = message.find("No effect!").unwrap_or_else(|| {
            panic!("{label}: the position gate `produces the shared no-effect result`, got {message:?}")
        });

        assert!(
            virtue_at < no_effect_at,
            "{label}: the heading and aloft line print *before* the gate is evaluated, got {message:?}"
        );

        // Phase 4 only runs "once the position matches", so the opposed
        // principle's word must be absent here.
        assert!(
            !message.contains("Truth"),
            "{label}: the cast-into-the-flame line must not print off-position, got {message:?}"
        );
        // Phase 6's closing line names the destroyed Shadowlord.
        assert!(
            !message.contains(SHADOWLORD_NAME_FAULINEI),
            "{label}: nothing is destroyed by a refused attempt, got {message:?}"
        );

        messages.push((label, message));
    }

    // "the shared no-effect result" is one result, not three: wrong cell,
    // wrong floor and wrong scene are the same refusal.
    let (first_label, first_message) = &messages[0];
    for (label, message) in &messages[1..] {
        assert_eq!(
            message, first_message,
            "{label} and {first_label} must produce the same shared no-effect result"
        );
    }
}

/// `catalogs/quest-graph.md §5`, "Presentation order":
///
/// Phase 4: "Once the position matches, it pauses, prints a line describing the
/// shard being cast into the Eternal Flame completed by the opposed principle's
/// word (Truth, Love, or Courage), and pauses again - **before** testing whether
/// a Shadowlord is on the flame and whether the handshake matches."
///
/// Phase 5: "If either of those two gates fails, the handler simply returns. It
/// prints no refusal line, so from the player's side the sequence stops after
/// the cast-into-the-flame line with nothing further happening."
///
/// The divergence the spec names explicitly: "printing a refusal for the
/// actor/handshake failures (in the original those are silent)."
///
/// Gate 2: "The handler queries the *active-object* layer ... at the cell one
/// row north of the party". Gate 3: "The active-Shadowlord id recorded by the
/// name/Yell path must equal the shard's own index. Using the Shard of
/// Falsehood on a summoned Astaroth refuses."
///
/// The offsets differ by design: "the name/Yell path drops the Shadowlord
/// **two** cells north of wherever the party was standing, while the
/// destruction gate reads the cell **one** north of the fixed destruction
/// position."
///
/// And: "a refused attempt (... no active Shadowlord, or a mismatched active
/// Shadowlord index) leaves the shard in the party's possession."
///
/// The turn assertions come from `systems/inventory.md §7`: "The item handler
/// does not decide the turn cost by writing the clock itself" - the outer
/// command layer owns the action cost, so no path through this handler may
/// advance the clock on a refusal.
#[test]
fn use_shadowlord_shard_actor_and_handshake_gates_return_silently() {
    // Baseline: every gate satisfied, so the full published sequence runs
    // through phase 6. Used only as the yardstick for "the sequence stops
    // after the cast-into-the-flame line".
    let mut success = recovered_lycaeum_falsehood_row();
    success.summoned_shadowlord = Some(SHADOWLORD_FALSEHOOD_INDEX);
    recovered_place_shadowlord_actor(&mut success, SHADOWLORD_FALSEHOOD_INDEX, 15, 8);
    assert_eq!(
        success
            .use_shadowlord_shard(SHADOWLORD_FALSEHOOD_INDEX, None)
            .unwrap(),
        MoveOutcome::Used,
        "the published Falsehood row destroys Faulinei when both silent gates pass"
    );
    assert!(
        success.message.contains(SHADOWLORD_NAME_FAULINEI),
        "phase 6 `closes with a line naming the destroyed Shadowlord`, got {:?}",
        success.message
    );
    let success_message = success.message.clone();

    // (a) Gate 2. The Shadowlord is on the map but two cells north - where the
    // Yell path drops it - not on the flame cell one north that the
    // destruction gate reads.
    let mut no_actor_on_flame = recovered_lycaeum_falsehood_row();
    no_actor_on_flame.summoned_shadowlord = Some(SHADOWLORD_FALSEHOOD_INDEX);
    recovered_place_shadowlord_actor(&mut no_actor_on_flame, SHADOWLORD_FALSEHOOD_INDEX, 15, 7);

    // (b) Gate 3. A Shadowlord stands on the flame, but the active-Shadowlord
    // id recorded by the name/Yell path is Astaroth, not Faulinei.
    let mut mismatched_handshake = recovered_lycaeum_falsehood_row();
    mismatched_handshake.summoned_shadowlord = Some(SHADOWLORD_HATRED_INDEX);
    recovered_place_shadowlord_actor(&mut mismatched_handshake, SHADOWLORD_HATRED_INDEX, 15, 8);

    let mut silent_messages = Vec::new();
    for (label, mut state) in [
        ("no Shadowlord on the flame", no_actor_on_flame),
        ("mismatched active Shadowlord index", mismatched_handshake),
    ] {
        let outcome = state
            .use_shadowlord_shard(SHADOWLORD_FALSEHOOD_INDEX, None)
            .unwrap();

        assert_eq!(outcome, MoveOutcome::Blocked, "{label}: the attempt refuses");
        assert_eq!(
            state.special_items[SPECIAL_ITEM_SHARD_FALSEHOOD_INDEX], 1,
            "{label}: `a refused attempt ... leaves the shard in the party's possession`"
        );
        assert!(
            !state.shadowlord_vanquished(SHADOWLORD_FALSEHOOD_INDEX),
            "{label}: no vanquished value is written"
        );
        assert_eq!(state.turn, 0, "{label}: no turn is consumed");

        let message = state.message.clone();

        // Phase 4 ran: it prints "before testing whether a Shadowlord is on the
        // flame and whether the handshake matches", completed by the opposed
        // principle's word - Truth, for Falsehood.
        assert!(
            message.contains("Truth"),
            "{label}: the cast-into-the-flame line prints before both of these gates, got {message:?}"
        );

        // "It prints no refusal line."
        assert!(
            !message.contains("No effect!"),
            "{label}: `only the position gate produces the shared no-effect result`, got {message:?}"
        );
        assert!(
            !message.contains(SHADOWLORD_NAME_FAULINEI),
            "{label}: phase 6 must not run, got {message:?}"
        );

        // "the sequence stops after the cast-into-the-flame line with nothing
        // further happening": the silent output is exactly the successful
        // sequence truncated at that line, with no extra text of its own.
        assert!(
            success_message.starts_with(&message),
            "{label}: the silent refusal must be the successful sequence truncated at the cast line, got {message:?} against {success_message:?}"
        );
        assert!(
            message.len() < success_message.len(),
            "{label}: the successful sequence continues past where the silent refusal stops"
        );

        silent_messages.push((label, message));
    }

    // Both gates "simply return", so neither one distinguishes itself from the
    // other in what the player sees.
    assert_eq!(
        silent_messages[0].1, silent_messages[1].1,
        "{} and {} are both silent, so their output is identical",
        silent_messages[0].0, silent_messages[1].0
    );
}

/// `systems/inventory.md §4.7`, "Pages, field labels and placeholders":
///
/// "There are **six** pages in all: the attribute page, the equipment page, and
/// four inventory pages." Its table publishes the per-page border label and slot
/// count: Attributes (none), Equipment (none, 6), Armaments (`Armaments`, 48),
/// Spells (`Spells`, 48), Reagents (`Reagents`, 8), Items (38).
///
/// "Empty-state placeholders, both parenthesised: `(None ready)` | The equipment
/// list has nothing readied. `(None owned!)` | An inventory page has no slot
/// with a non-zero count."
///
/// `§4.6`: "The stored literals are the bare words with their punctuation -
/// `Select:`, `Items:`, `Reagents`, `Spells`, `Armaments` - and the two
/// triangles are chrome, not characters. When neither a picker nor a member
/// selection is active, the panel's top border carries no label."
///
/// `§4.4` confirms the Items literal carries its colon: the U-Use flow "write[s]
/// the framed border label `Items:`". The `§4.7` table's bare `Items` is the odd
/// one out among four published statements of the same literal, so the colon is
/// taken as published.
///
/// `§4.4`: "The `U`-Use path calls it with a row count of **eight**", and `§4.7`:
/// "The party-wide inventory pages use the same eight-row frame and row renderer
/// as the R-Ready picker."
#[test]
fn z_stats_border_labels_and_placeholders_match_published_literals() {
    // §4.7 table, per page.
    assert_eq!(
        ZStatsPage::Stats.border_label(),
        None,
        "the attribute page has no border label"
    );
    assert_eq!(
        ZStatsPage::Equipment.border_label(),
        None,
        "the equipment page has no border label"
    );
    assert_eq!(ZStatsPage::EquipmentStock.border_label(), Some("Armaments"));
    assert_eq!(ZStatsPage::Spells.border_label(), Some("Spells"));
    assert_eq!(ZStatsPage::Reagents.border_label(), Some("Reagents"));
    assert_eq!(ZStatsPage::SpecialUse.border_label(), Some("Items:"));

    // §4.6: exactly these five stored literals exist, so no page may invent
    // one outside the published roster.
    const PUBLISHED_BORDER_LITERALS: [&str; 5] =
        ["Select:", "Items:", "Reagents", "Spells", "Armaments"];
    for page in ZStatsPage::ORDERED {
        if let Some(label) = page.border_label() {
            assert!(
                PUBLISHED_BORDER_LITERALS.contains(&label),
                "{label:?} is not one of the five stored border-label literals in §4.6"
            );
        }
    }

    // §4.7: "the attribute page, the equipment page, and four inventory pages"
    // - so exactly two of the six carry no label and exactly four do.
    let unlabelled = ZStatsPage::ORDERED
        .iter()
        .filter(|page| page.border_label().is_none())
        .count();
    assert_eq!(
        unlabelled, 2,
        "only the two character-specific pages carry no border label"
    );

    // §4.7 empty-state placeholders, both parenthesised.
    assert_eq!(Z_STATS_NONE_READY_PLACEHOLDER, "(None ready)");
    assert_eq!(Z_STATS_NONE_OWNED_PLACEHOLDER, "(None owned!)");

    // §4.7 slot counts.
    assert_eq!(Z_STATS_EQUIPMENT_SLOTS, 6);
    assert_eq!(Z_STATS_ARMAMENTS_SLOTS, 48);
    assert_eq!(Z_STATS_SPELLS_SLOTS, 48);
    assert_eq!(Z_STATS_REAGENTS_SLOTS, 8);
    assert_eq!(Z_STATS_ITEMS_SLOTS, 38);

    // §4.4 / §4.7 eight-row frame, shared by the U-Use picker, the R-Ready
    // picker and the party-wide inventory pages.
    assert_eq!(USE_PICKER_PANEL_ROWS, 8);
    assert_eq!(READY_PICKER_PANEL_ROWS, 8);
    assert_eq!(Z_STATS_INVENTORY_PANEL_ROWS, 8);
}

/// `systems/inventory.md §4.7`: "There are **six** pages in all: the attribute
/// page, the equipment page, and four inventory pages."
///
/// `§4`: "The first two pages are character-specific: page 1 is the primary stat
/// page and page 2 is the equipment page. Later inventory pages walk shared
/// counter bands for reagents, spell charges, special/use items, and the
/// weapons/armour stash." That sentence is the published order of the four
/// inventory pages.
///
/// `§4`: "The Z-stats page loop preserves a single page index. ... Direction-style
/// navigation moves backward or forward through the visible page sequence".
#[test]
fn z_stats_direction_navigation_cycles_exactly_the_six_published_pages() {
    // "There are **six** pages in all".
    assert_eq!(ZStatsPage::ORDERED.len(), 6);

    // Page 1 the stat page, page 2 the equipment page, then reagents, spell
    // charges, special/use items, and the weapons/armour stash.
    assert_eq!(
        ZStatsPage::ORDERED,
        [
            ZStatsPage::Stats,
            ZStatsPage::Equipment,
            ZStatsPage::Reagents,
            ZStatsPage::Spells,
            ZStatsPage::SpecialUse,
            ZStatsPage::EquipmentStock,
        ],
        "the published page sequence of §4 / §4.7"
    );

    let mut session = ZStatsSession::new(0);
    assert_eq!(
        session.page,
        ZStatsPage::Stats,
        "`page 1 is the primary stat page`"
    );

    // Forward navigation visits each published page exactly once and returns
    // to the first after exactly six steps.
    let mut forward = Vec::new();
    for _ in 0..ZStatsPage::ORDERED.len() {
        forward.push(session.page);
        session.move_next_page();
    }
    assert_eq!(forward, ZStatsPage::ORDERED.to_vec());
    assert_eq!(
        session.page,
        ZStatsPage::Stats,
        "the cycle is exactly six long"
    );

    // Backward navigation is the exact inverse over the same sequence.
    let mut backward = Vec::new();
    for _ in 0..ZStatsPage::ORDERED.len() {
        session.move_previous_page();
        backward.push(session.page);
    }
    let mut reversed = ZStatsPage::ORDERED.to_vec();
    reversed.reverse();
    assert_eq!(backward, reversed);
    assert_eq!(
        session.page,
        ZStatsPage::Stats,
        "backward navigation also wraps after exactly six pages"
    );

    // No page repeats within a cycle, and nothing outside the six published
    // pages is reachable in either direction.
    let unique: std::collections::BTreeSet<&'static str> =
        forward.iter().map(|page| page.title()).collect();
    assert_eq!(
        unique.len(),
        ZStatsPage::ORDERED.len(),
        "each of the six pages is visited exactly once per cycle"
    );
    for page in ZStatsPage::ORDERED {
        assert!(
            ZStatsPage::ORDERED.contains(&page.next()),
            "forward navigation stays inside the published sequence"
        );
        assert!(
            ZStatsPage::ORDERED.contains(&page.previous()),
            "backward navigation stays inside the published sequence"
        );
    }
}
