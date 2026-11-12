//! M3-10 layout tests: the gem canvas (D-M3.3) and the outdoor layout
//! (10.2.4): the party at the exact canvas centre, the cell counts, and
//! every canvas pixel covered by exactly one cell.

use super::*;

const CANVASES: [GemCanvas; 5] = [
    GemCanvas { w: 320, h: 200 },
    GemCanvas { w: 512, h: 240 },
    GemCanvas { w: 688, h: 240 },
    GemCanvas { w: 480, h: 216 },
    GemCanvas { w: 688, h: 216 },
];

#[test]
fn the_canvas_is_the_foundations_or_the_classic_320_by_200() {
    assert_eq!(
        gem_canvas(Some((512, 240)), (1920, 1080)),
        Some(GemCanvas { w: 512, h: 240 })
    );
    assert_eq!(
        gem_canvas(Some((688, 240)), (2560, 1080)),
        Some(GemCanvas { w: 688, h: 240 })
    );
    // 4:3 has no foundation canvas: the classic size.
    assert_eq!(gem_canvas(None, (1024, 768)), Some(GEM_CLASSIC_CANVAS));
    assert_eq!(gem_canvas(None, (320, 200)), Some(GEM_CLASSIC_CANVAS));
    // Below 320x200 the 1.0.1 view shows.
    assert_eq!(gem_canvas(None, (319, 200)), None);
    assert_eq!(gem_canvas(None, (320, 199)), None);
    assert_eq!(gem_canvas(Some((512, 240)), (300, 180)), None);
    // A degenerate foundation answer falls back too.
    assert_eq!(
        gem_canvas(Some((0, 240)), (800, 600)),
        Some(GEM_CLASSIC_CANVAS)
    );
}

#[test]
fn the_party_cell_centre_is_the_canvas_centre() {
    for canvas in CANVASES {
        let layout = OutdoorLayout::new(canvas);
        // Offset (0, 0), as the mockups' report.json checks.
        assert_eq!(
            layout.party_centre_x2(),
            (canvas.w as i32, canvas.h as i32),
            "{canvas:?}"
        );
        assert_eq!(
            layout.cell_rect(0, 0),
            (
                canvas.w as i32 / 2 - 2,
                canvas.h as i32 / 2 - 2,
                OUTDOOR_CELL_PX,
                OUTDOOR_CELL_PX
            )
        );
    }
}

#[test]
fn the_cell_counts_are_a_quarter_of_the_canvas() {
    let counts: Vec<(u32, u32)> = CANVASES[..3]
        .iter()
        .map(|&canvas| OutdoorLayout::new(canvas).cells())
        .collect();
    assert_eq!(counts, [(80, 50), (128, 60), (172, 60)]);
    assert_eq!(OutdoorLayout::new(CANVASES[3]).cells(), (120, 54));
}

#[test]
fn the_drawn_span_matches_the_mockups() {
    // report.json for O1 (party at 76,108): span_x 12..=140, span_y
    // 78..=138, that is 64 columns and 30 rows each way.
    let o1 = OutdoorLayout::new(GemCanvas { w: 512, h: 240 });
    assert_eq!((o1.dx, o1.dy), ((-64, 64), (-30, 30)));
    assert_eq!(o1.drawn(), (129, 61));
    let o2 = OutdoorLayout::new(GemCanvas { w: 688, h: 240 });
    assert_eq!((o2.dx, o2.dy), ((-86, 86), (-30, 30)));
    let o3 = OutdoorLayout::new(GemCanvas { w: 320, h: 200 });
    assert_eq!((o3.dx, o3.dy), ((-40, 40), (-25, 25)));
    assert_eq!(
        o1.gem_window(),
        GemWindow {
            half_cols: 64,
            half_rows: 30
        }
    );
}

#[test]
fn every_canvas_pixel_is_covered_by_exactly_one_cell() {
    for canvas in CANVASES {
        let layout = OutdoorLayout::new(canvas);
        let mut hits = vec![0u8; canvas.pixels()];
        for dy in layout.dy.0..=layout.dy.1 {
            for dx in layout.dx.0..=layout.dx.1 {
                let (x0, y0, w, h) = layout.cell_rect(dx, dy);
                let mut touched = false;
                for y in y0..y0 + h as i32 {
                    for x in x0..x0 + w as i32 {
                        if (0..canvas.w as i32).contains(&x) && (0..canvas.h as i32).contains(&y) {
                            hits[y as usize * canvas.w as usize + x as usize] += 1;
                            touched = true;
                        }
                    }
                }
                assert!(touched, "{canvas:?}: cell {dx},{dy} is off the canvas");
            }
        }
        assert!(hits.iter().all(|&n| n == 1), "{canvas:?}");
    }
}
