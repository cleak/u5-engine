//! The M6-07 review shots: the pin picker, the Quest filter and the card
//! steps drawn by the real `--visual` shell with the game's own IBM.CH
//! font and frame. Review only: skipped unless `U5_MOCKUP_PROFILE` names a
//! game copy and `U5_LINK_SHOTS` a folder; nothing is committed.

use bevy::input::ButtonState;
use u5_runtime::enhancements::TrackerEvidence;
use u5_runtime::enhancements::clues::{
    ClueEntry, DialogKey, GameMinute, NpcKey, RecordAddr, TestBits, game_minute,
};
use u5_runtime::quest_journal::trackers::{LinkTarget, QuestId, QuestKind, StepId};
use u5_runtime::test_fixtures::{configured_test_dir, copy_asset_writable, unique_scratch_dir};
use u5_runtime::{GameClock, PlayOptions, PlayState, PlayTarget, Scene as GameScene};

use super::*;
use crate::enh::equivalence::{Owner, Scene, Shell, TimedKey};
use crate::enh::journal::clues::ClueLogResource;

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
    shell.step(&[]);
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

fn ui(shell: &Shell) -> &QuestJournalUi {
    shell.app.world().resource::<QuestJournalUi>()
}

/// Down until the picker's row reads `label`.
fn pick(shell: &mut Shell, label: &str) {
    for _ in 0..40 {
        let picker = ui(shell).clue_ui().picking.clone().unwrap();
        if picker.rows[picker.cursor].label.as_str() == label {
            return;
        }
        tap(shell, KeyCode::ArrowDown);
    }
    panic!("{label} not offered");
}

/// Down until the card list's cursor is on `title`.
fn to_card(shell: &mut Shell, title: &str) {
    tap(shell, KeyCode::Home);
    let at = ui(shell)
        .tracker_ui()
        .unwrap()
        .cards()
        .iter()
        .position(|card| card.title.as_str() == title)
        .unwrap();
    for _ in 0..at {
        tap(shell, KeyCode::ArrowDown);
    }
}

#[test]
fn link_review_shots() {
    let (Some(profile), Some(out)) = (
        configured_test_dir("U5_MOCKUP_PROFILE"),
        configured_test_dir("U5_LINK_SHOTS"),
    ) else {
        eprintln!("skipped: set U5_MOCKUP_PROFILE and U5_LINK_SHOTS");
        return;
    };
    let game = unique_scratch_dir("link-review-shots");
    for entry in std::fs::read_dir(&profile).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            copy_asset_writable(&entry.path(), &game.join(entry.file_name())).unwrap();
        }
    }
    std::fs::create_dir_all(&out).unwrap();
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
    // Two people of the town family, by id: the Valor mantra's source of
    // quest-graph §8 (TOWNE:2 slot 10) with its name heard, and one more.
    let mut entries = Vec::new();
    for (scene_byte, slot, dialog_id, named) in [(3u8, 10u8, 16u8, true), (1, 4, 2, false)] {
        let npc = NpcKey {
            scene: scene_byte,
            slot,
        };
        let dialog = DialogKey {
            family: 0,
            dialog_id,
        };
        let mut records = vec![RecordAddr::Pair(0), RecordAddr::Pair(1)];
        if named {
            records.insert(0, RecordAddr::Leading(0));
        }
        for (at, record) in records.into_iter().enumerate() {
            entries.push(ClueEntry::heard(
                npc,
                dialog,
                record,
                0,
                TestBits::default(),
                GameMinute(now - 500 + at as u32),
            ));
        }
    }
    let valor = QuestId::new(QuestKind::Shrine, 2);

    let shell_with = |tier: SpoilerTier, evidence: TrackerEvidence, step_pin: bool| {
        let mut launch = VisualLaunchConfig::classic();
        for toggle in [Toggle::JournalClues, Toggle::JournalQuestTracker] {
            launch.enhancements.toggles.set(toggle, true);
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
            if step_pin {
                log.log_mut()
                    .add_link(LinkTarget::step(StepId::new(valor, 0)).link(1));
            }
        }
        {
            let mut ui = shell.app.world_mut().resource_mut::<QuestJournalUi>();
            ui.settings.tier = tier;
            ui.settings.clues.tracker_evidence = evidence;
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
    };

    // Tier 2: a topic picked on the Clues tab, the picker, the pin.
    let mut shell = shell_with(SpoilerTier::Goals, TrackerEvidence::Record, false);
    tap(&mut shell, KeyCode::Tab);
    tap(&mut shell, KeyCode::Enter);
    tap(&mut shell, KeyCode::ArrowDown);
    save("01-clue-topic-q-key", &picture(&shell));
    tap(&mut shell, KeyCode::KeyQ);
    save("02-picker-tier2", &picture(&shell));
    pick(&mut shell, "Valor");
    tap(&mut shell, KeyCode::Enter);
    save("03-picker-valor-pinned", &picture(&shell));
    tap(&mut shell, KeyCode::Escape);
    save("04-clue-pinned-to-detail", &picture(&shell));
    // The pinned card on the Quests tab, then Enter: the Quest filter.
    tap(&mut shell, KeyCode::Tab);
    tap(&mut shell, KeyCode::Tab);
    save("05-cards-tier2-pinned-valor", &picture(&shell));
    to_card(&mut shell, "Valor");
    tap(&mut shell, KeyCode::Enter);
    save("06-card-valor-tier2-pinned", &picture(&shell));
    tap(&mut shell, KeyCode::Enter);
    save("07-clues-quest-filter", &picture(&shell));

    // Tier 4 with a step pin: the step bar, the unlocked cell, K.
    let mut shell = shell_with(SpoilerTier::Full, TrackerEvidence::Record, true);
    to_card(&mut shell, "Valor");
    tap(&mut shell, KeyCode::Enter);
    save("08-card-valor-tier4-step-pin", &picture(&shell));
    tap(&mut shell, KeyCode::ArrowDown);
    save("09-card-valor-tier4-second-step", &picture(&shell));
    tap(&mut shell, KeyCode::ArrowUp);
    tap(&mut shell, KeyCode::Enter);
    save("10-clues-step-filter", &picture(&shell));
    tap(&mut shell, KeyCode::Escape);
    tap(&mut shell, KeyCode::Enter);
    tap(&mut shell, KeyCode::ArrowDown);
    tap(&mut shell, KeyCode::KeyQ);
    pick(&mut shell, "Valor");
    save("11-picker-tier4-steps", &picture(&shell));

    // Tier 3, `tracker_evidence = npc`: the heard count and the link line.
    let mut shell = shell_with(SpoilerTier::Hints, TrackerEvidence::Npc, false);
    to_card(&mut shell, "Valor");
    tap(&mut shell, KeyCode::Enter);
    save("12-card-valor-tier3-npc-heard", &picture(&shell));
    tap(&mut shell, KeyCode::Enter);
    tap(&mut shell, KeyCode::Enter);
    tap(&mut shell, KeyCode::ArrowDown);
    save("13-clue-tier3-npc-link-line", &picture(&shell));
}
