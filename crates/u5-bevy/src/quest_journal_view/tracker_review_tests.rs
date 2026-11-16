//! The M6-06 review shots: the tracker cards drawn by the real `--visual`
//! shell with the game's own IBM.CH font and frame. Review only: skipped
//! unless `U5_MOCKUP_PROFILE` names a game copy and `U5_TRACKER_SHOTS` a
//! folder; nothing is committed.

use bevy::input::ButtonState;
use u5_runtime::test_fixtures::{configured_test_dir, copy_asset_writable, unique_scratch_dir};
use u5_runtime::{GameClock, PlayOptions, PlayState, PlayTarget, Scene as GameScene};

use super::*;
use crate::enh::equivalence::{Owner, Scene, Shell, TimedKey};
use crate::enh::side::{PanelContent, PanelSide, SidePanelProvider, render_cell_panel};

fn key(key: KeyCode, state: ButtonState) -> TimedKey {
    TimedKey {
        key,
        state,
        repeat: false,
        owner: Owner::Feature,
    }
}

fn tap(shell: &mut Shell, code: KeyCode) {
    shell.step(&[key(code, ButtonState::Pressed)]);
    shell.step(&[key(code, ButtonState::Released)]);
}

fn picture(shell: &Shell) -> image::RgbaImage {
    let world = shell.app.world();
    let ui = world.resource::<QuestJournalUi>();
    let data = world
        .resource::<Assets<Image>>()
        .get(ui.page_image().unwrap())
        .unwrap()
        .data
        .clone()
        .unwrap();
    image::RgbaImage::from_raw(320, 200, data).unwrap()
}

fn scaled(picture: &image::RgbaImage, n: u32) -> image::RgbaImage {
    image::imageops::resize(
        picture,
        picture.width() * n,
        picture.height() * n,
        image::imageops::FilterType::Nearest,
    )
}

#[test]
fn tracker_review_shots() {
    let (Some(profile), Some(out)) = (
        configured_test_dir("U5_MOCKUP_PROFILE"),
        configured_test_dir("U5_TRACKER_SHOTS"),
    ) else {
        eprintln!("skipped: set U5_MOCKUP_PROFILE and U5_TRACKER_SHOTS");
        return;
    };
    let game = unique_scratch_dir("tracker-review-shots");
    for entry in std::fs::read_dir(&profile).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            copy_asset_writable(&entry.path(), &game.join(entry.file_name())).unwrap();
        }
    }
    std::fs::create_dir_all(&out).unwrap();
    let scene = Scene::from_game_dir(&game).unwrap();
    let state = || {
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
        // Demo progress: Compassion and Valor ordained, Honesty with its
        // Codex read, Sacrifice and Humility complete, Spirituality's
        // shrine in ruin; Deceit and Covetous opened; one Shadowlord
        // vanquished; a shard, the Crown, the grapple and the Glass Sword.
        state.shrine_ordained_mask = 0b0000_0111;
        state.shrine_codex_mask = 0b1001_0001;
        state.shrine_ruin_flags[6] = 0x80;
        state.word_of_power_seal_flags[0] = 0x80;
        state.word_of_power_seal_flags[4] = 0x80;
        state.shadowlord_hideouts = [0xFF, 3, 5];
        state.special_items[u5_runtime::SPECIAL_ITEM_SHARD_FALSEHOOD_INDEX + 1] = 1;
        state.special_items[u5_runtime::SPECIAL_ITEM_CROWN_LB_INDEX] = 1;
        state.climbing_gear = 1;
        state.equipment_stock[u5_runtime::equipment::EQUIPMENT_GLASS_SWORD] = 1;
        state
    };
    let save = |name: &str, picture: &image::RgbaImage| {
        picture.save(out.join(format!("{name}.png"))).unwrap();
        scaled(picture, 4)
            .save(out.join(format!("{name}-4x.png")))
            .unwrap();
    };
    let shell_with = |toggles: &[Toggle], tier: SpoilerTier| {
        let mut launch = VisualLaunchConfig::classic();
        for toggle in toggles {
            launch.enhancements.toggles.set(*toggle, true);
        }
        let mut shell = Shell::new(state(), &scene, &launch, |_| {});
        for _ in 0..30 {
            shell.step(&[]);
        }
        shell
            .app
            .world_mut()
            .resource_mut::<QuestJournalUi>()
            .settings
            .tier = tier;
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

    // The cards alone, tier 2: the list, then Valor's card.
    let mut shell = shell_with(&[Toggle::JournalQuestTracker], SpoilerTier::Goals);
    save("01-cards-alone-tier2", &picture(&shell));
    let at = shell
        .app
        .world()
        .resource::<QuestJournalUi>()
        .tracker_ui()
        .unwrap()
        .cards()
        .iter()
        .position(|card| card.title.as_str() == "Valor")
        .unwrap();
    for _ in 0..at {
        tap(&mut shell, KeyCode::ArrowDown);
    }
    shell.step(&[]);
    save("02-cards-alone-valor-selected", &picture(&shell));
    tap(&mut shell, KeyCode::Enter);
    shell.step(&[]);
    save("03-card-valor-tier2", &picture(&shell));

    // With chapter 5's page: the cards in the Virtues section's place.
    let shell = shell_with(
        &[Toggle::JournalPage, Toggle::JournalQuestTracker],
        SpoilerTier::Goals,
    );
    save("04-page-and-cards-tier2", &picture(&shell));

    // Tier 3 (SPOILERS): every card, the next cells.
    let shell = shell_with(&[Toggle::JournalQuestTracker], SpoilerTier::Hints);
    save("05-cards-tier3", &picture(&shell));

    // Tier 4: Honesty's card masked, asked, then revealed.
    let mut shell = shell_with(&[Toggle::JournalQuestTracker], SpoilerTier::Full);
    let at = shell
        .app
        .world()
        .resource::<QuestJournalUi>()
        .tracker_ui()
        .unwrap()
        .cards()
        .iter()
        .position(|card| card.title.as_str() == "Honesty")
        .unwrap();
    for _ in 0..at {
        tap(&mut shell, KeyCode::ArrowDown);
    }
    tap(&mut shell, KeyCode::Enter);
    shell.step(&[]);
    save("06-card-honesty-tier4-masked", &picture(&shell));
    tap(&mut shell, KeyCode::KeyR);
    shell.step(&[]);
    save("07-card-honesty-tier4-confirm", &picture(&shell));
    tap(&mut shell, KeyCode::KeyR);
    shell.step(&[]);
    save("08-card-honesty-tier4-revealed", &picture(&shell));

    // The side panel with the tracker's card lines.
    let shell = shell_with(
        &[Toggle::JournalQuestTracker, Toggle::JournalSidePanel],
        SpoilerTier::Goals,
    );
    let provider = JournalPanelProvider::new(Default::default(), Default::default()).with_tracker(
        std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true)),
    );
    let Some(PanelContent::Cells(rows)) = provider.snapshot(Some(&shell.visual().state)) else {
        panic!("the journal panel shows rows");
    };
    let world = shell.app.world();
    let play = image::RgbaImage::from_raw(
        320,
        200,
        world
            .resource::<Assets<Image>>()
            .get(&shell.visual().image_handle)
            .unwrap()
            .data
            .clone()
            .unwrap(),
    )
    .unwrap();
    let panel = image::RgbaImage::from_raw(
        crate::enh::side::PANEL_WIDTH,
        crate::enh::side::PANEL_HEIGHT,
        render_cell_panel(&scene.text_font, &rows, PanelSide::Right),
    )
    .unwrap();
    let mut sheet = image::RgbaImage::new(320 + panel.width(), 200);
    image::imageops::overlay(&mut sheet, &play, 0, 0);
    image::imageops::overlay(&mut sheet, &panel, 320, 0);
    save("09-panel-card-lines", &sheet);
}
