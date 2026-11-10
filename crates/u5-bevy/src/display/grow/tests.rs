//! The canvas rule (design 2.2.2, 2.2.11; build plan M1-08): the three
//! named layouts exact with the party exactly centred, the other named
//! windows, narrow pans and the classic fallbacks.

use super::*;

fn canvas(w: u32, h: u32) -> Canvas {
    Canvas::for_window(w, h, PixelAspect::Dos).unwrap()
}

/// Every canvas: whole tiles, the core inside the map window and clear of
/// the open column, the party tile on the core's centre tile.
fn assert_sound(c: &Canvas) {
    assert!(c.w.is_multiple_of(TILE), "{c:?}");
    let (mx0, my0, mx1, my1) = c.map_window();
    let (cx, cy) = c.core_origin();
    assert!(cx >= COLUMN_CLEAR_X, "core under the open column: {c:?}");
    assert!(cx >= mx0 && cx + CORE_SIDE - 1 <= mx1, "{c:?}");
    assert!(cy >= my0 && cy + CORE_SIDE - 1 <= my1, "{c:?}");
    assert!(c.party_x.is_multiple_of(8), "{c:?}");
}

#[test]
fn the_three_named_layouts_are_exact_with_the_party_centred() {
    // (window, canvas, party tile, map window, scale)
    let named = [
        (
            (2560, 1440),
            (512, 240),
            (248, 116),
            (8, 16, 503, 231),
            (5, 6),
        ),
        (
            (3440, 1440),
            (688, 240),
            (336, 116),
            (8, 16, 679, 231),
            (5, 6),
        ),
        (
            (1920, 1080),
            (480, 216),
            (232, 104),
            (8, 16, 471, 207),
            (4, 5),
        ),
    ];
    for ((ww, wh), (w, h), (px, py), map, (sx, sy)) in named {
        let c = canvas(ww, wh);
        assert_eq!((c.w, c.h), (w, h), "{ww}x{wh}");
        assert_eq!((c.party_x, c.party_y), (px, py), "{ww}x{wh}");
        assert_eq!(c.pan_tiles, 0, "{ww}x{wh}");
        assert!(c.party_centred(), "{ww}x{wh}: {c:?}");
        assert_eq!(c.map_window(), map, "{ww}x{wh}");
        // Integer-exact: whole factors that fill the window and give the
        // canvas's own pixel aspect.
        assert_eq!((c.w * sx, c.h * sy), (ww, wh), "{ww}x{wh}");
        assert_eq!(sy * c.aspect.width, sx * c.aspect.height, "{ww}x{wh}");
        assert_sound(&c);
    }
    // The approved mockups' 16:9 and 21:9 canvases (c04f LAYOUTS).
    assert_eq!(canvas(2560, 1440).aspect, Aspect::DOS);
    assert_eq!(canvas(1920, 1080).aspect, ASPECT_1080);
    // 16:9 at any height that is not a multiple of 1080 is the 512 layout.
    for (ww, wh) in [(1280, 720), (1600, 900), (3200, 1800)] {
        let c = canvas(ww, wh);
        assert_eq!((c.w, c.h), (512, 240), "{ww}x{wh}");
    }
    // A 2160-line window is the 1080-line layout at 8x10.
    assert_eq!(canvas(3840, 2160), canvas(1920, 1080));
}

#[test]
fn the_other_named_windows() {
    // 2560x1080: 216 rows at 1.25, W = 16 * floor(270 * 2560 / 1080 / 16).
    let c = canvas(2560, 1080);
    assert_eq!(
        (c.w, c.h, c.party_x, c.party_y, c.pan_tiles),
        (640, 216, 312, 104, 0)
    );
    assert!(c.party_centred());
    assert_eq!(c.w * 4, 2560);
    assert_sound(&c);

    // 1920x1200 (16:10): 240 rows, W = 16 * floor(288 * 1.6 / 16) = 448,
    // narrower than 480, so the map pans one tile right: the centred core
    // would start at x 136, under the open column.
    let c = canvas(1920, 1200);
    assert_eq!((c.w, c.h, c.pan_tiles), (448, 240, 1));
    assert_eq!((c.party_x, c.party_y), (216 + 16, 116));
    assert_eq!(c.core_origin(), (152, 36));
    assert!(!c.party_centred());
    assert_sound(&c);

    // 1280x1024 (5:4): W = 16 * floor(288 * 1.25 / 16) = 352, under 384:
    // the classic frame.
    assert_eq!(Canvas::for_window(1280, 1024, PixelAspect::Dos), None);
}

#[test]
fn narrow_canvases_pan_by_whole_tiles_and_4_3_is_the_last() {
    // 4:3 is the narrowest canvas: 384 wide, panned three tiles.
    let c = canvas(1600, 1200);
    assert_eq!((c.w, c.pan_tiles, c.party_x), (384, 3, 232));
    assert_eq!(c.core_origin().0, COLUMN_CLEAR_X);
    assert_sound(&c);
    // Anything narrower is classic.
    assert_eq!(Canvas::for_window(1599, 1200, PixelAspect::Dos), None);
    // Every canvas from 384 to 480 pans just enough, never more.
    for ww in (1600..=2600).step_by(7) {
        let c = canvas(ww, 1200);
        assert_sound(&c);
        let centred = (c.w - TILE) / 2;
        assert_eq!(c.pan_tiles == 0, c.w >= NARROW_BELOW, "{c:?}");
        if c.pan_tiles > 0 {
            assert!(
                c.party_x - TILE - CORE_PARTY_OFFSET < COLUMN_CLEAR_X,
                "{c:?}"
            );
            assert_eq!(c.party_x, centred + u32::from(c.pan_tiles) * TILE);
        }
    }
}

#[test]
fn square_pixels_and_the_small_window_fallback() {
    // Square pixels: 240 rows at aspect 1, so 2560x1440 is 6x6 over
    // 416 (16 * floor(240 * 2560 / 1440 / 16)).
    let c = Canvas::for_window(2560, 1440, PixelAspect::Square).unwrap();
    assert_eq!((c.w, c.h, c.aspect), (416, 240, Aspect::SQUARE));
    assert_sound(&c);
    // ...and a 1080-line window keeps 216 rows, 5x5 over 384.
    let c = Canvas::for_window(1920, 1080, PixelAspect::Square).unwrap();
    assert_eq!((c.w, c.h, c.aspect), (384, 216, Aspect::SQUARE));
    assert_eq!((c.w * 5, c.h * 5), (1920, 1080));
    assert_sound(&c);

    // Below the frame at scale 1 there is no canvas (2.2.11).
    assert_eq!(Canvas::for_window(319, 240, PixelAspect::Dos), None);
    assert_eq!(Canvas::for_window(1000, 239, PixelAspect::Dos), None);
    assert_eq!(Canvas::for_window(0, 0, PixelAspect::Dos), None);
    // A wide window just tall enough still gets one.
    assert_eq!(canvas(1000, 240).w, 16 * (240 * 6 * 1000 / (5 * 240 * 16)));
}
