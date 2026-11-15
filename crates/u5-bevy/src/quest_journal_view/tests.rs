//! The journal page shell (plan M5-03; design 5.6 G1, G5-G8): when the
//! plugin and the page exist, opening and closing through the overlay
//! model, the play image untouched, the order rule, the refusals, closing
//! when the game gets busy, the page over the frame under every layout, and
//! the frame's pixels.

use u5_runtime::enhancements::{LockReason, LockState, SpoilerTier};
use u5_runtime::z_stats::{YesNoPromptKind, YesNoPromptSession};
use u5_runtime::{
    CH_FONT_LEN, COMBAT_ACTOR_FLAG_SELECTABLE_80, CombatActorDescriptor, IBM_CH_FILE, parse_ch_font,
};

use super::*;
use crate::enh::input::HotkeyFellThrough;
use crate::enh::overlay::OverlayRefused;
use crate::enh::overlay::tests::Game;
use crate::enh::toast::Toasts;
use crate::enh::{EnhancementsPlugin, add_enhancements};
use crate::{SpeakerVoice, TypedKey};

fn launch(toggles: &[Toggle]) -> VisualLaunchConfig {
    let mut launch = VisualLaunchConfig::classic();
    for toggle in toggles {
        launch.enhancements.toggles.set(*toggle, true);
    }
    launch
}

fn page_launch() -> VisualLaunchConfig {
    launch(&[Toggle::JournalPage])
}

/// The headless game at its idle prompt with the foundation and the
/// journal as `launch` adds them (nothing under Classic).
fn game(launch: &VisualLaunchConfig) -> Game {
    let mut game = Game::new(false);
    add_enhancements(&mut game.app, launch);
    game.frame();
    game
}

fn has_system(app: &App, name: &str) -> bool {
    app.world()
        .resource::<bevy::ecs::schedule::Schedules>()
        .iter()
        .any(|(_, schedule)| {
            schedule
                .graph()
                .systems()
                .any(|(_, system, _)| system.name().contains(name))
        })
}

fn journal_systems(app: &App) -> bool {
    has_system(app, "quest_journal_view")
}

fn tap_ctrl_j(game: &mut Game) {
    game.press(KeyCode::ControlLeft);
    game.press(KeyCode::KeyJ);
    game.frame();
    game.release(KeyCode::KeyJ);
    game.release(KeyCode::ControlLeft);
    game.frame();
}

fn tap(game: &mut Game, key: KeyCode) {
    game.press(key);
    game.frame();
    game.release(key);
    game.frame();
}

fn open(game: &Game) -> bool {
    game.app.world().resource::<OverlayState>().open() == Some(OverlayId::Journal)
}

fn ui(game: &Game) -> &QuestJournalUi {
    game.app.world().resource::<QuestJournalUi>()
}

fn page_visibility(game: &mut Game) -> Visibility {
    let entity = ui(game).page_entity().unwrap();
    *game.app.world().get::<Visibility>(entity).unwrap()
}

fn play_image(game: &Game) -> Vec<u8> {
    let world = game.app.world();
    world
        .resource::<Assets<Image>>()
        .get(&world.resource::<VisualState>().image_handle)
        .and_then(|image| image.data.clone())
        .unwrap()
}

fn page_pixels(game: &Game) -> Vec<u8> {
    let world = game.app.world();
    let handle = world.resource::<QuestJournalUi>().page_image().unwrap();
    world
        .resource::<Assets<Image>>()
        .get(handle)
        .and_then(|image| image.data.clone())
        .unwrap()
}

// ---------------------------------------------------------------------------
// When the plugin and the page exist.

#[test]
fn classic_the_lock_and_other_features_add_no_journal() {
    let mut locked = page_launch();
    locked.enhancements.lock = LockState::Locked(LockReason::QaEnv);
    let mut other = launch(&[Toggle::GemMapEnhanced]);
    other.enhancements.toggles.set(Toggle::GemMapEnhanced, true);
    for (label, launch) in [
        ("classic", VisualLaunchConfig::classic()),
        ("locked", locked),
        ("gem map alone", other),
    ] {
        let mut app = App::new();
        add_enhancements(&mut app, &launch);
        assert!(!app.is_plugin_added::<QuestJournalPlugin>(), "{label}");
        assert!(
            !app.world().contains_resource::<QuestJournalUi>(),
            "{label}"
        );
        assert!(!journal_systems(&app), "{label}");
    }
}

#[test]
fn the_page_toggle_adds_the_plugin_and_the_page() {
    let mut app = App::new();
    add_enhancements(&mut app, &page_launch());
    assert!(app.is_plugin_added::<EnhancementsPlugin>());
    assert!(app.is_plugin_added::<QuestJournalPlugin>());
    let ui = app.world().resource::<QuestJournalUi>();
    assert!(ui.settings().page);
    assert_eq!(ui.settings().tier, SpoilerTier::Goals);
    assert!(app.world().contains_resource::<OverlayState>());
    for system in [
        "journal_page_input",
        "paint_quest_journal_page",
        "align_journal_page",
    ] {
        assert!(has_system(&app, system), "{system}");
    }
}

/// The side panel alone (M5-05) adds the plugin but not the page: no
/// `QuestJournalUi` and Ctrl+J stays unbound.
#[test]
fn the_side_panel_alone_adds_no_page() {
    let mut app = App::new();
    add_enhancements(&mut app, &launch(&[Toggle::JournalSidePanel]));
    assert!(app.is_plugin_added::<QuestJournalPlugin>());
    assert!(!app.world().contains_resource::<QuestJournalUi>());
    assert!(!app.world().contains_resource::<OverlayState>());
}

/// One tab (Quests) and no strip: the seam chapter 8 adds tabs to.
#[test]
fn the_page_has_one_tab_and_no_strip() {
    let ui = QuestJournalUi::new(JournalSettings {
        page: true,
        ..JournalSettings::default()
    });
    assert_eq!(ui.tabs(), [JournalTab::Quests]);
    assert_eq!(ui.tab(), JournalTab::Quests);
    assert!(!ui.draws_tab_strip());
}

/// The plugin's systems are in both app chains, the `--visual` app and the
/// default player path.
#[test]
fn the_page_systems_exist_in_both_app_chains() {
    for intro in [false, true] {
        let mut app = App::new();
        if intro {
            crate::add_visual_intro_update_systems(&mut app);
        } else {
            crate::add_visual_update_systems(&mut app);
        }
        add_enhancements(&mut app, &page_launch());
        for system in [
            "journal_page_input",
            "paint_quest_journal_page",
            "align_journal_page",
            "follow_journal_toggle",
        ] {
            assert!(has_system(&app, system), "{system} (intro {intro})");
        }
    }
}

/// The menu overlay switching the page on and off in play inserts and
/// removes the page, its sprite and its image.
#[test]
fn the_page_follows_its_toggle_in_play() {
    let mut game = game(&page_launch());
    let entity = ui(&game).page_entity().unwrap();
    let image = ui(&game).page_image().unwrap().clone();
    game.app
        .world_mut()
        .resource_mut::<Toggles>()
        .0
        .set(Toggle::JournalPage, false);
    game.frame();
    assert!(!game.app.world().contains_resource::<QuestJournalUi>());
    assert!(game.app.world().get_entity(entity).is_err());
    assert!(
        game.app
            .world()
            .resource::<Assets<Image>>()
            .get(&image)
            .is_none()
    );
    game.app
        .world_mut()
        .resource_mut::<Toggles>()
        .0
        .set(Toggle::JournalPage, true);
    game.frame();
    game.frame();
    assert!(ui(&game).page_entity().is_some());
}

// ---------------------------------------------------------------------------
// G1: the page absent.

/// Without the page (Classic, and the side panel alone) Ctrl+J is dropped
/// and F1 gives the faithful refusal; the game and its frame are the
/// Classic run's.
#[test]
fn g1_without_the_page_ctrl_j_is_dropped_and_f1_is_faithful() {
    let mut classic = game(&VisualLaunchConfig::classic());
    let mut panel = game(&launch(&[Toggle::JournalSidePanel]));
    for game in [&mut classic, &mut panel] {
        tap_ctrl_j(game);
        assert!(!game.app.world().contains_resource::<QuestJournalUi>());
        assert!(game.visual().typed_keys.is_empty());
    }
    assert_eq!(classic.game_view(), panel.game_view());
    let mut quiet = game(&VisualLaunchConfig::classic());
    quiet.frame();
    quiet.frame();
    assert_eq!(
        classic.where_the_avatar_is(),
        quiet.where_the_avatar_is(),
        "Ctrl+J did nothing"
    );
    for game in [&mut classic, &mut panel] {
        tap(game, KeyCode::F1);
        for _ in 0..30 {
            game.frame();
        }
    }
    assert_eq!(classic.game_view(), panel.game_view());
    assert!(
        classic.visual().state.message.contains("What?"),
        "{:?}",
        classic.visual().state.message
    );
}

// ---------------------------------------------------------------------------
// Opening and closing.

#[test]
fn ctrl_j_opens_the_page_and_esc_or_ctrl_j_close_it() {
    let mut game = game(&page_launch());
    assert!(!open(&game));
    assert_eq!(page_visibility(&mut game), Visibility::Hidden);
    tap_ctrl_j(&mut game);
    assert!(open(&game));
    assert_eq!(page_visibility(&mut game), Visibility::Visible);
    assert!(ui(&game).model().is_some());
    assert!(game.visual().typed_keys.is_empty());

    tap(&mut game, KeyCode::Escape);
    assert!(!open(&game));
    assert_eq!(page_visibility(&mut game), Visibility::Hidden);
    assert!(
        game.visual().typed_keys.is_empty(),
        "Esc never reaches the game"
    );

    tap_ctrl_j(&mut game);
    assert!(open(&game));
    // J alone does not close it; Ctrl+J does.
    tap(&mut game, KeyCode::KeyJ);
    assert!(open(&game));
    tap_ctrl_j(&mut game);
    assert!(!open(&game));
    assert!(game.visual().typed_keys.is_empty());
}

// ---------------------------------------------------------------------------
// G5: the play image.

/// The play image's bytes are the same with the page off, on but closed,
/// and open: the page is its own image on its own sprite.
#[test]
fn g5_the_play_image_is_the_same_off_closed_and_open() {
    let mut off = game(&VisualLaunchConfig::classic());
    let mut closed = game(&page_launch());
    let mut opened = game(&page_launch());
    tap_ctrl_j(&mut opened);
    for _ in 0..2 {
        off.frame();
        closed.frame();
    }
    assert!(open(&opened));
    for frame in 0..120 {
        off.frame();
        closed.frame();
        opened.frame();
        assert_eq!(play_image(&off), play_image(&closed), "frame {frame}");
        assert_eq!(play_image(&off), play_image(&opened), "frame {frame}");
        assert_eq!(off.game_view(), opened.game_view(), "frame {frame}");
    }
    assert_ne!(page_pixels(&opened), play_image(&opened));
}

// ---------------------------------------------------------------------------
// G6: the order rule.

/// Ctrl+J behind another key of the same frame, or with a key queued, does
/// not open the page, and the other key goes to the game first.
#[test]
fn g6_ctrl_j_behind_another_key_does_not_open() {
    let mut game = game(&page_launch());
    game.press(KeyCode::KeyX);
    game.press(KeyCode::ControlLeft);
    game.press(KeyCode::KeyJ);
    game.app.update();
    assert_eq!(
        game.events::<HotkeyFellThrough>(),
        [HotkeyFellThrough(EnhAction::OpenJournal)]
    );
    game.app
        .world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear();
    assert!(!open(&game));
    for key in [KeyCode::KeyX, KeyCode::KeyJ, KeyCode::ControlLeft] {
        game.release(key);
    }
    game.frame();
    assert!(!open(&game));

    let mut queued = self::game(&page_launch());
    queued.visual_mut().typed_keys.push_back(TypedKey {
        key: KeyCode::KeyX,
        shift: false,
        control: false,
    });
    queued.press(KeyCode::ControlLeft);
    queued.press(KeyCode::KeyJ);
    queued.app.update();
    assert_eq!(
        queued.events::<HotkeyFellThrough>(),
        [HotkeyFellThrough(EnhAction::OpenJournal)]
    );
    assert!(!open(&queued));
}

// ---------------------------------------------------------------------------
// G7: refusals.

/// Each state the idle predicate refuses, and a combat member's prompt,
/// refuses the page with one toast.
#[test]
fn g7_each_busy_state_refuses_with_one_toast() {
    let refusals: [(&str, fn(&mut Game)); 5] = [
        ("yes/no prompt", |game| {
            game.visual_mut().state.active_yes_no_prompt = Some(YesNoPromptSession {
                kind: YesNoPromptKind::SaveGame,
            });
        }),
        ("speaker voice", |game| {
            game.app.world_mut().spawn(SpeakerVoice);
        }),
        ("idle redraw owed", |game| {
            game.visual_mut().sweep_idle_redraw_pending = true;
        }),
        ("party asleep", |game| {
            for member in &mut game.visual_mut().state.party {
                member.status = b'S';
            }
        }),
        ("combat prompt", |game| {
            let mut visual = game.visual_mut();
            let state = &mut visual.state;
            state.combat_active = true;
            state.combat_actors[0] = CombatActorDescriptor::from_row([
                30,
                1,
                COMBAT_ACTOR_FLAG_SELECTABLE_80,
                0,
                0,
                0,
                5,
                5,
            ]);
            state.pending_combat_actor_slot = Some(0);
        }),
    ];
    for (label, setup) in refusals {
        let mut game = game(&page_launch());
        setup(&mut game);
        game.press(KeyCode::ControlLeft);
        game.press(KeyCode::KeyJ);
        game.app.update();
        assert!(!open(&game), "{label}");
        assert_eq!(
            game.events::<OverlayRefused>(),
            [OverlayRefused(OverlayId::Journal)],
            "{label}: one refusal"
        );
        assert_eq!(
            game.app.world().resource::<Toasts>().current(),
            Some(OverlayId::Journal.refusal()),
            "{label}"
        );
        assert_eq!(page_visibility(&mut game), Visibility::Hidden, "{label}");
    }
}

// ---------------------------------------------------------------------------
// G8: busy under the page.

/// A busy state that begins under the open page closes it; no key typed
/// while it was open reaches the game.
#[test]
fn g8_the_page_closes_when_the_game_gets_busy() {
    let mut game = game(&page_launch());
    tap_ctrl_j(&mut game);
    assert!(open(&game));
    for key in [KeyCode::KeyA, KeyCode::ArrowUp, KeyCode::Enter] {
        tap(&mut game, key);
    }
    let start = game.where_the_avatar_is();
    game.visual_mut().state.active_yes_no_prompt = Some(YesNoPromptSession {
        kind: YesNoPromptKind::SaveGame,
    });
    game.frame();
    assert!(!open(&game));
    assert_eq!(page_visibility(&mut game), Visibility::Hidden);
    assert!(game.visual().typed_keys.is_empty());
    assert_eq!(game.where_the_avatar_is(), start);
}

// ---------------------------------------------------------------------------
// The page over the frame.

fn sprites(game: &mut Game) -> ((Vec2, Vec2), (Vec2, Vec2)) {
    let world = game.app.world_mut();
    let mut frame = None;
    let mut page = None;
    let mut query = world.query::<(&Sprite, &Transform, Has<JournalPageSprite>)>();
    for (sprite, transform, is_page) in query.iter(world) {
        let rect = (
            sprite.custom_size.unwrap(),
            transform.translation.truncate(),
        );
        if is_page {
            assert_eq!(transform.translation.z, PAGE_Z);
            page = Some(rect);
        } else {
            frame = Some(rect);
        }
    }
    (frame.unwrap(), page.unwrap())
}

fn spawn_frame(game: &mut Game, size: Vec2, at: Vec2) -> Entity {
    let image = game.visual().image_handle.clone();
    game.app
        .world_mut()
        .spawn((
            Sprite {
                image,
                custom_size: Some(size),
                ..default()
            },
            Transform::from_xyz(at.x, at.y, 0.0),
        ))
        .id()
}

#[test]
fn the_page_covers_the_faithful_frame_and_a_moved_frame() {
    let mut game = game(&page_launch());
    // The faithful fit: the 4:3 frame, centred.
    let frame = spawn_frame(&mut game, Vec2::new(1280.0, 960.0), Vec2::ZERO);
    game.frame();
    let (frame_rect, page_rect) = sprites(&mut game);
    assert_eq!(frame_rect, page_rect);
    // A granted panel moves and shrinks the frame; the page follows.
    {
        let world = game.app.world_mut();
        world.get_mut::<Sprite>(frame).unwrap().custom_size = Some(Vec2::new(1000.0, 750.0));
        world.get_mut::<Transform>(frame).unwrap().translation = Vec3::new(140.0, -15.0, 0.0);
    }
    game.frame();
    let (frame_rect, page_rect) = sprites(&mut game);
    assert_eq!(frame_rect, page_rect);
    assert_eq!(
        page_rect,
        (Vec2::new(1000.0, 750.0), Vec2::new(140.0, -15.0))
    );
}

/// On a grow-left canvas the frame sprite shows the whole canvas; the
/// page keeps the game's own pixel size and sits centred on it.
#[test]
fn the_page_keeps_the_game_pixel_size_on_a_grow_left_canvas() {
    let mut game = game(&page_launch());
    game.app
        .world_mut()
        .insert_resource(FrameImageSize(Some(UVec2::new(512, 240))));
    spawn_frame(&mut game, Vec2::new(2560.0, 1440.0), Vec2::new(0.0, 0.0));
    game.frame();
    let (_, (size, centre)) = sprites(&mut game);
    assert_eq!(size, Vec2::new(1600.0, 1200.0));
    assert_eq!(centre, Vec2::ZERO);
    assert_eq!(
        page_rect(
            Vec2::new(3440.0, 1440.0),
            Vec2::new(5.0, 7.0),
            UVec2::new(688, 240)
        ),
        (Vec2::new(1600.0, 1200.0), Vec2::new(5.0, 7.0))
    );
    assert_eq!(
        page_rect(Vec2::new(640.0, 480.0), Vec2::ZERO, UVec2::new(320, 200)),
        (Vec2::new(640.0, 480.0), Vec2::ZERO)
    );
}

// ---------------------------------------------------------------------------
// The painter.

fn solid_font() -> FixedCellFont {
    parse_ch_font(&vec![0xff; CH_FONT_LEN], IBM_CH_FILE).unwrap()
}

fn blank_font() -> FixedCellFont {
    parse_ch_font(&vec![0x00; CH_FONT_LEN], IBM_CH_FILE).unwrap()
}

fn paint(font: &FixedCellFont, model: Option<&JournalModel>) -> Vec<u8> {
    let mut rgba = vec![0; PAGE_WIDTH * PAGE_HEIGHT * 4];
    paint_page(
        &mut rgba,
        font,
        &PageView::new(model, model.and_then(|model| model.sections.first())),
    );
    rgba
}

fn pixel(rgba: &[u8], x: usize, y: usize) -> Rgb {
    let offset = (y * PAGE_WIDTH + x) * 4;
    [rgba[offset], rgba[offset + 1], rgba[offset + 2]]
}

const BLUE: Rgb = BAND;

#[test]
fn the_frame_bands_rules_and_corners_are_the_game_s() {
    let rgba = paint(&blank_font(), None);
    // Bands on all four sides, rules inside them.
    for (x, y) in [(3, 100), (316, 100), (40, 3), (20, 196)] {
        assert_eq!(pixel(&rgba, x, y), BLUE, "band at ({x}, {y})");
    }
    for (x, y) in [(7, 100), (312, 100), (50, 7), (20, 192), (300, 192)] {
        assert_eq!(pixel(&rgba, x, y), WHITE, "rule at ({x}, {y})");
    }
    assert_eq!(pixel(&rgba, 100, 100), BLACK, "interior");
    // The rounded corners carry the gameplay frame's profile.
    for (row, start) in CHROME_CORNER_PROFILE.iter().enumerate() {
        let start = *start as usize;
        for (x, y) in [
            (start, row),
            (319 - start, row),
            (start, 199 - row),
            (319 - start, 199 - row),
        ] {
            assert_eq!(pixel(&rgba, x, y), BLUE, "corner fill ({x}, {y})");
        }
        if start > 0 {
            for (x, y) in [
                (start - 1, row),
                (320 - start, row),
                (start - 1, 199 - row),
                (320 - start, 199 - row),
            ] {
                assert_eq!(pixel(&rgba, x, y), BLACK, "corner carve ({x}, {y})");
            }
        }
    }
    // The rules close the corners with no gap.
    for (x, y) in [(7, 7), (312, 7), (7, 192), (312, 192)] {
        assert_eq!(pixel(&rgba, x, y), WHITE, "rule corner ({x}, {y})");
    }
}

/// A divider band joins the side bands: the side rules stop at its two
/// rules, which run from side rule to side rule.
#[test]
fn the_divider_bands_join_the_side_bands() {
    let rgba = paint(&blank_font(), None);
    for row in [UPPER_DIVIDER_ROW, LOWER_DIVIDER_ROW] {
        let (top, under) = divider_rules(row);
        for x in RULE_LEFT_X..=RULE_RIGHT_X {
            assert_eq!(pixel(&rgba, x, top), WHITE, "row {row} top rule x {x}");
            assert_eq!(pixel(&rgba, x, under), WHITE, "row {row} lower rule x {x}");
        }
        for y in top + 1..under {
            for x in 0..PAGE_WIDTH {
                assert_eq!(pixel(&rgba, x, y), BLUE, "row {row} band ({x}, {y})");
            }
        }
        // The side rules meet the divider's rules: no gap, no stub.
        assert_eq!(pixel(&rgba, RULE_LEFT_X, top - 1), WHITE);
        assert_eq!(pixel(&rgba, RULE_LEFT_X, under + 1), WHITE);
        assert_eq!(pixel(&rgba, RULE_LEFT_X - 1, top), BLUE);
        assert_eq!(pixel(&rgba, RULE_RIGHT_X + 1, under), BLUE);
    }
    // Every side rule pixel outside the bands is white.
    for y in RULE_TOP_Y..=RULE_BOTTOM_Y {
        let in_band = [UPPER_DIVIDER_ROW, LOWER_DIVIDER_ROW].iter().any(|row| {
            let (top, under) = divider_rules(*row);
            y > top && y < under
        });
        let want = if in_band { BLUE } else { WHITE };
        assert_eq!(pixel(&rgba, RULE_LEFT_X, y), want, "left rule y {y}");
        assert_eq!(pixel(&rgba, RULE_RIGHT_X, y), want, "right rule y {y}");
    }
}

/// The title and key help tabs are punched through their bands as the
/// gameplay frame's labels are: black gap, rule stopping at the caps.
#[test]
fn the_title_and_key_help_tabs_cut_their_bands() {
    let rgba = paint(&blank_font(), None);
    for (row, cells, rule_y) in [
        (TITLE_ROW, text::TITLE.len(), RULE_TOP_Y),
        (KEY_HELP_ROW, KEY_HELP.len(), RULE_BOTTOM_Y),
    ] {
        let (left_cap, first, right_cap) = band_tab_columns(cells);
        assert_eq!(first, left_cap + 1);
        assert_eq!(right_cap as usize, first as usize + cells);
        // Centred: the margins differ by at most one cell.
        let left = left_cap as usize;
        let right = 39 - right_cap as usize;
        assert!(left.abs_diff(right) <= 1, "row {row}: {left} / {right}");
        let x0 = left_cap as usize * 8;
        let x1 = right_cap as usize * 8 + 7;
        assert_eq!(pixel(&rgba, x0, rule_y), WHITE, "row {row} rule meets cap");
        assert_eq!(pixel(&rgba, x1, rule_y), WHITE, "row {row} rule meets cap");
        for x in x0 + 8..x1 - 7 {
            for y in row as usize * 8..row as usize * 8 + 8 {
                assert_eq!(pixel(&rgba, x, y), BLACK, "row {row} gap ({x}, {y})");
            }
        }
    }
}

/// Text sits on the cell grid inside the frame: the title in white, the
/// header in brown, body rows inside the rules; nothing paints over a band.
#[test]
fn text_sits_on_the_grid_inside_the_frame() {
    let state = u5_runtime::test_fixtures::test_state(u5_runtime::test_fixtures::open_grid(), 5, 5);
    let model = derive(&QuestSnapshot::from_play_state(&state), SpoilerTier::Goals);
    let rgba = paint(&solid_font(), Some(&model));
    let (_, first, _) = band_tab_columns(text::TITLE.len());
    assert_eq!(pixel(&rgba, first as usize * 8, 0), WHITE, "title text");
    assert_eq!(
        pixel(&rgba, TEXT_COLUMN as usize * 8, HEADER_ROW as usize * 8),
        BROWN,
        "header"
    );
    assert_eq!(BROWN, [0xAA, 0x55, 0x00], "brown #AA5500, not slot 6");
    assert_eq!(pixel(&rgba, 3, 100), BLUE, "the band is not painted over");
    assert_eq!(pixel(&rgba, RULE_RIGHT_X, 100), WHITE, "nor the rule");
    // The first row is selected: white on the blue bar.
    let columns = columns(SectionId::Virtues, SpoilerTier::Goals, None);
    let body = ROWS_FIRST_ROW as usize * 8;
    assert_eq!(pixel(&rgba, columns.label as usize * 8, body), WHITE);
    assert_eq!(
        pixel(&rgba, columns.mark.unwrap() as usize * 8, body),
        ega(model.sections[0].rows[0].mark.colour)
    );
}

/// No-spoiler rule (foundation 1.4.7): below tier 3 the title carries no
/// SPOILERS word and the title band has no brown; at tiers 3 and 4 it does.
#[test]
fn spoilers_shows_in_the_title_only_at_tiers_3_and_4() {
    let state = u5_runtime::test_fixtures::test_state(u5_runtime::test_fixtures::open_grid(), 5, 5);
    for tier in SpoilerTier::ALL {
        let model = derive(&QuestSnapshot::from_play_state(&state), tier);
        let rgba = paint(&solid_font(), Some(&model));
        let brown = (0..8).any(|y| (0..PAGE_WIDTH).any(|x| pixel(&rgba, x, y) == BROWN));
        assert_eq!(brown, tier.is_spoiler(), "{tier:?}");
    }
}

/// The page repaints only when it opens or its model changes.
#[test]
fn the_page_repaints_only_on_a_change() {
    let mut game = game(&page_launch());
    tap_ctrl_j(&mut game);
    let painted = page_pixels(&game);
    assert!(!ui(&game).dirty);
    game.frame();
    assert_eq!(page_pixels(&game), painted);
    game.visual_mut().state.shrine_ordained_mask = 0x01;
    game.frame();
    assert_ne!(ui(&game).model().unwrap().header, text::NOTHING_RECORDED);
    assert!(!ui(&game).dirty);
}
