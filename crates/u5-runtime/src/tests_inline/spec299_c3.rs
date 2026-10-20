// Conformance for u5-spec a031913 (issue #299 follow-ups 13-19, R569-R572).
// Each test names the answer it pins and the published sentence it rests on.

/// u5-spec#299 Q17 / `RETRACTIONS.md` R571, `blackthorn.md §7` step 11: the
/// rescue spins "the ordinary clock advance in nine-minute steps until the
/// hour is 6. The hour is checked before each step and the minute is never
/// reset." The worked vectors: "4-28-139 23:55 becomes 5-1-139 06:04, and
/// 13-28-139 13:10 becomes 1-1-140 06:07"; "an entry hour of 6 makes no step
/// and keeps its date and minute unchanged"; an entry hour of 0 to 5 "ends the
/// same day at 06:00 to 06:08".
#[test]
fn spec299_q17_rescue_clock_spins_in_nine_minute_steps_to_hour_six() {
    let cases = [
        ((139, 4, 28, 23, 55), (139, 5, 1, 6, 4)),
        ((139, 13, 28, 13, 10), (140, 1, 1, 6, 7)),
        ((139, 3, 9, 6, 30), (139, 3, 9, 6, 30)),
        ((139, 3, 9, 5, 55), (139, 3, 9, 6, 4)),
        ((139, 3, 9, 0, 0), (139, 3, 9, 6, 0)),
    ];
    for ((year, month, day, hour, minute), expected) in cases {
        let dir = debug_game_dir();
        let mut state = test_state(open_grid(), 1, 1);
        state.clock = GameClock::with_date(year, month, day, hour, minute).unwrap();
        state.party[0].status = b'D';
        state.party[0].hp = 0;
        state.repaint_stats_panel();

        state.apply_blackthorn_rescue_refuge(&dir).unwrap();
        state.run_blackthorn_rescue_to_handoff(&dir).unwrap();

        let clock = state.clock;
        assert_eq!(
            (clock.year, clock.month, clock.day, clock.hour, clock.minute),
            expected,
            "rescue from {year}-{month}-{day} {hour:02}:{minute:02}"
        );
        // "A midnight crossing runs the ordinary day rollover once, which
        // repaints the whole stats panel straight away, so the date row shows
        // the new date."
        let live = crate::stats_panel::stats_panel_snapshot_of(&state);
        assert_eq!(state.displayed_stats_panel().date_row, live.date_row);
        let _ = fs::remove_dir_all(dir);
    }
}

/// u5-spec#299 Q16, `shops.md §8.5` "A negative quantity" and `input.md §8`:
/// "The reader keeps a leading `-`, and the sign uses one of its two
/// positions, so `-1` is the only practical negative entry: `-99` echoes `-9`
/// and drops the last key, and `-0` or a bare `-` reads as zero (the
/// `Hrumph.` path). A leading `+` is accepted and ignored, so `+5` buys five."
#[test]
fn spec299_q16_provision_reader_sign_uses_one_of_two_positions() {
    fn typed(keys: &str) -> crate::shop_runtime::TypedQuantityEntry {
        let mut entry = crate::shop_runtime::TypedQuantityEntry::default();
        for byte in keys.bytes() {
            assert!(!entry.apply(byte));
        }
        entry
    }
    assert_eq!(typed("-99").as_str(), "-9");
    assert_eq!(typed("99").as_str(), "99");
    assert_eq!(typed("999").as_str(), "99");
    assert_eq!(typed("+5").quantity(), 5);
    assert_eq!(typed("+55").as_str(), "+5");
    assert_eq!(typed("-0").quantity(), 0);
    assert_eq!(typed("-").quantity(), 0);
    assert_eq!(typed("-1").quantity(), u16::MAX);
}

/// u5-spec#299 Q16, `shops.md §8.5`: the worked vectors "at a unit price of
/// 14" for a `-1` entry. "Each pass pays for one pack and adds its 25
/// servings **before** the loop tests the 9999 food stop"; "Food reaches
/// 9999 ... takes the completed-purchase tail"; "Gold runs out first ...
/// `"Thou canst\nafford only `, the number of packs actually bought"; "Not
/// one pack affordable ... the two nothing-served rows above apply".
#[test]
fn spec299_q16_negative_provision_quantity_buys_until_gold_or_ceiling() {
    use crate::shops::{
        ProvisionPurchaseCompletion, ProvisionPurchaseError, Tavern,
        apply_provision_purchase_at_unit_price,
    };
    let minus_one = u16::MAX;
    for (gold, food, packs, gold_after, food_after, completion) in [
        (100, 100, 7, 2, 275, ProvisionPurchaseCompletion::GoldExhausted),
        (9999, 100, 396, 4455, 9999, ProvisionPurchaseCompletion::Completed),
        (9999, 9990, 1, 9985, 9999, ProvisionPurchaseCompletion::Completed),
        (100, 9999, 1, 86, 9999, ProvisionPurchaseCompletion::Completed),
        (14, 9990, 1, 0, 9999, ProvisionPurchaseCompletion::Completed),
    ] {
        let (mut g, mut f) = (gold, food);
        let outcome = apply_provision_purchase_at_unit_price(
            &mut g,
            &mut f,
            Tavern::TheHonestMeal,
            minus_one,
            14,
        )
        .unwrap_or_else(|err| panic!("{gold}/{food}: {err:?}"));
        assert_eq!(outcome.purchased_quantity, packs, "{gold}/{food}");
        assert_eq!((g, f), (gold_after, food_after), "{gold}/{food}");
        assert_eq!(outcome.completion, completion, "{gold}/{food}");
    }
    for (gold, food) in [(13u16, 9999u16), (0, 100)] {
        let (mut g, mut f) = (gold, food);
        assert_eq!(
            apply_provision_purchase_at_unit_price(
                &mut g,
                &mut f,
                Tavern::TheHonestMeal,
                minus_one,
                14
            )
            .unwrap_err(),
            ProvisionPurchaseError::NoNeed
        );
        assert_eq!((g, f), (gold, food));
    }
    let (mut g, mut f) = (0u16, 2u16);
    let scraps = apply_provision_purchase_at_unit_price(
        &mut g,
        &mut f,
        Tavern::TheHonestMeal,
        minus_one,
        14,
    )
    .unwrap();
    assert_eq!(scraps.completion, ProvisionPurchaseCompletion::Charity);
    assert!(matches!(f, 3 | 4), "table scraps: {f}");
    assert_eq!(g, 0);
}
