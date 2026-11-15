//! The M8-02 review shots: the Cheats page drawn by the real `--visual`
//! shell over a real frame, beside the approved c04f look. Review only:
//! skipped unless `U5_MOCKUP_PROFILE` names a game copy and
//! `U5_CHEATS_SHOTS` a folder (`U5_C04F` may name the c04f folder for the
//! side-by-side); nothing is committed.

use bevy::input::ButtonState;
use bevy::window::WindowResolution;
use u5_runtime::test_fixtures::{configured_test_dir, copy_asset_writable, unique_scratch_dir};
use u5_runtime::{GameClock, PlayOptions, PlayState, PlayTarget, Scene as GameScene};

use super::*;
use crate::enh::equivalence::{Owner, Scene, Shell, Timeline};

fn every_cheat_launch() -> VisualLaunchConfig {
    let mut launch = VisualLaunchConfig::classic();
    for spec in u5_runtime::enhancements::TOGGLES {
        if spec.toggle.key().starts_with("cheats.")
            && !matches!(
                spec.toggle,
                Toggle::CheatsStatEdit | Toggle::CheatsTeleport | Toggle::CheatsStateWatch
            )
        {
            launch.enhancements.toggles.set(spec.toggle, true);
        }
    }
    launch
}

/// The game frame with the page over it, as the window shows them.
fn composite(shell: &Shell) -> image::RgbaImage {
    let world = shell.app.world();
    let images = world.resource::<Assets<Image>>();
    let frame = images.get(&shell.visual().image_handle).unwrap();
    let size = frame.size();
    let mut out = image::RgbaImage::from_raw(size.x, size.y, frame.data.clone().unwrap()).unwrap();
    if let Some(handle) = world.resource::<CheatPage>().image()
        && let Some(page) = images.get(handle)
        && world.resource::<OverlayState>().open() == Some(OverlayId::Cheats)
    {
        let page = page.data.clone().unwrap();
        for (i, pixel) in out.pixels_mut().enumerate() {
            let p = &page[i * 4..i * 4 + 4];
            let a = u32::from(p[3]);
            for (c, src) in pixel.0.iter_mut().zip(p).take(3) {
                *c = ((u32::from(*src) * a + u32::from(*c) * (255 - a)) / 255) as u8;
            }
        }
    }
    out
}

fn scaled(img: &image::RgbaImage, k: u32) -> image::RgbaImage {
    image::imageops::resize(
        img,
        img.width() * k,
        img.height() * k,
        image::imageops::FilterType::Nearest,
    )
}

fn crop4(img: &image::RgbaImage, x: u32, y: u32, w: u32, h: u32) -> image::RgbaImage {
    scaled(&image::imageops::crop_imm(img, x, y, w, h).to_image(), 4)
}

#[test]
fn cheats_page_review_shots() {
    let (Some(profile), Some(out)) = (
        configured_test_dir("U5_MOCKUP_PROFILE"),
        configured_test_dir("U5_CHEATS_SHOTS"),
    ) else {
        eprintln!("skipped: set U5_MOCKUP_PROFILE and U5_CHEATS_SHOTS");
        return;
    };
    let game = unique_scratch_dir("cheats-review-shots");
    for entry in std::fs::read_dir(&profile).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            copy_asset_writable(&entry.path(), &game.join(entry.file_name())).unwrap();
        }
    }
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
    for member in &mut state.party {
        member.hp = member.max_hp / 3;
    }
    let mut shell = Shell::new(state, &scene, &every_cheat_launch(), |_| {});
    shell.app.world_mut().spawn(Window {
        resolution: WindowResolution::new(1600.0, 1200.0),
        ..default()
    });
    let mut timeline = Timeline::new(40);
    timeline.event(
        6,
        KeyCode::ControlLeft,
        ButtonState::Pressed,
        false,
        Owner::Feature,
    );
    timeline.feature_tap(7, KeyCode::F12);
    timeline.event(
        9,
        KeyCode::ControlLeft,
        ButtonState::Released,
        false,
        Owner::Feature,
    );
    timeline.feature_tap(12, KeyCode::Digit3);
    timeline.feature_tap(16, KeyCode::KeyH);
    let mut shots = Vec::new();
    for frame in 0..timeline.len() {
        shell.step(timeline.frame(frame));
        if frame == 10 {
            shots.push(("main", composite(&shell)));
        }
    }
    shots.push(("after-heal", composite(&shell)));
    // The confirmation.
    let mut close = Timeline::new(4);
    close.feature_tap(0, KeyCode::Escape);
    for frame in 0..close.len() {
        shell.step(close.frame(frame));
    }
    shell.app.world_mut().send_event(AllowCheatsRequest);
    for _ in 0..4 {
        shell.step(&[]);
    }
    shots.push(("confirm", composite(&shell)));

    for (name, img) in &shots {
        img.save(out.join(format!("engine-cheats-{name}-320x200.png")))
            .unwrap();
        scaled(img, 4)
            .save(out.join(format!("engine-cheats-{name}-4x.png")))
            .unwrap();
    }
    let main = &shots[0].1;
    for (name, x, y, w, h) in [
        ("corner-top-left", 0, 0, 64, 40),
        ("corner-top-right", 256, 0, 64, 40),
        ("corner-bottom-left", 0, 160, 64, 40),
        ("corner-bottom-right", 256, 160, 64, 40),
        ("title-tab", 120, 0, 80, 24),
        ("footer-tab", 112, 176, 96, 24),
        ("divider-left", 0, 8, 48, 24),
        ("divider-right", 272, 144, 48, 24),
        ("text", 8, 8, 160, 112),
    ] {
        crop4(main, x, y, w, h)
            .save(out.join(format!("closeup-{name}.png")))
            .unwrap();
    }
    if let Some(c04f) = configured_test_dir("U5_C04F") {
        let mock = image::open(c04f.join("c04f-G-status.png"))
            .unwrap()
            .to_rgba8();
        let ours = scaled(main, 2);
        let mock = scaled(&mock, 2);
        let mut side = image::RgbaImage::new(
            ours.width() + 16 + mock.width(),
            ours.height().max(mock.height()),
        );
        image::imageops::overlay(&mut side, &ours, 0, 0);
        image::imageops::overlay(&mut side, &mock, i64::from(ours.width() + 16), 0);
        side.save(out.join("compare-c04f-G-status.png")).unwrap();
    }
    assert_eq!(
        shell.app.world().resource::<CheatPage>().mode(),
        PageMode::Confirm
    );
    let _ = std::fs::remove_dir_all(game);
}

/// The M8-03 review shots: the Edit party and stores face, drawn by the
/// real shell over a real frame. Skipped unless `U5_MOCKUP_PROFILE` names a
/// game copy and `U5_CHEATS_EDIT_SHOTS` a folder; nothing is committed.
#[test]
fn edit_page_review_shots() {
    let (Some(profile), Some(out)) = (
        configured_test_dir("U5_MOCKUP_PROFILE"),
        configured_test_dir("U5_CHEATS_EDIT_SHOTS"),
    ) else {
        eprintln!("skipped: set U5_MOCKUP_PROFILE and U5_CHEATS_EDIT_SHOTS");
        return;
    };
    let game = unique_scratch_dir("cheats-edit-review-shots");
    for entry in std::fs::read_dir(&profile).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            copy_asset_writable(&entry.path(), &game.join(entry.file_name())).unwrap();
        }
    }
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
    // Three made-up companions (no game names), one of them Dead, and an
    // out-of-range value an edited save might hold: the `!` marker.
    for (slot, (class_byte, status, name, xp)) in [
        (b'B', b'P', *b"PAT\0\0\0\0\0\0", 1730u16),
        (b'F', b'G', *b"KIM\0\0\0\0\0\0", 2210),
        (b'M', b'D', *b"LEE\0\0\0\0\0\0", 1688),
    ]
    .into_iter()
    .enumerate()
    {
        let mut member = state.party[0];
        member.slot = slot as u8 + 1;
        member.class_byte = class_byte;
        member.status = status;
        member.level = 5;
        member.max_hp = 150;
        member.hp = if status == b'D' { 0 } else { 150 };
        member.mana = 0;
        state.party.push(member);
        state.party_names.push(name);
        state.party_experience.push(xp);
        state.party_stay_counters.push(0);
        state.party_strengths.push(19);
        state.party_combat_defense.push(0);
        state.party_intelligence.push(18);
        let equipment = state.party_equipment[0];
        state.party_equipment.push(equipment);
    }
    state.party_strengths[2] = 45;
    state.food = 12_000;
    let mut launch = every_cheat_launch();
    launch
        .enhancements
        .toggles
        .set(Toggle::CheatsStatEdit, true);
    let mut shell = Shell::new(state, &scene, &launch, |_| {});
    shell.app.world_mut().spawn(Window {
        resolution: WindowResolution::new(1600.0, 1200.0),
        ..default()
    });
    let mut timeline = Timeline::new(60);
    timeline.event(
        6,
        KeyCode::ControlLeft,
        ButtonState::Pressed,
        false,
        Owner::Feature,
    );
    timeline.feature_tap(7, KeyCode::F12);
    timeline.event(
        9,
        KeyCode::ControlLeft,
        ButtonState::Released,
        false,
        Owner::Feature,
    );
    // Main face (with the More row), then [E], STR +1, [L].
    timeline.feature_tap(14, KeyCode::KeyE);
    timeline.feature_tap(18, KeyCode::ArrowRight);
    timeline.feature_tap(22, KeyCode::KeyL);
    // Down to INT, Enter, 2: typing.
    timeline.feature_tap(26, KeyCode::ArrowDown);
    timeline.feature_tap(30, KeyCode::ArrowDown);
    timeline.feature_tap(34, KeyCode::Enter);
    timeline.feature_tap(38, KeyCode::Digit2);
    let mut shots = Vec::new();
    for frame in 0..timeline.len() {
        shell.step(timeline.frame(frame));
        match frame {
            12 => shots.push(("main-more", composite(&shell))),
            25 => shots.push(("edit-draft", composite(&shell))),
            _ => {}
        }
    }
    shots.push(("edit-typing", composite(&shell)));
    // Enter, Apply.
    let mut apply = Timeline::new(10);
    apply.feature_tap(0, KeyCode::Enter);
    apply.feature_tap(4, KeyCode::KeyA);
    for frame in 0..apply.len() {
        shell.step(apply.frame(frame));
    }
    shots.push(("edit-applied", composite(&shell)));

    for (name, img) in &shots {
        img.save(out.join(format!("engine-{name}-320x200.png")))
            .unwrap();
        scaled(img, 4)
            .save(out.join(format!("engine-{name}-4x.png")))
            .unwrap();
    }
    let draft = &shots[1].1;
    for (name, x, y, w, h) in [
        ("corner-top-left", 0, 0, 64, 40),
        ("corner-top-right", 256, 0, 64, 40),
        ("corner-bottom-left", 0, 160, 64, 40),
        ("corner-bottom-right", 256, 160, 64, 40),
        ("title-tab", 112, 0, 96, 24),
        ("footer-tab", 72, 176, 176, 24),
        ("dividers-left", 0, 56, 48, 112),
        ("dividers-right", 272, 56, 48, 112),
        ("table", 0, 0, 280, 72),
        ("fields", 0, 64, 280, 48),
        ("stores", 0, 104, 320, 56),
        ("help-result", 0, 152, 320, 48),
    ] {
        crop4(draft, x, y, w, h)
            .save(out.join(format!("closeup-{name}.png")))
            .unwrap();
    }
    assert_eq!(
        shell.app.world().resource::<CheatPage>().mode(),
        PageMode::Edit
    );
    let _ = std::fs::remove_dir_all(game);
}
