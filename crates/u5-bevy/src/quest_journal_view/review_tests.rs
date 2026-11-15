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

/// The M6-03 review shots (build plan M6 point 5): the Clues tab (People,
/// one group open) beside c04f's tab strip, the strip with the Quests tab,
/// the Topics tab, the Place view, a search and the side panel's clue
/// lines. Names and keywords are read live from the profile's own `.TLK`
/// files; nothing is committed.
#[test]
fn clue_tabs_review_shots() {
    use crate::enh::journal::clues::ClueLogResource;
    use crate::enh::side::{PanelContent, PanelSide, SidePanelProvider, render_cell_panel};
    use u5_runtime::enhancements::clues::{
        ClueEntry, DialogKey, EntryFlags, GameMinute, NpcKey, RecordAddr, TestBits, game_minute,
    };
    let (Some(profile), Some(out)) = (
        configured_test_dir("U5_MOCKUP_PROFILE"),
        configured_test_dir("U5_JOURNAL_SHOTS"),
    ) else {
        eprintln!("skipped: set U5_MOCKUP_PROFILE and U5_JOURNAL_SHOTS");
        return;
    };
    let game = unique_scratch_dir("clue-tab-shots");
    for entry in std::fs::read_dir(&profile).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            copy_asset_writable(&entry.path(), &game.join(entry.file_name())).unwrap();
        }
    }
    std::fs::create_dir_all(&out).unwrap();
    // Three dialogs of the town family with at least three topics, by id.
    let towne = u5_runtime::parse_tlk_raw(&game.join("TOWNE.TLK")).unwrap();
    let mut ids: Vec<u16> = towne
        .iter()
        .filter(|(_, fields)| fields.len() >= 11)
        .map(|(id, _)| *id)
        .collect();
    ids.sort_unstable();
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
    state.talk_branch_flags.insert(2, 1 << 5);
    let now = game_minute(&state).0;
    let mut entries = Vec::new();
    for (person, id) in ids.iter().take(3).enumerate() {
        let npc = NpcKey {
            scene: 1,
            slot: 2 + person as u8,
        };
        let dialog = DialogKey {
            family: 0,
            dialog_id: *id as u8,
        };
        let at = |minutes: u32| GameMinute(now - 2000 * (3 - person as u32) + minutes);
        if person == 1 {
            entries.push(ClueEntry::heard(
                npc,
                dialog,
                RecordAddr::Leading(0),
                0,
                TestBits::default(),
                at(0),
            ));
        }
        for (k, minutes) in [(0, 1), (1, 3), (2, 7)] {
            let mut entry = ClueEntry::heard(
                npc,
                dialog,
                RecordAddr::Pair(k),
                0,
                TestBits::default(),
                at(minutes),
            );
            if person == 1 && k == 1 {
                entry.count = 2;
                entry.flags.set(EntryFlags::RUNIC, true);
            }
            entries.push(entry);
        }
    }

    let shell_with = |toggles: &[Toggle]| {
        let mut launch = VisualLaunchConfig::classic();
        for toggle in toggles {
            launch.enhancements.toggles.set(*toggle, true);
        }
        let mut shell = Shell::new(state.clone(), &scene, &launch, |_| {});
        for _ in 0..30 {
            shell.step(&[]);
        }
        {
            let mut log = shell.app.world_mut().resource_mut::<ClueLogResource>();
            for entry in &entries {
                log.log_mut().fold(*entry);
            }
            // The player starred one and read the first person's topics.
            log.log_mut().set_flag(2, EntryFlags::STAR, true);
        }
        shell.step(&[]);
        shell.step(&[
            key(KeyCode::ControlLeft, ButtonState::Pressed),
            key(KeyCode::KeyJ, ButtonState::Pressed),
        ]);
        shell.step(&[
            key(KeyCode::KeyJ, ButtonState::Released),
            key(KeyCode::ControlLeft, ButtonState::Released),
        ]);
        shell.step(&[]);
        shell
    };
    let save = |name: &str, picture: &image::RgbaImage| {
        picture.save(out.join(format!("{name}.png"))).unwrap();
        scaled(picture, 4)
            .save(out.join(format!("{name}-4x.png")))
            .unwrap();
        display(picture, 3)
            .save(out.join(format!("{name}-display-3x.png")))
            .unwrap();
    };

    // Clues only: People, the first group open, the cursor on a topic.
    let mut shell = shell_with(&[Toggle::JournalClues]);
    tap_key(&mut shell, KeyCode::Enter);
    tap_key(&mut shell, KeyCode::ArrowDown);
    tap_key(&mut shell, KeyCode::ArrowDown);
    shell.step(&[]);
    let people = page_picture(&shell);
    save("10-clues-people", &people);
    tap_key(&mut shell, KeyCode::Tab);
    tap_key(&mut shell, KeyCode::Enter);
    tap_key(&mut shell, KeyCode::ArrowDown);
    shell.step(&[]);
    save("12-topics", &page_picture(&shell));
    tap_key(&mut shell, KeyCode::Tab);
    tap_key(&mut shell, KeyCode::KeyV);
    tap_key(&mut shell, KeyCode::Enter);
    shell.step(&[]);
    save("13-place", &page_picture(&shell));
    tap_key(&mut shell, KeyCode::Slash);
    tap_key(&mut shell, KeyCode::KeyA);
    shell.step(&[]);
    save("14-search", &page_picture(&shell));

    // With the page: the three-tab strip.
    let mut shell = shell_with(&[Toggle::JournalPage, Toggle::JournalClues]);
    save("11-strip-quests", &page_picture(&shell));
    tap_key(&mut shell, KeyCode::Tab);
    tap_key(&mut shell, KeyCode::Enter);
    shell.step(&[]);
    save("11-strip-clues", &page_picture(&shell));

    if let Some(c04f) = std::env::var_os("U5_C04F_SHOT") {
        let c04f = image::open(c04f).unwrap().to_rgba8();
        let c04f = image::imageops::resize(
            &c04f,
            c04f.width() * 2,
            c04f.height() * 2,
            image::imageops::FilterType::Nearest,
        );
        beside(&c04f, &display(&people, 2))
            .save(out.join("10-clues-people-beside-c04f.png"))
            .unwrap();
    }

    // The side panel with the clue lines and two pins.
    let mut shell = shell_with(&[Toggle::JournalClues, Toggle::JournalSidePanel]);
    tap_key(&mut shell, KeyCode::Escape);
    {
        let mut log = shell.app.world_mut().resource_mut::<ClueLogResource>();
        log.log_mut().set_flag(1, EntryFlags::PANEL, true);
        log.log_mut().set_flag(6, EntryFlags::PANEL, true);
    }
    for _ in 0..3 {
        shell.step(&[]);
    }
    let world = shell.app.world();
    let lines = world.resource::<JournalPanelClues>().0.clone();
    eprintln!("panel clue lines: {:?}", lines.lock().unwrap());
    let provider = JournalPanelProvider::with_clues(Default::default(), Default::default(), lines);
    let Some(PanelContent::Cells(rows)) = provider.snapshot(Some(&shell.visual().state)) else {
        panic!("the journal panel shows rows");
    };
    let play = rgba_image(
        320,
        200,
        world
            .resource::<Assets<Image>>()
            .get(&shell.visual().image_handle)
            .unwrap()
            .data
            .clone()
            .unwrap(),
    );
    let panel = rgba_image(
        crate::enh::side::PANEL_WIDTH,
        crate::enh::side::PANEL_HEIGHT,
        render_cell_panel(&scene.text_font, &rows, PanelSide::Right),
    );
    let mut sheet = image::RgbaImage::new(320 + panel.width(), 200);
    image::imageops::overlay(&mut sheet, &play, 0, 0);
    image::imageops::overlay(&mut sheet, &panel, 320, 0);
    save("15-panel-clues", &sheet);
}

/// The M6-04 review shots: a note being written, the clue with its note,
/// a clue's text (static render, and the replay line), the Notes tab, and
/// the scrollback panel beside the play frame during a real conversation.
/// Names, keywords and text are read live from the profile's own `.TLK`
/// files; nothing is committed.
#[test]
fn clue_text_notes_and_scrollback_review_shots() {
    use crate::enh::journal::clues::ClueLogResource;
    use crate::enh::journal::scrollback::{ConversationPanelProvider, ScrollbackState};
    use crate::enh::side::{
        PanelContent, PanelSide, SidePanelProvider, render_cell_panel_with_runes,
    };
    use u5_runtime::enhancements::ClueText;
    use u5_runtime::enhancements::clues::{
        ClueEntry, DialogKey, GameMinute, NpcKey, RecordAddr, TestBits, game_minute, static_render,
    };
    let (Some(profile), Some(out)) = (
        configured_test_dir("U5_MOCKUP_PROFILE"),
        configured_test_dir("U5_JOURNAL_SHOTS"),
    ) else {
        eprintln!("skipped: set U5_MOCKUP_PROFILE and U5_JOURNAL_SHOTS");
        return;
    };
    let game = unique_scratch_dir("clue-text-shots");
    for entry in std::fs::read_dir(&profile).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            copy_asset_writable(&entry.path(), &game.join(entry.file_name())).unwrap();
        }
    }
    std::fs::create_dir_all(&out).unwrap();
    let towne = u5_runtime::parse_tlk_raw(&game.join("TOWNE.TLK")).unwrap();
    // A dialog whose first pair renders statically and whose pairs include
    // one that does not (a branch), by id.
    let mut ids: Vec<u16> = towne.keys().copied().collect();
    ids.sort_unstable();
    let pick = ids
        .iter()
        .copied()
        .find(|id| {
            let fields = &towne[id];
            fields.len() >= 13
                && static_render(fields, RecordAddr::Pair(0), "Avatar").is_some()
                && (1..4).any(|k| static_render(fields, RecordAddr::Pair(k), "Avatar").is_none())
        })
        .or_else(|| ids.first().copied())
        .unwrap();
    let fields = towne[&pick].clone();
    let replay_k = (1..4)
        .find(|k| static_render(&fields, RecordAddr::Pair(*k), "Avatar").is_none())
        .unwrap_or(1);
    let scene = Scene::from_game_dir(&game).unwrap();
    let state = PlayState::load_scene(
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
    let now = game_minute(&state).0;
    let npc = NpcKey { scene: 1, slot: 3 };
    let dialog = DialogKey {
        family: 0,
        dialog_id: pick as u8,
    };
    let mut entries = vec![ClueEntry::heard(
        npc,
        dialog,
        RecordAddr::Leading(0),
        0,
        TestBits::default(),
        GameMinute(now - 30),
    )];
    for (k, minutes) in [(0u8, 20u32), (replay_k, 10)] {
        entries.push(ClueEntry::heard(
            npc,
            dialog,
            RecordAddr::Pair(k),
            0,
            TestBits::default(),
            GameMinute(now - minutes),
        ));
    }
    let save = |name: &str, picture: &image::RgbaImage| {
        picture.save(out.join(format!("{name}.png"))).unwrap();
        scaled(picture, 4)
            .save(out.join(format!("{name}-4x.png")))
            .unwrap();
        display(picture, 3)
            .save(out.join(format!("{name}-display-3x.png")))
            .unwrap();
    };
    let mut launch = VisualLaunchConfig::classic();
    for toggle in [Toggle::JournalClues, Toggle::JournalNotes] {
        launch.enhancements.toggles.set(toggle, true);
    }
    let mut shell = Shell::new(state.clone(), &scene, &launch, |_| {});
    for _ in 0..30 {
        shell.step(&[]);
    }
    {
        let world = shell.app.world_mut();
        let mut log = world.resource_mut::<ClueLogResource>();
        for entry in &entries {
            log.log_mut().fold(*entry);
        }
        log.set_text(ClueText::Live);
        world.resource_mut::<QuestJournalUi>().clue.text = ClueText::Live;
    }
    shell.step(&[]);
    shell.step(&[
        key(KeyCode::ControlLeft, ButtonState::Pressed),
        key(KeyCode::KeyJ, ButtonState::Pressed),
    ]);
    shell.step(&[
        key(KeyCode::KeyJ, ButtonState::Released),
        key(KeyCode::ControlLeft, ButtonState::Released),
    ]);
    shell.step(&[]);
    tap_key(&mut shell, KeyCode::Enter);
    tap_key(&mut shell, KeyCode::ArrowDown);
    tap_key(&mut shell, KeyCode::KeyN);
    for code in [
        KeyCode::KeyC,
        KeyCode::KeyH,
        KeyCode::KeyE,
        KeyCode::KeyC,
        KeyCode::KeyK,
        KeyCode::Space,
        KeyCode::KeyA,
        KeyCode::KeyT,
        KeyCode::Space,
        KeyCode::KeyN,
        KeyCode::KeyI,
        KeyCode::KeyG,
        KeyCode::KeyH,
        KeyCode::KeyT,
    ] {
        tap_key(&mut shell, code);
    }
    shell.step(&[]);
    save("20-note-writing", &page_picture(&shell));
    tap_key(&mut shell, KeyCode::Enter);
    shell.step(&[]);
    save("21-clue-with-note", &page_picture(&shell));
    tap_key(&mut shell, KeyCode::Enter);
    shell.step(&[]);
    shell.step(&[]);
    save("22-text-static", &page_picture(&shell));
    tap_key(&mut shell, KeyCode::Escape);
    tap_key(&mut shell, KeyCode::ArrowDown);
    tap_key(&mut shell, KeyCode::Enter);
    shell.step(&[]);
    shell.step(&[]);
    save("23-text-replay", &page_picture(&shell));
    tap_key(&mut shell, KeyCode::Escape);
    tap_key(&mut shell, KeyCode::Tab);
    tap_key(&mut shell, KeyCode::Tab);
    shell.step(&[]);
    save("24-notes-tab", &page_picture(&shell));

    // The scrollback beside the frame: a person of the profile's own blob
    // placed east of the party, then Talk, JOB and NAME typed through the
    // game's keys.
    let mut state = state;
    state.player.facing = u5_runtime::Direction::East;
    let (x, y) = (state.player.x as u8, state.player.y as u8);
    state.load_scheduled_npcs(&[u5_runtime::NpcSlot {
        slot: 1,
        type_byte: 1,
        dialog_id: pick as u8,
        schedule: [0, 0, 0, x + 1, x + 1, x + 1, y, y, y, 0, 0, 0, 0, 8, 16, 20],
        name: None,
    }]);
    let mut launch = VisualLaunchConfig::classic();
    launch
        .enhancements
        .toggles
        .set(Toggle::JournalClueScrollback, true);
    let mut shell = Shell::new(state, &scene, &launch, |_| {});
    for _ in 0..30 {
        shell.step(&[]);
    }
    tap_key(&mut shell, KeyCode::KeyT);
    tap_key(&mut shell, KeyCode::ArrowRight);
    for _ in 0..40 {
        shell.step(&[]);
    }
    for word in [
        &[KeyCode::KeyJ, KeyCode::KeyO, KeyCode::KeyB][..],
        &[KeyCode::KeyN, KeyCode::KeyA, KeyCode::KeyM, KeyCode::KeyE][..],
    ] {
        for code in word {
            tap_key(&mut shell, *code);
        }
        tap_key(&mut shell, KeyCode::Enter);
        for _ in 0..40 {
            shell.step(&[]);
        }
    }
    let world = shell.app.world();
    let rows = world
        .resource::<ScrollbackState>()
        .scrollback()
        .rows()
        .map(<[_]>::to_vec);
    eprintln!("scrollback rows: {}", rows.as_ref().map_or(0, Vec::len));
    let provider = ConversationPanelProvider::new(std::sync::Arc::new(std::sync::Mutex::new(rows)));
    let Some(PanelContent::Cells(cells)) = provider.snapshot(Some(&shell.visual().state)) else {
        panic!("the scrollback shows rows");
    };
    let play = rgba_image(
        320,
        200,
        world
            .resource::<Assets<Image>>()
            .get(&shell.visual().image_handle)
            .unwrap()
            .data
            .clone()
            .unwrap(),
    );
    let panel = rgba_image(
        crate::enh::side::PANEL_WIDTH,
        crate::enh::side::PANEL_HEIGHT,
        render_cell_panel_with_runes(
            &scene.text_font,
            Some(&scene.rune_font),
            &cells,
            PanelSide::Right,
        ),
    );
    let mut sheet = image::RgbaImage::new(320 + panel.width(), 200);
    image::imageops::overlay(&mut sheet, &play, 0, 0);
    image::imageops::overlay(&mut sheet, &panel, 320, 0);
    save("25-scrollback", &sheet);
}
