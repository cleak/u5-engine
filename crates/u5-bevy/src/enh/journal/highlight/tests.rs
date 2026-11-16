//! The `[highlight]` step (design 8.6: T2 faithful frame, T3b absent when
//! off; build plan M6-05) and its review shot. Every blob is synthetic:
//! invented words in the `formats/tlk.md` layout.

use std::collections::HashMap;

use bevy::input::ButtonState;
use u5_runtime::enhancements::{Toggle, fnv1a64};
use u5_runtime::test_fixtures::{
    configured_test_dir, copy_asset_writable, fix_host_clock_for_tests, open_grid, test_state,
    unique_scratch_dir,
};
use u5_runtime::{
    CH_FONT_LEN, Direction, IBM_CH_FILE, MESSAGE_WINDOW_BOTTOM, MESSAGE_WINDOW_LEFT,
    MESSAGE_WINDOW_RIGHT, MESSAGE_WINDOW_TOP, NpcSlot, parse_ch_font,
};

use super::*;
use crate::enh::equivalence::{Owner, Scene, Shell, TimedKey};
use crate::enh::present::PresentHost;

const DIALOG_ID: u8 = 0x10;

/// Bit 7 set, as the file stores text.
fn text(s: &str) -> Vec<u8> {
    s.bytes().map(|b| b | 0x80).collect()
}

/// An invented person whose second answer names the first topic.
fn fields() -> Vec<Vec<u8>> {
    vec![
        text("Quorvel"),
        text("a dusty quillwright"),
        text("Hail."),
        text("I mend quills."),
        text("Fare well."),
        text("gran"),
        text("Old one by the mill, seek the torq."),
        text("mill"),
        text("Wheels turn."),
        text("torq"),
        text("Brass."),
    ]
}

fn decoded(fields: &[Vec<u8>]) -> Vec<String> {
    fields
        .iter()
        .map(|field| field.iter().map(|b| char::from(b & 0x7F)).collect())
        .collect()
}

/// A font whose letters are distinct patterns with a clear bottom row, the
/// space blank: enough for the cell check to tell glyphs apart.
fn pattern_font() -> FixedCellFont {
    let mut bytes = vec![0u8; CH_FONT_LEN];
    for code in 0x21..0x80usize {
        for row in 0..7 {
            bytes[code * CELL + row] = (code as u8).wrapping_mul(37).rotate_left(row as u32) | 0x10;
        }
    }
    parse_ch_font(&bytes, IBM_CH_FILE).unwrap()
}

fn launch(highlight: Option<&str>, style: &str, clues: bool) -> VisualLaunchConfig {
    let mut cli = Vec::new();
    if let Some(mode) = highlight {
        cli.push(u5_runtime::enhancements::CliSetting::Set {
            key: "journal.clue_highlight".into(),
            value: mode.into(),
        });
    }
    cli.push(u5_runtime::enhancements::CliSetting::Set {
        key: "journal.clue_highlight_style".into(),
        value: style.into(),
    });
    let mut launch = VisualLaunchConfig {
        enhancements: u5_runtime::enhancements::resolve_as_shipped(
            u5_runtime::enhancements::ResolveInput {
                stored: None,
                cli: &cli,
                lock: u5_runtime::enhancements::LockState::Off,
                caps: u5_runtime::enhancements::HostCapabilities::ALL,
            },
            &[
                u5_runtime::enhancements::CLUES_P1A,
                u5_runtime::enhancements::CLUES_P1C,
            ],
        ),
        ..VisualLaunchConfig::classic()
    };
    let toggles = &mut launch.enhancements.toggles;
    *toggles = FeatureToggles::CLASSIC;
    toggles.set(HIGHLIGHT, highlight.is_some_and(|mode| mode != "off"));
    toggles.set(Toggle::JournalClues, clues);
    launch
}

/// A town with the person east of the party, talking already.
fn talking_state() -> PlayState {
    let mut state = test_state(open_grid(), 5, 5);
    state.player.facing = Direction::East;
    state.load_scheduled_npcs(&[
        NpcSlot {
            slot: 0,
            type_byte: 0,
            dialog_id: 0,
            schedule: [0; 16],
            name: None,
        },
        NpcSlot {
            slot: 1,
            type_byte: 1,
            dialog_id: DIALOG_ID,
            schedule: [0, 0, 0, 6, 6, 6, 5, 5, 5, 0, 0, 0, 0, 8, 16, 20],
            name: None,
        },
    ]);
    let fields = fields();
    let dialogue = HashMap::from([(u16::from(DIALOG_ID), decoded(&fields))]);
    let raw = HashMap::from([(u16::from(DIALOG_ID), fields)]);
    assert!(state.open_conversation_session(&dialogue, &raw).is_some());
    state
}

fn key(key: KeyCode, state: ButtonState) -> TimedKey {
    TimedKey {
        key,
        state,
        repeat: false,
        owner: Owner::Game,
    }
}

fn letter(c: char) -> KeyCode {
    match c {
        'a' => KeyCode::KeyA,
        'b' => KeyCode::KeyB,
        'g' => KeyCode::KeyG,
        'i' => KeyCode::KeyI,
        'j' => KeyCode::KeyJ,
        'l' => KeyCode::KeyL,
        'm' => KeyCode::KeyM,
        'n' => KeyCode::KeyN,
        'o' => KeyCode::KeyO,
        'r' => KeyCode::KeyR,
        _ => panic!("no key for {c:?}"),
    }
}

/// Types `line` and Enter, then lets the answer print. Returns the
/// faithful frame hash of every frame.
fn say(shell: &mut Shell, line: &str) -> Vec<u64> {
    let mut hashes = Vec::new();
    let codes = line.chars().map(letter).chain([KeyCode::Enter]);
    for code in codes {
        hashes.push(shell.step(&[key(code, ButtonState::Pressed)]).frame);
        hashes.push(shell.step(&[key(code, ButtonState::Released)]).frame);
    }
    for _ in 0..20 {
        hashes.push(shell.step(&[]).frame);
    }
    hashes
}

fn faithful(shell: &Shell) -> Vec<u8> {
    let world = shell.app.world();
    world
        .resource::<Assets<Image>>()
        .get(&shell.visual().image_handle)
        .unwrap()
        .data
        .clone()
        .unwrap()
}

fn presented(shell: &Shell) -> Option<Vec<u8>> {
    let world = shell.app.world();
    let handle = world.get_resource::<PresentHost>()?.presented()?.clone();
    world.resource::<Assets<Image>>().get(&handle)?.data.clone()
}

fn steps(shell: &Shell) -> Vec<StepOrder> {
    shell
        .app
        .world()
        .get_resource::<PresentSteps>()
        .map(PresentSteps::orders)
        .unwrap_or_default()
}

fn scene() -> Scene {
    let font = pattern_font();
    Scene {
        text_font: font.clone(),
        rune_font: font,
        ..Scene::synthetic()
    }
}

/// Window 2's pixels (`systems/text-output.md §10.1`).
fn in_window(x: usize, y: usize) -> bool {
    let cells = |a: u8, b: u8| usize::from(a) * CELL..(usize::from(b) + 1) * CELL;
    cells(MESSAGE_WINDOW_LEFT, MESSAGE_WINDOW_RIGHT).contains(&x)
        && cells(MESSAGE_WINDOW_TOP, MESSAGE_WINDOW_BOTTOM).contains(&y)
}

/// The pixels where `a` and `b` differ, as `(x, y)`.
fn differing(a: &[u8], b: &[u8]) -> Vec<(usize, usize)> {
    (0..FRAME.0 * FRAME.1)
        .filter(|i| a[i * 4..i * 4 + 4] != b[i * 4..i * 4 + 4])
        .map(|i| (i % FRAME.0, i / FRAME.0))
        .collect()
}

// ---------------------------------------------------------------------------
// T2: the faithful frame, and marks only inside window 2.

#[test]
fn t2_faithful_frames_are_equal_and_marks_stay_in_window_two() {
    // Two shells are compared frame by frame: a host-clock reseed in the
    // conversation must read the same time in both (as T1 does).
    fix_host_clock_for_tests();
    let mut off = Shell::new(
        talking_state(),
        &scene(),
        &launch(None, "dotted", false),
        |_| {},
    );
    let mut on = Shell::new(
        talking_state(),
        &scene(),
        &launch(Some("asked"), "dotted", false),
        |_| {},
    );
    for shell in [&mut off, &mut on] {
        for _ in 0..10 {
            shell.step(&[]);
        }
    }
    for line in ["mill", "gran"] {
        assert_eq!(
            say(&mut off, line),
            say(&mut on, line),
            "faithful frames of `{line}`"
        );
    }
    assert_eq!(faithful(&off), faithful(&on));
    assert_eq!(fnv1a64(&faithful(&off)), fnv1a64(&faithful(&on)));
    assert!(presented(&off).is_none(), "no host without a step");

    let plan = on.app.world().resource::<HighlightFeed>().plan();
    let marks: Vec<Mark> = plan.cells.iter().map(|cell| cell.mark).collect();
    assert_eq!(
        marks,
        [Mark::Same; 4],
        "`mill` in the second answer: {plan:?}"
    );
    let shown = presented(&on).expect("the step presents a copy");
    let changed = differing(&faithful(&on), &shown);
    assert!(!changed.is_empty(), "the marks are drawn");
    assert!(
        changed
            .iter()
            .all(|&(x, y)| in_window(x, y) && y % CELL == CELL - 1),
        "only bottom pixel rows of window 2 change: {changed:?}"
    );
}

#[test]
fn t2_the_tint_style_repaints_only_the_marked_cells() {
    let mut on = Shell::new(
        talking_state(),
        &scene(),
        &launch(Some("new"), "tint", false),
        |_| {},
    );
    for _ in 0..10 {
        on.step(&[]);
    }
    say(&mut on, "mill");
    say(&mut on, "gran");
    let plan = on.app.world().resource::<HighlightFeed>().plan();
    assert!(plan.cells.iter().any(|cell| cell.mark == Mark::Same));
    assert!(
        plan.cells.iter().any(|cell| cell.mark == Mark::New),
        "`torq` is a topic not yet asked"
    );
    let shown = presented(&on).unwrap();
    let changed = differing(&faithful(&on), &shown);
    let marked = |x: usize, y: usize| {
        plan.cells
            .iter()
            .any(|cell| usize::from(cell.column) == x / CELL && usize::from(cell.row) == y / CELL)
    };
    assert!(!changed.is_empty());
    assert!(
        changed
            .iter()
            .all(|&(x, y)| in_window(x, y) && marked(x, y))
    );
}

// ---------------------------------------------------------------------------
// The cell check.

fn painted_frame(rows: &[(u8, &str)], font: &FixedCellFont) -> Vec<u8> {
    let mut rgba = [0, 0, 0, 0xFF].repeat(FRAME.0 * FRAME.1);
    for (row, line) in rows {
        for (offset, b) in line.bytes().enumerate() {
            paint_fixed_cell_glyph(
                &mut rgba,
                FRAME.0,
                FRAME.1,
                font,
                b,
                MESSAGE_WINDOW_LEFT + offset as u8,
                *row,
                CHROME_RULE_INDEX,
            );
        }
    }
    rgba
}

fn plan_for(
    word: &str,
    row: u8,
    column: u8,
    font: &FixedCellFont,
    style: HighlightStyle,
) -> HighlightPlan {
    HighlightPlan {
        revision: 1,
        style,
        cells: word
            .bytes()
            .enumerate()
            .map(|(i, b)| PlanCell {
                column: column + i as u8,
                row,
                mark: Mark::MayDiffer,
                glyph: glyph_mask(font, b),
            })
            .collect(),
    }
}

fn copy_of(rgba: &[u8]) -> PresentedFrame {
    PresentedFrame {
        size: UVec2::new(FRAME.0 as u32, FRAME.1 as u32),
        rgba: rgba.to_vec(),
        ..PresentedFrame::default()
    }
}

#[test]
fn a_cell_that_differs_from_the_faithful_image_marks_nothing() {
    let font = pattern_font();
    let size = UVec2::new(FRAME.0 as u32, FRAME.1 as u32);
    // The recomputed rows say `mill` where a shop screen painted `sell`.
    let faithful = painted_frame(&[(20, "the sell")], &font);
    let plan = plan_for(
        "mill",
        20,
        MESSAGE_WINDOW_LEFT + 4,
        &font,
        HighlightStyle::Dotted,
    );
    let mut frame = copy_of(&faithful);
    assert!(!paint_plan(&plan, &faithful, size, &mut frame));
    assert_eq!(frame.rgba, faithful);

    let faithful = painted_frame(&[(20, "the mill")], &font);
    let mut frame = copy_of(&faithful);
    assert!(paint_plan(&plan, &faithful, size, &mut frame));
    let changed = differing(&faithful, &frame.rgba);
    // Dashed: three pixels of every four along the word's bottom row.
    assert_eq!(changed.len(), 4 * CELL * 3 / 4);
    assert!(changed.iter().all(|&(_, y)| y == 20 * CELL + CELL - 1));
    let pen6 = pen(6);
    let (x, y) = changed[0];
    assert_eq!(frame.rgba[(y * FRAME.0 + x) * 4..][..4], pen6);
}

#[test]
fn a_frame_of_another_size_is_left_alone() {
    let font = pattern_font();
    let faithful = painted_frame(&[(20, "mill")], &font);
    let plan = plan_for("mill", 20, MESSAGE_WINDOW_LEFT, &font, HighlightStyle::Tint);
    let mut frame = PresentedFrame::default();
    frame.resize(UVec2::new(640, 400));
    let before = frame.clone();
    assert!(!paint_plan(
        &plan,
        &faithful,
        UVec2::new(320, 200),
        &mut frame
    ));
    assert_eq!(frame, before);
}

// ---------------------------------------------------------------------------
// T3b: absent when off, and following the key in play.

#[test]
fn t3b_the_step_and_its_plan_exist_only_while_the_key_is_on() {
    let off = Shell::new(
        talking_state(),
        &scene(),
        &launch(None, "dotted", true),
        |_| {},
    );
    assert!(!off.app.world().contains_resource::<HighlightFeed>());
    assert!(!steps(&off).contains(&StepOrder::HIGHLIGHT));

    let classic = Shell::new(
        talking_state(),
        &scene(),
        &VisualLaunchConfig::classic(),
        |_| {},
    );
    assert!(!classic.app.world().contains_resource::<HighlightFeed>());
    assert!(!classic.app.world().contains_resource::<PresentSteps>());

    let mut on = Shell::new(
        talking_state(),
        &scene(),
        &launch(Some("asked"), "dotted", true),
        |_| {},
    );
    assert!(on.app.world().contains_resource::<HighlightFeed>());
    assert_eq!(steps(&on), [StepOrder::HIGHLIGHT]);
    let mut toggles = on.app.world().resource::<Toggles>().0;
    toggles.set(HIGHLIGHT, false);
    on.app.world_mut().resource_mut::<Toggles>().0 = toggles;
    on.step(&[]);
    on.step(&[]);
    assert!(!on.app.world().contains_resource::<HighlightFeed>());
    assert!(steps(&on).is_empty());
    assert!(
        presented(&on).is_none(),
        "the sprite shows the faithful image again"
    );
    toggles.set(HIGHLIGHT, true);
    on.app.world_mut().resource_mut::<Toggles>().0 = toggles;
    on.step(&[]);
    assert!(on.app.world().contains_resource::<HighlightFeed>());
    assert_eq!(steps(&on), [StepOrder::HIGHLIGHT]);
}

#[test]
fn the_revision_changes_only_with_the_marked_cells() {
    let mut on = Shell::new(
        talking_state(),
        &scene(),
        &launch(Some("asked"), "dotted", false),
        |_| {},
    );
    for _ in 0..10 {
        on.step(&[]);
    }
    let revision = |shell: &Shell| {
        shell
            .app
            .world()
            .resource::<HighlightFeed>()
            .plan()
            .revision
    };
    let before = revision(&on);
    say(&mut on, "mill");
    assert_eq!(revision(&on), before, "nothing asked shows yet");
    say(&mut on, "gran");
    let marked = revision(&on);
    assert_ne!(marked, before);
    for _ in 0..10 {
        on.step(&[]);
    }
    assert_eq!(revision(&on), marked, "a still window keeps its revision");
}

// ---------------------------------------------------------------------------
// Review shot (build plan M6-05 exit): window 2 of a real conversation in
// each mode. Review output only, never committed: skipped unless
// `U5_MOCKUP_PROFILE` names a game copy and `U5_HIGHLIGHT_SHOTS` a folder.

fn scaled(rgba: &[u8], n: u32) -> image::RgbaImage {
    let picture = image::RgbaImage::from_raw(320, 200, rgba.to_vec()).unwrap();
    image::imageops::resize(
        &picture,
        320 * n,
        200 * n,
        image::imageops::FilterType::Nearest,
    )
}

#[test]
fn highlight_review_shots() {
    use u5_runtime::enhancements::clues::{RecordAddr, record_dependence, topic_keyword};
    use u5_runtime::{GameClock, PlayOptions, PlayTarget, Scene as GameScene};
    let (Some(profile), Some(out)) = (
        configured_test_dir("U5_MOCKUP_PROFILE"),
        configured_test_dir("U5_HIGHLIGHT_SHOTS"),
    ) else {
        eprintln!("skipped: set U5_MOCKUP_PROFILE and U5_HIGHLIGHT_SHOTS");
        return;
    };
    let game = unique_scratch_dir("highlight-shots");
    for entry in std::fs::read_dir(&profile).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            copy_asset_writable(&entry.path(), &game.join(entry.file_name())).unwrap();
        }
    }
    std::fs::create_dir_all(&out).unwrap();
    // A person whose Job answer names one of their own topics: that topic
    // is put in the log as asked, then JOB is typed, so the answer shows
    // the word asked.
    let towne = u5_runtime::parse_tlk_raw(&game.join("TOWNE.TLK")).unwrap();
    let mut ids: Vec<u16> = towne.keys().copied().collect();
    ids.sort_unstable();
    let decode = |field: &[u8]| -> String { field.iter().map(|b| char::from(b & 0x7F)).collect() };
    // A person with no labelled block whose answer to one topic (`ask`)
    // names another of their topics (`k`), which goes in the log as asked.
    let (pick, ask, k) = ids
        .iter()
        .find_map(|id| {
            let fields = &towne[id];
            // A greeting that opens no prompt of its own.
            if !record_dependence(fields, RecordAddr::Leading(2)).renders_statically() {
                return None;
            }
            let topics: Vec<(u8, String)> = (0..16u8)
                .filter_map(|k| {
                    let word =
                        String::from_utf8(topic_keyword(fields, RecordAddr::Pair(k))?).ok()?;
                    (word.len() >= 3 && word.bytes().all(|b| b.is_ascii_alphabetic()))
                        .then_some((k, word))
                })
                .collect();
            topics.iter().find_map(|(ask, asked_word)| {
                let answer = decode(RecordAddr::Pair(*ask).field().and_then(|f| fields.get(f))?)
                    .to_ascii_uppercase();
                topics
                    .iter()
                    .find(|(k, word)| {
                        k != ask
                            && answer.contains(word.as_str())
                            && record_dependence(fields, RecordAddr::Pair(*ask))
                                .renders_statically()
                    })
                    .map(|(k, _)| (*id, asked_word.to_ascii_lowercase(), *k))
            })
        })
        .expect("an answer that names another topic");
    let scene = Scene::from_game_dir(&game).unwrap();
    let mut state = PlayState::load_scene(
        &game,
        PlayOptions {
            target: PlayTarget::Town(GameScene::from_key("TOWNE:1").unwrap()),
            floor: 0,
            start: Some((15, 17)),
            clock: GameClock::new(10, 0).unwrap(),
            ..PlayOptions::default()
        },
    )
    .unwrap();
    state.player.facing = Direction::East;
    let (x, y) = (state.player.x as u8, state.player.y as u8);
    let now = u5_runtime::enhancements::clues::game_minute(&state).0;
    let asked = ClueEntry::heard(
        u5_runtime::enhancements::clues::NpcKey { scene: 1, slot: 1 },
        DialogKey {
            family: 0,
            dialog_id: pick as u8,
        },
        RecordAddr::Pair(k),
        0,
        u5_runtime::enhancements::clues::TestBits::default(),
        u5_runtime::enhancements::clues::GameMinute(now - 10),
    );
    state.load_scheduled_npcs(&[NpcSlot {
        slot: 1,
        type_byte: 1,
        dialog_id: pick as u8,
        schedule: [0, 0, 0, x + 1, x + 1, x + 1, y, y, y, 0, 0, 0, 0, 8, 16, 20],
        name: None,
    }]);
    let code = |c: char| match c.to_ascii_lowercase() {
        c @ 'a'..='z' => {
            let codes = [
                KeyCode::KeyA,
                KeyCode::KeyB,
                KeyCode::KeyC,
                KeyCode::KeyD,
                KeyCode::KeyE,
                KeyCode::KeyF,
                KeyCode::KeyG,
                KeyCode::KeyH,
                KeyCode::KeyI,
                KeyCode::KeyJ,
                KeyCode::KeyK,
                KeyCode::KeyL,
                KeyCode::KeyM,
                KeyCode::KeyN,
                KeyCode::KeyO,
                KeyCode::KeyP,
                KeyCode::KeyQ,
                KeyCode::KeyR,
                KeyCode::KeyS,
                KeyCode::KeyT,
                KeyCode::KeyU,
                KeyCode::KeyV,
                KeyCode::KeyW,
                KeyCode::KeyX,
                KeyCode::KeyY,
                KeyCode::KeyZ,
            ];
            codes[(c as u8 - b'a') as usize]
        }
        _ => KeyCode::Space,
    };
    let tap = |shell: &mut Shell, k: KeyCode| {
        shell.step(&[key(k, ButtonState::Pressed)]);
        shell.step(&[key(k, ButtonState::Released)]);
    };
    for (name, mode, style) in [
        ("00-off", None, "dotted"),
        ("01-asked-dotted", Some("asked"), "dotted"),
        (
            "02-asked-elsewhere-dotted",
            Some("asked_elsewhere"),
            "dotted",
        ),
        ("03-new-dotted", Some("new"), "dotted"),
        ("04-new-tint", Some("new"), "tint"),
    ] {
        let mut shell = Shell::new(state.clone(), &scene, &launch(mode, style, true), |_| {});
        for _ in 0..30 {
            shell.step(&[]);
        }
        shell
            .app
            .world_mut()
            .resource_mut::<ClueLogResource>()
            .log_mut()
            .fold(asked);
        tap(&mut shell, KeyCode::KeyT);
        tap(&mut shell, KeyCode::ArrowRight);
        for _ in 0..40 {
            shell.step(&[]);
        }
        for c in ask.chars() {
            tap(&mut shell, code(c));
        }
        tap(&mut shell, KeyCode::Enter);
        for _ in 0..60 {
            shell.step(&[]);
        }
        let plain = faithful(&shell);
        // The faithful frame of a real town is not the same from one launch
        // to the next (the people walk), so the shot proves only that the
        // marks stay inside window 2; T2 above compares the faithful frames.
        image::RgbaImage::from_raw(320, 200, plain.clone())
            .unwrap()
            .save(out.join(format!("{name}-faithful.png")))
            .unwrap();
        let shown = presented(&shell).unwrap_or_else(|| plain.clone());
        let changed = differing(&plain, &shown);
        assert!(changed.iter().all(|&(x, y)| in_window(x, y)), "{name}");
        let cells = shell
            .app
            .world()
            .get_resource::<HighlightFeed>()
            .map_or(0, |feed| feed.plan().cells.len());
        eprintln!(
            "{name}: {cells} marked cells, {} pixels changed",
            changed.len()
        );
        let picture = image::RgbaImage::from_raw(320, 200, shown.clone()).unwrap();
        picture.save(out.join(format!("{name}.png"))).unwrap();
        scaled(&shown, 4)
            .save(out.join(format!("{name}-4x.png")))
            .unwrap();
        let window =
            image::imageops::crop_imm(&scaled(&shown, 6), 184 * 6, 80 * 6, 136 * 6, 120 * 6)
                .to_image();
        window
            .save(out.join(format!("{name}-window-6x.png")))
            .unwrap();
    }
}
