//! The menu on screen (plan M4-05): the kit's render in the shell, the
//! node over the frame sprite at every window, the backdrop, the sweep,
//! resizing, keys during the sweep and the faithful picture untouched.

use bevy::input::ButtonState;
use bevy::window::WindowResolution;
use u5_runtime::enhancements::{
    CliSetting, HostCapabilities, LockState, ResolveInput, Toggle, resolve,
};
use u5_runtime::test_fixtures::{open_grid, open_world_grid, test_state, world_state};
use u5_runtime::{CH_FONT_LEN, IBM_CH_FILE, PlayState, parse_ch_font};

use super::*;
use crate::enh::VisualLaunchConfig;
use crate::enh::equivalence::{FRAME, Owner, Scene, Shell, TimedKey};
use crate::enh::overlay::OverlayState;
use crate::menu::pages::{MenuModel, SettingsTab};

// ---------------------------------------------------------------------------
// Helpers.

/// A block font: the two cap triangles, and for every printable code a
/// distinct pattern in rows 0-6 (space blank), so every glyph shows.
fn block_font() -> FixedCellFont {
    let mut bytes = vec![0u8; CH_FONT_LEN];
    let right: [u8; 8] = [0x80, 0xe0, 0xf8, 0xfc, 0xfc, 0xf8, 0xe0, 0x80];
    for (row, bits) in right.iter().enumerate() {
        bytes[2 * 8 + row] = *bits;
        bytes[8 + row] = bits.reverse_bits();
    }
    for code in 0x05..0x80usize {
        if code == 0x20 {
            continue;
        }
        for row in 0..7 {
            bytes[code * 8 + row] = ((code * 37 + row * 11) & 0xfe) as u8 | 0x40;
        }
    }
    parse_ch_font(&bytes, IBM_CH_FILE).unwrap()
}

fn scene() -> Scene {
    let mut scene = Scene::synthetic();
    scene.text_font = block_font();
    scene
}

/// A launch with the catalogue keys in `keys` on: the shipped ones through
/// the resolver as a host's command line would, the menu's (not shipped
/// yet, as in the plugin tests) set on the resolved toggles.
fn launch(keys: &[&str]) -> VisualLaunchConfig {
    let (menu, shipped): (Vec<&str>, Vec<&str>) =
        keys.iter().partition(|key| key.starts_with("menu."));
    let cli: Vec<CliSetting> = shipped
        .iter()
        .map(|key| CliSetting::Enable((*key).into()))
        .collect();
    let mut resolved = resolve(ResolveInput {
        stored: None,
        cli: &cli,
        lock: LockState::Off,
        caps: HostCapabilities::ALL,
    });
    assert!(resolved.notes.is_empty(), "{:?}", resolved.notes);
    for key in menu {
        let toggle = match key {
            "menu.overlay" => Toggle::MenuOverlay,
            "menu.title_layer" => Toggle::MenuTitleLayer,
            other => panic!("{other}"),
        };
        resolved.toggles.set(toggle, true);
    }
    VisualLaunchConfig {
        enhancements: resolved,
        ..VisualLaunchConfig::classic()
    }
}

fn menu_43() -> VisualLaunchConfig {
    launch(&["menu.overlay"])
}

fn menu_grow() -> VisualLaunchConfig {
    launch(&["menu.overlay", "display.grow_left"])
}

const W43: (f32, f32) = (1600.0, 1200.0);
const W169: (f32, f32) = (2560.0, 1440.0);
const W219: (f32, f32) = (3440.0, 1440.0);

/// A synthetic town at its idle command prompt.
fn town() -> PlayState {
    test_state(open_grid(), 5, 5)
}

struct Game {
    shell: Shell,
    next: Vec<TimedKey>,
}

impl Game {
    fn new(state: PlayState, launch: &VisualLaunchConfig, window: Option<(f32, f32)>) -> Self {
        Self::with_resolution(
            state,
            launch,
            window.map(|(w, h)| WindowResolution::new(w, h)),
        )
    }

    fn with_resolution(
        state: PlayState,
        launch: &VisualLaunchConfig,
        window: Option<WindowResolution>,
    ) -> Self {
        let mut shell = Shell::new(state, &scene(), launch, |_| {});
        if let Some(resolution) = window {
            shell.app.world_mut().spawn(Window {
                resolution,
                ..default()
            });
        }
        let mut game = Self {
            shell,
            next: Vec::new(),
        };
        game.frames(10);
        game
    }

    fn event(&mut self, key: KeyCode, state: ButtonState) -> &mut Self {
        self.next.push(TimedKey {
            key,
            state,
            repeat: false,
            owner: Owner::Game,
        });
        self
    }

    fn press(&mut self, key: KeyCode) -> &mut Self {
        self.event(key, ButtonState::Pressed)
    }

    fn release(&mut self, key: KeyCode) -> &mut Self {
        self.event(key, ButtonState::Released)
    }

    fn frame(&mut self) -> &mut Self {
        let events = std::mem::take(&mut self.next);
        self.shell.step(&events);
        self
    }

    fn frames(&mut self, count: usize) -> &mut Self {
        for _ in 0..count {
            self.frame();
        }
        self
    }

    fn tap(&mut self, key: KeyCode) -> &mut Self {
        self.press(key).frame().release(key).frame()
    }

    /// F12, then frames until the open sweep is over.
    fn open_menu(&mut self) -> &mut Self {
        self.tap(KeyCode::F12);
        assert_eq!(self.open(), Some(OverlayId::Menu));
        self.frames(16);
        assert_eq!(self.view().sweep.step, None, "the sweep is over");
        self
    }

    fn resize(&mut self, size: (f32, f32)) -> &mut Self {
        let world = self.shell.app.world_mut();
        let mut windows = world.query::<&mut Window>();
        for mut window in windows.iter_mut(world) {
            window.resolution.set(size.0, size.1);
        }
        self
    }

    fn world(&self) -> &World {
        self.shell.app.world()
    }

    fn open(&self) -> Option<OverlayId> {
        self.world().resource::<OverlayState>().open()
    }

    fn view(&self) -> &MenuView {
        self.world().resource::<MenuView>()
    }

    fn model(&self) -> Option<&MenuModel> {
        self.world().resource::<MenuState>().model()
    }

    fn image(&self, handle: &Handle<Image>) -> (UVec2, Vec<u8>) {
        let image = self
            .world()
            .resource::<Assets<Image>>()
            .get(handle)
            .expect("the image");
        (image.size(), image.data.clone().unwrap())
    }

    /// The menu nodes: image, layer, node and layout rounding.
    fn nodes(&mut self) -> Vec<(Handle<Image>, GlobalZIndex, Node, LayoutConfig)> {
        let world = self.shell.app.world_mut();
        let mut query = world
            .query_filtered::<(&ImageNode, &GlobalZIndex, &Node, &LayoutConfig), With<MenuNode>>();
        query
            .iter(world)
            .map(|(image, z, node, config)| (image.image.clone(), *z, node.clone(), *config))
            .collect()
    }

    fn backdrops(&mut self) -> Vec<(GlobalZIndex, Node, BackgroundColor)> {
        let world = self.shell.app.world_mut();
        let mut query =
            world.query_filtered::<(&GlobalZIndex, &Node, &BackgroundColor), With<MenuBackdrop>>();
        query
            .iter(world)
            .map(|(z, node, colour)| (*z, node.clone(), *colour))
            .collect()
    }

    /// The node's picture, or `None` with no node.
    fn picture(&mut self) -> Option<(UVec2, Vec<u8>)> {
        let nodes = self.nodes();
        assert!(nodes.len() <= 1, "one menu node");
        nodes.first().map(|(handle, ..)| self.image(handle))
    }

    /// The frame sprite: its rectangle and the size of the image it shows.
    fn sprite(&mut self) -> (Rect, UVec2) {
        let world = self.shell.app.world_mut();
        let mut windows = world.query::<&Window>();
        let window = windows.iter(world).next().unwrap().clone();
        let mut sprites = world.query::<(&Sprite, &Transform)>();
        let (sprite, transform) = sprites
            .iter(world)
            .next()
            .map(|(s, t)| (s.clone(), *t))
            .unwrap();
        let rect = frame_rect(&window, &sprite, &transform).unwrap();
        let size = world
            .resource::<Assets<Image>>()
            .get(&sprite.image)
            .unwrap()
            .size();
        (rect, size)
    }

    /// The faithful frame's bytes, and the presented picture's when the
    /// pipeline shows one.
    fn frames_shown(&self) -> (Vec<u8>, Option<Vec<u8>>) {
        let world = self.world();
        let faithful = self.image(&self.shell.visual().image_handle).1;
        let presented = world
            .get_resource::<PresentHost>()
            .and_then(PresentHost::presented)
            .map(|handle| self.image(handle).1);
        (faithful, presented)
    }

    /// What the menu shows now, drawn by the renderer straight from the
    /// model, as the node shows it at the sweep's step.
    fn expected(&self, picture: &MenuPicture, column: ColumnView) -> MenuImage {
        let world = self.world();
        let state = world.resource::<MenuState>().model().unwrap().current();
        let snap = snapshot(world.resource::<MenuShell>(), world.resource::<MenuDoors>());
        let drawn = render_page(&block_font(), picture, column, state, &snap);
        sweep_image_over(&drawn.image, drawn.panel, self.view().sweep.step, CLEAR)
    }
}

fn node_rect(node: &Node) -> Rect {
    let px = |v: Val| match v {
        Val::Px(px) => px,
        other => panic!("not pixels: {other:?}"),
    };
    Rect::from_corners(
        Vec2::new(px(node.left), px(node.top)),
        Vec2::new(
            px(node.left) + px(node.width),
            px(node.top) + px(node.height),
        ),
    )
}

/// The top menu exactly as the kit draws it, from its primitives: the
/// narrow panel with a three-row footer box, the rows from the second row
/// of the list, "The game waits." on the footer's second row and the
/// two tabs.
fn kit_top_menu(
    picture: &MenuPicture,
    column: ColumnView,
    items: &[&str],
    cursor: usize,
) -> MenuImage {
    let font = block_font();
    let rect = panel_rect(PageShape::Narrow, picture, column);
    let panel = Panel::new(rect).divider(rect.bottom() - 4);
    let mut img = MenuImage::for_picture(picture);
    draw_panel(&mut img, &panel);
    let boxes = panel.boxes();
    let (list, footer) = (boxes[0], boxes[1]);
    for (i, item) in items.iter().enumerate() {
        let line = Line::new(list.w).text(1, item, Ink::Plain);
        draw_row(
            &mut img,
            &font,
            list,
            list.row + 1 + i as u32,
            &line,
            i == cursor,
        );
    }
    let waits = Line::new(footer.w).text(1, t::WAITING, Ink::Help);
    draw_row(&mut img, &font, footer, footer.row + 1, &waits, false);
    draw_title_tab(&mut img, &font, rect, t::MENU_TITLE);
    draw_footer_tab(&mut img, &font, rect, t::KEY_CLOSE);
    img
}

/// An empty Settings tab as the kit draws it: the wide panel with a
/// four-row footer box, the tab strip and "Esc: back".
fn kit_settings(picture: &MenuPicture, column: ColumnView, tab: usize) -> MenuImage {
    let font = block_font();
    let rect = panel_rect(PageShape::Wide, picture, column);
    let panel = Panel::new(rect).divider(rect.bottom() - 5);
    let mut img = MenuImage::for_picture(picture);
    draw_panel(&mut img, &panel);
    draw_tab_strip(&mut img, &font, rect, &t::SETTINGS_TABS, tab);
    draw_footer_tab(&mut img, &font, rect, t::KEY_BACK);
    img
}

/// The `--visual` harness shell's short menu.
const SHORT: [&str; 3] = [t::RESUME, t::SETTINGS, t::QUIT_TO_DESKTOP];

fn canvas_picture(window: (f32, f32)) -> MenuPicture {
    let canvas = Canvas::for_window(
        window.0 as u32,
        window.1 as u32,
        u5_runtime::enhancements::PixelAspect::Dos,
    )
    .unwrap();
    menu_picture(Some(&canvas))
}

fn top(cursor: usize) -> PageState {
    PageState {
        page: Page::Top,
        cursor,
    }
}

// ---------------------------------------------------------------------------
// The pure parts.

#[test]
fn the_sweep_opens_in_eight_steps_of_25_ms_and_turns_back() {
    let mut sweep = MenuSweep::default();
    assert!(!sweep.visible());
    sweep.start_open();
    assert_eq!(sweep.step, Some(1));
    let mut steps = vec![sweep.step];
    for _ in 0..7 {
        sweep.tick(MENU_SWEEP_STEP);
        steps.push(sweep.step);
    }
    let want: Vec<Option<u8>> = (1..8).map(Some).chain([None]).collect();
    assert_eq!(steps, want);
    assert!(sweep.open && sweep.visible());

    sweep.start_close();
    assert_eq!(sweep.step, Some(7));
    for k in (1..7).rev() {
        sweep.tick(MENU_SWEEP_STEP);
        assert_eq!(sweep.step, Some(k));
    }
    sweep.tick(MENU_SWEEP_STEP);
    assert!(!sweep.visible(), "gone after step 1");

    // An open interrupted by a close turns back from its step, and the
    // reverse.
    sweep.start_open();
    sweep.tick(MENU_SWEEP_STEP * 3);
    assert_eq!(sweep.step, Some(4));
    sweep.start_close();
    assert_eq!(sweep.step, Some(4));
    sweep.tick(MENU_SWEEP_STEP);
    sweep.start_open();
    assert_eq!(sweep.step, Some(3));

    // Frames shorter than a step add up: at 60 Hz the 175 ms from step 1
    // to the whole menu take 11 frames.
    let mut sweep = MenuSweep::default();
    sweep.start_open();
    let mut frames = 0;
    while sweep.step.is_some() {
        sweep.tick(FRAME);
        frames += 1;
    }
    assert_eq!(frames, 11);
    sweep.stop();
    assert_eq!(sweep, MenuSweep::default());
}

#[test]
fn a_list_scrolls_only_past_its_last_row() {
    assert_eq!(first_shown(0, 3, 10), 0);
    assert_eq!(first_shown(2, 3, 10), 0);
    assert_eq!(first_shown(9, 20, 10), 0);
    assert_eq!(first_shown(10, 20, 10), 1);
    assert_eq!(first_shown(19, 20, 10), 10);
    assert_eq!(first_shown(40, 20, 10), 10);
    assert_eq!(first_shown(3, 20, 0), 0);
}

/// The shell's picture over the backdrop is the kit's dimmed picture over
/// the frame, pixel for pixel: a clear pixel shows the 0.6 dim, an opaque
/// one covers it.
#[test]
fn the_backdrop_under_the_picture_is_the_kit_s_dim() {
    let font = block_font();
    let snap = ShellSnapshot::default();
    let frame: Vec<u8> = (0..320 * 200)
        .flat_map(|i: u32| [(i % 251) as u8, (i % 113) as u8, (i % 7 * 30) as u8, 0xff])
        .collect();
    let drawn = render_page(
        &font,
        &MenuPicture::FRAME,
        ColumnView::default(),
        top(0),
        &snap,
    );
    let mut dimmed = drawn.image.clone();
    dim_backdrop(&mut dimmed);
    let mut backdrop = MenuImage::for_picture(&MenuPicture::FRAME);
    dim_backdrop(&mut backdrop);
    let shell = composite_over(&composite_over(&frame, &backdrop), &drawn.image);
    assert_eq!(shell, composite_over(&frame, &dimmed));
    assert_eq!(f32::from(DIM_ALPHA) / 255.0, 0.6);
}

/// The GPU blends the backdrop in linear light: its opacity puts every EGA
/// level within 4 of the sheet's sRGB 0.6 dim, where 0.6 itself would
/// leave white at 170.
#[test]
fn the_gpu_backdrop_matches_the_sheet_s_dim() {
    fn linear(c: u8) -> f32 {
        Color::srgb_u8(c, c, c).to_linear().red
    }
    fn srgb(l: f32) -> i32 {
        i32::from(
            Color::LinearRgba(LinearRgba::rgb(l, l, l))
                .to_srgba()
                .to_u8_array()[0],
        )
    }
    let gpu = |c: u8, alpha: f32| srgb(linear(c) * (1.0 - alpha));
    assert_eq!(backdrop_colour().alpha(), BACKDROP_ALPHA);
    for c in [0u8, 85, 170, 255] {
        let sheet = {
            let frame = vec![c, c, c, 0xff];
            let mut dim = MenuImage::new(1, 1);
            dim.set(0, 0, DIM);
            i32::from(composite_over(&frame, &dim)[0])
        };
        let got = gpu(c, BACKDROP_ALPHA);
        assert!((got - sheet).abs() <= 4, "level {c}: {got} against {sheet}");
    }
    assert_eq!(gpu(255, 0.6), 170, "0.6 in linear light is too light");
}

/// Every pixel the shell's picture holds is clear or an opaque EGA colour,
/// and the opaque ones lie inside the panel.
#[test]
fn the_picture_is_the_panel_and_nothing_else() {
    let font = block_font();
    let snap = ShellSnapshot {
        intro_shell: true,
        slots: true,
        ..ShellSnapshot::default()
    };
    for picture in [
        MenuPicture::FRAME,
        canvas_picture(W169),
        canvas_picture(W219),
    ] {
        for page in [Page::Top, Page::Settings(SettingsTab::Game)] {
            let drawn = render_page(
                &font,
                &picture,
                ColumnView::default(),
                PageState { page, cursor: 1 },
                &snap,
            );
            let (x0, y0, x1, y1) = drawn.panel.pixels();
            for y in 0..picture.h {
                for x in 0..picture.w {
                    let pixel = drawn.image.at(x, y);
                    if pixel == CLEAR {
                        continue;
                    }
                    assert!(ega_index(pixel).is_some(), "EGA at ({x}, {y})");
                    assert!(
                        (x0..=x1).contains(&x) && (y0..=y1).contains(&y),
                        "inside the panel at ({x}, {y})"
                    );
                }
            }
        }
    }
}

/// A greyed row is grey in the bar too, and its reason shows in the footer
/// in place of "The game waits." while it is selected.
#[test]
fn a_greyed_row_s_reason_shows_in_the_footer() {
    let font = block_font();
    let snap = ShellSnapshot {
        intro_shell: true,
        slots: true,
        ..ShellSnapshot::default()
    };
    let picture = MenuPicture::FRAME;
    let column = ColumnView::default();
    let save = render_page(&font, &picture, column, top(1), &snap);
    let rect = save.panel;
    let footer = CellRect::new(rect.col + 1, rect.bottom() - 3, rect.w - 2, 3);
    let list = CellRect::new(rect.col + 1, rect.row + 1, rect.w - 2, rect.h - 6);

    let mut want = MenuImage::for_picture(&picture);
    draw_panel(&mut want, &Panel::new(rect).divider(rect.bottom() - 4));
    let rows = page_rows(Page::Top, &snap);
    for (i, row) in rows.iter().enumerate() {
        let line = top_line(row, list.w);
        draw_row(
            &mut want,
            &font,
            list,
            list.row + 1 + i as u32,
            &line,
            i == 1,
        );
    }
    let lines = t::wrap(t::SAVE_UNAVAILABLE, (footer.w - 2) as usize);
    assert_eq!(lines.len(), 2);
    for (i, line) in lines.iter().enumerate() {
        let l = Line::new(footer.w).text(1, line, Ink::Help);
        draw_row(&mut want, &font, footer, footer.row + i as u32, &l, false);
    }
    draw_title_tab(&mut want, &font, rect, t::MENU_TITLE);
    draw_footer_tab(&mut want, &font, rect, t::KEY_CLOSE);
    assert_eq!(save.image, want);
    assert!(!rows[1].enabled);
    assert!(
        top_line(&rows[1], list.w)
            .cells
            .iter()
            .all(|(_, ink)| *ink == Ink::Grey)
    );
}

// ---------------------------------------------------------------------------
// The shell.

/// The top menu and a Settings tab drawn in the shell are the kit's render
/// of the same rows, byte for byte, at 4:3 and on two canvases.
#[test]
fn the_shell_draws_exactly_the_kit_s_pages() {
    for (launch, window, picture) in [
        (menu_43(), W43, MenuPicture::FRAME),
        (menu_grow(), W169, canvas_picture(W169)),
        (menu_grow(), W219, canvas_picture(W219)),
    ] {
        let mut game = Game::new(town(), &launch, Some(window));
        game.open_menu();
        let column = game
            .world()
            .get_resource::<ColumnState>()
            .map(ColumnState::view)
            .unwrap_or_default();
        let (size, bytes) = game.picture().expect("the node");
        assert_eq!(size, UVec2::new(picture.w, picture.h));
        assert_eq!(
            bytes,
            kit_top_menu(&picture, column, &SHORT, 0).rgba,
            "the top menu at {window:?}"
        );

        game.tap(KeyCode::ArrowDown);
        let (_, bytes) = game.picture().unwrap();
        assert_eq!(
            bytes,
            kit_top_menu(&picture, column, &SHORT, 1).rgba,
            "the bar on Settings"
        );

        game.tap(KeyCode::Enter).tap(KeyCode::ArrowRight);
        assert_eq!(
            game.model().unwrap().current().page,
            Page::Settings(SettingsTab::Display)
        );
        let (_, bytes) = game.picture().unwrap();
        assert_eq!(
            bytes,
            kit_settings(&picture, column, 1).rgba,
            "Settings, Display tab, at {window:?}"
        );
        assert_eq!(game.view().sweep.step, None, "page changes are instant");
    }
}

/// The faithful frame, and on grow-left the presented canvas, are the same
/// bytes every frame with the menu opened as without it.
#[test]
fn the_game_picture_is_untouched_by_the_menu() {
    for (launch, window) in [(menu_43(), W43), (menu_grow(), W169), (menu_grow(), W219)] {
        let mut with = Game::new(town(), &launch, Some(window));
        let mut without = Game::new(town(), &launch, Some(window));
        with.press(KeyCode::F12);
        for frame in 0..40 {
            match frame {
                1 => {
                    with.release(KeyCode::F12);
                }
                6 => {
                    with.press(KeyCode::ArrowDown);
                }
                7 => {
                    with.release(KeyCode::ArrowDown);
                }
                _ => {}
            }
            with.frame();
            without.frame();
            assert_eq!(with.open(), Some(OverlayId::Menu));
            let (a, b) = (with.frames_shown(), without.frames_shown());
            assert_eq!(a.0, b.0, "the faithful frame, frame {frame} at {window:?}");
            assert_eq!(u5_runtime::hash_bytes(&a.0), u5_runtime::hash_bytes(&b.0));
            assert_eq!(a.1, b.1, "the presented picture, frame {frame}");
            if window != W43 {
                assert!(a.1.is_some(), "grow-left presents a canvas");
            }
        }
        assert_eq!(with.nodes().len(), 1);
        assert!(without.nodes().is_empty());
    }
}

/// The node is the frame sprite's rectangle exactly, with layout rounding
/// off, and its picture is the size of the sprite's: one menu pixel is one
/// game pixel, at five windows and a 150% display scale.
#[test]
fn the_node_lies_exactly_over_the_frame_sprite() {
    let cases: Vec<(VisualLaunchConfig, WindowResolution)> = vec![
        (menu_43(), WindowResolution::new(1600.0, 1200.0)),
        (menu_43(), WindowResolution::new(1366.0, 768.0)),
        (menu_grow(), WindowResolution::new(1920.0, 1080.0)),
        (menu_grow(), WindowResolution::new(1366.0, 768.0)),
        (menu_grow(), WindowResolution::new(3440.0, 1440.0)),
        (menu_grow(), WindowResolution::new(1280.0, 1024.0)),
        (
            menu_grow(),
            WindowResolution::new(2560.0, 1440.0).with_scale_factor_override(1.5),
        ),
    ];
    for (launch, resolution) in cases {
        let label = format!(
            "{}x{} at {}",
            resolution.physical_width(),
            resolution.physical_height(),
            resolution.scale_factor()
        );
        let mut game = Game::with_resolution(town(), &launch, Some(resolution));
        game.open_menu();
        let (sprite, shown) = game.sprite();
        let nodes = game.nodes();
        assert_eq!(nodes.len(), 1, "{label}");
        let (handle, z, node, config) = &nodes[0];
        assert_eq!(node.position_type, PositionType::Absolute);
        assert_eq!(node_rect(node), sprite, "{label}: the sprite's rect");
        assert!(
            !config.use_rounding,
            "{label}: never rounded off the sprite"
        );
        let (size, _) = game.image(handle);
        assert_eq!(size, shown, "{label}: the sprite's picture's size");
        assert_eq!(Some(size), game.view().picture_size());
        // Each menu pixel covers what one game pixel covers.
        let per_pixel = node_rect(node).size() / size.as_vec2();
        assert_eq!(per_pixel, sprite.size() / shown.as_vec2(), "{label}");
        assert_eq!(*z, GlobalZIndex(MENU_NODE_Z));

        let backdrops = game.backdrops();
        assert_eq!(backdrops.len(), 1, "{label}");
        let (bz, bnode, colour) = &backdrops[0];
        assert_eq!(*bz, GlobalZIndex(MENU_BACKDROP_Z));
        assert_eq!((bnode.left, bnode.top), (Val::Px(0.0), Val::Px(0.0)));
        assert_eq!(
            (bnode.width, bnode.height),
            (Val::Percent(100.0), Val::Percent(100.0))
        );
        assert_eq!(colour.0, backdrop_colour());
    }
    // Above the side fill and the side panels, under the toasts: the gem
    // map's layers.
    assert_eq!(MENU_NODE_Z, i32::MAX - 2);
    assert_eq!(MENU_BACKDROP_Z, MENU_NODE_Z - 1);
    const { assert!(MENU_BACKDROP_Z > -1) };
    assert_eq!((MENU_NODE_Z, MENU_BACKDROP_Z), (GEM_NODE_Z, GEM_BEFORE_Z));
}

/// Resizing while the menu is open re-lays it out: a new canvas redraws
/// the page once, and the node follows the sprite.
#[test]
fn resizing_while_open_re_lays_out() {
    let mut game = Game::new(town(), &menu_grow(), Some(W169));
    game.open_menu();
    let renders = game.view().renders;
    let composes = game.view().composes;
    game.frames(30);
    assert_eq!(game.view().renders, renders, "idle frames redraw nothing");
    assert_eq!(game.view().composes, composes, "nor recompose");

    game.resize(W219).frames(2);
    let picture = canvas_picture(W219);
    assert_eq!(game.view().renders, renders + 1, "one redraw");
    assert_eq!(game.view().composes, composes + 1);
    let (size, bytes) = game.picture().unwrap();
    assert_eq!(size, UVec2::new(picture.w, picture.h));
    assert_eq!(
        bytes,
        kit_top_menu(&picture, ColumnView::default(), &SHORT, 0).rgba
    );
    let (sprite, _) = game.sprite();
    assert_eq!(node_rect(&game.nodes()[0].2), sprite);

    // A 4:3 window: the 384x240 canvas.
    game.resize((1280.0, 960.0)).frames(2);
    let (size, _) = game.picture().unwrap();
    let (sprite, shown) = game.sprite();
    assert_eq!(size, shown);
    assert_eq!(node_rect(&game.nodes()[0].2), sprite);
}

/// The open sweep: from step 1 on the frame F12 opens the menu, a step
/// every 25 ms; a key in the middle acts on the menu that frame and the
/// sweep goes on from where it was.
#[test]
fn a_key_during_the_open_sweep_acts_that_frame() {
    let mut game = Game::new(town(), &menu_43(), Some(W43));
    let picture = MenuPicture::FRAME;
    let column = ColumnView::default();
    game.press(KeyCode::F12).frame();
    assert_eq!(game.view().sweep.step, Some(1));
    let (_, bytes) = game.picture().unwrap();
    assert_eq!(bytes, game.expected(&picture, column).rgba);
    game.release(KeyCode::F12).frame();
    let step = game.view().sweep.step.unwrap();
    assert!((1..4).contains(&step), "{step}");

    game.press(KeyCode::ArrowDown).frame();
    let during = game.view().sweep.step.expect("still sweeping");
    assert!(during >= step);
    assert_eq!(game.model().unwrap().current(), top(1), "acted at once");
    let (_, bytes) = game.picture().unwrap();
    let full = kit_top_menu(&picture, column, &SHORT, 1);
    let rect = panel_rect(PageShape::Narrow, &picture, column);
    assert_eq!(
        bytes,
        sweep_image_over(&full, rect, Some(during), CLEAR).rgba,
        "the moved bar, at step {during}"
    );
    game.release(KeyCode::ArrowDown);
    let mut seen = vec![during];
    while let Some(step) = game.frame().view().sweep.step {
        seen.push(step);
    }
    assert!(seen.windows(2).all(|w| w[0] <= w[1]), "{seen:?}");
    let (_, bytes) = game.picture().unwrap();
    assert_eq!(bytes, full.rgba);
}

/// Escape closes with the sweep falling from step 7; the node and the
/// backdrop go when it is over. A close because the game left the prompt
/// skips it.
#[test]
fn the_close_sweeps_out_unless_the_game_left_the_prompt() {
    let mut game = Game::new(town(), &menu_43(), Some(W43));
    game.open_menu();
    let picture = MenuPicture::FRAME;
    let full = MenuImage {
        w: picture.w,
        h: picture.h,
        rgba: game.picture().unwrap().1,
    };
    game.press(KeyCode::Escape).frame();
    assert_eq!(game.open(), None);
    assert_eq!(game.view().sweep.step, Some(7));
    assert!(game.model().is_none());
    let panel = panel_rect(PageShape::Narrow, &picture, ColumnView::default());
    let (_, bytes) = game.picture().unwrap();
    assert_eq!(bytes, sweep_image_over(&full, panel, Some(7), CLEAR).rgba);
    assert_eq!(game.backdrops().len(), 1);
    game.release(KeyCode::Escape);
    let mut frames = 1;
    while game.view().sweep.visible() {
        game.frame();
        frames += 1;
        assert!(frames < 20);
    }
    assert!((10..=12).contains(&frames), "about 175 ms: {frames}");
    assert!(game.nodes().is_empty());
    assert!(game.backdrops().is_empty());
    assert!(game.view().image().is_none());

    // The game leaves the prompt: the overlay model closes the menu with
    // `NotIdle`, and it goes that frame.
    game.open_menu();
    assert_eq!(game.nodes().len(), 1);
    game.shell
        .app
        .world_mut()
        .send_event(OverlayChanged::Closed {
            id: OverlayId::Menu,
            why: CloseReason::NotIdle,
        });
    game.frame();
    assert!(!game.view().sweep.visible());
    assert!(game.nodes().is_empty());
    assert!(game.backdrops().is_empty());
}

/// Without `menu.overlay` at launch F12 shows nothing: no node and no
/// backdrop, and with neither menu toggle no menu resource at all.
#[test]
fn with_the_overlay_off_no_node_is_ever_made() {
    for launch in [launch(&["menu.title_layer"]), VisualLaunchConfig::classic()] {
        let mut game = Game::new(town(), &launch, Some(W43));
        game.tap(KeyCode::F12).frames(20);
        assert!(game.nodes().is_empty());
        assert!(game.backdrops().is_empty());
        assert_eq!(
            game.world().contains_resource::<MenuView>(),
            launch.enhancements.toggles.on(Toggle::MenuTitleLayer)
        );
    }
}

/// A gem view is never the idle prompt, so the menu cannot open over the
/// gem map, with which it shares its layers.
#[test]
fn the_menu_never_opens_over_the_gem_map() {
    let mut state = world_state(open_world_grid(), 100, 100);
    state.gems = 3;
    let launch = launch(&["menu.overlay", "gem_map.enhanced", "display.grow_left"]);
    let mut game = Game::new(state, &launch, Some(W169));
    game.tap(KeyCode::KeyV).frames(30);
    assert!(u5_automap::gem::gem_view_open(&game.shell.visual().state));
    game.tap(KeyCode::F12).frames(5);
    assert_eq!(game.open(), None, "refused over the gem view");
    assert!(game.nodes().is_empty());
    assert!(!game.view().sweep.visible());
}
