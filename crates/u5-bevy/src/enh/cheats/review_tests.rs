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
