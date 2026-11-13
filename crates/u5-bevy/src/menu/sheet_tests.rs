//! The M4-01 review sheet: every screen of chapter 3.2 drawn through the
//! frame kit over real engine frames, for the user's approval of the look
//! before the pages are built (build plan M4-01).
//!
//! Review output only, never committed, and `#[ignore]`d: run with
//! `U5_MOCKUP_PROFILE` naming a `test-assets` or `prepare-profile` game copy
//! and `U5_MENU_SHEET` an output folder:
//!
//! ```text
//! U5_MOCKUP_PROFILE=<copy> U5_MENU_SHEET=~/artifacts/u5/mockups/menus/m4-01 \
//!   cargo test -p u5-bevy --lib menu::sheet_tests -- --ignored
//! ```
//!
//! The game copy is copied to a scratch folder first; nothing is written to
//! it. Two scenes: a town of the c04f mockups (`TOWNE:1` at 15,19), and the
//! forest road west of Britain (74,111), both at 11:50 (the c04f mockups' spot). Each is drawn by the shell's
//! own frame and canvas renderer (`render_shell_frame_and_canvas_with_column`)
//! at 320x200 and on the 512x240, 688x240 and 480x216 canvases with the
//! column shut and open; the title layer is drawn over the real title menu.
//!
//! Written: `<scene>-<picture>-<screen>.png` (each screen at 2x), one contact
//! sheet per scene and picture, 4x close-ups of the corners, junctions,
//! tabs and bars (`closeup-*.png`), the open and close sweep frames
//! (`sweep-*-NN.png`, assembled into GIFs outside the test), `geometry.tsv`
//! and `wording.tsv`.

use std::fs;
use std::path::Path;

use u5_runtime::enhancements::PixelAspect;
use u5_runtime::test_fixtures::{configured_test_dir, copy_asset_writable, unique_scratch_dir};
use u5_runtime::{
    GameClock, PeekLightPolicy, PlayState, PlayTarget, TileGraphicsDepth,
    load_play_options_from_save, load_runes_ch_font, load_tile_atlas,
};

use super::frame::*;
use super::geometry::*;
use super::sheet_screens::{Inputs, Screen, all_screens};
use super::text as t;
use crate::display::crawl::sync::CrawlState;
use crate::display::grow::column::ColumnView;
use crate::display::grow::{Canvas, render_shell_frame_and_canvas_with_column};

fn save_png(path: &Path, w: u32, h: u32, rgba: &[u8]) {
    image::RgbaImage::from_raw(w, h, rgba.to_vec())
        .unwrap()
        .save(path)
        .unwrap();
}

/// `rgba` scaled by `k`, nearest neighbour.
fn scaled(w: u32, h: u32, rgba: &[u8], k: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity((w * h * k * k * 4) as usize);
    for y in 0..h * k {
        for x in 0..w * k {
            let i = (((y / k) * w + x / k) * 4) as usize;
            out.extend_from_slice(&rgba[i..i + 4]);
        }
    }
    out
}

/// A crop of `rgba` (`w` wide), clipped to the picture.
fn crop(
    w: u32,
    h: u32,
    rgba: &[u8],
    (x0, y0, cw, ch): (u32, u32, u32, u32),
) -> (u32, u32, Vec<u8>) {
    let (x0, y0) = (x0.min(w - 1), y0.min(h - 1));
    let (cw, ch) = (cw.min(w - x0), ch.min(h - y0));
    let mut out = Vec::with_capacity((cw * ch * 4) as usize);
    for y in y0..y0 + ch {
        let i = ((y * w + x0) * 4) as usize;
        out.extend_from_slice(&rgba[i..i + (cw * 4) as usize]);
    }
    (cw, ch, out)
}

struct Scene {
    name: &'static str,
    state: PlayState,
}

fn scenes(game: &Path) -> Vec<Scene> {
    let mut options = load_play_options_from_save(game).unwrap();
    options.target = PlayTarget::from_key("TOWNE:1").unwrap();
    options.saved_active_objects = None;
    options.start = Some((15, 19));
    options.clock = GameClock::new(11, 50).unwrap();
    let start = PlayState::load_scene(game, options).unwrap();
    let mut options = load_play_options_from_save(game).unwrap();
    options.target = PlayTarget::from_key("BRITANNIA").unwrap();
    options.saved_active_objects = None;
    options.start = Some((74, 111));
    options.clock = GameClock::new(11, 50).unwrap();
    let mut road = PlayState::load_scene(game, options).unwrap();
    for line in ["North", "North", "East", "East", "North"] {
        road.emit_command_echo_line(line);
    }
    road.flush_text_hold();
    vec![
        Scene {
            name: "town",
            state: start,
        },
        Scene {
            name: "road",
            state: road,
        },
    ]
}

/// The real title menu, as the intro draws it with Journey Onward lit.
fn title_frame(game: &Path, font: &u5_runtime::FixedCellFont) -> Vec<u8> {
    let depth = TileGraphicsDepth::Ega16;
    let ticks = crate::load_ultima_title_tick_frames(game, depth).unwrap();
    let mut buffer = crate::new_intro_display_buffer();
    crate::draw_visual_intro_start_menu_to_buffer(
        &mut buffer,
        game,
        depth,
        0,
        &ticks,
        Some(font),
        Some(crate::IntroSubflow::JourneyOnward),
        crate::intro_menu_select_caption_cursor_glyph(3),
    );
    buffer.to_rgba()
}

#[derive(Clone, Copy)]
struct PictureCase {
    label: &'static str,
    window: Option<(u32, u32)>,
    open: bool,
}

const CASES: [PictureCase; 7] = [
    PictureCase {
        label: "320x200",
        window: None,
        open: false,
    },
    PictureCase {
        label: "512x240",
        window: Some((2560, 1440)),
        open: false,
    },
    PictureCase {
        label: "512x240-open",
        window: Some((2560, 1440)),
        open: true,
    },
    PictureCase {
        label: "688x240",
        window: Some((3440, 1440)),
        open: false,
    },
    PictureCase {
        label: "688x240-open",
        window: Some((3440, 1440)),
        open: true,
    },
    PictureCase {
        label: "480x216",
        window: Some((1920, 1080)),
        open: false,
    },
    PictureCase {
        label: "480x216-open",
        window: Some((1920, 1080)),
        open: true,
    },
];

/// Close-ups at 4x: `(case, scene, screen, name, x, y, w, h)` in picture
/// pixels.
const CLOSEUPS: &[(&str, &str, &str, &str, (u32, u32, u32, u32))] = &[
    ("320x200", "town", "top", "corner-top-left", (0, 0, 32, 24)),
    (
        "320x200",
        "town",
        "top",
        "corner-top-right",
        (168, 0, 32, 24),
    ),
    (
        "320x200",
        "town",
        "top",
        "corner-bottom-left",
        (0, 168, 32, 32),
    ),
    (
        "320x200",
        "town",
        "top",
        "corner-bottom-right",
        (168, 168, 32, 32),
    ),
    ("320x200", "town", "top", "title-tab", (56, 0, 80, 24)),
    ("320x200", "town", "top", "footer-tab", (48, 176, 96, 24)),
    (
        "320x200",
        "town",
        "top",
        "divider-junction-left",
        (0, 160, 40, 32),
    ),
    (
        "320x200",
        "town",
        "top",
        "divider-junction-right",
        (160, 160, 40, 32),
    ),
    (
        "320x200",
        "town",
        "top",
        "selection-bar-left",
        (0, 8, 64, 24),
    ),
    (
        "320x200",
        "town",
        "top",
        "selection-bar-right",
        (136, 8, 64, 24),
    ),
    (
        "320x200",
        "town",
        "top-save-greyed",
        "greyed-row-in-bar",
        (0, 16, 120, 24),
    ),
    (
        "320x200",
        "town",
        "load",
        "split-top-junction",
        (144, 0, 40, 24),
    ),
    (
        "320x200",
        "town",
        "load",
        "split-bottom-junction",
        (144, 144, 40, 32),
    ),
    (
        "320x200",
        "town",
        "load",
        "thumbnail-box",
        (184, 0, 120, 80),
    ),
    (
        "320x200",
        "town",
        "load",
        "list-and-cheats-mark",
        (0, 8, 168, 48),
    ),
    (
        "320x200",
        "town",
        "load-imported",
        "imported-box",
        (184, 0, 120, 80),
    ),
    (
        "320x200",
        "town",
        "settings-enhancements",
        "tab-strip",
        (0, 0, 320, 16),
    ),
    (
        "320x200",
        "town",
        "settings-enhancements",
        "scroll-marks-and-spoiler",
        (160, 16, 160, 56),
    ),
    (
        "320x200",
        "town",
        "settings-enhancements",
        "cheats-group",
        (0, 120, 240, 40),
    ),
    (
        "320x200",
        "town",
        "settings-audio",
        "slider-and-checks",
        (0, 8, 320, 40),
    ),
    (
        "320x200",
        "town",
        "save-name",
        "input-cap-and-cursor",
        (32, 72, 160, 48),
    ),
    (
        "320x200",
        "town",
        "save-overwrite",
        "dialog-corner-and-buttons",
        (24, 64, 272, 72),
    ),
    (
        "320x200",
        "town",
        "title-slots",
        "title-layer",
        (0, 112, 320, 88),
    ),
    (
        "512x240-open",
        "road",
        "top",
        "canvas-top-ring",
        (216, 0, 120, 32),
    ),
    (
        "512x240-open",
        "road",
        "top",
        "canvas-bottom-ring",
        (216, 208, 120, 32),
    ),
    (
        "512x240-open",
        "road",
        "load",
        "canvas-wide-left-edge",
        (136, 0, 56, 240),
    ),
];

#[test]
#[ignore = "review sheet: needs U5_MOCKUP_PROFILE and U5_MENU_SHEET"]
fn menu_review_sheet() {
    let (Some(profile), Some(out)) = (
        configured_test_dir("U5_MOCKUP_PROFILE"),
        configured_test_dir("U5_MENU_SHEET"),
    ) else {
        eprintln!("skipped: set U5_MOCKUP_PROFILE and U5_MENU_SHEET");
        return;
    };
    fs::create_dir_all(&out).unwrap();
    let game = unique_scratch_dir("menu-sheet");
    for entry in fs::read_dir(&profile).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            copy_asset_writable(&entry.path(), &game.join(entry.file_name())).unwrap();
        }
    }
    let atlas = load_tile_atlas(&game, TileGraphicsDepth::Ega16).unwrap();
    let ibm = u5_runtime::load_ibm_ch_font(&game).unwrap();
    let runes = load_runes_ch_font(&game).unwrap();
    let title = title_frame(&game, &ibm);
    save_png(&out.join("title-original.png"), 320, 200, &title);

    let mut scenes = scenes(&game);
    let mut thumb = Vec::new();
    let mut avatar = String::new();
    let mut rendered: Vec<(
        String,
        String,
        String,
        u32,
        u32,
        Vec<u8>,
        Option<CellRect>,
        MenuImage,
    )> = Vec::new();
    // The road's 320x200 frame is the sample slot's thumbnail.
    for pass in 0..2 {
        for scene in &mut scenes {
            if pass == 0 && scene.name != "road" {
                continue;
            }
            for case in CASES {
                if pass == 0 && case.window.is_some() {
                    continue;
                }
                let canvas = case
                    .window
                    .map(|(w, h)| Canvas::for_window(w, h, PixelAspect::Dos).unwrap());
                let column = ColumnView {
                    open: case.open,
                    ..ColumnView::default()
                };
                let mut crawl = CrawlState::default();
                let (frame, composed) = render_shell_frame_and_canvas_with_column(
                    &mut scene.state,
                    &atlas,
                    (&ibm, &runes),
                    &game,
                    canvas
                        .as_ref()
                        .unwrap_or(&Canvas::for_window(2560, 1440, PixelAspect::Dos).unwrap()),
                    PeekLightPolicy::Scaled,
                    &mut crawl,
                    true,
                    "",
                    column,
                    false,
                );
                if pass == 0 {
                    thumb = thumbnail_from_frame(&frame);
                    let bytes = scene.state.party_names[0];
                    avatar = String::from_utf8_lossy(&bytes)
                        .trim_end_matches('\0')
                        .to_string();
                    continue;
                }
                let picture = menu_picture(canvas.as_ref());
                let base = match canvas {
                    Some(_) => composed.expect("the top-down view has a canvas"),
                    None => frame,
                };
                let inputs = Inputs {
                    font: &ibm,
                    picture,
                    column,
                    thumb: &thumb,
                    avatar: &avatar,
                };
                for screen in all_screens(&inputs) {
                    let Screen {
                        name, image, panel, ..
                    } = screen;
                    let is_title = name.starts_with("title");
                    if is_title && canvas.is_some() {
                        continue;
                    }
                    let under = if is_title { &title } else { &base };
                    let shown = composite_over(under, &image);
                    rendered.push((
                        scene.name.to_string(),
                        case.label.to_string(),
                        name.to_string(),
                        picture.w,
                        picture.h,
                        shown,
                        panel.map(|p| p.rect),
                        image,
                    ));
                }
                // The picture without the menu, for comparison.
                rendered.push((
                    scene.name.to_string(),
                    case.label.to_string(),
                    "game".to_string(),
                    picture.w,
                    picture.h,
                    base.clone(),
                    None,
                    MenuImage::for_picture(&picture),
                ));
            }
        }
    }

    // Each screen at 2x, and a contact sheet per scene and picture.
    let mut sheets: Vec<(String, u32, u32, Vec<Vec<u8>>)> = Vec::new();
    for (scene, case, name, w, h, shown, _, _) in &rendered {
        let file = format!("{scene}-{case}-{name}.png");
        save_png(&out.join(&file), w * 2, h * 2, &scaled(*w, *h, shown, 2));
        let key = format!("{scene}-{case}");
        match sheets.iter_mut().find(|s| s.0 == key) {
            Some(sheet) => sheet.3.push(shown.clone()),
            None => sheets.push((key, *w, *h, vec![shown.clone()])),
        }
    }
    for (key, w, h, shots) in &sheets {
        let cols = 4u32;
        let gap = 8u32;
        let rows = (shots.len() as u32).div_ceil(cols);
        let (sw, sh) = (cols * (w + gap) + gap, rows * (h + gap) + gap);
        let mut sheet = vec![0x30u8; (sw * sh * 4) as usize];
        for (i, shot) in shots.iter().enumerate() {
            let (cx, cy) = (
                gap + (i as u32 % cols) * (w + gap),
                gap + (i as u32 / cols) * (h + gap),
            );
            for y in 0..*h {
                let from = (y * w * 4) as usize;
                let at = (((cy + y) * sw + cx) * 4) as usize;
                sheet[at..at + (w * 4) as usize]
                    .copy_from_slice(&shot[from..from + (w * 4) as usize]);
            }
        }
        save_png(&out.join(format!("sheet-{key}.png")), sw, sh, &sheet);
    }

    // Close-ups at 4x.
    for (case, scene, screen, name, rect) in CLOSEUPS {
        let found = rendered
            .iter()
            .find(|r| r.0 == *scene && r.1 == *case && r.2 == *screen)
            .unwrap_or_else(|| panic!("no {scene} {case} {screen}"));
        let (cw, ch, px) = crop(found.3, found.4, &found.5, *rect);
        save_png(
            &out.join(format!("closeup-{name}.png")),
            cw * 4,
            ch * 4,
            &scaled(cw, ch, &px, 4),
        );
    }

    // The sweep: open (steps 1..7 then the whole menu) and close (7..1, then
    // the backdrop gone), frames 00..08.
    for (case, scene, screen) in [
        ("320x200", "town", "top"),
        ("512x240-open", "road", "load"),
        ("688x240", "road", "settings-display"),
    ] {
        let found = rendered
            .iter()
            .find(|r| r.0 == scene && r.1 == case && r.2 == screen)
            .unwrap();
        let base = rendered
            .iter()
            .find(|r| r.0 == scene && r.1 == case && r.2 == "game")
            .unwrap();
        let (w, h, rect, full) = (found.3, found.4, found.6.unwrap(), &found.7);
        let mut open_frames = vec![base.5.clone()];
        for k in 1..8u8 {
            open_frames.push(composite_over(&base.5, &sweep_image(full, rect, Some(k))));
        }
        open_frames.push(composite_over(&base.5, full));
        let mut close_frames = open_frames.clone();
        close_frames.reverse();
        for (dir, frames) in [("open", &open_frames), ("close", &close_frames)] {
            for (i, f) in frames.iter().enumerate() {
                save_png(
                    &out.join(format!("sweep-{case}-{screen}-{dir}-{i:02}.png")),
                    w * 2,
                    h * 2,
                    &scaled(w, h, f, 2),
                );
            }
        }
    }

    // The geometry table.
    let mut geometry = String::from(
        "picture\tcolumn\tpage\tcol\trow\tw\th\tx0\ty0\tx1\ty1\trule_left\trule_top\trule_right\trule_bottom\n",
    );
    let mut pictures = vec![("320x200".to_string(), MenuPicture::FRAME)];
    for (w, h) in [
        (2560, 1440),
        (3440, 1440),
        (1920, 1080),
        (3440, 2160),
        (1600, 1200),
        (1280, 720),
    ] {
        if let Some(c) = Canvas::for_window(w, h, PixelAspect::Dos) {
            pictures.push((
                format!("{}x{} ({w}x{h} window)", c.w, c.h),
                menu_picture(Some(&c)),
            ));
        }
    }
    for (label, picture) in &pictures {
        for open in [false, true] {
            if open && picture.canvas.is_none() {
                continue;
            }
            let column = ColumnView {
                open,
                ..ColumnView::default()
            };
            for (page, shape) in [
                ("narrow (top, Save)", PageShape::Narrow),
                ("wide (Load, Settings)", PageShape::Wide),
                ("title layer", PageShape::Title),
                ("dialog (5 lines)", PageShape::Dialog { lines: 5 }),
            ] {
                if shape == PageShape::Title && picture.canvas.is_some() {
                    continue;
                }
                let r = panel_rect(shape, picture, column);
                let (x0, y0, x1, y1) = r.pixels();
                geometry.push_str(&format!(
                    "{label}\t{}\t{page}\t{}\t{}\t{}\t{}\t{x0}\t{y0}\t{x1}\t{y1}\t{}\t{}\t{}\t{}\n",
                    if open { "open" } else { "shut" },
                    r.col,
                    r.row,
                    r.w,
                    r.h,
                    x0 + 7,
                    y0 + 7,
                    x1 - 7,
                    y1 - 7,
                ));
            }
        }
    }
    fs::write(out.join("geometry.tsv"), geometry).unwrap();

    // The wording table.
    let wording: &[(&str, &str)] = &[
        ("Top menu title", t::MENU_TITLE),
        ("Resume", t::RESUME),
        ("Save", t::SAVE),
        ("Load", t::LOAD),
        ("Journal door", t::JOURNAL),
        ("Map door", t::MAP),
        ("Settings", t::SETTINGS),
        ("Cheats door", t::CHEATS),
        ("Quit to title", t::QUIT_TO_TITLE),
        ("Quit to desktop", t::QUIT_TO_DESKTOP),
        ("Footer: turns since save", t::LAST_SAVE_TURNS),
        ("Footer: just saved", t::LAST_SAVE_NOW),
        ("Footer: not saved", t::LAST_SAVE_NONE),
        ("Footer: live world", t::WAITING),
        ("Save unavailable (help)", t::SAVE_UNAVAILABLE),
        ("Key: close", t::KEY_CLOSE),
        ("Key: back", t::KEY_BACK),
        ("Load keys", t::LOAD_KEYS),
        ("Save title", t::SAVE_TITLE),
        ("New slot", t::NEW_SLOT),
        ("Name prompt", t::NAME_PROMPT),
        ("Default slot name", t::DEFAULT_SLOT_NAME),
        ("Save footer", t::SAVE_FOOTER),
        ("Load title", t::LOAD_TITLE),
        ("Damaged slot", t::DAMAGED),
        ("Imported slot", t::IMPORTED),
        ("Saved with cheats", t::SAVED_WITH_CHEATS),
        ("Turn", t::TURN),
        ("No slots", t::NO_SLOTS),
        ("Overwrite", t::OVERWRITE),
        ("Delete", t::DELETE),
        ("Delete warning", t::DELETE_WARNING),
        ("Load ask", t::LOAD_ASK),
        ("Quit to title ask", t::QUIT_TO_TITLE_ASK),
        ("Quit to desktop ask", t::QUIT_TO_DESKTOP_ASK),
        ("Unsaved turns", t::UNSAVED_TURNS),
        ("Unsaved session", t::UNSAVED_SESSION),
        ("Title layer title", t::TITLE_LAYER_TITLE),
        ("Continue a slot", t::CONTINUE_SLOT),
        ("Continue", t::CONTINUE),
        ("Quit (title)", t::QUIT),
        ("Tabs", "Audio | Display | Game | Extras"),
        ("Volume", t::VOLUME),
        ("Mute", t::MUTE),
        ("Mute on focus loss", t::MUTE_ON_FOCUS_LOSS),
        ("Audio help", t::AUDIO_HELP),
        ("Sound", t::SOUND),
        ("Buffer", t::BUFFER),
        ("Start sound off", t::START_SOUND_OFF),
        ("Start buffer on", t::START_BUFFER_ON),
        ("Confirm unsaved", t::CONFIRM_UNSAVED),
        ("Pacing", t::PACING),
        ("Reserved", t::RESERVED),
        ("Preset", t::PRESET),
        ("Custom preset", t::CUSTOM),
        ("Next prompt", t::NEXT_PROMPT),
        ("Next launch", t::NEXT_LAUNCH),
        ("Set by command line", t::SET_BY_COMMAND_LINE),
        ("Spoiler word", t::SPOILER),
        ("Cheats group", t::CHEATS_GROUP),
        ("Toast: opens at prompt", t::OPENS_AT_PROMPT),
        ("Toast: saved", t::SAVED_TO),
        ("Toast: save failed", t::SAVE_FAILED),
        ("Toast: slot write failed", t::SLOT_WRITE_FAILED),
        ("Toast: slots busy", t::SLOTS_BUSY),
        ("Toast: load failed", t::LOAD_FAILED),
    ];
    let mut table = String::from("where\ttext\n");
    for (place, text) in wording {
        table.push_str(&format!("{place}\t{text}\n"));
    }
    fs::write(out.join("wording.tsv"), table).unwrap();
    eprintln!(
        "menu sheet: {} screens to {}",
        rendered.len(),
        out.display()
    );
}
