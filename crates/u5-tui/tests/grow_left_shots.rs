//! Phase B M1-09, M1-10, M1-12 and M1-13 review shots: the grow-left canvas
//! (chrome, ring, faithful core, the ribbon, the tabs and the message
//! crawl) for the approved mockup scenes, rendered by the engine's own frame
//! renderer and compositor next to the approved c04f mockups.
//!
//! The crawl (M1-12): the "quiet" mockups (A, G, G6) show only the live
//! prompt, which is the crawl with `display.crawl` off; the crawl mockups
//! (B, E) walk the scene's script one command at a time, syncing the crawl
//! after each as the shell does, so the rows age turn by turn. The rows are
//! the engine's own transcript of that walk (the mockups' rows are a
//! script of their own), so their text and ages are live data.
//!
//! The docked column (M1-13): the D mockups are the B walk with the column
//! open (Ctrl+P). The engine's open window shows the walk's rows on the
//! crawl's pixels and the transcript's older rows above them (live data);
//! the mockup's own seventeen rows (`HISTORY_B + SCRIPT_B`) are also drawn
//! through the engine's open-window paint over the engine's open canvas
//! for a pixel comparison.
//!
//! The G and G6 shots edit the party as the mockups do (Iolo poisoned; for
//! G6 three more members with the mockup's names and numbers) before the
//! frame is drawn, so the ribbon can be compared cell for cell.
//!
//! Review output only, never committed: the scene is rebuilt at test time
//! from a local game-data copy (`U5_MOCKUP_PROFILE`), exactly as the
//! mockups and the M1-07 ring test build it, and the shell's faithful frame
//! and its canvas come from `render_shell_frame_and_canvas_with_crawl`: the shell's
//! play-frame render and `compose_for_state`, the function the
//! `[grow-left compose]` step runs. Each shot is written to
//! `U5_GROW_SHOTS` as `<name>-engine.png`, next to a copy of the mockup and
//! a side-by-side strip, and the differing pixels are counted per region.
//! Skipped unless `U5_MOCKUP_PROFILE`, `U5_GROW_MOCKUPS` (the c04f folder)
//! and `U5_GROW_SHOTS` are set.

#![cfg(feature = "visual")]

use std::path::{Path, PathBuf};

use u5_bevy::display::crawl::layer::{CrawlCell, CrawlLine, LineKind};
use u5_bevy::display::crawl::sync::{CrawlState, glyph_mask};
use u5_bevy::display::grow::column::{ColumnText, ColumnView, page_mode_active, paint_open_window};
use u5_bevy::display::grow::crawl_paint::{CrawlPaint, paint_crawl};
use u5_bevy::display::grow::{
    Canvas, ComposeOptions, bands, compose, compose_for_state, render_shell_frame_and_canvas,
    render_shell_frame_and_canvas_with_column, render_shell_frame_and_canvas_with_crawl,
};
use u5_runtime::enhancements::PixelAspect;
use u5_runtime::*;
use u5_tui::{
    complete_headless_blocking_presentations, parse_start_arg, parse_time_arg,
    replay_play_script_commands, split_play_script,
};

const UP: &str = "\x1b[A";
const RIGHT: &str = "\x1b[C";

/// How the shot edits the save's party (the mockups' G and G6 states).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Party {
    /// The save's own three members.
    Save,
    /// Iolo poisoned (G).
    IoloPoisoned,
    /// Iolo poisoned and three more members (G6).
    Six,
}

/// What the shot draws of the message crawl.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Crawl {
    /// The live prompt only (`display.crawl` off): the quiet mockups.
    Quiet,
    /// The crawl, synced after every command of the walk.
    Walk,
    /// The walk, then a conversation (invented, as the mockup's is): the
    /// keyword prompt over the live row, a word being typed.
    Talk,
}

struct Shot {
    name: &'static str,
    mockup: &'static str,
    window: (u32, u32),
    time: &'static str,
    torch: Option<u8>,
    party: Party,
    crawl: Crawl,
    /// The shell's input line when the shot is drawn.
    typing: &'static str,
    /// The scene: `true` for S2, inside Britain.
    town: bool,
    /// The docked column open (Ctrl+P).
    column: bool,
    /// `display.centre_party` (M1-14).
    centre: bool,
}

const fn shot(name: &'static str, mockup: &'static str, window: (u32, u32), party: Party) -> Shot {
    Shot {
        name,
        mockup,
        window,
        time: "11:50",
        torch: None,
        party,
        crawl: Crawl::Quiet,
        typing: "",
        town: false,
        column: false,
        centre: false,
    }
}

const fn walk(name: &'static str, mockup: &'static str, window: (u32, u32)) -> Shot {
    Shot {
        crawl: Crawl::Walk,
        ..shot(name, mockup, window, Party::Save)
    }
}

const W16: (u32, u32) = (2560, 1440);
const W21: (u32, u32) = (3440, 1440);

const fn open(name: &'static str, mockup: &'static str, window: (u32, u32)) -> Shot {
    Shot {
        column: true,
        ..walk(name, mockup, window)
    }
}

const SHOTS: [Shot; 13] = [
    shot("S1-16x9", "c04f-A-S1-quiet.png", W16, Party::Save),
    shot("S1-21x9", "c04f-uw-A-S1-quiet.png", W21, Party::Save),
    walk("B-16x9", "c04f-B-S1-crawl.png", W16),
    walk("B-21x9", "c04f-uw-B-S1-crawl.png", W21),
    Shot {
        name: "S3-16x9",
        mockup: "c04f-E-S3-night.png",
        window: W16,
        time: "21:50",
        torch: Some(90),
        party: Party::Save,
        crawl: Crawl::Walk,
        typing: "",
        town: false,
        column: false,
        centre: false,
    },
    Shot {
        name: "C-16x9",
        mockup: "c04f-C-S2-talk.png",
        window: W16,
        time: "12:00",
        torch: None,
        party: Party::Save,
        crawl: Crawl::Talk,
        typing: "JO",
        town: true,
        column: false,
        centre: false,
    },
    shot("G-16x9", "c04f-G-status.png", W16, Party::IoloPoisoned),
    shot("G-21x9", "c04f-uw-G-status.png", W21, Party::IoloPoisoned),
    shot("G6-16x9", "c04f-G6-status.png", W16, Party::Six),
    shot("G6-21x9", "c04f-uw-G6-status.png", W21, Party::Six),
    open("D-16x9", "c04f-D-S1-open.png", W16),
    open("D-21x9", "c04f-uw-D-S1-open.png", W21),
    Shot {
        centre: true,
        ..open("D-centred-16x9", "c04f-D-centred.png", W16)
    },
];

/// The mockups' party edits (make_c04f.py: G_PARTY, G6_PARTY).
fn edit_party(state: &mut PlayState, party: Party) {
    if party == Party::Save {
        return;
    }
    state.party[2].status = b'P';
    if party == Party::Six {
        for (name, status, hp, max_hp) in [
            (&b"Jaana"[..], b'G', 18, 90),
            (&b"Dupre"[..], b'P', 120, 150),
            (&b"Geoffrey"[..], b'G', 240, 240),
        ] {
            let mut member = state.party[1];
            member.status = status;
            member.hp = hp;
            member.max_hp = max_hp;
            state.party.push(member);
            let mut bytes = [0; SAVE_CHARACTER_NAME_LEN];
            bytes[..name.len()].copy_from_slice(name);
            state.party_names.push(bytes);
        }
    }
    state.repaint_stats_panel();
}

/// The rows the crawl mockups script (make_c04f.py: `rows_of(SCRIPT_B,
/// NOW_B)` and `rows_of(SCRIPT_E, NOW_E)`): text, typed, age.
fn mockup_script(name: &str) -> Option<&'static [(&'static str, bool, u64)]> {
    const B: &[(&str, bool, u64)] = &[
        ("North", true, 4),
        ("East", true, 3),
        ("East", true, 2),
        ("Slow progress!", false, 2),
        ("North", true, 0),
    ];
    const E: &[(&str, bool, u64)] = &[
        ("North", true, 3),
        ("East", true, 2),
        ("East", true, 1),
        ("Slow progress!", false, 1),
        ("North", true, 0),
    ];
    // make_c04f.py `talk_rows(TALK_OPEN + TALK_1)`; the prompt row is
    // drawn by the live row's rule, so it is listed here as game text.
    const C: &[(&str, bool, u64)] = &[
        ("Talk-North", true, 0),
        ("A baker waves.", false, 0),
        ("bread", true, 0),
        ("\"Rye and honey!\"", false, 0),
        ("Your interest?", false, 0),
    ];
    // make_c04f.py `rows_of(HISTORY_B + SCRIPT_B, None)`: the open window's
    // seventeen rows, all fresh (the ages do not matter in the open window).
    const D: &[(&str, bool, u64)] = &[
        ("North", true, 0),
        ("North", true, 0),
        ("West", true, 0),
        ("Slow progress!", false, 0),
        ("North", true, 0),
        ("West", true, 0),
        ("West", true, 0),
        ("Slow progress!", false, 0),
        ("North", true, 0),
        ("North", true, 0),
        ("Slow progress!", false, 0),
        ("West", true, 0),
        ("North", true, 0),
        ("East", true, 0),
        ("East", true, 0),
        ("Slow progress!", false, 0),
        ("North", true, 0),
    ];
    match name {
        "D-16x9" | "D-21x9" | "D-centred-16x9" => Some(D),
        "B-16x9" | "B-21x9" => Some(B),
        "S3-16x9" => Some(E),
        "C-16x9" => Some(C),
        _ => None,
    }
}

fn scripted_line(ibm: &FixedCellFont, text: &str, typed: bool, age: u64) -> CrawlLine {
    CrawlLine {
        kind: LineKind::Text,
        age,
        cells: text
            .bytes()
            .enumerate()
            .map(|(cell, byte)| CrawlCell {
                cell: cell as u8,
                byte,
                mask: glyph_mask(ibm, byte),
                typed,
            })
            .collect(),
        frame_row: 0,
    }
}

fn env_dir(name: &str) -> Option<PathBuf> {
    Some(PathBuf::from(std::env::var_os(name)?))
}

/// The fonts the shell draws with.
struct Fonts {
    ibm: FixedCellFont,
    runes: FixedCellFont,
}

/// S1 and S3 of the mockups: the forest road west of Britain, walked
/// up, up, right, right, up. With `crawl`, one command at a time, the crawl
/// synced on the shell's frame after each.
fn load_scene(
    profile: &Path,
    shot: &Shot,
    atlas: &TileAtlas,
    crawl: Option<(&mut CrawlState, &Fonts, &Canvas)>,
) -> PlayState {
    let mut options = load_play_options_from_save(profile).unwrap();
    let (target, at) = if shot.town {
        ("TOWNE:1", "15,19")
    } else {
        ("BRITANNIA", "74,111")
    };
    options.target = PlayTarget::from_key(target).unwrap();
    options.saved_active_objects = None;
    options.start = Some(parse_start_arg(at).unwrap());
    options.clock = parse_time_arg(shot.time).unwrap();
    if let Some(torch) = shot.torch {
        options.torch_counter = torch;
    }
    let mut state = PlayState::load_scene(profile, options).unwrap();
    let script = if shot.town {
        [UP, UP].join(";")
    } else {
        [UP, UP, RIGHT, RIGHT, UP].join(";")
    };
    let commands = split_play_script(&script);
    let Some((crawl, fonts, canvas)) = crawl else {
        replay_play_script_commands(&mut state, profile, &commands, |_, _, _| Ok(())).unwrap();
        complete_headless_blocking_presentations(&mut state, Some(atlas)).unwrap();
        return state;
    };
    let frame = |state: &mut PlayState, crawl: &mut CrawlState| {
        let _ = render_shell_frame_and_canvas_with_crawl(
            state,
            atlas,
            (&fonts.ibm, &fonts.runes),
            profile,
            canvas,
            PeekLightPolicy::Scaled,
            crawl,
            true,
            "",
        );
    };
    frame(&mut state, crawl);
    for command in commands {
        replay_play_script_commands(&mut state, profile, &[command], |_, _, _| Ok(())).unwrap();
        complete_headless_blocking_presentations(&mut state, Some(atlas)).unwrap();
        frame(&mut state, crawl);
    }
    if shot.crawl == Crawl::Talk {
        // An invented exchange, as the mockup's is: the talk echo, the
        // conversation's greeting, then the keyword prompt.
        state.emit_command_echo_line("Talk-North");
        let lines = ["Baker", "a baker", "Rye and honey!", "I bake.", "Bye."];
        let raw = lines
            .iter()
            .map(|text| text.bytes().map(|byte| byte ^ TLK_TEXT_XOR_MASK).collect())
            .collect();
        let decoded = lines.iter().map(|text| text.to_string()).collect();
        state.active_conversation = Some(Box::new(
            u5_runtime::conversation_session::ConversationSession::new(raw, decoded),
        ));
        state.advance_active_conversation_greeting();
        state.flush_staged_narration();
        state.flush_text_hold();
        complete_headless_blocking_presentations(&mut state, Some(atlas)).unwrap();
        frame(&mut state, crawl);
        eprintln!(
            "{}: transcript tail {:?}, rows left out as unshown {}",
            shot.name,
            state
                .message_entries()
                .iter()
                .rev()
                .take(6)
                .map(|entry| entry.text.as_str())
                .collect::<Vec<_>>(),
            crawl.unshown_rows()
        );
    }
    state
}

fn save(path: &Path, w: u32, h: u32, rgba: Vec<u8>) {
    image::RgbaImage::from_raw(w, h, rgba)
        .unwrap()
        .save(path)
        .unwrap();
}

#[test]
fn grow_left_review_shots() {
    let (Some(profile), Some(mockups), Some(out)) = (
        env_dir("U5_MOCKUP_PROFILE"),
        env_dir("U5_GROW_MOCKUPS"),
        env_dir("U5_GROW_SHOTS"),
    ) else {
        eprintln!("skipped: set U5_MOCKUP_PROFILE, U5_GROW_MOCKUPS and U5_GROW_SHOTS");
        return;
    };
    std::fs::create_dir_all(&out).unwrap();
    let atlas = load_tile_atlas(&profile, TileGraphicsDepth::Ega16).unwrap();
    let fonts = Fonts {
        ibm: load_ibm_ch_font(&profile).unwrap(),
        runes: load_runes_ch_font(&profile).unwrap(),
    };
    let (ibm, runes) = (&fonts.ibm, &fonts.runes);
    for shot in &SHOTS {
        let canvas = Canvas::for_window(shot.window.0, shot.window.1, PixelAspect::Dos).unwrap();
        let mut crawl = CrawlState::default();
        let walked = (shot.crawl != Crawl::Quiet).then_some((&mut crawl, &fonts, &canvas));
        let mut state = load_scene(&profile, shot, &atlas, walked);
        edit_party(&mut state, shot.party);
        // The shell's own frame and canvas: the bytes the play upload
        // sites write, in the shell's palette, with the crawl (or only the
        // live row, for the quiet mockups).
        let column = ColumnView {
            open: shot.column,
            ..ColumnView::default()
        };
        let (frame, ours) = render_shell_frame_and_canvas_with_column(
            &mut state,
            &atlas,
            (ibm, runes),
            &profile,
            &canvas,
            PeekLightPolicy::Scaled,
            &mut crawl,
            shot.crawl != Crawl::Quiet,
            shot.typing,
            column,
            shot.centre,
        );
        if let (true, Some(script)) = (shot.column, mockup_script(shot.name)) {
            // The mockup's own rows through the engine's open-window paint,
            // over the engine's open canvas of the same frame without text.
            let snapshot = bands::band_snapshot(&state);
            let options = ComposeOptions {
                light: PeekLightPolicy::Scaled,
                bands: Some(compose::Bands {
                    snapshot: &snapshot,
                    font: ibm,
                    hp_colours: true,
                }),
                crawl: None,
                column,
                centre_party: shot.centre,
            };
            let mut scripted = compose_for_state(&frame, &state, &atlas, &canvas, options)
                .expect("the top-down view");
            let mut rows = crawl.rows();
            let live = rows.lines.pop().expect("the live row");
            rows.older.clear();
            rows.lines = script
                .iter()
                .map(|(text, typed, age)| scripted_line(ibm, text, *typed, *age))
                .collect();
            rows.lines.push(live);
            paint_open_window(
                &mut scripted,
                &canvas,
                ColumnText {
                    rows: &rows,
                    typed_colour: true,
                },
            );
            let mockup = image::open(mockups.join(shot.mockup)).unwrap().to_rgba8();
            let differing = scripted
                .chunks(4)
                .zip(mockup.as_raw().chunks(4))
                .filter(|(a, b)| a[..3] != b[..3])
                .count();
            eprintln!(
                "{}: the mockup's rows through the engine's open column: {differing} differing pixels",
                shot.name
            );
            save(
                &out.join(format!("{}-script-engine.png", shot.name)),
                canvas.w,
                canvas.h,
                scripted,
            );
        } else if let Some(script) = mockup_script(shot.name) {
            // The mockup's own rows, drawn by the engine's step 7 over the
            // engine's quiet canvas of the same scene: the paint itself,
            // compared pixel for pixel.
            let (_, base) = render_shell_frame_and_canvas(
                &mut state.clone(),
                &atlas,
                (ibm, runes),
                &profile,
                &canvas,
                PeekLightPolicy::Scaled,
            );
            let mut scripted = base.expect("the top-down view");
            let mut rows = crawl.rows();
            let mut live = rows.lines.pop().expect("the live row");
            rows.lines = script
                .iter()
                .map(|(text, typed, age)| scripted_line(ibm, text, *typed, *age))
                .collect();
            if shot.crawl == Crawl::Talk {
                // The mockup's live row: `:` in white, `jo` typed.
                live = scripted_line(ibm, ":jo", true, 0);
                live.kind = LineKind::Live;
                live.cells[0].typed = false;
                if let Some(cursor) = rows.cursor.as_mut() {
                    cursor.cell = 3;
                }
                let prompt = rows.lines.last_mut().unwrap();
                prompt.kind = LineKind::Prompt;
            }
            rows.lines.push(live);
            paint_crawl(
                &mut scripted,
                &canvas,
                CrawlPaint {
                    rows: &rows,
                    crawl: true,
                    typed_colour: true,
                },
            );
            let mockup = image::open(mockups.join(shot.mockup)).unwrap().to_rgba8();
            let differing = scripted
                .chunks(4)
                .zip(mockup.as_raw().chunks(4))
                .filter(|(a, b)| a[..3] != b[..3])
                .count();
            eprintln!(
                "{}: the mockup's script through the engine's crawl paint: {differing} differing pixels",
                shot.name
            );
            save(
                &out.join(format!("{}-script-engine.png", shot.name)),
                canvas.w,
                canvas.h,
                scripted,
            );
        }
        if shot.crawl != Crawl::Quiet {
            let rows = crawl.rows();
            for line in &rows.older {
                eprintln!("{}: older {:?}", shot.name, line.text());
            }
            for line in &rows.lines {
                eprintln!(
                    "{}: crawl {:?} age {} {:?}",
                    shot.name,
                    line.kind,
                    line.age,
                    line.text()
                );
            }
        }
        let ours = ours.expect("the top-down view");
        let (w, h) = (canvas.w, canvas.h);
        let mockup = image::open(mockups.join(shot.mockup)).unwrap().to_rgba8();
        assert_eq!((mockup.width(), mockup.height()), (w, h), "{}", shot.name);
        let theirs = mockup.into_raw();

        // Count differing pixels per region: the chrome bands (the ribbon
        // and the tabs), the core, the ring (where the crawl of a later
        // package goes).
        let (cx, cy, side) = compose::core_rect(&canvas.for_column(column, shot.centre));
        let (mx0, my0, mx1, my1) = canvas.map_window();
        let mut diff = vec![0u8; (w * h * 4) as usize];
        let (mut bands, mut core, mut ring) = (0, 0, 0);
        for y in 0..h {
            for x in 0..w {
                let i = ((y * w + x) * 4) as usize;
                let same = ours[i..i + 3] == theirs[i..i + 3];
                let px = if same {
                    // Dimmed context.
                    [ours[i] / 4, ours[i + 1] / 4, ours[i + 2] / 4, 0xff]
                } else {
                    [0xff, 0x00, 0xff, 0xff]
                };
                diff[i..i + 4].copy_from_slice(&px);
                if same {
                    continue;
                }
                let in_core = (cx..cx + side).contains(&(x as usize))
                    && (cy..cy + side).contains(&(y as usize));
                let in_map = (mx0..=mx1).contains(&x) && (my0..=my1).contains(&y);
                if in_core {
                    core += 1;
                } else if in_map {
                    ring += 1;
                } else {
                    bands += 1;
                }
            }
        }
        eprintln!(
            "{}: {}x{} canvas; differing pixels: bands {bands}, core {core}, ring {ring}",
            shot.name, w, h
        );
        save(
            &out.join(format!("{}-engine.png", shot.name)),
            w,
            h,
            ours.clone(),
        );
        save(
            &out.join(format!("{}-mockup.png", shot.name)),
            w,
            h,
            theirs.clone(),
        );
        save(&out.join(format!("{}-diff.png", shot.name)), w, h, diff);
        // Side by side, mockup above the engine, 2x.
        let mut strip = image::RgbaImage::new(w * 2, h * 4 + 8);
        for (row, rgba) in [(0, &theirs), (1, &ours)] {
            for y in 0..h * 2 {
                for x in 0..w * 2 {
                    let i = (((y / 2) * w + x / 2) * 4) as usize;
                    strip.put_pixel(
                        x,
                        y + row * (h * 2 + 8),
                        image::Rgba([rgba[i], rgba[i + 1], rgba[i + 2], 0xff]),
                    );
                }
            }
        }
        strip
            .save(out.join(format!("{}-side-by-side.png", shot.name)))
            .unwrap();
    }
}

/// The pages of M1-14's render check (2.2.5, D2.20).
#[derive(Clone, Copy, Debug)]
enum Page {
    ZStats,
    Picker,
    ArmsSell,
    InnRegister,
}

/// Put the original's right column on `page`, as the command that opens
/// it would. The shop pages are set on the session directly (the scene
/// stands on the road, not in a shop), so their text is what the panel
/// paints for that session state.
fn open_page(state: &mut PlayState, page: Page) {
    use u5_runtime::shop_runtime::{ArmsSellBrowser, ArmsShopState, InnkeeperState};
    use u5_runtime::shop_session::ActiveShopSession;
    match page {
        Page::ZStats => {
            state.emit_command_echo_line("Ztats");
            state.z_stats_for_party(0);
        }
        Page::Picker => {
            state.emit_command_echo_line("Use item");
            state.start_use_item();
        }
        Page::ArmsSell => {
            state.active_shop = Some(ActiveShopSession::Arms(ArmsShopState::SellPickItem(
                ArmsSellBrowser {
                    selected: 0,
                    page_start: 0,
                },
            )));
        }
        Page::InnRegister => {
            let mut guest_indices = [0; u5_runtime::shops::INN_REGISTRY_CAP];
            guest_indices[1] = 1;
            state.active_shop = Some(ActiveShopSession::Innkeeper(
                InnkeeperState::PickUpCompanion {
                    inn: u5_runtime::shops::Inn::TheWayfarerInn,
                    guest_indices,
                    guest_count: 2,
                    base_lodging_charge: 20,
                    highlight: 0,
                },
            ));
        }
    }
    state.repaint_stats_panel();
}

/// `rgba` (w x h) at 2x over the faithful frame at 2x, for review.
fn save_with_frame(path: &Path, (w, h): (u32, u32), rgba: &[u8], frame: &[u8]) {
    let mut strip = image::RgbaImage::from_pixel(
        w.max(320) * 2,
        h * 2 + 8 + 200 * 2,
        image::Rgba([0x40, 0x40, 0x40, 0xff]),
    );
    for (top, (iw, ih), bytes) in [(0, (w, h), rgba), (h * 2 + 8, (320, 200), frame)] {
        for y in 0..ih * 2 {
            for x in 0..iw * 2 {
                let i = (((y / 2) * iw + x / 2) * 4) as usize;
                strip.put_pixel(
                    x,
                    top + y,
                    image::Rgba([bytes[i], bytes[i + 1], bytes[i + 2], 0xff]),
                );
            }
        }
    }
    strip.save(path).unwrap();
}

/// M1-14's render check (not mocked): the walk of the B mockup on the
/// named layouts and the narrow ones, the column collapsed, open, and open
/// with `display.centre_party`; then each page over the same walk with the
/// player's column collapsed. Each render is written at 2x over the
/// faithful frame it was composed from (`render-<layout>-<state>.png`).
#[test]
fn grow_left_page_mode_and_layout_renders() {
    let (Some(profile), Some(_), Some(out)) = (
        env_dir("U5_MOCKUP_PROFILE"),
        env_dir("U5_GROW_MOCKUPS"),
        env_dir("U5_GROW_SHOTS"),
    ) else {
        eprintln!("skipped: set U5_MOCKUP_PROFILE, U5_GROW_MOCKUPS and U5_GROW_SHOTS");
        return;
    };
    std::fs::create_dir_all(&out).unwrap();
    let atlas = load_tile_atlas(&profile, TileGraphicsDepth::Ega16).unwrap();
    let fonts = Fonts {
        ibm: load_ibm_ch_font(&profile).unwrap(),
        runes: load_runes_ch_font(&profile).unwrap(),
    };
    let layouts = [
        ("16x9", W16),
        ("21x9", W21),
        ("1080", (1920, 1080)),
        ("16x10", (1920, 1200)),
        ("4x3", (1600, 1200)),
    ];
    for (layout, window) in layouts {
        let canvas = Canvas::for_window(window.0, window.1, PixelAspect::Dos).unwrap();
        let shot = walk("render", "", window);
        let mut crawl = CrawlState::default();
        let state = load_scene(&profile, &shot, &atlas, Some((&mut crawl, &fonts, &canvas)));
        let mut render = |name: String, mut state: PlayState, open: bool, centre: bool| {
            let (frame, ours) = render_shell_frame_and_canvas_with_column(
                &mut state,
                &atlas,
                (&fonts.ibm, &fonts.runes),
                &profile,
                &canvas,
                PeekLightPolicy::Scaled,
                &mut crawl,
                true,
                "",
                ColumnView {
                    open,
                    ..ColumnView::default()
                },
                centre,
            );
            let ours = ours.expect("the top-down view");
            eprintln!(
                "{name}: {}x{} canvas, pan {} tiles, page {}",
                canvas.w,
                canvas.h,
                canvas.pan_tiles,
                page_mode_active(&state)
            );
            save_with_frame(
                &out.join(format!("render-{name}.png")),
                (canvas.w, canvas.h),
                &ours,
                &frame,
            );
        };
        for (name, open, centre) in [
            ("collapsed", false, false),
            ("open", true, false),
            ("centred", true, true),
        ] {
            render(format!("{layout}-{name}"), state.clone(), open, centre);
        }
        for page in [
            Page::ZStats,
            Page::Picker,
            Page::ArmsSell,
            Page::InnRegister,
        ] {
            let mut paged = state.clone();
            open_page(&mut paged, page);
            complete_headless_blocking_presentations(&mut paged, Some(&atlas)).unwrap();
            assert!(page_mode_active(&paged), "{page:?}");
            render(format!("{layout}-page-{page:?}"), paged, false, false);
        }
    }
}
