//! The crawl in the shell (design 2.2.4, 2.2.9, 2.3.8, 2.4.3): present only
//! with `display.grow_left`, its rows on the faithful frame where the
//! original window placed them, ageing over a walk, a keyword exchange
//! (the prompt row only while live, nothing kept after a submit, the clock
//! frozen), the presented canvas drawing it, and the game left alone with
//! the crawl on.

use bevy::window::WindowResolution;
use u5_runtime::enhancements::{
    CliSetting, DISPLAY_P1, DISPLAY_P2, DISPLAY_P8, HostCapabilities, LockState, ResolveInput,
    resolve_as_shipped,
};
use u5_runtime::test_fixtures::{open_grid, synthetic_tile_atlas, test_state};
use u5_runtime::{
    CH_FONT_LEN, IBM_CH_FILE, MESSAGE_WINDOW_RIGHT, TileGraphicsDepth, parse_ch_font,
};

use super::*;
use crate::display::crawl::layer::LineKind;
use crate::enh::VisualLaunchConfig;
use crate::enh::equivalence::{Scene, Shell, Timeline, check_shell_equivalence};

fn launch(cli: &[CliSetting]) -> VisualLaunchConfig {
    VisualLaunchConfig {
        enhancements: resolve_as_shipped(
            ResolveInput {
                stored: None,
                cli,
                lock: LockState::Off,
                caps: HostCapabilities::ALL,
            },
            &[DISPLAY_P1, DISPLAY_P2, DISPLAY_P8],
        ),
        ..VisualLaunchConfig::classic()
    }
}

fn grow_on() -> VisualLaunchConfig {
    launch(&[CliSetting::Enable("display.grow_left".into())])
}

/// A font whose glyphs all have ink and differ, so the frame checks mean
/// something.
fn scene() -> Scene {
    let mut bytes = vec![0u8; CH_FONT_LEN];
    for code in 0..CH_FONT_LEN / 8 {
        let c = code as u8;
        let rows = [c | 1, !c, c | 0x80, c, !c | 1, c ^ 0x55, c | 0x10, 0x81];
        bytes[code * 8..code * 8 + 8].copy_from_slice(&rows);
    }
    let font = parse_ch_font(&bytes, IBM_CH_FILE).unwrap();
    Scene {
        game_dir: ".".into(),
        atlas: synthetic_tile_atlas(TileGraphicsDepth::Ega16),
        text_font: font.clone(),
        rune_font: font,
    }
}

fn shell(launch: &VisualLaunchConfig, state: PlayState) -> Shell {
    let mut shell = Shell::new(state, &scene(), launch, |_| {});
    shell.app.world_mut().spawn(Window {
        resolution: WindowResolution::new(2560.0, 1440.0),
        ..default()
    });
    shell
}

fn faithful(shell: &Shell) -> Vec<u8> {
    let handle = shell.visual().image_handle.clone();
    shell
        .app
        .world()
        .resource::<Assets<Image>>()
        .get(&handle)
        .and_then(|image| image.data.clone())
        .unwrap()
}

fn crawl(shell: &Shell) -> CrawlRows {
    shell.app.world().resource::<CrawlState>().rows()
}

/// Every crawl line is on the faithful frame, on the row the window placed
/// it, its cells side by side as drawn.
fn assert_lines_on_the_frame(shell: &Shell, context: &str) {
    let frame = faithful(shell);
    let rows = crawl(shell);
    for line in &rows.lines {
        if line.cells.is_empty() {
            continue;
        }
        let on_frame = (MESSAGE_WINDOW_LEFT..=MESSAGE_WINDOW_RIGHT).any(|column| {
            line.cells
                .iter()
                .enumerate()
                .all(|(at, cell)| cell_shows(&frame, column + at as u8, line.frame_row, &cell.mask))
        });
        assert!(on_frame, "{context}: {:?} is not on the frame", line.text());
    }
    assert_eq!(
        shell.app.world().resource::<CrawlState>().unshown_rows(),
        0,
        "{context}"
    );
}

#[test]
fn the_crawl_exists_only_with_grow_left() {
    let off = shell(&launch(&[]), test_state(open_grid(), 5, 5));
    assert!(!off.app.world().contains_resource::<CrawlState>());
    let mut on = shell(&grow_on(), test_state(open_grid(), 5, 5));
    on.step(&[]);
    on.step(&[]);
    assert!(on.app.world().contains_resource::<CrawlState>());
    // The live row is there from the first frame.
    let rows = crawl(&on);
    assert!(rows.live().is_some(), "{rows:?}");
    assert!(rows.cursor.is_some());
    assert!(rows.cap.is_some());
}

#[test]
fn a_walk_fills_the_crawl_from_the_faithful_window_and_ages_it() {
    let mut shell = shell(&grow_on(), test_state(open_grid(), 5, 5));
    let mut timeline = Timeline::new(80);
    timeline
        .game_tap(5, KeyCode::ArrowUp)
        .game_tap(20, KeyCode::ArrowLeft)
        .game_tap(35, KeyCode::ArrowDown)
        .game_tap(50, KeyCode::ArrowRight)
        .game_tap(65, KeyCode::ArrowUp);
    let mut ages_seen = Vec::new();
    for frame in 0..timeline.len() {
        shell.step(timeline.frame(frame));
        assert_lines_on_the_frame(&shell, &format!("frame {frame}"));
        let rows = crawl(&shell);
        let texts: Vec<(String, u64, bool)> = rows
            .lines
            .iter()
            .filter(|line| line.kind == LineKind::Text)
            .map(|line| (line.text(), line.age, line.cells.iter().all(|c| c.typed)))
            .collect();
        ages_seen.push(texts);
    }
    let last = ages_seen.last().unwrap();
    // Five moves, each echoed as typed text, the newest fresh and the first
    // four turns old.
    let echoes: Vec<&(String, u64, bool)> = last.iter().filter(|(_, _, typed)| *typed).collect();
    assert!(echoes.len() >= 5, "{last:?}");
    let newest = echoes.last().unwrap();
    let first = echoes[echoes.len() - 5];
    assert_eq!(newest.1, 0, "{last:?}");
    assert_eq!(first.1, 4, "{last:?}");
    assert_eq!(
        shell.app.world().resource::<CrawlState>().clock(),
        shell.visual().state.turn
    );
}

#[test]
fn the_presented_canvas_draws_the_crawl() {
    let mut shell = shell(&grow_on(), test_state(open_grid(), 5, 5));
    let mut timeline = Timeline::new(20);
    timeline.game_tap(5, KeyCode::ArrowUp);
    for frame in 0..timeline.len() {
        shell.step(timeline.frame(frame));
    }
    let world = shell.app.world_mut();
    let mut sprites = world.query::<&Sprite>();
    let handle = sprites.single(world).unwrap().image.clone();
    let image = world.resource::<Assets<Image>>().get(&handle).unwrap();
    assert_eq!(image.size(), UVec2::new(512, 240));
    let rgba = image.data.clone().unwrap();
    // The fresh echo, bright cyan, somewhere in the crawl's rows; and the
    // cap's blue in the left border beside the live row.
    let cyan = EGA_PALETTE_RGB[11];
    let found = (184..232).any(|y| {
        (16..152).any(|x| {
            let i = (y * 512 + x) * 4;
            rgba[i..i + 3] == cyan
        })
    });
    assert!(found, "no bright cyan in the crawl");
    let rows = crawl(&shell);
    let cap = rows.cap.expect("the cap");
    for (row, pixels) in cap.iter().enumerate() {
        for (col, pixel) in pixels.iter().enumerate() {
            if let Some(index) = pixel {
                let i = ((224 + row) * 512 + 8 + col) * 4;
                assert_eq!(
                    rgba[i..i + 3],
                    EGA_PALETTE_RGB[usize::from(*index)],
                    "cap ({col}, {row})"
                );
            }
        }
    }
}

fn talking() -> PlayState {
    let mut state = test_state(open_grid(), 5, 5);
    crate::seed_visual_key_route_conversation(&mut state);
    state
}

#[test]
fn a_keyword_exchange_keeps_no_prompt_and_freezes_the_ages() {
    let mut shell = shell(&grow_on(), talking());
    shell.step(&[]);
    shell.step(&[]);
    assert_lines_on_the_frame(&shell, "greeting");
    let rows = crawl(&shell);
    let kinds: Vec<LineKind> = rows.lines.iter().map(|line| line.kind).collect();
    assert_eq!(
        kinds[kinds.len() - 2..],
        [LineKind::Prompt, LineKind::Live],
        "{rows:#?}"
    );
    let clock = shell.app.world().resource::<CrawlState>().clock();

    // Type a word: the live row shows it in the typed colour.
    let mut timeline = Timeline::new(40);
    timeline
        .game_tap(2, KeyCode::KeyJ)
        .game_tap(6, KeyCode::KeyO)
        .game_tap(10, KeyCode::KeyB);
    for frame in 0..timeline.len() {
        shell.step(timeline.frame(frame));
        assert_lines_on_the_frame(&shell, &format!("typing {frame}"));
    }
    let rows = crawl(&shell);
    let live = rows.live().expect("the live row");
    // The shell types upper case, as the original echoes it.
    assert!(live.text().eq_ignore_ascii_case(":job"), "{live:?}");
    assert_eq!(
        live.cells.iter().map(|c| c.typed).collect::<Vec<_>>(),
        [false, true, true, true]
    );

    // Submit: the prompt row above the committed word is gone; the word
    // stays, typed, without its colon.
    let mut timeline = Timeline::new(40);
    timeline.game_tap(2, KeyCode::Enter);
    for frame in 0..timeline.len() {
        shell.step(timeline.frame(frame));
        assert_lines_on_the_frame(&shell, &format!("submit {frame}"));
        let rows = crawl(&shell);
        for (index, line) in rows.lines.iter().enumerate() {
            if line.kind == LineKind::Prompt {
                assert_eq!(
                    rows.lines.get(index + 1).map(|below| below.kind),
                    Some(LineKind::Live),
                    "frame {frame}: {rows:#?}"
                );
            }
        }
    }
    let rows = crawl(&shell);
    let job = rows
        .lines
        .iter()
        .find(|line| line.kind == LineKind::Text && line.text().eq_ignore_ascii_case("job"))
        .unwrap_or_else(|| panic!("the committed word: {rows:#?}"));
    assert!(job.cells.iter().all(|cell| cell.typed));
    assert_eq!(job.cells[0].cell, 0);
    // The reply and the next prompt follow; the whole exchange stays fresh
    // while the conversation is open.
    assert!(shell.visual().state.active_conversation.is_some());
    let kinds: Vec<LineKind> = rows.lines.iter().map(|line| line.kind).collect();
    assert_eq!(kinds[kinds.len() - 2..], [LineKind::Prompt, LineKind::Live]);
    assert_eq!(shell.app.world().resource::<CrawlState>().clock(), clock);
    assert!(rows.lines.iter().all(|line| line.age == 0), "{rows:#?}");
}

/// A long run with the crawl on: walking, talking and typing.
fn long_timeline() -> Timeline {
    let mut timeline = Timeline::new(240);
    let keys = [
        KeyCode::ArrowUp,
        KeyCode::ArrowLeft,
        KeyCode::ArrowDown,
        KeyCode::ArrowRight,
        KeyCode::KeyJ,
        KeyCode::KeyO,
        KeyCode::KeyB,
        KeyCode::Enter,
        KeyCode::KeyB,
        KeyCode::KeyY,
        KeyCode::KeyE,
        KeyCode::Enter,
    ];
    for (index, key) in keys.iter().cycle().take(22).enumerate() {
        timeline.game_tap(5 + index * 10, *key);
    }
    timeline
}

#[test]
fn the_crawl_leaves_the_game_alone() {
    for start in [test_state(open_grid(), 5, 5), talking()] {
        for launch in [
            grow_on(),
            launch(&[
                CliSetting::Enable("display.grow_left".into()),
                CliSetting::Disable("display.crawl".into()),
                CliSetting::Disable("display.typed_colour".into()),
            ]),
        ] {
            let mismatches =
                check_shell_equivalence(&start, &scene(), &long_timeline(), &launch, |app| {
                    app.world_mut().spawn(Window {
                        resolution: WindowResolution::new(2560.0, 1440.0),
                        ..default()
                    });
                });
            assert_eq!(mismatches, []);
        }
    }
}

#[test]
fn the_transcript_is_unchanged_after_a_long_run_with_the_crawl_on() {
    let timeline = long_timeline();
    let mut on = shell(&grow_on(), talking());
    let mut off = shell(&VisualLaunchConfig::classic(), talking());
    for frame in 0..timeline.len() {
        on.step(timeline.frame(frame));
        off.step(timeline.frame(frame));
    }
    assert!(!on.visual().state.message_entries().is_empty());
    assert_eq!(
        on.visual().state.message_entries(),
        off.visual().state.message_entries()
    );
    assert_eq!(on.visual().state.turn, off.visual().state.turn);
    assert!(
        crawl(&on)
            .lines
            .iter()
            .any(|line| line.kind == LineKind::Text)
    );
}

fn strings(texts: &[&str]) -> Vec<String> {
    texts.iter().map(|text| text.to_string()).collect()
}

/// The open column's older rows line up with the placed rows (M1-13,
/// 2.3.8): the projection's tail runs into the placed rows, or the placed
/// rows sit whole in it with the live line's own row after them.
#[test]
fn older_rows_line_up_with_the_placed_rows() {
    let projected = strings(&["North", "Slow progress!", "North", "East", "North"]);
    // The tail is the start of the placed rows (more placed below it).
    assert_eq!(
        line_up(&projected, &strings(&["East", "North", "Looking..."])),
        Some(3)
    );
    // The longest overlap wins over a repeated row.
    assert_eq!(
        line_up(&projected, &strings(&["North", "East", "North"])),
        Some(2)
    );
    // The placed rows end above the projection's last row: the prompt the
    // live line shows (`Save game?`), or a row being completed.
    let quit = strings(&["East", "Pass", "Quit:", "Save game?"]);
    assert_eq!(line_up(&quit, &strings(&["Quit:"])), Some(2));
    let camp = strings(&[
        "Save game?",
        "Hole up & camp!",
        "For how many",
        "hours? (1-9)",
    ]);
    assert_eq!(
        line_up(&camp, &strings(&["Hole up & camp!", "For how many"])),
        Some(1)
    );
    // No more than two rows may follow.
    assert_eq!(line_up(&quit, &strings(&["East"])), None);
    // Nothing in common: no older rows rather than a guess.
    assert_eq!(line_up(&quit, &strings(&["Looking..."])), None);
}
