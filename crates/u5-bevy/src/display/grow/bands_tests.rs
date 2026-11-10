//! The ribbon, the HP colours and the tabs (design 2.2.3, 2.2.6, 2.3.7
//! steps 5-6, 2.3.8, D2.11, D2.17; build plan M1-10): the layout for one to
//! six members at 16:9 and 21:9, odd names, statuses and HP values, the
//! counters and the ship, the colours at their edges, the shared tab
//! centre, and the no-reveal rules.

use u5_runtime::stats_panel::StatsPanelSnapshot;
use u5_runtime::test_fixtures::{open_grid, test_state};
use u5_runtime::{
    CH_FONT_LEN, FIRST_PLAYABLE_FRIGATE_TILE, IBM_CH_FILE, PartyMember, SAVE_CHARACTER_NAME_LEN,
    TRANSPORT_MARKER_SHIP_FURLED_FIRST, TransportState, parse_ch_font,
};

use super::bands::*;
use super::compose::{BAND_BLUE, RULE_WHITE, paint_chrome};
use super::*;
use crate::EGA_PALETTE_RGB;

fn canvas(w: u32, h: u32) -> Canvas {
    Canvas::for_window(w, h, PixelAspect::Dos).unwrap()
}

fn c16x9() -> Canvas {
    canvas(2560, 1440)
}

fn c21x9() -> Canvas {
    canvas(3440, 1440)
}

/// A font whose every glyph is a solid block except the space, so a drawn
/// cell is a solid square of its colour and a space draws nothing.
fn block_font() -> u5_runtime::FixedCellFont {
    let mut bytes = vec![0xff; CH_FONT_LEN];
    for row in 0..8 {
        bytes[usize::from(b' ') * 8 + row] = 0;
    }
    parse_ch_font(&bytes, IBM_CH_FILE).unwrap()
}

fn name(text: &[u8]) -> [u8; SAVE_CHARACTER_NAME_LEN] {
    let mut out = [0; SAVE_CHARACTER_NAME_LEN];
    out[..text.len()].copy_from_slice(text);
    out
}

fn member(status: u8, hp: u16, max_hp: u16) -> PartyMember {
    PartyMember {
        slot: 0,
        class_byte: b'A',
        status,
        climb_stat: 20,
        mana: 0,
        hp,
        max_hp,
        level: 1,
    }
}

const NAMES: [&[u8]; 6] = [
    b"Avatar",
    b"Shamino",
    b"Iolo",
    b"Jaana",
    b"Dupre",
    b"Geoffrey",
];

/// A state whose party is `members` (named from [`NAMES`]), with its
/// panel repainted.
fn party_state(members: &[PartyMember]) -> PlayState {
    let mut state = test_state(open_grid(), 5, 5);
    state.party = members.to_vec();
    state.party_names = (0..members.len()).map(|i| name(NAMES[i])).collect();
    state.food = 63;
    state.gold = 150;
    state.repaint_stats_panel();
    state
}

fn the_mockup_party() -> Vec<PartyMember> {
    vec![
        member(b'G', 60, 60),
        member(b'G', 5, 60),
        member(b'G', 90, 90),
    ]
}

fn cells_text(cells: &[(u8, CellPart)]) -> String {
    cells.iter().map(|(code, _)| char::from(*code)).collect()
}

fn rgb_at(rgba: &[u8], c: &Canvas, x: u32, y: u32) -> [u8; 3] {
    let i = ((y * c.w + x) * 4) as usize;
    [rgba[i], rgba[i + 1], rgba[i + 2]]
}

fn ega(index: u8) -> [u8; 3] {
    EGA_PALETTE_RGB[usize::from(index)]
}

fn blank(c: &Canvas) -> Vec<u8> {
    let mut rgba = vec![0; (c.w * c.h * 4) as usize];
    paint_chrome(&mut rgba, c);
    rgba
}

// --- The snapshot. ---------------------------------------------------------

#[test]
fn the_snapshot_reads_the_rows_the_panel_shows() {
    let state = party_state(&the_mockup_party());
    let ribbon = ribbon_snapshot(&state);
    let shown: Vec<_> = ribbon
        .members
        .iter()
        .map(|m| (m.name.as_str(), m.hp.as_str(), m.status, m.max_hp))
        .collect();
    assert_eq!(
        shown,
        [
            ("Avatar", "60", Some(b'G'), 60),
            ("Shamino", "5", Some(b'G'), 60),
            ("Iolo", "90", Some(b'G'), 90),
        ]
    );
    // The counters and the date with their padding runs closed.
    assert_eq!(ribbon.counters, "F:63 G:150");
    assert_eq!(
        ribbon.date,
        close_runs(u5_runtime::stats_panel_date_text(&state))
    );
    assert!(!ribbon.date.contains("  ") && !ribbon.date.starts_with(' '));
}

#[test]
fn food_and_gold_at_9999_and_a_ship() {
    let mut state = party_state(&the_mockup_party());
    state.food = 9999;
    state.gold = 9999;
    state.repaint_stats_panel();
    assert_eq!(ribbon_snapshot(&state).counters, "F:9999 G:9999");

    // The ship variant appears exactly when the panel shows it.
    state.player.transport = TransportState::Ship {
        type_byte: TRANSPORT_MARKER_SHIP_FURLED_FIRST,
        tile: FIRST_PLAYABLE_FRIGATE_TILE,
        sails_hoisted: false,
        hull: 150,
        skiffs: 1,
    };
    let before = ribbon_snapshot(&state).counters;
    assert_eq!(before, "F:9999 G:9999", "not before the panel repaints");
    state.repaint_stats_panel();
    let ship = ribbon_snapshot(&state).counters;
    assert_eq!(
        ship,
        close_runs(u5_runtime::stats_panel_counter_text(&state))
    );
    assert_ne!(ship, before);
    assert!(!ship.contains("G:"), "{ship}");

    // Right-aligned, ending 8 pixels inside the right rule, on row y 0.
    let font = block_font();
    for c in [c16x9(), c21x9(), canvas(1920, 1080)] {
        let mut rgba = blank(&c);
        let ribbon = RibbonSnapshot {
            members: Vec::new(),
            counters: "F:9999 G:9999".to_string(),
            date: "4-5-139".to_string(),
        };
        paint_ribbon(&mut rgba, &c, &ribbon, &font, true, false);
        let right = c.w - 16;
        assert_eq!(counters_right(&c), right);
        assert_eq!(rgb_at(&rgba, &c, right - 1, 0), ega(RULE_WHITE));
        assert_eq!(rgb_at(&rgba, &c, right, 0), ega(BAND_BLUE));
        assert_eq!(rgb_at(&rgba, &c, right - 13 * 8, 0), ega(RULE_WHITE));
        assert_eq!(rgb_at(&rgba, &c, right - 13 * 8 - 1, 0), ega(BAND_BLUE));
        // The date on row y 8, right-aligned to the same edge.
        assert_eq!(rgb_at(&rgba, &c, right - 1, 8), ega(RULE_WHITE));
        assert_eq!(rgb_at(&rgba, &c, right - 7 * 8, 8), ega(RULE_WHITE));
        assert_eq!(rgb_at(&rgba, &c, right - 7 * 8 - 1, 8), ega(BAND_BLUE));
    }
}

#[test]
fn a_nine_byte_name_with_no_terminator() {
    let mut state = party_state(&the_mockup_party());
    state.party_names[0] = *b"Abcdefghi";
    state.repaint_stats_panel();
    let ribbon = ribbon_snapshot(&state);
    assert_eq!(ribbon.members[0].name, "Abcdefghi");
    assert_eq!(ribbon.members[0].hp, "60");
    let full = ribbon_layout(&c21x9(), false);
    let initials = ribbon_layout(&c16x9(), false);
    // The full name keeps its field: eight characters and the colon.
    assert_eq!(
        cells_text(&member_cells(&ribbon.members[0], &full)),
        "Abcdefgh: 60G"
    );
    assert_eq!(
        cells_text(&member_cells(&ribbon.members[0], &initials)),
        "A: 60G"
    );
}

#[test]
fn a_control_byte_in_a_status_is_only_a_glyph() {
    let mut state = party_state(&the_mockup_party());
    state.party[1].status = 0x07;
    state.repaint_stats_panel();
    let ribbon = ribbon_snapshot(&state);
    assert_eq!(ribbon.members[1].status, Some(0x07));
    let layout = ribbon_layout(&c16x9(), false);
    let cells = member_cells(&ribbon.members[1], &layout);
    assert_eq!(cells.last(), Some(&(0x07, CellPart::Status)));
    // Drawn as glyph 7 in white at the status cell; nothing else moves.
    let font = block_font();
    let c = c16x9();
    let mut rgba = blank(&c);
    paint_ribbon(&mut rgba, &c, &ribbon, &font, true, false);
    let (x, y) = layout.place(1);
    assert_eq!(rgb_at(&rgba, &c, x + 5 * 8, y), ega(RULE_WHITE));
    assert_eq!(rgb_at(&rgba, &c, x + 6 * 8, y), ega(BAND_BLUE));
    // And the band's other rows are untouched.
    assert_eq!(rgb_at(&rgba, &c, x + 5 * 8, 16), ega(0));
}

#[test]
fn hp_0_240_and_an_edited_9999() {
    let layout = ribbon_layout(&c16x9(), false);
    for (hp, text) in [(0, "A:  0G"), (240, "A:240G"), (9999, "A:99+G")] {
        let state = party_state(&[member(b'G', hp, 240)]);
        let ribbon = ribbon_snapshot(&state);
        assert_eq!(ribbon.members[0].hp, hp.to_string());
        assert_eq!(
            cells_text(&member_cells(&ribbon.members[0], &layout)),
            text,
            "{hp}"
        );
    }
    // HP 0 is low.
    let state = party_state(&[member(b'G', 0, 240)]);
    assert_eq!(
        ribbon_hp_colour(&ribbon_snapshot(&state).members[0], true),
        HP_LOW
    );
}

// --- The layout. -------------------------------------------------------------

#[test]
fn ribbon_layout_for_one_to_six_members() {
    let all: Vec<_> = (0..6).map(|i| member(b'G', 50 + i, 100)).collect();
    // (canvas, open, positions, one member's text)
    let cases = [
        (
            c16x9(),
            false,
            [(16, 0), (16, 8), (72, 0), (72, 8), (128, 0), (128, 8)],
            "A: 50G",
        ),
        (
            c16x9(),
            true,
            [(16, 0), (16, 8), (72, 0), (72, 8), (128, 0), (128, 8)],
            "A: 50G",
        ),
        (
            canvas(1920, 1080),
            false,
            [(16, 0), (16, 8), (72, 0), (72, 8), (128, 0), (128, 8)],
            "A: 50G",
        ),
        (
            c21x9(),
            false,
            [(16, 0), (16, 8), (128, 0), (128, 8), (456, 0), (456, 8)],
            "Avatar:   50G",
        ),
        (
            c21x9(),
            true,
            [(16, 0), (16, 8), (128, 0), (128, 8), (240, 0), (240, 8)],
            "Avatar:   50G",
        ),
    ];
    let font = block_font();
    for (c, open, places, first) in cases {
        let layout = ribbon_layout(&c, open);
        for n in 1..=6 {
            let state = party_state(&all[..n]);
            let ribbon = ribbon_snapshot(&state);
            assert_eq!(ribbon.members.len(), n);
            assert_eq!(
                cells_text(&member_cells(&ribbon.members[0], &layout)),
                first
            );
            let mut rgba = blank(&c);
            paint_ribbon(&mut rgba, &c, &ribbon, &font, true, open);
            for (i, (x, y)) in places.iter().copied().enumerate() {
                assert_eq!(layout.place(i), (x, y), "{} member {i}", c.w);
                // The initial (first cell) is drawn exactly for the members
                // there are; the first cell of an empty place stays blue.
                let want = if i < n {
                    ega(RULE_WHITE)
                } else {
                    ega(BAND_BLUE)
                };
                assert_eq!(rgb_at(&rgba, &c, x, y), want, "{} n={n} i={i}", c.w);
                // (Row 7 of the second row is the rule row, y 15.)
                assert_eq!(rgb_at(&rgba, &c, x + 7, y + 6), want);
            }
        }
        // Six members stay clear of the sky tab in both states: the ribbon
        // ends left of it (or starts right of it).
        let state = party_state(&all);
        let snapshot = band_snapshot(&state);
        let sky = snapshot.gaps.top_rect().unwrap();
        let centre = tab_centre(&c, open, sky.width() as u32, &snapshot.ribbon);
        let (x0, x1) = (tab_left(sky, centre), tab_left(sky, centre) + 112);
        let member_w = layout.member_cells() as u32 * 8;
        for i in 0..6 {
            let x = layout.place(i).0;
            assert!(x + member_w <= x0 || x >= x1, "{} {open} {i}", c.w);
        }
    }
    // 16:9 initials end at x 175 in both states, so they never move.
    let layout = ribbon_layout(&c16x9(), false);
    assert_eq!(layout.place(5).0 + layout.member_cells() as u32 * 8, 176);
    // 21:9: equal 48-pixel gaps either side of the tab's strip.
    let layout = ribbon_layout(&c21x9(), false);
    assert_eq!(layout.place(3).0 + 13 * 8, 232);
    assert_eq!(232 + 48, 344 - 64);
    assert_eq!(344 + 64 + 48, layout.place(4).0);
}

#[test]
fn only_members_of_the_current_party_are_shown() {
    // A panel body still on screen from a larger party: the ribbon shows
    // only the current party's members.
    let six: Vec<_> = (0..6).map(|i| member(b'G', 50 + i, 100)).collect();
    let mut state = party_state(&six);
    state.party.truncate(2);
    state.party_names.truncate(2);
    let ribbon = ribbon_snapshot(&state);
    assert_eq!(ribbon.members.len(), 2);
    let names: Vec<_> = ribbon.members.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(names, ["Avatar", "Shamino"]);
    // An empty party shows nobody.
    state.party.clear();
    assert!(ribbon_snapshot(&state).members.is_empty());
}

#[test]
fn every_character_is_one_the_panel_shows_even_while_a_body_is_held() {
    let mut state = party_state(&the_mockup_party());
    // A staged presentation holds the old panel body while the game has
    // already changed HP, status, food and gold: the ribbon follows the
    // panel on screen, never the game ahead of it.
    let held: StatsPanelSnapshot = state.stats_panel.clone();
    state.stats_panel_hold = Some(held.clone());
    state.party[1].hp = 1;
    state.party[2].status = b'P';
    state.food = 9;
    state.gold = 9;
    state.repaint_stats_panel();
    let ribbon = ribbon_snapshot(&state);
    assert_eq!(ribbon.members[1].hp, "5");
    assert_eq!(ribbon.members[2].status, Some(b'G'));
    assert_eq!(ribbon.counters, "F:63 G:150");
    for (i, m) in ribbon.members.iter().enumerate() {
        let row = &held.rows[i];
        assert!(row.starts_with(&m.name) && row.contains(&m.hp), "{row:?}");
        assert_eq!(m.status, held.statuses[i]);
        // Every glyph the ribbon draws, bar its own colon, `+` and padding,
        // is in that row.
        for layout in [
            ribbon_layout(&c16x9(), false),
            ribbon_layout(&c21x9(), false),
        ] {
            for (code, _) in member_cells(m, &layout) {
                if matches!(code, b':' | b' ' | HP_OVERFLOW) {
                    continue;
                }
                assert!(row.contains(char::from(code)), "{row:?} {code}");
            }
        }
    }
    for word in ribbon.counters.split(' ') {
        assert!(held.counter_row.contains(word));
    }
    for word in ribbon.date.split(' ') {
        assert!(held.date_row.contains(word));
    }
}

// --- The colours. ------------------------------------------------------------

#[test]
fn hp_colours_at_the_quarter_and_poison() {
    let m = |status: u8, hp: u16, max_hp: u16| RibbonMember {
        name: "Avatar".to_string(),
        hp: hp.to_string(),
        status: Some(status),
        max_hp,
    };
    // Exactly a quarter is low; just above is not.
    assert_eq!(ribbon_hp_colour(&m(b'G', 15, 60), true), HP_LOW);
    assert_eq!(ribbon_hp_colour(&m(b'G', 16, 60), true), RULE_WHITE);
    assert_eq!(ribbon_hp_colour(&m(b'G', 60, 240), true), HP_LOW);
    assert_eq!(ribbon_hp_colour(&m(b'G', 61, 240), true), RULE_WHITE);
    // Poisoned and low together: green wins.
    assert_eq!(ribbon_hp_colour(&m(b'P', 5, 60), true), HP_POISONED);
    assert_eq!(ribbon_hp_colour(&m(b'P', 60, 60), true), HP_POISONED);
    // The toggle off: always white.
    assert_eq!(ribbon_hp_colour(&m(b'P', 5, 60), false), RULE_WHITE);
    assert_eq!(ribbon_hp_colour(&m(b'G', 5, 60), false), RULE_WHITE);
    assert_eq!((HP_LOW, HP_POISONED), (12, 10));

    // On the canvas: only the HP cells take the colour; the name and the
    // status letter stay white.
    let font = block_font();
    let c = c16x9();
    let mut party = the_mockup_party();
    party[2].status = b'P';
    let state = party_state(&party);
    let ribbon = ribbon_snapshot(&state);
    let mut rgba = blank(&c);
    paint_ribbon(&mut rgba, &c, &ribbon, &font, true, false);
    let layout = ribbon_layout(&c, false);
    for (i, hp) in [(0, RULE_WHITE), (1, HP_LOW), (2, HP_POISONED)] {
        let (x, y) = layout.place(i);
        assert_eq!(rgb_at(&rgba, &c, x, y), ega(RULE_WHITE), "name {i}");
        assert_eq!(rgb_at(&rgba, &c, x + 8, y), ega(RULE_WHITE), "colon {i}");
        assert_eq!(rgb_at(&rgba, &c, x + 4 * 8, y), ega(hp), "hp {i}");
        assert_eq!(
            rgb_at(&rgba, &c, x + 5 * 8, y),
            ega(RULE_WHITE),
            "status {i}"
        );
    }
    let mut off = blank(&c);
    paint_ribbon(&mut off, &c, &ribbon, &font, false, false);
    let (x, y) = layout.place(1);
    assert_eq!(rgb_at(&off, &c, x + 4 * 8, y), ega(RULE_WHITE));
}

// --- The tabs. ---------------------------------------------------------------

#[test]
fn both_tabs_share_one_centre_in_both_states() {
    let state = party_state(&the_mockup_party());
    let snapshot = band_snapshot(&state);
    let sky = snapshot.gaps.top_rect().expect("the sky tab");
    let wind = snapshot.gaps.bottom_rect().expect("the wind tab");
    // In the frame the wind caption sits 4 pixels right of the sky tab's
    // centre (x 40..151 against 48..151).
    assert_eq!((sky.x0, sky.x1, wind.x0, wind.x1), (40, 151, 48, 151));
    // (canvas, open, centre, sky left, wind left): the mockups' strips at
    // x 192 / 264 (16:9) and 280 / 352 (21:9), sky tab 8 and wind tab 16
    // pixels into the strip, the wind strip 4 pixels left.
    for (c, open, centre, sky_x, wind_x) in [
        (c16x9(), false, 256, 200, 204),
        (c16x9(), true, 328, 272, 276),
        (c21x9(), false, 344, 288, 292),
        (c21x9(), true, 416, 360, 364),
        (canvas(1920, 1080), false, 240, 184, 188),
    ] {
        let got = tab_centre(&c, open, sky.width() as u32, &snapshot.ribbon);
        assert_eq!(got, centre, "{} {open}", c.w);
        let (s, w) = (tab_left(sky, got), tab_left(wind, got));
        assert_eq!((s, w), (sky_x, wind_x), "{} {open}", c.w);
        assert_eq!(2 * s + sky.width() as u32, 2 * w + wind.width() as u32);
    }
}

#[test]
fn the_tabs_are_the_frames_own_pixels() {
    let state = party_state(&the_mockup_party());
    let snapshot = band_snapshot(&state);
    let c = c16x9();
    // A frame whose every pixel differs from its neighbours.
    let frame: Vec<u8> = (0..320 * 200)
        .flat_map(|i: usize| {
            let [r, g, b] = EGA_PALETTE_RGB[(i * 7 + i / 320) % 16];
            [r, g, b, 0xff]
        })
        .collect();
    let mut rgba = blank(&c);
    let font = block_font();
    paint_bands(&mut rgba, &c, &frame, &snapshot, &font, true, false);
    let pixel = |buf: &[u8], w: usize, x: usize, y: usize| {
        let i = (y * w + x) * 4;
        [buf[i], buf[i + 1], buf[i + 2]]
    };
    for (rect, y0, x0) in [
        (snapshot.gaps.top_rect().unwrap(), 8, 200),
        (snapshot.gaps.bottom_rect().unwrap(), 232, 204),
    ] {
        for row in 0..8 {
            for col in 0..rect.width() {
                assert_eq!(
                    pixel(&rgba, 512, x0 + col, y0 + row),
                    pixel(&frame, 320, rect.x0 + col, rect.y0 + row)
                );
            }
        }
        // Beside the tab, the band and the rule are the canvas's own.
        assert_eq!(pixel(&rgba, 512, x0 - 1, y0 + 1), ega(BAND_BLUE));
        assert_eq!(pixel(&rgba, 512, x0 + rect.width(), y0 + 1), ega(BAND_BLUE));
    }
    // With no tab in the frame (a gap the chrome pass leaves out), the
    // band and the rule are unbroken.
    let none = BandSnapshot {
        ribbon: snapshot.ribbon.clone(),
        gaps: u5_runtime::ChromeGapExtents::default(),
    };
    let mut plain = blank(&c);
    paint_bands(&mut plain, &c, &frame, &none, &font, true, false);
    assert_eq!(pixel(&plain, 512, 256, 15), ega(RULE_WHITE));
    assert_eq!(pixel(&plain, 512, 256, 232), ega(RULE_WHITE));
    assert_eq!(pixel(&plain, 512, 256, 10), ega(BAND_BLUE));
}

#[test]
fn a_large_counter_stops_the_tab_short_on_a_narrow_canvas() {
    let state = party_state(&the_mockup_party());
    let mut snapshot = band_snapshot(&state);
    let sky = snapshot.gaps.top_rect().unwrap();
    // 1080-line open: the visible map's centre is x 312, which would put
    // the tab (x 256..367) over "F:9999 G:9999" (x 360..463).
    let c = canvas(1920, 1080);
    snapshot.ribbon.counters = "F:9999 G:9999".to_string();
    let centre = tab_centre(&c, true, sky.width() as u32, &snapshot.ribbon);
    assert_eq!(tab_centre_wanted(&c, true), 312);
    assert_eq!(centre, 360 - 56);
    assert!(tab_left(sky, centre) + 112 <= counters_left(&c, &snapshot.ribbon));
    // The ordinary values leave it centred.
    snapshot.ribbon.counters = "F:63 G:150".to_string();
    assert_eq!(
        tab_centre(&c, true, sky.width() as u32, &snapshot.ribbon),
        312
    );
    // Narrow canvases keep the tab right of the six-member ribbon.
    let narrow = canvas(1920, 1200);
    assert_eq!(narrow.w, 448);
    let centre = tab_centre(&narrow, false, sky.width() as u32, &snapshot.ribbon);
    assert!(tab_left(sky, centre) >= 176, "{centre}");
}

// --- Timing (D2.11). -------------------------------------------------------------

#[test]
fn the_snapshot_is_retaken_only_when_the_faithful_frame_changes() {
    let settings = u5_runtime::enhancements::DisplaySettings::default();
    let mut step = GrowLeftCompose::of(&settings);
    let mut state = party_state(&the_mockup_party());
    assert_eq!(step.bands_for(&state, 1).ribbon.members[1].hp, "5");
    // The game moves on, but the faithful frame is the same: the ribbon
    // keeps what the screen shows.
    state.party[1].hp = 40;
    state.repaint_stats_panel();
    assert_eq!(step.bands_for(&state, 1).ribbon.members[1].hp, "5");
    // The frame is re-presented: now it follows.
    assert_eq!(step.bands_for(&state, 2).ribbon.members[1].hp, "40");
}
