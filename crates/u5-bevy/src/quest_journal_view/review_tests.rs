//! The M5-03 review shots (build plan M5 point 1): the empty page and the
//! Virtues section, drawn by the real `--visual` shell with the page on,
//! beside the approved c04f look. Review only: skipped unless
//! `U5_MOCKUP_PROFILE` names a game copy and `U5_JOURNAL_SHOTS` a folder;
//! nothing is committed. `U5_C04F_SHOT` may name the c04f picture to put
//! beside them.
//!
//! The Virtues shot uses a demo progress (three complete, two in
//! progress, one ruined, as in the 5.2.4 sketch), set on the loaded state
//! before the page opens.

use bevy::input::ButtonState;
use u5_runtime::test_fixtures::{configured_test_dir, copy_asset_writable, unique_scratch_dir};
use u5_runtime::{GameClock, PlayOptions, PlayState, PlayTarget, Scene as GameScene};

use super::*;
use crate::enh::equivalence::{Owner, Scene, Shell, TimedKey};

fn key(key: KeyCode, state: ButtonState) -> TimedKey {
    TimedKey {
        key,
        state,
        repeat: false,
        owner: Owner::Feature,
    }
}

fn rgba_image(w: u32, h: u32, rgba: Vec<u8>) -> image::RgbaImage {
    image::RgbaImage::from_raw(w, h, rgba).unwrap()
}

/// The 320x200 picture at `n` times, rows stretched by the display's 1.2.
fn display(picture: &image::RgbaImage, n: u32) -> image::RgbaImage {
    image::imageops::resize(
        picture,
        picture.width() * n,
        picture.height() * n * 6 / 5,
        image::imageops::FilterType::Nearest,
    )
}

fn scaled(picture: &image::RgbaImage, n: u32) -> image::RgbaImage {
    image::imageops::resize(
        picture,
        picture.width() * n,
        picture.height() * n,
        image::imageops::FilterType::Nearest,
    )
}

/// `left` and `right` side by side on black, bottoms aligned.
fn beside(left: &image::RgbaImage, right: &image::RgbaImage) -> image::RgbaImage {
    let gap = 16;
    let h = left.height().max(right.height());
    let mut sheet = image::RgbaImage::from_pixel(
        left.width() + gap + right.width(),
        h,
        image::Rgba([0, 0, 0, 255]),
    );
    image::imageops::overlay(&mut sheet, left, 0, i64::from(h - left.height()));
    image::imageops::overlay(
        &mut sheet,
        right,
        i64::from(left.width() + gap),
        i64::from(h - right.height()),
    );
    sheet
}

#[test]
fn journal_page_review_shots() {
    let (Some(profile), Some(out)) = (
        configured_test_dir("U5_MOCKUP_PROFILE"),
        configured_test_dir("U5_JOURNAL_SHOTS"),
    ) else {
        eprintln!("skipped: set U5_MOCKUP_PROFILE and U5_JOURNAL_SHOTS");
        return;
    };
    let game = unique_scratch_dir("journal-review-shots");
    for entry in std::fs::read_dir(&profile).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            copy_asset_writable(&entry.path(), &game.join(entry.file_name())).unwrap();
        }
    }
    std::fs::create_dir_all(&out).unwrap();
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
    // Demo progress: Compassion and Valor in progress; Honesty, Sacrifice
    // and Humility complete; Spirituality's shrine in ruin.
    state.shrine_ordained_mask = 0b0000_0110;
    state.shrine_codex_mask = 0b1001_0011;
    state.shrine_ruin_flags[6] = 0x80;

    let mut shell = Shell::new(state, &scene, &page_launch_for_shots(), |_| {});
    for _ in 0..30 {
        shell.step(&[]);
    }
    let play = {
        let world = shell.app.world();
        let image = world
            .resource::<Assets<Image>>()
            .get(&shell.visual().image_handle)
            .unwrap();
        rgba_image(320, 200, image.data.clone().unwrap())
    };
    shell.step(&[
        key(KeyCode::ControlLeft, ButtonState::Pressed),
        key(KeyCode::KeyJ, ButtonState::Pressed),
    ]);
    shell.step(&[
        key(KeyCode::KeyJ, ButtonState::Released),
        key(KeyCode::ControlLeft, ButtonState::Released),
    ]);
    shell.step(&[]);
    let world = shell.app.world();
    assert_eq!(
        world.resource::<OverlayState>().open(),
        Some(OverlayId::Journal),
        "the page opened"
    );
    let ui = world.resource::<QuestJournalUi>();
    let virtues = rgba_image(
        320,
        200,
        world
            .resource::<Assets<Image>>()
            .get(ui.page_image().unwrap())
            .unwrap()
            .data
            .clone()
            .unwrap(),
    );
    eprintln!("header: {}", ui.model().unwrap().header);

    let mut empty = vec![0; PAGE_WIDTH * PAGE_HEIGHT * 4];
    paint_page(&mut empty, &scene.text_font, &PageView::new(None, None));
    let empty = rgba_image(320, 200, empty);

    for (name, picture) in [
        ("01-empty-page", &empty),
        ("02-virtues", &virtues),
        ("00-play-frame-under", &play),
    ] {
        picture.save(out.join(format!("{name}.png"))).unwrap();
        scaled(picture, 4)
            .save(out.join(format!("{name}-4x.png")))
            .unwrap();
        display(picture, 3)
            .save(out.join(format!("{name}-display-3x.png")))
            .unwrap();
    }
    if let Some(c04f) = std::env::var_os("U5_C04F_SHOT") {
        let c04f = image::open(c04f).unwrap().to_rgba8();
        // c04f is a 512x240 canvas of 1.2-aspect pixels; both at 2x.
        let c04f = image::imageops::resize(
            &c04f,
            c04f.width() * 2,
            c04f.height() * 2,
            image::imageops::FilterType::Nearest,
        );
        for (name, picture) in [("01-empty-page", &empty), ("02-virtues", &virtues)] {
            beside(&c04f, &display(picture, 2))
                .save(out.join(format!("{name}-beside-c04f.png")))
                .unwrap();
        }
    }
}

fn page_launch_for_shots() -> VisualLaunchConfig {
    let mut launch = VisualLaunchConfig::classic();
    launch.enhancements.toggles.set(Toggle::JournalPage, true);
    launch
}

fn tap_key(shell: &mut Shell, code: KeyCode) {
    shell.step(&[key(code, ButtonState::Pressed)]);
    shell.step(&[key(code, ButtonState::Released)]);
}

fn page_picture(shell: &Shell) -> image::RgbaImage {
    let world = shell.app.world();
    let ui = world.resource::<QuestJournalUi>();
    rgba_image(
        320,
        200,
        world
            .resource::<Assets<Image>>()
            .get(ui.page_image().unwrap())
            .unwrap()
            .data
            .clone()
            .unwrap(),
    )
}

/// A snapshot with something in every section, for the paint-only shots.
fn demo_snapshot() -> QuestSnapshot {
    use u5_runtime::quest_journal::{CompanionFact, CompanionPlace, ShadowlordState};
    let mut snap = QuestSnapshot::from_play_state(&u5_runtime::test_fixtures::test_state(
        u5_runtime::test_fixtures::open_grid(),
        5,
        5,
    ));
    snap.ordained_mask = 0b0000_0110;
    snap.codex_mask = 0b1001_0011;
    snap.ruin[6] = 0x80;
    snap.ruin[3] = 0x01;
    snap.seals[0] = 0x80;
    snap.seals[4] = 0x80;
    snap.shadowlords = [
        ShadowlordState::Vanquished,
        ShadowlordState::AtLarge,
        ShadowlordState::AtLarge,
    ];
    snap.special_items[u5_runtime::SPECIAL_ITEM_SHARD_FALSEHOOD_INDEX + 1] = 1;
    snap.special_items[u5_runtime::SPECIAL_ITEM_CROWN_LB_INDEX] = 1;
    snap.special_items[u5_runtime::SPECIAL_ITEM_SEXTANT_INDEX] = 1;
    snap.climbing_gear = 1;
    snap.companions = ["Avatar", "Iolo", "Shamino", "Dupre", "Katrina", "Jaana"]
        .iter()
        .enumerate()
        .map(|(at, name)| CompanionFact {
            name: (*name).to_string(),
            place: match at {
                0 | 1 => CompanionPlace::Party,
                3 => CompanionPlace::Executed,
                _ => CompanionPlace::Lodged { scene: 2 },
            },
        })
        .collect();
    snap.introduced = (1..=16).map(|scene| (scene, scene % 5 + 1)).collect();
    snap.moral_standing = 42;
    snap
}

#[test]
fn journal_page_content_review_shots() {
    let (Some(profile), Some(out)) = (
        configured_test_dir("U5_MOCKUP_PROFILE"),
        configured_test_dir("U5_JOURNAL_SHOTS"),
    ) else {
        eprintln!("skipped: set U5_MOCKUP_PROFILE and U5_JOURNAL_SHOTS");
        return;
    };
    let game = unique_scratch_dir("journal-content-shots");
    for entry in std::fs::read_dir(&profile).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            copy_asset_writable(&entry.path(), &game.join(entry.file_name())).unwrap();
        }
    }
    std::fs::create_dir_all(&out).unwrap();
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
    state.shrine_ordained_mask = 0b0000_0110;
    state.shrine_codex_mask = 0b1001_0011;
    state.shrine_ruin_flags[6] = 0x80;
    state.shrine_ruin_flags[3] = 0x01;

    let mut shots: Vec<(String, image::RgbaImage)> = Vec::new();
    for tier in [SpoilerTier::Goals, SpoilerTier::Full] {
        let mut shell = Shell::new(state.clone(), &scene, &page_launch_for_shots(), |_| {});
        shell
            .app
            .world_mut()
            .resource_mut::<QuestJournalUi>()
            .settings
            .tier = tier;
        for _ in 0..30 {
            shell.step(&[]);
        }
        shell.step(&[
            key(KeyCode::ControlLeft, ButtonState::Pressed),
            key(KeyCode::KeyJ, ButtonState::Pressed),
        ]);
        shell.step(&[
            key(KeyCode::KeyJ, ButtonState::Released),
            key(KeyCode::ControlLeft, ButtonState::Released),
        ]);
        // Down to Valor (tier 2) or Compassion, revealed (tier 4).
        let downs = if tier == SpoilerTier::Full { 1 } else { 2 };
        for _ in 0..downs {
            tap_key(&mut shell, KeyCode::ArrowDown);
        }
        if tier == SpoilerTier::Full {
            tap_key(&mut shell, KeyCode::Enter);
        }
        shell.step(&[]);
        shots.push((
            format!("03-virtues-tier{}", tier.number()),
            page_picture(&shell),
        ));
        if tier == SpoilerTier::Full {
            tap_key(&mut shell, KeyCode::ArrowDown);
            shell.step(&[]);
            shots.push(("04-virtues-tier4-masked-row".into(), page_picture(&shell)));
        }
    }

    // The other sections, painted from a demo snapshot with the real font.
    let snap = demo_snapshot();
    let paint_one = |tier: SpoilerTier, id: SectionId, cursor: usize, revealed: &[RowKey]| {
        let model = derive(&snap, tier);
        let section = model.section(id).unwrap();
        let mut view = PageView::new(Some(&model), Some(section));
        view.cursor = cursor;
        view.scroll = cursor.saturating_sub(VISIBLE_ROWS - 1);
        view.revealed = revealed;
        let mut rgba = vec![0; PAGE_WIDTH * PAGE_HEIGHT * 4];
        paint_page(&mut rgba, &scene.text_font, &view);
        rgba_image(320, 200, rgba)
    };
    shots.push((
        "05-dungeons-tier4".into(),
        paint_one(SpoilerTier::Full, SectionId::Dungeons, 0, &[]),
    ));
    shots.push((
        "06-shadowlords-tier2".into(),
        paint_one(SpoilerTier::Goals, SectionId::Shadowlords, 1, &[]),
    ));
    shots.push((
        "07-regalia-tier2".into(),
        paint_one(SpoilerTier::Goals, SectionId::Regalia, 0, &[]),
    ));
    shots.push((
        "08-items-tier2".into(),
        paint_one(SpoilerTier::Goals, SectionId::Items, 1, &[]),
    ));
    shots.push((
        "09-people-tier2-scrolled".into(),
        paint_one(SpoilerTier::Goals, SectionId::People, 18, &[]),
    ));
    shots.push((
        "10-endgame-tier3".into(),
        paint_one(SpoilerTier::Hints, SectionId::Endgame, 0, &[]),
    ));
    shots.push((
        "11-shadowlords-tier4-revealed".into(),
        paint_one(
            SpoilerTier::Full,
            SectionId::Shadowlords,
            0,
            &[RowKey::Shadowlord(0)],
        ),
    ));
    for (name, picture) in &shots {
        picture.save(out.join(format!("{name}.png"))).unwrap();
        scaled(picture, 4)
            .save(out.join(format!("{name}-4x.png")))
            .unwrap();
    }
}

/// The M5-05 review shots: the side panel beside the play frame, both
/// drawn by the real shell (the journal provider's rows through the
/// foundation's cell-panel renderer, in the game's IBM.CH), on the right
/// (`auto`) and on the left, at 1x and 4x. Same skip rule as above.
#[test]
fn journal_side_panel_review_shots() {
    use crate::enh::side::{
        PanelContent, PanelKind, PanelRegistry, PanelSide, SidePanelProvider, render_cell_panel,
    };
    let (Some(profile), Some(out)) = (
        configured_test_dir("U5_MOCKUP_PROFILE"),
        configured_test_dir("U5_JOURNAL_SHOTS"),
    ) else {
        eprintln!("skipped: set U5_MOCKUP_PROFILE and U5_JOURNAL_SHOTS");
        return;
    };
    let game = unique_scratch_dir("journal-panel-shots");
    for entry in std::fs::read_dir(&profile).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            copy_asset_writable(&entry.path(), &game.join(entry.file_name())).unwrap();
        }
    }
    std::fs::create_dir_all(&out).unwrap();
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
    let fresh_state = state.clone();
    state.shrine_ordained_mask = 0b0000_0110;
    state.shrine_codex_mask = 0b1001_0011;
    state.shrine_ruin_flags[6] = 0x80;
    state.shrine_ruin_flags[3] = 0x01;

    let mut launch = VisualLaunchConfig::classic();
    launch
        .enhancements
        .toggles
        .set(Toggle::JournalSidePanel, true);
    for (name, state) in [
        ("12-panel-fresh", fresh_state),
        ("13-panel-progress", state),
    ] {
        let mut shell = Shell::new(state, &scene, &launch, |_| {});
        for _ in 0..30 {
            shell.step(&[]);
        }
        let world = shell.app.world();
        let play = world
            .resource::<Assets<Image>>()
            .get(&shell.visual().image_handle)
            .unwrap()
            .data
            .clone()
            .unwrap();
        let registry = world.resource::<PanelRegistry>();
        assert_eq!(registry.claims()[0].kind, PanelKind::Journal);
        // What the registered provider hands the host.
        let provider = JournalPanelProvider::new(Default::default(), Default::default());
        let Some(PanelContent::Cells(rows)) = provider.snapshot(Some(&shell.visual().state)) else {
            panic!("the journal panel shows rows");
        };
        let play = rgba_image(320, 200, play);
        for side in [PanelSide::Right, PanelSide::Left] {
            let panel = rgba_image(
                crate::enh::side::PANEL_WIDTH,
                crate::enh::side::PANEL_HEIGHT,
                render_cell_panel(&scene.text_font, &rows, side),
            );
            let mut sheet = image::RgbaImage::new(320 + panel.width(), 200);
            let (frame_x, panel_x) = match side {
                PanelSide::Right => (0, 320),
                PanelSide::Left => (panel.width(), 0),
            };
            image::imageops::overlay(&mut sheet, &play, i64::from(frame_x), 0);
            image::imageops::overlay(&mut sheet, &panel, i64::from(panel_x), 0);
            let side = match side {
                PanelSide::Right => "right",
                PanelSide::Left => "left",
            };
            sheet.save(out.join(format!("{name}-{side}.png"))).unwrap();
            scaled(&sheet, 4)
                .save(out.join(format!("{name}-{side}-4x.png")))
                .unwrap();
            display(&sheet, 3)
                .save(out.join(format!("{name}-{side}-display-3x.png")))
                .unwrap();
        }
    }
}
