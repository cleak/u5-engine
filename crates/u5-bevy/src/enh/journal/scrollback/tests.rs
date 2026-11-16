//! The scrollback panel (design 8.2.6, 8.3.11; plan M6-04): absent when
//! off, rows from the Talk echo through the game's own keys, cleared at
//! the end, runic cells, and hidden under grow-left with one toast. Every
//! blob is synthetic. The cap is covered by the runtime's tests of
//! [`Scrollback`].

use std::collections::HashMap;

use bevy::window::Window;
use u5_runtime::enhancements::PixelAspect;
use u5_runtime::{Direction, NpcSlot};

use super::*;
use crate::display::grow::Canvas;
use crate::enh::add_enhancements;
use crate::enh::overlay::tests::Game;
use crate::enh::side::{FrameImageSize, PANEL_HIDDEN, SideLayout};
use crate::enh::toast::Toasts;

const DIALOG_ID: u8 = 0x10;

fn launch(toggles: &[Toggle]) -> VisualLaunchConfig {
    let mut launch = VisualLaunchConfig::classic();
    for toggle in toggles {
        launch.enhancements.toggles.set(*toggle, true);
    }
    launch
}

fn text(s: &str) -> Vec<u8> {
    s.bytes().map(|b| b | 0x80).collect()
}

fn fields() -> Vec<Vec<u8>> {
    let mut rune = text("See ");
    rune.push(0x8E);
    rune.extend(text("abc"));
    rune.push(0x8E);
    vec![
        text("Quorvel"),
        text("a dusty quillwright"),
        text("Hail."),
        text("I mend quills."),
        text("Fare well."),
        text("gran"),
        text("Old one."),
        text("rune"),
        rune,
    ]
}

/// The headless game with a 32:9 window and the foundation as `launch`
/// adds it, a scheduled person beside the party.
fn game(launch: &VisualLaunchConfig) -> Game {
    let mut game = Game::new(false);
    {
        let state = &mut game.visual_mut().state;
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
    }
    add_enhancements(&mut game.app, launch);
    let mut window = Window::default();
    window.resolution.set(3840.0, 1080.0);
    game.app.world_mut().spawn(window);
    game.frame();
    game
}

fn open_talk(game: &mut Game) {
    let fields = fields();
    let decoded: Vec<String> = fields
        .iter()
        .map(|field| field.iter().map(|b| char::from(b & 0x7F)).collect())
        .collect();
    let dialogue = HashMap::from([(u16::from(DIALOG_ID), decoded)]);
    let raw = HashMap::from([(u16::from(DIALOG_ID), fields)]);
    let state = &mut game.visual_mut().state;
    state.push_message_entry("Talk-East", true);
    assert!(state.open_conversation_session(&dialogue, &raw).is_some());
    game.frame();
}

fn tap(game: &mut Game, key: KeyCode) {
    game.press(key);
    game.frame();
    game.release(key);
    game.frame();
}

fn say(game: &mut Game, line: &str) {
    for c in line.chars() {
        let key = match c {
            'g' => KeyCode::KeyG,
            'r' => KeyCode::KeyR,
            'a' => KeyCode::KeyA,
            'n' => KeyCode::KeyN,
            'u' => KeyCode::KeyU,
            'e' => KeyCode::KeyE,
            other => panic!("no key for {other:?}"),
        };
        tap(game, key);
    }
    tap(game, KeyCode::Enter);
    for _ in 0..3 {
        game.frame();
    }
}

fn shared(game: &Game) -> Option<Vec<TextRow>> {
    game.app
        .world()
        .resource::<ScrollbackState>()
        .shared
        .lock()
        .unwrap()
        .clone()
}

fn texts(rows: &[TextRow]) -> Vec<String> {
    rows.iter()
        .map(|row| row.bytes.iter().map(|b| char::from(*b)).collect())
        .collect()
}

fn has_provider(game: &Game) -> bool {
    game.app
        .world()
        .get_resource::<PanelRegistry>()
        .is_some_and(|registry| registry.get(PanelKind::Conversation).is_some())
}

fn granted(game: &Game) -> Vec<PanelKind> {
    game.app
        .world()
        .resource::<SideLayout>()
        .granted
        .iter()
        .map(|(kind, _)| *kind)
        .collect()
}

#[test]
fn absent_when_off_and_with_classic() {
    for launch in [
        VisualLaunchConfig::classic(),
        launch(&[Toggle::JournalSidePanel]),
        launch(&[Toggle::JournalClues]),
    ] {
        let game = game(&launch);
        assert!(!has_provider(&game), "{launch:?}");
        assert!(!game.app.world().contains_resource::<ScrollbackState>());
    }
    let game = game(&launch(&[Toggle::JournalClueScrollback]));
    assert!(
        has_provider(&game),
        "needs neither the log nor its observer"
    );
    assert!(game.app.world().contains_resource::<ScrollbackState>());
    assert!(
        !game
            .app
            .world()
            .contains_resource::<crate::enh::journal::clues::ClueObserver>()
    );
}

#[test]
fn turning_it_off_and_on_in_play_follows_the_key() {
    let mut game = game(&launch(&[Toggle::JournalClueScrollback]));
    game.app
        .world_mut()
        .resource_mut::<Toggles>()
        .0
        .set(SCROLLBACK, false);
    game.frame();
    assert!(!has_provider(&game));
    assert!(!game.app.world().contains_resource::<ScrollbackState>());
    game.app
        .world_mut()
        .resource_mut::<Toggles>()
        .0
        .set(SCROLLBACK, true);
    game.frame();
    assert!(has_provider(&game));
}

#[test]
fn rows_run_from_the_talk_echo_and_clear_at_the_end() {
    let mut game = game(&launch(&[Toggle::JournalClueScrollback]));
    assert_eq!(shared(&game), None, "nothing outside a conversation");
    open_talk(&mut game);
    say(&mut game, "gran");
    say(&mut game, "rune");
    let rows = shared(&game).expect("a conversation is open");
    let lines = texts(&rows);
    assert_eq!(lines[0], "Talk-East", "{lines:?}");
    assert!(
        lines.iter().any(|row| row.contains("Old one.")),
        "{lines:?}"
    );
    assert!(
        rows.iter().any(|row| row.runic != 0),
        "the runic span keeps its mask"
    );
    // The provider's cells: the title and the last rows, masks included.
    let registry = game.app.world().resource::<PanelRegistry>();
    let Some(PanelContent::Cells(cells)) = registry
        .get(PanelKind::Conversation)
        .unwrap()
        .snapshot(Some(&game.visual().state))
    else {
        panic!("cells");
    };
    assert_eq!(&cells.title()[..4], SCROLLBACK_TITLE.as_bytes());
    let shown = rows.len().min(PANEL_BODY_ROWS);
    let first = rows.len() - shown;
    for (index, row) in rows[first..].iter().enumerate() {
        assert_eq!(&cells.row(index)[..row.bytes.len()], row.bytes.as_slice());
        assert_eq!(cells.runic(index), row.runic);
    }
    say(&mut game, "");
    assert!(game.visual().state.active_conversation.is_none());
    assert_eq!(shared(&game), None, "cleared when the conversation ends");
    assert_eq!(
        game.app
            .world()
            .resource::<ScrollbackState>()
            .scrollback()
            .dropped(),
        0
    );
}

#[test]
fn hidden_under_grow_left_with_one_toast() {
    let mut game = game(&launch(&[Toggle::JournalClueScrollback]));
    assert_eq!(granted(&game), [PanelKind::Conversation]);
    let canvas = Canvas::for_window(3840, 1080, PixelAspect::Dos).unwrap();
    game.app
        .world_mut()
        .insert_resource(FrameImageSize(Some(UVec2::new(canvas.w, canvas.h))));
    game.frame();
    assert!(granted(&game).is_empty());
    assert_eq!(
        game.app.world().resource::<Toasts>().current(),
        Some(PANEL_HIDDEN)
    );
    for _ in 0..300 {
        game.frame();
    }
    game.app.world_mut().insert_resource(FrameImageSize(None));
    game.frame();
    assert_eq!(granted(&game), [PanelKind::Conversation]);
    game.app
        .world_mut()
        .insert_resource(FrameImageSize(Some(UVec2::new(canvas.w, canvas.h))));
    game.frame();
    assert_eq!(
        game.app.world().resource::<Toasts>().current(),
        None,
        "said once"
    );
}
