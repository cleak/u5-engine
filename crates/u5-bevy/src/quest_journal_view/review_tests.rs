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
    paint_page(
        &mut empty,
        &scene.text_font,
        &PageView {
            model: None,
            section: None,
            tab_strip: false,
        },
    );
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
