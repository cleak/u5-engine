//! The journal side panel (plan M5-05; design 5.2.10, 5.3.5): absent when
//! off, its side against a stub scrollback provider with the
//! `panel_priority` loser, hidden under grow-left's world view, the page
//! off or open, rows only on a model change, and the play image and the
//! frame size untouched unless a panel is granted.

use bevy::window::Window;
use u5_runtime::enhancements::{JournalPanelSide, PixelAspect};
use u5_runtime::quest_journal::ShadowlordState;
use u5_runtime::{
    CH_FONT_LEN, IBM_CH_FILE, PlayState, SPECIAL_ITEM_CROWN_LB_INDEX,
    SPECIAL_ITEM_WOODEN_BOX_INDEX, ShrineVirtue, parse_ch_font,
};

use super::panel::register_journal_panel;
use super::*;
use crate::display::grow::Canvas;
use crate::enh::add_enhancements;
use crate::enh::overlay::tests::Game;
use crate::enh::side::{
    PANEL_HIDDEN, PanelContent, PanelKind, PanelRegistry, PanelRows, PanelSide, SideLayout,
    SidePanelNode, SidePanelProvider, SidePreference, SideSpaceSettings, faithful_fit_owns_frame,
};
use crate::enh::toast::Toasts;
use crate::{fit_display_to_window, fitted_display_size};

const W_4_3: Vec2 = Vec2::new(1024.0, 768.0);
const W_16_9: Vec2 = Vec2::new(1920.0, 1080.0);
const W_32_9: Vec2 = Vec2::new(3840.0, 1080.0);

fn launch(toggles: &[Toggle]) -> VisualLaunchConfig {
    let mut launch = VisualLaunchConfig::classic();
    for toggle in toggles {
        launch.enhancements.toggles.set(*toggle, true);
    }
    launch
}

fn panel_launch() -> VisualLaunchConfig {
    launch(&[Toggle::JournalSidePanel])
}

/// The headless game at its idle prompt with the faithful fit, a window,
/// the frame sprite, a font that inks every cell, and the foundation and
/// journal as `launch` adds them.
fn game(launch: &VisualLaunchConfig, window: Vec2) -> Game {
    let mut game = Game::new(false);
    // Each glyph row is its code, so every character draws differently.
    let font: Vec<u8> = (0..CH_FONT_LEN).map(|at| (at / 8) as u8).collect();
    game.visual_mut().text_font = parse_ch_font(&font, IBM_CH_FILE).unwrap();
    add_enhancements(&mut game.app, launch);
    game.app.add_systems(
        Update,
        fit_display_to_window.run_if(faithful_fit_owns_frame),
    );
    let mut w = Window::default();
    w.resolution.set(window.x, window.y);
    game.app.world_mut().spawn(w);
    let image = game.visual().image_handle.clone();
    game.app.world_mut().spawn((
        Sprite {
            image,
            custom_size: Some(Vec2::new(960.0, 720.0)),
            ..default()
        },
        Transform::default(),
    ));
    game.frame();
    game
}

/// A stub scrollback panel (chapter 8 is another lane).
struct StubConversation(SidePreference);

impl SidePanelProvider for StubConversation {
    fn kind(&self) -> PanelKind {
        PanelKind::Conversation
    }

    fn side(&self) -> SidePreference {
        self.0
    }

    fn snapshot(&self, state: Option<&PlayState>) -> Option<PanelContent> {
        state?;
        Some(PanelContent::Cells(PanelRows::new("SCROLLBACK", &[])))
    }
}

fn registry(game: &mut Game) -> Mut<'_, PanelRegistry> {
    game.app.world_mut().resource_mut::<PanelRegistry>()
}

/// The granted panels' kinds and sides.
fn granted(game: &Game) -> Vec<(PanelKind, PanelSide)> {
    let layout = game.app.world().resource::<SideLayout>();
    layout
        .granted
        .iter()
        .map(|(kind, _)| (*kind, layout.side_of(*kind).unwrap()))
        .collect()
}

/// The drawn panels' kinds and images.
fn nodes(game: &mut Game) -> Vec<(PanelKind, Handle<Image>)> {
    let world = game.app.world_mut();
    // `SidePanelNode`'s kind is private to the host: tell them apart by
    // the layout, which grants at most one panel per kind.
    let Some(layout) = world.get_resource::<SideLayout>().cloned() else {
        return Vec::new();
    };
    let mut query = world.query::<(&SidePanelNode, &Node, &ImageNode)>();
    query
        .iter(world)
        .map(|(_, node, image)| {
            let kind = layout
                .granted
                .iter()
                .find(|(_, rect)| node.left == Val::Px(rect.min.x))
                .map(|(kind, _)| *kind)
                .expect("a drawn panel is granted");
            (kind, image.image.clone())
        })
        .collect()
}

fn journal_image(game: &mut Game) -> Option<Vec<u8>> {
    let handle = nodes(game)
        .into_iter()
        .find(|(kind, _)| *kind == PanelKind::Journal)?
        .1;
    game.app
        .world()
        .resource::<Assets<Image>>()
        .get(&handle)
        .and_then(|image| image.data.clone())
}

fn sprite_size(game: &mut Game) -> Vec2 {
    let world = game.app.world_mut();
    let mut query = world.query_filtered::<&Sprite, Without<JournalPageSprite>>();
    query.single(world).unwrap().custom_size.unwrap()
}

fn play_image(game: &Game) -> Vec<u8> {
    let world = game.app.world();
    world
        .resource::<Assets<Image>>()
        .get(&world.resource::<VisualState>().image_handle)
        .and_then(|image| image.data.clone())
        .unwrap()
}

fn frames(game: &mut Game, n: usize) {
    for _ in 0..n {
        game.frame();
    }
}

fn tap_ctrl_j(game: &mut Game) {
    game.press(KeyCode::ControlLeft);
    game.press(KeyCode::KeyJ);
    game.frame();
    game.release(KeyCode::KeyJ);
    game.release(KeyCode::ControlLeft);
    game.frame();
}

// ---------------------------------------------------------------------------
// Registration.

#[test]
fn no_provider_when_the_side_panel_is_off() {
    for (label, launch) in [
        ("classic", VisualLaunchConfig::classic()),
        ("page only", launch(&[Toggle::JournalPage])),
    ] {
        let mut game = game(&launch, W_32_9);
        frames(&mut game, 3);
        let world = game.app.world();
        assert!(
            world
                .get_resource::<PanelRegistry>()
                .is_none_or(|registry| registry.is_empty()),
            "{label}"
        );
        assert!(
            !world.contains_resource::<JournalPanelPageOpen>(),
            "{label}"
        );
        assert!(!world.contains_resource::<SideLayout>(), "{label}");
        assert!(nodes(&mut game).is_empty(), "{label}");
    }
}

#[test]
fn the_side_panel_registers_the_journal_provider() {
    let mut game = game(&panel_launch(), W_32_9);
    frames(&mut game, 2);
    let claims = game.app.world().resource::<PanelRegistry>().claims();
    assert_eq!(claims.len(), 1);
    assert_eq!(claims[0].kind, PanelKind::Journal);
    assert_eq!(claims[0].side, SidePreference::Auto);
    assert_eq!(granted(&game), [(PanelKind::Journal, PanelSide::Right)]);
    assert!(journal_image(&mut game).is_some(), "drawn");
}

#[test]
fn the_provider_follows_its_toggle_in_play() {
    let mut game = game(&panel_launch(), W_32_9);
    let mut toggles = game.app.world().resource::<Toggles>().0;
    toggles.set(Toggle::JournalSidePanel, false);
    game.app.world_mut().resource_mut::<Toggles>().0 = toggles;
    frames(&mut game, 2);
    assert!(game.app.world().resource::<PanelRegistry>().is_empty());
    assert!(nodes(&mut game).is_empty());
    toggles.set(Toggle::JournalSidePanel, true);
    game.app.world_mut().resource_mut::<Toggles>().0 = toggles;
    frames(&mut game, 2);
    assert_eq!(granted(&game), [(PanelKind::Journal, PanelSide::Right)]);
    assert_eq!(nodes(&mut game).len(), 1);
}

// ---------------------------------------------------------------------------
// The side, beside the scrollback panel.

/// `journal.side_panel_side` beside a stub scrollback provider at 32:9,
/// where both fit: `auto` takes the side the other does not use; a
/// conflict goes by `panel_priority` and the loser moves, with no toast.
#[test]
fn auto_left_and_right_beside_the_scrollback_panel() {
    use PanelSide::{Left, Right};
    let cases = [
        // (journal side, scrollback side, priority journal first?) -> sides
        (
            JournalPanelSide::Auto,
            SidePreference::Left,
            true,
            Right,
            Left,
        ),
        (
            JournalPanelSide::Auto,
            SidePreference::Right,
            true,
            Left,
            Right,
        ),
        (
            JournalPanelSide::Left,
            SidePreference::Auto,
            true,
            Left,
            Right,
        ),
        (
            JournalPanelSide::Right,
            SidePreference::Auto,
            true,
            Right,
            Left,
        ),
        // One side wanted by both: the winner keeps it, the loser moves.
        (
            JournalPanelSide::Left,
            SidePreference::Left,
            true,
            Left,
            Right,
        ),
        (
            JournalPanelSide::Right,
            SidePreference::Right,
            true,
            Right,
            Left,
        ),
        (
            JournalPanelSide::Left,
            SidePreference::Left,
            false,
            Right,
            Left,
        ),
        (
            JournalPanelSide::Right,
            SidePreference::Right,
            false,
            Left,
            Right,
        ),
    ];
    for (journal, scrollback, journal_first, journal_at, scrollback_at) in cases {
        let what = format!("{journal:?} {scrollback:?} journal first {journal_first}");
        let mut game = game(&panel_launch(), W_32_9);
        if !journal_first {
            game.app.world_mut().insert_resource(SideSpaceSettings {
                priority: vec![PanelKind::Conversation, PanelKind::Journal],
                ..SideSpaceSettings::default()
            });
        }
        register_journal_panel(game.app.world_mut(), journal, false);
        registry(&mut game).register(StubConversation(scrollback));
        frames(&mut game, 2);
        let mut sides = granted(&game);
        sides.sort_by_key(|(kind, _)| *kind);
        assert_eq!(
            sides,
            [
                (PanelKind::Journal, journal_at),
                (PanelKind::Conversation, scrollback_at)
            ],
            "{what}"
        );
        assert_eq!(nodes(&mut game).len(), 2, "{what}");
        assert_eq!(
            game.app.world().resource::<Toasts>().current(),
            None,
            "{what}: a move is not a hide"
        );
    }
}

/// At 16:9 only one panel keeps the frame within the guard: the
/// `panel_priority` loser is hidden with one toast, said once.
#[test]
fn the_priority_loser_is_hidden_with_one_toast() {
    for journal_first in [true, false] {
        let mut game = game(&panel_launch(), W_16_9);
        if !journal_first {
            game.app.world_mut().insert_resource(SideSpaceSettings {
                priority: vec![PanelKind::Conversation, PanelKind::Journal],
                ..SideSpaceSettings::default()
            });
        }
        registry(&mut game).register(StubConversation(SidePreference::Auto));
        game.frame();
        let winner = if journal_first {
            PanelKind::Journal
        } else {
            PanelKind::Conversation
        };
        assert_eq!(granted(&game).len(), 1, "journal first {journal_first}");
        assert_eq!(granted(&game)[0].0, winner);
        assert_eq!(
            game.app.world().resource::<Toasts>().current(),
            Some(PANEL_HIDDEN)
        );
        frames(&mut game, 200);
        assert_eq!(
            game.app.world().resource::<Toasts>().current(),
            None,
            "said once"
        );
        assert_eq!(granted(&game)[0].0, winner);
    }
}

// ---------------------------------------------------------------------------
// Grow-left.

/// While grow-left shows the world its canvas fills the window and there
/// is no bar: the panel is hidden with one toast. On a fixed screen (no
/// canvas) it shows again, at every wide aspect.
#[test]
fn hidden_under_grow_left_and_shown_on_its_fixed_screens() {
    for window in [W_16_9, Vec2::new(2560.0, 1080.0), W_32_9] {
        let mut game = game(&panel_launch(), window);
        let canvas = Canvas::for_window(window.x as u32, window.y as u32, PixelAspect::Dos)
            .expect("a wide window has a canvas");
        // The world view: grow-left presents its canvas.
        game.app
            .world_mut()
            .insert_resource(FrameImageSize(Some(UVec2::new(canvas.w, canvas.h))));
        game.frame();
        assert!(granted(&game).is_empty(), "{window}: hidden");
        assert!(nodes(&mut game).is_empty(), "{window}");
        assert_eq!(
            game.app.world().resource::<Toasts>().current(),
            Some(PANEL_HIDDEN),
            "{window}"
        );
        // A fixed screen (combat, the dungeon view): the 4:3 frame again.
        game.app.world_mut().insert_resource(FrameImageSize(None));
        game.frame();
        assert_eq!(
            granted(&game),
            [(PanelKind::Journal, PanelSide::Right)],
            "{window}: shown"
        );
        assert_eq!(nodes(&mut game).len(), 1, "{window}");
        // Back to the world view: hidden again, and not said again.
        frames(&mut game, 200);
        game.app
            .world_mut()
            .insert_resource(FrameImageSize(Some(UVec2::new(canvas.w, canvas.h))));
        game.frame();
        assert!(granted(&game).is_empty(), "{window}");
        assert_eq!(game.app.world().resource::<Toasts>().current(), None);
    }
}

// ---------------------------------------------------------------------------
// The page.

/// With the page off the panel works and Ctrl+J is still dropped.
#[test]
fn works_with_the_page_off_and_ctrl_j_is_dropped() {
    let mut game = game(&panel_launch(), W_32_9);
    frames(&mut game, 2);
    let before = journal_image(&mut game).expect("drawn");
    tap_ctrl_j(&mut game);
    assert!(!game.app.world().contains_resource::<QuestJournalUi>());
    assert!(!game.app.world().contains_resource::<OverlayState>());
    assert!(game.visual().typed_keys.is_empty(), "dropped, not typed");
    assert_eq!(journal_image(&mut game), Some(before));
}

/// While the page is open the panel shows nothing (the page shows it
/// all); its slot is kept, and it comes back when the page closes.
#[test]
fn hidden_while_the_page_is_open() {
    let mut game = game(
        &launch(&[Toggle::JournalPage, Toggle::JournalSidePanel]),
        W_32_9,
    );
    frames(&mut game, 2);
    assert_eq!(nodes(&mut game).len(), 1);
    let frame = sprite_size(&mut game);
    tap_ctrl_j(&mut game);
    assert_eq!(
        game.app.world().resource::<OverlayState>().open(),
        Some(OverlayId::Journal)
    );
    assert!(nodes(&mut game).is_empty(), "no panel under the page");
    assert_eq!(granted(&game).len(), 1, "the slot is kept");
    assert_eq!(sprite_size(&mut game), frame, "the frame does not move");
    game.press(KeyCode::Escape);
    game.frame();
    game.release(KeyCode::Escape);
    game.frame();
    assert_eq!(game.app.world().resource::<OverlayState>().open(), None);
    assert_eq!(nodes(&mut game).len(), 1, "back after close");
}

#[test]
fn absent_in_the_intro_with_no_play_state() {
    let provider = JournalPanelProvider::new(JournalPanelSide::Auto, Default::default());
    assert_eq!(provider.snapshot(None), None);
}

// ---------------------------------------------------------------------------
// Rows, the play image and the frame.

#[test]
fn rows_change_only_when_the_model_changes() {
    let mut game = game(&panel_launch(), W_32_9);
    frames(&mut game, 2);
    let image = nodes(&mut game)[0].1.clone();
    let pixels = journal_image(&mut game).unwrap();
    let assets = game.app.world().resource::<Assets<Image>>().len();
    // The game moves on (time, animation) but no journal fact changes.
    frames(&mut game, 30);
    assert_eq!(nodes(&mut game)[0].1, image);
    assert_eq!(journal_image(&mut game).unwrap(), pixels);
    assert_eq!(game.app.world().resource::<Assets<Image>>().len(), assets);
    // A shrine quest begins: the same image, repainted.
    game.visual_mut().state.shrine_ordained_mask = 0b0000_0001;
    game.frame();
    assert_eq!(nodes(&mut game)[0].1, image);
    assert_ne!(journal_image(&mut game).unwrap(), pixels);
}

#[test]
fn the_play_image_is_the_same_with_the_panel_on_or_off() {
    let mut off = game(&VisualLaunchConfig::classic(), W_32_9);
    let mut on = game(&panel_launch(), W_32_9);
    for _ in 0..20 {
        off.frame();
        on.frame();
        assert!(play_image(&off) == play_image(&on));
    }
    assert_eq!(nodes(&mut on).len(), 1);
}

#[test]
fn the_frame_size_is_unchanged_unless_a_panel_is_granted() {
    // 4:3: no room, nothing granted, the 1.0.1 fit.
    let mut narrow = game(&panel_launch(), W_4_3);
    frames(&mut narrow, 3);
    assert!(granted(&narrow).is_empty());
    assert_eq!(
        sprite_size(&mut narrow),
        fitted_display_size(VISUAL_PLAY_FRAME_WIDTH, VISUAL_PLAY_FRAME_HEIGHT, W_4_3)
    );
    // 32:9: the panel fits beside the unshrunk frame.
    let mut wide = game(&panel_launch(), W_32_9);
    frames(&mut wide, 3);
    assert_eq!(granted(&wide).len(), 1);
    let fitted = fitted_display_size(VISUAL_PLAY_FRAME_WIDTH, VISUAL_PLAY_FRAME_HEIGHT, W_32_9);
    assert!((sprite_size(&mut wide) - fitted).abs().max_element() < 0.01);
    // 16:9: granted, and the frame shrinks within the guard.
    let mut granted_16_9 = game(&panel_launch(), W_16_9);
    frames(&mut granted_16_9, 3);
    let frame = granted_16_9
        .app
        .world()
        .resource::<SideLayout>()
        .frame
        .expect("granted");
    assert_eq!(sprite_size(&mut granted_16_9), frame.size());
}

// ---------------------------------------------------------------------------
// The model and its rows (5.2.10).

fn body(model: &SidePanelModel) -> Vec<String> {
    let rows = model.rows();
    assert_eq!(
        std::str::from_utf8(rows.title()).unwrap().trim_end(),
        PANEL_TITLE
    );
    (0..crate::enh::side::PANEL_BODY_ROWS)
        .map(|row| {
            std::str::from_utf8(rows.row(row))
                .unwrap()
                .trim_end()
                .to_string()
        })
        .collect()
}

fn fresh() -> QuestSnapshot {
    QuestSnapshot::from_play_state(&u5_runtime::test_fixtures::test_state(
        u5_runtime::test_fixtures::open_grid(),
        5,
        5,
    ))
}

#[test]
fn nothing_found_shows_the_title_the_note_and_eight_virtues() {
    let rows = body(&SidePanelModel::from_snapshot(&fresh()));
    assert_eq!(rows[0], "Nothing recorded");
    assert_eq!(rows[1], "yet");
    assert_eq!(rows[2], "");
    let names: Vec<&str> = ShrineVirtue::ALL.iter().map(|v| v.name()).collect();
    for (at, name) in names.iter().enumerate() {
        assert_eq!(rows[3 + at], format!("{name:<14}-"));
    }
    assert!(rows[11..].iter().all(String::is_empty), "{rows:?}");
}

#[test]
fn the_sketch_rows() {
    let mut snap = fresh();
    snap.ordained_mask = 0b0000_0110;
    snap.codex_mask = 0b1001_0011;
    snap.ruin[6] = 0x80;
    snap.ruin[3] = 0x01;
    snap.seals[0] = 0x80;
    snap.seals[4] = 0x80;
    snap.shadowlords[0] = ShadowlordState::Vanquished;
    snap.shadowlords[1] = ShadowlordState::AtLarge;
    snap.special_items[SPECIAL_ITEM_CROWN_LB_INDEX] = 1;
    snap.special_items[SPECIAL_ITEM_WOODEN_BOX_INDEX] = 1;
    let rows = body(&SidePanelModel::from_snapshot(&snap));
    assert_eq!(
        rows[..20],
        [
            "3 done",
            "2 in progress",
            "1 ruined",
            "",
            "Honesty       *",
            "Compassion    <",
            "Valor         >",
            "Justice       -",
            "Sacrifice     *",
            "Honor         -",
            "Spirituality  !",
            "Humility      *",
            "",
            "Vanquished   1",
            "Seals open   2",
            "Crown",
            "Sandalwood Box",
            "",
            "",
            "",
        ]
    );
}

/// The panel never shows a mantra, a name not found, a denominator, a
/// spoiler label, or a Shadowlord only at large.
#[test]
fn the_panel_shows_nothing_above_tier_2() {
    let mut snap = fresh();
    snap.ordained_mask = 0;
    snap.codex_mask = 0xff;
    snap.shadowlords = [ShadowlordState::AtLarge; 3];
    let rows = body(&SidePanelModel::from_snapshot(&snap));
    let all = rows.join("\n");
    for virtue in ShrineVirtue::ALL {
        assert!(
            !all.contains(u5_runtime::shrine_mantra_for(virtue)),
            "{virtue:?}'s mantra"
        );
    }
    assert!(!all.contains('/'), "no denominator");
    assert!(!all.contains(text::SPOILERS));
    assert!(!all.contains(PANEL_VANQUISHED), "at large is not evidence");
    assert!(!all.contains(PANEL_SEALS_OPEN));
    for (label, _, _) in text::REGALIA {
        assert!(!all.contains(label), "{label} is not carried");
    }
    // The same rows whatever tier the page uses: the panel has no tier.
    assert!(all.contains("8 done"));
}

#[test]
fn panel_side_maps_every_word() {
    assert_eq!(panel_side(JournalPanelSide::Auto), SidePreference::Auto);
    assert_eq!(panel_side(JournalPanelSide::Left), SidePreference::Left);
    assert_eq!(panel_side(JournalPanelSide::Right), SidePreference::Right);
}
