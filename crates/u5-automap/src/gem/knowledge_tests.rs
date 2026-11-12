//! M3-05 record tests: the shop-trade table of 10.3.3, folding, and the
//! counts-only `Debug`.

use u5_runtime::shop_session::{ActiveShopSession, shop_session_for_talk_context};

use super::*;

#[test]
fn the_shop_trade_table_has_seven_signs() {
    let signs: Vec<(u8, Trade)> = SIGN_TRADES.to_vec();
    assert_eq!(
        signs,
        [
            (0xF2, Trade::Arms),
            (0xF3, Trade::Healer),
            (0xF4, Trade::HorseTrader),
            (0xF5, Trade::Guild),
            (0xF6, Trade::Innkeeper),
            (0xF7, Trade::Reagent),
            (0xF9, Trade::ShipBroker),
        ]
    );
    for (tile, trade) in SIGN_TRADES {
        assert_eq!(Trade::from_sign_tile(tile), Some(trade));
        assert_eq!(trade.sign_tile(), Some(tile));
        assert!(!trade.is_unsigned());
    }
}

#[test]
fn the_provisioner_and_government_signs_stay_unmapped() {
    // S10.1: no session kind matches them, so they stay "?".
    assert_eq!(Trade::from_sign_tile(0xF0), None);
    assert_eq!(Trade::from_sign_tile(0xF1), None);
    assert_eq!(Trade::from_sign_tile(0xF8), None);
    for tile in 0..=u8::MAX {
        if !(0xF2..=0xF7).contains(&tile) && tile != 0xF9 {
            assert_eq!(Trade::from_sign_tile(tile), None, "tile {tile:#04x}");
        }
    }
}

#[test]
fn tavern_and_sage_have_no_sign() {
    assert!(Trade::Tavern.is_unsigned());
    assert!(Trade::Sage.is_unsigned());
    assert_eq!(Trade::Tavern.sign_tile(), None);
    assert_eq!(Trade::Sage.sign_tile(), None);
}

#[test]
fn every_shop_session_maps_to_its_trade() {
    let expect = [
        (0x81, Trade::Arms),
        (0x82, Trade::Tavern),
        (0x83, Trade::HorseTrader),
        (0x84, Trade::ShipBroker),
        (0x85, Trade::Reagent),
        (0x86, Trade::Guild),
        (0x87, Trade::Healer),
        (0x88, Trade::Innkeeper),
    ];
    for (dialog, trade) in expect {
        let session = shop_session_for_talk_context(dialog, None).unwrap();
        assert_eq!(Trade::of_session(&session), trade, "dialog {dialog:#04x}");
    }
    assert_eq!(
        Trade::of_session(&ActiveShopSession::Sage(Default::default())),
        Trade::Sage
    );
    // The arms sessions with a shop or a stock table are still Arms.
    for scene in 0..=u8::MAX {
        if let Some(session) = shop_session_for_talk_context(0x81, Some(scene)) {
            assert_eq!(Trade::of_session(&session), Trade::Arms);
        }
    }
}

#[test]
fn trade_codes_round_trip() {
    for (i, trade) in TRADES.iter().enumerate() {
        assert_eq!(trade.code() as usize, i);
        assert_eq!(Trade::from_code(trade.code()), Some(*trade));
    }
    assert_eq!(Trade::from_code(9), None);
}

#[test]
fn a_signed_trade_never_records_a_keeper() {
    let mut know = GemKnowledge::new();
    assert!(know.apply(&KnowEvent::ShopOpened {
        scene: 5,
        trade: Trade::Healer,
        keeper: Some((0, 3, 4)),
    }));
    assert_eq!(know.keepers().len(), 0);
    assert!(know.apply(&KnowEvent::ShopOpened {
        scene: 5,
        trade: Trade::Tavern,
        keeper: Some((0, 3, 4)),
    }));
    assert_eq!(know.keepers_in(5).count(), 1);
    assert_eq!(know.keepers_in(6).count(), 0);
}

#[test]
fn an_empty_record_knows_nothing() {
    let know = GemKnowledge::new();
    assert!(know.is_empty());
    let keys = [
        FeatureKey::Place { scene: 1 },
        FeatureKey::Entrance { record: 0 },
        FeatureKey::Shrine { virtue: 0 },
        FeatureKey::Codex,
        FeatureKey::Underworld,
        FeatureKey::Gate(GateCell {
            plane: 0,
            x: 1,
            y: 1,
        }),
        FeatureKey::Sign(SignCell {
            scene: 1,
            floor: 0,
            x: 1,
            y: 1,
        }),
        FeatureKey::Trade {
            scene: 1,
            trade: Trade::Arms,
        },
        FeatureKey::TownLink {
            scene: 1,
            floor: 0,
            x: 1,
            y: 1,
        },
        FeatureKey::Exit(ExitRun {
            scene: 1,
            side: EdgeSide::North,
            at: 1,
        }),
        FeatureKey::DungeonMark(DungeonMark {
            record: 0,
            level: 0,
            x: 0,
            y: 0,
            kind: MarkKind::Ladder,
        }),
    ];
    for key in keys {
        assert!(!know.is_known(&key), "{key:?}");
    }
    assert!(!know.remembered(0, 0, 0, 0));
}

#[test]
fn out_of_range_bits_change_nothing() {
    let mut know = GemKnowledge::new();
    assert!(!know.apply(&KnowEvent::ShrineEntered { virtue: 8 }));
    assert!(!know.apply(&KnowEvent::EntranceNamed { record: 8 }));
    assert!(!know.apply(&KnowEvent::SideCellSeen {
        scene: 1,
        index: (SIDE_WINDOW_BYTES * 8) as u16,
    }));
    assert!(know.is_empty());
}

#[test]
fn cells_and_side_cells_fold() {
    let mut know = GemKnowledge::new();
    let mut rows = [0u8; 8];
    rows[3] = 0b0010_0000;
    assert!(know.apply(&KnowEvent::DungeonCellsSeen {
        record: 2,
        level: 4,
        rows,
    }));
    assert!(know.remembered(2, 4, 5, 3));
    assert!(!know.remembered(2, 4, 4, 3));
    assert!(know.apply(&KnowEvent::SideCellSeen {
        scene: 7,
        index: 48 * 2 + 9,
    }));
    assert!(know.side_seen(7, 48 * 2 + 9));
    assert!(!know.side_seen(7, 48 * 2 + 10));
    assert!(!know.side_seen(8, 48 * 2 + 9));
}

#[test]
fn the_debug_print_holds_counts_only() {
    let mut know = GemKnowledge::new();
    // Distinctive values; none of them may appear in the print.
    let events = [
        KnowEvent::PlaceEntered { scene: 0xAB },
        KnowEvent::ShrineEntered { virtue: 6 },
        KnowEvent::GateUsed(GateCell {
            plane: 1,
            x: 213,
            y: 177,
        }),
        KnowEvent::SignRead(SignCell {
            scene: 0xAB,
            floor: -1,
            x: 29,
            y: 23,
        }),
        KnowEvent::ShopOpened {
            scene: 0xAB,
            trade: Trade::Tavern,
            keeper: Some((2, 19, 27)),
        },
        KnowEvent::TownLinkUsed(TownLink {
            scene: 0xAB,
            floor: 2,
            x: 17,
            y: 26,
            dir: LinkDir::Up,
        }),
        KnowEvent::TownExit(ExitRun {
            scene: 0xAB,
            side: EdgeSide::West,
            at: 14,
        }),
        KnowEvent::DungeonMarked(DungeonMark {
            record: 7,
            level: 6,
            x: 5,
            y: 4,
            kind: MarkKind::SecretDoor,
        }),
    ];
    for event in &events {
        know.apply(event);
    }
    let printed = format!("{know:?}");
    for forbidden in [
        "171",
        "213",
        "177",
        "29",
        "23",
        "19",
        "27",
        "17",
        "26",
        "14",
        "Tavern",
        "Up",
        "West",
        "SecretDoor",
        "x:",
        "y:",
        "scene",
    ] {
        assert!(
            !printed.contains(forbidden),
            "Debug leaks {forbidden}: {printed}"
        );
    }
    assert!(printed.contains("entered: 1"));
    assert!(printed.contains("trades: 1"));
    assert!(printed.contains("keepers: 1"));
}
