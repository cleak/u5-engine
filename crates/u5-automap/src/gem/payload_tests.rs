//! The `GEMK` payload codec and attach (10.3.7, M3-07). No game data: the
//! records are built from events with synthetic coordinates.

use u5_runtime::ShrineVirtue;

use super::*;
use crate::gem::knowledge::TRADES;
use crate::section::SECTION_HEADER_LEN;

/// A small deterministic generator (xorshift64*), so the randomised
/// round trip needs no dependency and every failure reproduces.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn byte(&mut self) -> u8 {
        self.next() as u8
    }
    fn below(&mut self, n: u64) -> u8 {
        (self.next() % n) as u8
    }
}

/// Any event the observer can send, with arguments past their ranges
/// too (which the record ignores).
fn random_event(rng: &mut Rng) -> KnowEvent {
    match rng.below(13) {
        0 => KnowEvent::PlaceEntered { scene: rng.byte() },
        1 => KnowEvent::UnderworldVisited,
        2 => KnowEvent::ShrineEntered {
            virtue: rng.below(10),
        },
        3 => KnowEvent::CodexEntered,
        4 => KnowEvent::EntranceNamed {
            record: rng.below(10),
        },
        5 => KnowEvent::GateUsed(GateCell {
            plane: rng.below(2),
            x: rng.byte(),
            y: rng.byte(),
        }),
        6 => KnowEvent::SignRead(SignCell {
            scene: rng.byte(),
            floor: rng.byte() as i8,
            x: rng.below(32),
            y: rng.below(32),
        }),
        7 => KnowEvent::ShopOpened {
            scene: rng.byte(),
            trade: Trade::from_code(rng.below(9)).unwrap(),
            keeper: (rng.below(2) == 0).then(|| (rng.byte() as i8, rng.below(32), rng.below(32))),
        },
        8 => KnowEvent::TownLinkUsed(TownLink {
            scene: rng.byte(),
            floor: rng.byte() as i8,
            x: rng.below(32),
            y: rng.below(32),
            dir: if rng.below(2) == 0 {
                LinkDir::Up
            } else {
                LinkDir::Down
            },
        }),
        9 => KnowEvent::TownExit(ExitRun {
            scene: rng.byte(),
            side: edge_side(rng.below(4)).unwrap(),
            at: rng.below(32),
        }),
        10 => KnowEvent::DungeonMarked(DungeonMark {
            record: rng.below(8),
            level: rng.below(8),
            x: rng.below(8),
            y: rng.below(8),
            kind: mark_kind(rng.below(5)).unwrap(),
        }),
        11 => KnowEvent::DungeonCellsSeen {
            record: rng.below(9),
            level: rng.below(9),
            rows: std::array::from_fn(|_| rng.byte() & rng.byte()),
        },
        _ => KnowEvent::SideCellSeen {
            scene: rng.below(40),
            index: (rng.next() % 1800) as u16,
        },
    }
}

fn random_record(rng: &mut Rng, events: usize) -> GemKnowledge {
    let mut k = GemKnowledge::new();
    for _ in 0..events {
        k.apply(&random_event(rng));
    }
    k
}

/// A record the size of a finished game, and larger than any real one:
/// every place entered, every shrine and entrance, many signs, trades,
/// links and exits, every dungeon level fully seen with eight marks each,
/// and a side window for each of 32 scenes. Coordinates are synthetic.
fn finished_game() -> GemKnowledge {
    let mut k = GemKnowledge::new();
    let mut apply = |event| {
        k.apply(&event);
    };
    for scene in 1..=40 {
        apply(KnowEvent::PlaceEntered { scene });
    }
    apply(KnowEvent::UnderworldVisited);
    apply(KnowEvent::CodexEntered);
    for virtue in 0..8 {
        apply(KnowEvent::ShrineEntered { virtue });
        apply(KnowEvent::EntranceNamed { record: virtue });
    }
    for index in 0..16u8 {
        apply(KnowEvent::GateUsed(GateCell {
            plane: index / 8,
            x: 16 + index * 13,
            y: 200 - index * 11,
        }));
    }
    for scene in 1..=32u8 {
        for n in 0..4u8 {
            apply(KnowEvent::SignRead(SignCell {
                scene,
                floor: (n % 2) as i8,
                x: n * 7 + 1,
                y: scene % 32,
            }));
            apply(KnowEvent::TownLinkUsed(TownLink {
                scene,
                floor: n as i8 - 1,
                x: n * 5 + 2,
                y: 31 - n,
                dir: if n % 2 == 0 {
                    LinkDir::Up
                } else {
                    LinkDir::Down
                },
            }));
        }
        for trade in TRADES.into_iter().take(3) {
            apply(KnowEvent::ShopOpened {
                scene,
                trade,
                keeper: None,
            });
        }
        apply(KnowEvent::ShopOpened {
            scene,
            trade: Trade::Tavern,
            keeper: Some((0, 10, 12)),
        });
        for side in [EdgeSide::North, EdgeSide::South] {
            apply(KnowEvent::TownExit(ExitRun {
                scene,
                side,
                at: 15,
            }));
        }
        // A third of each window seen.
        for index in (0..1728u16).step_by(3) {
            apply(KnowEvent::SideCellSeen { scene, index });
        }
    }
    for record in 0..8 {
        for level in 0..8 {
            apply(KnowEvent::DungeonCellsSeen {
                record,
                level,
                rows: [0xFF; 8],
            });
            for n in 0..8u8 {
                apply(KnowEvent::DungeonMarked(DungeonMark {
                    record,
                    level,
                    x: n,
                    y: 7 - n,
                    kind: mark_kind(n % 5).unwrap(),
                }));
            }
        }
    }
    k
}

/// The sections of a payload, for structural checks.
fn sections_of(payload: &[u8]) -> Vec<(u8, [u8; 3], Vec<u8>)> {
    let count = u16::from_le_bytes([payload[0], payload[1]]);
    let (sections, used) = read_sections(&payload[2..], count).unwrap();
    assert_eq!(used, payload.len() - 2);
    sections
        .into_iter()
        .map(|s| (s.tag, s.key, s.payload.to_vec()))
        .collect()
}

#[test]
fn an_empty_record_is_two_bytes_and_writes_no_section() {
    let payload = encode_payload(&GemKnowledge::new());
    assert_eq!(payload, [0, 0]);
    assert!(decode_payload(&payload, 1).unwrap().is_empty());
}

#[test]
fn randomised_records_round_trip() {
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    for round in 0..300 {
        let events = (rng.next() % 400) as usize;
        let k = random_record(&mut rng, events);
        let payload = encode_payload(&k);
        let back = decode_payload(&payload, GEM_PAYLOAD_VERSION)
            .unwrap_or_else(|err| panic!("round {round}: {err}"));
        assert_eq!(back, k, "round {round}");
        assert_eq!(
            encode_payload(&back),
            payload,
            "round {round}: stable bytes"
        );
        // No section is empty, and tags come in ascending order.
        let sections = sections_of(&payload);
        assert!(sections.iter().all(|(_, _, bytes)| !bytes.is_empty()));
        assert!(sections.windows(2).all(|w| w[0].0 <= w[1].0));
    }
}

#[test]
fn every_tag_has_its_documented_layout() {
    let mut k = GemKnowledge::new();
    for event in [
        KnowEvent::PlaceEntered { scene: 5 },
        KnowEvent::PlaceEntered { scene: 2 },
        KnowEvent::ShrineEntered { virtue: 3 },
        KnowEvent::CodexEntered,
        KnowEvent::EntranceNamed { record: 6 },
        KnowEvent::GateUsed(GateCell {
            plane: 1,
            x: 40,
            y: 41,
        }),
        KnowEvent::SignRead(SignCell {
            scene: 7,
            floor: -1,
            x: 3,
            y: 4,
        }),
        KnowEvent::ShopOpened {
            scene: 7,
            trade: Trade::Sage,
            keeper: Some((1, 9, 10)),
        },
        KnowEvent::TownLinkUsed(TownLink {
            scene: 7,
            floor: 0,
            x: 11,
            y: 12,
            dir: LinkDir::Down,
        }),
        KnowEvent::TownExit(ExitRun {
            scene: 7,
            side: EdgeSide::West,
            at: 16,
        }),
        KnowEvent::DungeonCellsSeen {
            record: 2,
            level: 3,
            rows: [1, 2, 4, 8, 16, 32, 64, 128],
        },
        KnowEvent::DungeonMarked(DungeonMark {
            record: 2,
            level: 3,
            x: 5,
            y: 6,
            kind: MarkKind::SecretDoor,
        }),
        KnowEvent::SideCellSeen {
            scene: 9,
            index: 49,
        },
    ] {
        assert!(k.apply(&event));
    }
    let mut side = vec![0u8; SIDE_WINDOW_BYTES];
    side[6] = 1 << 1;
    let expected: Vec<(u8, [u8; 3], Vec<u8>)> = vec![
        (1, [0; 3], vec![2, 5]),
        (2, [0; 3], vec![1 << 3, 1 << 1, 1 << 6]),
        (3, [0; 3], vec![1, 40, 41]),
        (4, [0; 3], vec![7, 0xFF, 3, 4]),
        (5, [0; 3], vec![7, 8]),
        (6, [0; 3], vec![7, 1, 9, 10, 8]),
        (7, [0; 3], vec![7, 0, 11, 12, 1]),
        (8, [0; 3], vec![7, 3, 16]),
        (9, [2, 3, 0], vec![1, 2, 4, 8, 16, 32, 64, 128]),
        (10, [2, 3, 0], vec![5, 6, 4]),
        (11, [9, 0, 0], side),
    ];
    let payload = encode_payload(&k);
    assert_eq!(sections_of(&payload), expected);
    assert_eq!(
        payload.len(),
        2 + expected
            .iter()
            .map(|(_, _, b)| SECTION_HEADER_LEN + b.len())
            .sum::<usize>()
    );
}

#[test]
fn an_unknown_tag_is_skipped_by_its_length() {
    let mut k = GemKnowledge::new();
    k.apply(&KnowEvent::PlaceEntered { scene: 3 });
    k.apply(&KnowEvent::TownExit(ExitRun {
        scene: 3,
        side: EdgeSide::East,
        at: 9,
    }));
    // A later build's tag 12 between tags 1 and 8, whose bytes look like
    // a section header.
    let mut list = SectionList::new();
    list.push(1, [0; 3], &[3]).unwrap();
    list.push(12, [4, 5, 6], &[8, 0, 0, 0, 3, 0, 0, 0, 1, 2, 3])
        .unwrap();
    list.push(8, [0; 3], &[3, 1, 9]).unwrap();
    let mut payload = Vec::new();
    list.write_counted(&mut payload);
    assert_eq!(decode_payload(&payload, 1).unwrap(), k);
}

#[test]
fn unknown_codes_inside_known_tags_skip_the_entry() {
    let mut list = SectionList::new();
    list.push(5, [0; 3], &[4, 9, 4, 0]).unwrap(); // trade 9 unknown
    list.push(7, [0; 3], &[4, 0, 1, 1, 2, 4, 0, 1, 1, 0])
        .unwrap(); // dir 2 unknown
    list.push(8, [0; 3], &[4, 4, 1, 4, 0, 1]).unwrap(); // side 4 unknown
    list.push(10, [1, 1, 0], &[1, 1, 5, 2, 2, 0, 9, 1, 0])
        .unwrap(); // kind 5, x 9
    let mut payload = Vec::new();
    list.write_counted(&mut payload);
    let k = decode_payload(&payload, 1).unwrap();
    assert_eq!(k.trades().len(), 1);
    assert_eq!(k.town_links().len(), 1);
    assert_eq!(k.exits().len(), 1);
    assert_eq!(k.marks().len(), 1);
}

#[test]
fn a_newer_version_reads_and_may_carry_trailing_bytes() {
    let k = finished_game();
    let mut payload = encode_payload(&k);
    payload.extend_from_slice(&[0xAB; 5]);
    assert_eq!(
        decode_payload(&payload, GEM_PAYLOAD_VERSION),
        Err(PayloadError::TrailingBytes {
            at: payload.len() - 5
        })
    );
    assert_eq!(
        decode_payload(&payload, GEM_PAYLOAD_VERSION + 1).unwrap(),
        k
    );
    assert_eq!(decode_payload(&[0, 0], 0), Err(PayloadError::BadVersion));
}

#[test]
fn every_truncation_is_an_error_never_a_panic() {
    let payload = encode_payload(&finished_game());
    for cut in 0..payload.len() {
        assert!(
            decode_payload(&payload[..cut], GEM_PAYLOAD_VERSION).is_err(),
            "cut at {cut}"
        );
    }
    assert_eq!(decode_payload(&[], 1), Err(PayloadError::MissingCount));
    assert_eq!(decode_payload(&[1], 1), Err(PayloadError::MissingCount));
}

#[test]
fn a_bad_entry_length_is_an_error() {
    for (tag, bytes) in [
        (3u8, vec![1, 2]),
        (4, vec![1, 2, 3]),
        (5, vec![1]),
        (6, vec![1, 2, 3, 4]),
        (7, vec![1; 6]),
        (8, vec![1; 4]),
        (9, vec![1; 7]),
        (10, vec![1; 2]),
        (11, vec![1; 215]),
        (2, vec![1, 2]),
    ] {
        let mut list = SectionList::new();
        list.push(tag, [0; 3], &bytes).unwrap();
        let mut payload = Vec::new();
        list.write_counted(&mut payload);
        assert_eq!(
            decode_payload(&payload, 1),
            Err(PayloadError::BadLength {
                tag,
                len: bytes.len()
            }),
            "tag {tag}"
        );
    }
}

#[test]
fn random_bytes_never_panic() {
    let mut rng = Rng(7);
    for _ in 0..2000 {
        let len = (rng.next() % 64) as usize;
        let mut bytes: Vec<u8> = (0..len).map(|_| rng.byte()).collect();
        if bytes.len() >= 2 {
            bytes[1] = 0; // a plausible count
        }
        let _ = decode_payload(&bytes, 1);
    }
}

#[test]
fn a_finished_game_is_under_12_kb() {
    let payload = encode_payload(&finished_game());
    eprintln!("finished-game payload: {} bytes", payload.len());
    assert!(
        payload.len() < 12 * 1024,
        "a finished game took {} bytes",
        payload.len()
    );
    // Tag 11 is the bulk, as the design says: about 7 KB.
    let sides: usize = sections_of(&payload)
        .iter()
        .filter(|(tag, _, _)| *tag == tag::SIDE_CELLS)
        .map(|(_, _, b)| SECTION_HEADER_LEN + b.len())
        .sum();
    assert!(sides <= 7 * 1024 + 256, "side windows took {sides} bytes");
}

/// Every name the engine knows: place names and virtues, in the case the
/// game prints them and in lower case.
fn engine_names() -> Vec<Vec<u8>> {
    let mut names: Vec<String> = u5_runtime::published_world_location_entries()
        .into_iter()
        .filter_map(|entry| entry.proper_name.map(str::to_owned))
        .collect();
    names.extend(ShrineVirtue::ALL.iter().map(|v| v.name().to_owned()));
    assert!(names.len() > 8, "the scan has names to look for");
    names
        .into_iter()
        .flat_map(|name| [name.to_uppercase(), name.to_lowercase(), name])
        .filter(|name| name.len() >= 3)
        .map(String::into_bytes)
        .collect()
}

#[test]
fn the_payload_holds_no_text() {
    let names = engine_names();
    let mut rng = Rng(0xC0FFEE);
    let mut payloads = vec![encode_payload(&finished_game())];
    payloads.extend((0..50).map(|_| encode_payload(&random_record(&mut rng, 300))));
    for payload in &payloads {
        for name in &names {
            assert!(
                !payload.windows(name.len()).any(|w| w == name.as_slice()),
                "a name's bytes appear in the payload"
            );
        }
    }
    // The finished game, whose coordinates are small, has no run of four
    // ASCII letters at all.
    let letters = payloads[0]
        .windows(4)
        .filter(|w| w.iter().all(u8::is_ascii_alphabetic))
        .count();
    assert_eq!(letters, 0);
}

// ---- attach policy (the store-backed cases are in tests/gem_attach.rs) --

fn small_record() -> GemKnowledge {
    let mut k = GemKnowledge::new();
    k.apply(&KnowEvent::PlaceEntered { scene: 4 });
    k.apply(&KnowEvent::DungeonCellsSeen {
        record: 1,
        level: 0,
        rows: [3; 8],
    });
    k
}

#[test]
fn the_policy_maps_every_freshness_class() {
    let k = small_record();
    let payload = || {
        Some(SidecarPayload {
            version: GEM_PAYLOAD_VERSION,
            bytes: encode_payload(&small_record()),
        })
    };
    let cases = [
        (Freshness::Fresh, AttachOutcome::Restored, true),
        (Freshness::SameLineage, AttachOutcome::RestoredStale, true),
        (
            Freshness::Foreign,
            AttachOutcome::Empty(EmptyReason::Foreign),
            false,
        ),
        (
            Freshness::Missing,
            AttachOutcome::Empty(EmptyReason::Missing),
            false,
        ),
        (
            Freshness::Corrupt,
            AttachOutcome::Empty(EmptyReason::Corrupt),
            false,
        ),
    ];
    for (freshness, expected, kept) in cases {
        let (back, outcome) = attach_payload(freshness, payload());
        assert_eq!(outcome, expected, "{freshness:?}");
        assert_eq!(back == k, kept, "{freshness:?}");
        assert_eq!(back.is_empty(), !kept, "{freshness:?}");
    }
    assert_eq!(
        attach_payload(Freshness::Fresh, None).1,
        AttachOutcome::Empty(EmptyReason::Missing)
    );
}
