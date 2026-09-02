//! Clean semantic display-driver surface for the EGA-compatible v1 renderer.

use std::io;
use std::ops::Range;

use crate::*;

pub const DISPLAY_SURFACE_WIDTH: usize = TITLE_SURFACE_WIDTH as usize;
pub const DISPLAY_SURFACE_HEIGHT: usize = TITLE_SURFACE_HEIGHT as usize;
pub const DISPLAY_SURFACE_PIXELS: usize = DISPLAY_SURFACE_WIDTH * DISPLAY_SURFACE_HEIGHT;
pub const DISPLAY_TEXT_COLUMNS: usize = TEXT_SCREEN_COLUMNS as usize;
pub const DISPLAY_TEXT_ROWS: usize = TEXT_SCREEN_ROWS as usize;
/// `display-driver.md §2` boot-time user-interface colour table. The
/// startup pass that selects the graphics-resource family also publishes
/// this small table of colour indices, and the table is read all over the
/// program — the gameplay frame's accent and chrome pens, the sky strip's
/// markers, the Return-to-View caption panel, the Ultima IV transfer
/// preview, and the dungeon minimap's energy-field bands
/// (`dungeon-mode.md §12.5`) all name *slots* rather than raw indices.
pub const UI_COLOUR_TABLE_SLOTS: usize = 7;
/// `display-driver.md §2` high-colour set (EGA and Tandy).
pub const UI_COLOUR_TABLE_HIGH: [u8; UI_COLOUR_TABLE_SLOTS] = [4, 15, 1, 2, 5, 14, 7];
/// `display-driver.md §2` low-colour set (CGA and Hercules). The values
/// all sit inside `0..3` because those drivers mask the drawing colour
/// to two bits; on Hercules read them as pen selectors, not hues.
pub const UI_COLOUR_TABLE_LOW: [u8; UI_COLOUR_TABLE_SLOTS] = [2, 3, 1, 1, 2, 3, 3];
/// `dungeon-mode.md §12.5`: the amount a caller adds to a slot value to
/// bias it "into the bright half of the palette".
pub const UI_COLOUR_BRIGHT_BIAS: u8 = 8;

/// `display-driver.md §2`: resolve one user-interface colour-table slot.
/// `high_colour` selects the EGA/Tandy set over the CGA/Hercules one.
/// Out-of-range slots return zero rather than panicking, because the
/// table is fixed-size and every published consumer names a slot inside
/// it.
pub const fn ui_colour_slot(slot: usize, high_colour: bool) -> u8 {
    if slot >= UI_COLOUR_TABLE_SLOTS {
        return 0;
    }
    if high_colour {
        UI_COLOUR_TABLE_HIGH[slot]
    } else {
        UI_COLOUR_TABLE_LOW[slot]
    }
}

/// `display-driver.md §2` + `dungeon-mode.md §12.5`: a slot resolved and
/// then biased into the bright half of the palette.
pub const fn ui_colour_slot_bright(slot: usize, high_colour: bool) -> u8 {
    ui_colour_slot(slot, high_colour) + UI_COLOUR_BRIGHT_BIAS
}

pub const EGA_DRIVER_SLOT_COUNT: u8 = 38;
pub const EGA_DRIVER_LAST_DISPATCH_OFFSET: u8 = (EGA_DRIVER_SLOT_COUNT - 1) * 3;

/// `display-driver-abi.md §9.5`: dispatch offset `0x27` picks between two
/// live bodies "by the rectangle's left edge", and the message-panel fast
/// path is the one whose "left edge at pixel column 192".
pub const MESSAGE_PANEL_SCROLL_LEFT_EDGE: usize = 192;
/// `display-driver-abi.md §9.5`, message-panel fast path: "Pixel columns
/// 192 through 319 inclusive (a 128-pixel-wide right-side text panel, 16
/// character cells wide)."
pub const MESSAGE_PANEL_SCROLL_RIGHT_EDGE: usize = 319;
/// `display-driver-abi.md §9.5`, message-panel fast path: "Pixel rows 88
/// through 199".
pub const MESSAGE_PANEL_SCROLL_TOP: usize = 88;
pub const MESSAGE_PANEL_SCROLL_BOTTOM: usize = 199;
/// `display-driver-abi.md §9.5`, message-panel fast path: "Exactly eight
/// scanlines upward, hardcoded. The caller's distance argument is not
/// read on this path."
pub const MESSAGE_PANEL_SCROLL_SCANLINES: usize = 8;
/// `display-driver-abi.md §9.5`, general path: the vacated band is filled
/// "with colour index `0`" after the current drawing colour is saved, and
/// the colour is restored afterwards.
pub const SCROLL_VACATED_BAND_COLOUR: u8 = 0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisplayRenderTarget {
    Front,
    Back,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EgaDispatchResult {
    None,
    ScreenHeight(u16),
    BackBufferSegment(u16),
    Pixel(u8),
    Rect(DisplayPixelRect),
}

pub enum EgaDisplayOperation<'a> {
    ScreenHeight,
    EnterGraphicsMode,
    InitBackBuffer,
    ReleaseBackBuffer,
    SetRenderTarget(DisplayRenderTarget),
    PrepareBackBufferState,
    SetCurrentColor(u8),
    ReadPixel {
        x: usize,
        y: usize,
    },
    PlotPixel {
        x: usize,
        y: usize,
    },
    DrawLine {
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
    },
    /// `display-driver-abi.md §9.5` dispatch offset `0x27`: "Scroll a
    /// rectangle vertically by a signed row distance, on whichever
    /// surface the render-target selector names, blanking the vacated
    /// band. A hardwired fast path handles the message panel's
    /// eight-scanline scroll-up without blanking."
    ScrollRect {
        rect: DisplayPixelRect,
        rows: i32,
    },
    FillBackRect {
        rect: DisplayPixelRect,
        color: u8,
    },
    FillOldRect {
        rect: DisplayPixelRect,
        color: u8,
    },
    FillClippedRect(DisplayPixelRect),
    CopyBackToFront(DisplayPixelRect),
    DissolveBackToFront(DisplayPixelRect),
    DrawTile {
        atlas: &'a TileAtlas,
        tile: usize,
        dst_x: i32,
        dst_y: i32,
    },
    DrawGlyph {
        font: &'a FixedCellFont,
        code: u8,
        cell_x: usize,
        cell_y: usize,
        foreground: u8,
        background: u8,
    },
    AdvanceTitleTick,
    SaveLoadedTileGraphics {
        atlas: &'a TileAtlas,
        saved: &'a mut EgaLoadedTileGraphicsSave,
    },
    RestoreLoadedTileGraphics {
        atlas: &'a mut TileAtlas,
        saved: &'a EgaLoadedTileGraphicsSave,
    },
    /// `display-driver-abi.md §10` offset `0x6C`, red/green plane-swap
    /// mode. Its only real selector is the dungeon look/view code; combat
    /// never selects it (`RETRACTIONS.md` R304). See
    /// [`EGA_RED_GREEN_PLANE_SWAP_TILES`] for the published tile set and
    /// the full five-caller census.
    SwapLoadedTileRedGreenPlanes {
        atlas: &'a mut TileAtlas,
        tile_range: Range<usize>,
    },
    PresentFrame,
    NoOp,
}

pub fn ega_driver_dispatch_slot(dispatch_offset: u8) -> Option<u8> {
    if dispatch_offset % 3 != 0 || dispatch_offset > EGA_DRIVER_LAST_DISPATCH_OFFSET {
        return None;
    }
    Some(dispatch_offset / 3)
}

pub fn ega_driver_dispatch_offset(slot: u8) -> Option<u8> {
    if slot >= EGA_DRIVER_SLOT_COUNT {
        return None;
    }
    Some(slot * 3)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DisplayPixelRect {
    pub x0: usize,
    pub y0: usize,
    pub x1: usize,
    pub y1: usize,
}

impl DisplayPixelRect {
    pub const fn width(self) -> usize {
        self.x1 - self.x0 + 1
    }

    pub const fn height(self) -> usize {
        self.y1 - self.y0 + 1
    }
}

pub fn normalize_clamp_pixel_rect(x0: i32, y0: i32, x1: i32, y1: i32) -> Option<DisplayPixelRect> {
    let min_x = if x0 < x1 { x0 } else { x1 };
    let max_x = if x0 < x1 { x1 } else { x0 };
    let min_y = if y0 < y1 { y0 } else { y1 };
    let max_y = if y0 < y1 { y1 } else { y0 };
    if max_x < 0
        || max_y < 0
        || min_x >= DISPLAY_SURFACE_WIDTH as i32
        || min_y >= DISPLAY_SURFACE_HEIGHT as i32
    {
        return None;
    }
    Some(DisplayPixelRect {
        x0: min_x.max(0) as usize,
        y0: min_y.max(0) as usize,
        x1: max_x.min(DISPLAY_SURFACE_WIDTH as i32 - 1) as usize,
        y1: max_y.min(DISPLAY_SURFACE_HEIGHT as i32 - 1) as usize,
    })
}

pub fn text_cell_rect_to_pixel_rect(
    cell_x0: i32,
    cell_y0: i32,
    cell_x1: i32,
    cell_y1: i32,
) -> Option<DisplayPixelRect> {
    let min_cell_x = cell_x0.min(cell_x1);
    let max_cell_x = cell_x0.max(cell_x1);
    let min_cell_y = cell_y0.min(cell_y1);
    let max_cell_y = cell_y0.max(cell_y1);

    normalize_clamp_pixel_rect(
        min_cell_x * CH_CELL_SIDE as i32,
        min_cell_y * CH_CELL_SIDE as i32,
        max_cell_x * CH_CELL_SIDE as i32 + (CH_CELL_SIDE as i32 - 1),
        max_cell_y * CH_CELL_SIDE as i32 + (CH_CELL_SIDE as i32 - 1),
    )
}

/// The dispatch `0x66` rectangle-dissolve state, carry clear
/// (`systems/display-driver-abi.md` section 9.6).
///
/// This is the driver-surface entry: it walks the rectangle over the surface's
/// own back and front buffers and supports partial steps, which is what the
/// dispatch site needs. The visit order itself comes from the shared
/// [`crate::DissolveVisitOrder`], so this and the caller-side
/// [`crate::RectangleDissolve`] scatter identically - one published operation,
/// one order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EgaDissolveState {
    rect: DisplayPixelRect,
    order: crate::DissolveVisitOrder,
}

impl EgaDissolveState {
    pub fn new(rect: DisplayPixelRect) -> Self {
        let total = rect.width() * rect.height();
        Self {
            rect,
            order: crate::DissolveVisitOrder::new(total)
                .expect("a display rectangle always fits the dissolve tap inventory"),
        }
    }

    pub const fn rect(&self) -> DisplayPixelRect {
        self.rect
    }

    pub const fn total_pixels(&self) -> usize {
        self.order.count()
    }

    pub const fn copied_pixels(&self) -> usize {
        self.order.visited()
    }

    pub const fn remaining_pixels(&self) -> usize {
        self.order.remaining()
    }

    pub const fn is_finished(&self) -> bool {
        self.order.is_finished()
    }

    pub fn next_pixel(&mut self) -> Option<(usize, usize)> {
        let visit = self.order.next_index()?;
        Some((
            self.rect.x0 + (visit % self.rect.width()),
            self.rect.y0 + (visit / self.rect.width()),
        ))
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EgaLoadedTileGraphicsSave {
    pixels: Option<Vec<u8>>,
}

impl EgaLoadedTileGraphicsSave {
    pub fn has_saved_pixels(&self) -> bool {
        self.pixels.is_some()
    }

    pub fn saved_pixels(&self) -> Option<&[u8]> {
        self.pixels.as_deref()
    }

    pub fn save_from_atlas(&mut self, atlas: &TileAtlas) {
        self.pixels = Some(atlas.pixels.clone());
    }

    pub fn restore_to_atlas(&self, atlas: &mut TileAtlas) -> io::Result<()> {
        let Some(pixels) = &self.pixels else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "loaded tile graphics restore requested before save",
            ));
        };
        atlas.pixels.clone_from(pixels);
        Ok(())
    }
}

/// `display-driver-abi.md §10`, dispatch offset `0x6C`: the exact tile set
/// the red/green plane-swap mode covers - "tiles `0x05`, `0x1E`, `0x1F`,
/// `0x4C`, `0xCA`, `0x20..0x26`, `0x30..0x37` and `0x60..0x6F`".
///
/// **Caller correction (`RETRACTIONS.md` R304).** An earlier revision
/// described this mode as "used for combat-style terrain coloration", and
/// the entry as reached from three resident dispatch sites. Both are
/// withdrawn: "Those three sites are thin trampolines, not callers." The
/// five real callers of `0x6C`, and the modes they select, are
///
/// 1. the dungeon look/view code, twice - once for the mode that saves
///    eight bytes of two unrelated tiles and sets a flag, and once for
///    **this** plane-swap mode, the entry's only selector of it;
/// 2. the **per-turn clock advance**, which selects the moon/sun phase
///    painter and edits only the moon/sun phase tiles - "this runs once per
///    game turn, not at a scene transition, so 'this entry fires only on
///    scene transitions' is also withdrawn";
/// 3. the post-combat restore path, which selects the restore-eight-bytes
///    mode, gated on the flag the dungeon path set;
/// 4. the endgame sequence, which selects the whole-tileset remap - the
///    only one of the five that touches fire fixtures, and then only
///    `0xB0`, `0xB1` and `0xBF`, one-shot and not an animation.
///
/// **Combat never selects the plane swap.** The combat framer's own reached
/// call uses mode value `1`, the restoration step.
///
/// "It touches no water tile and no fire fixture directly, but `0x34..0x37`
/// and `0x60..0x6F` are two of the three destination groups of the per-step
/// water composite (`animation.md §12.3`), so the swap and the water
/// animation write to the same bitmaps and interact."
pub const EGA_RED_GREEN_PLANE_SWAP_TILES: [u8; 36] = [
    0x05, 0x1E, 0x1F, 0x4C, 0xCA, 0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x30, 0x31, 0x32, 0x33,
    0x34, 0x35, 0x36, 0x37, 0x60, 0x61, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0x6A, 0x6B,
    0x6C, 0x6D, 0x6E, 0x6F,
];

/// Is `tile` covered by the `§10` red/green plane-swap mode?
pub const fn ega_red_green_plane_swap_covers_tile(tile: u8) -> bool {
    matches!(
        tile,
        0x05 | 0x1E | 0x1F | 0x4C | 0xCA | 0x20..=0x26 | 0x30..=0x37 | 0x60..=0x6F
    )
}

/// Apply the plane swap to exactly the `§10` tile set, rather than to a
/// caller-chosen range. This is the shape the one real selector - the
/// dungeon look/view path - uses.
///
/// **No production path calls this yet.** This engine's dungeon look/view
/// code does not select the `0x6C` plane-swap mode, so the published tile
/// set is currently recorded and tested rather than applied.
pub fn swap_loaded_tile_red_green_planes_over_published_set(atlas: &mut TileAtlas) {
    for tile in 0u16..=0xFF {
        let tile = tile as u8;
        if ega_red_green_plane_swap_covers_tile(tile) {
            let start = usize::from(tile);
            swap_loaded_tile_red_green_planes(atlas, start..start + 1);
        }
    }
}

pub fn swap_loaded_tile_red_green_planes(atlas: &mut TileAtlas, tile_range: Range<usize>) {
    let start = tile_range.start.saturating_mul(TILE_ATLAS_TILE_PIXELS);
    let end = tile_range
        .end
        .saturating_mul(TILE_ATLAS_TILE_PIXELS)
        .min(atlas.pixels.len());
    if start >= end {
        return;
    }
    for pixel in &mut atlas.pixels[start..end] {
        let value = *pixel & 0x0f;
        *pixel = (value & !0x06) | ((value & 0x02) << 1) | ((value & 0x04) >> 1);
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EgaDisplaySurface {
    front_pixels: Vec<u8>,
    back_pixels: Vec<u8>,
    current_color: u8,
    title_tick_frame: u8,
    title_tick_frames: Option<TitleTickFrameSet>,
    presented_frames: u64,
    render_target: DisplayRenderTarget,
    back_buffer_active: bool,
}

impl Default for EgaDisplaySurface {
    fn default() -> Self {
        Self::new()
    }
}

impl EgaDisplaySurface {
    pub fn new() -> Self {
        Self {
            front_pixels: vec![0; DISPLAY_SURFACE_PIXELS],
            back_pixels: vec![0; DISPLAY_SURFACE_PIXELS],
            current_color: 0,
            title_tick_frame: 0,
            title_tick_frames: None,
            presented_frames: 0,
            render_target: DisplayRenderTarget::Front,
            back_buffer_active: false,
        }
    }

    pub fn with_title_tick_frames(frames: TitleTickFrameSet) -> Self {
        let mut surface = Self::new();
        surface.title_tick_frames = Some(frames);
        surface
    }

    pub fn front_pixels(&self) -> &[u8] {
        &self.front_pixels
    }

    pub fn back_pixels(&self) -> &[u8] {
        &self.back_pixels
    }

    pub fn current_color(&self) -> u8 {
        self.current_color
    }

    pub fn title_tick_frame(&self) -> u8 {
        self.title_tick_frame
    }

    pub fn set_title_tick_frames(&mut self, frames: TitleTickFrameSet) {
        self.title_tick_frames = Some(frames);
    }

    pub fn presented_frames(&self) -> u64 {
        self.presented_frames
    }

    pub fn render_target(&self) -> DisplayRenderTarget {
        self.render_target
    }

    pub fn back_buffer_active(&self) -> bool {
        self.back_buffer_active
    }

    pub fn set_render_target(&mut self, target: DisplayRenderTarget) {
        self.render_target = target;
    }

    /// The surface the render-target selector currently names.
    ///
    /// `display-driver-abi.md §6`, `§8` and `§9.2`: the clipped
    /// rectangle fill (`0x3F`), the 16-by-16 tile entry (`0x51`), the
    /// fixed-cell glyph entry (`0x5D`) and the pixel plot (`0x30`) each
    /// read the descriptor's render-target selector and branch to a
    /// *separate, complete* back-buffer body. Every one of those bodies
    /// draws for real; the spec explicitly withdraws the earlier reading
    /// that any of them was front-buffer-only or a silent no-op on the
    /// hidden surface. Routing them all through one accessor keeps that
    /// contract in a single place.
    fn render_pixels_mut(&mut self) -> &mut [u8] {
        match self.render_target {
            DisplayRenderTarget::Front => &mut self.front_pixels,
            DisplayRenderTarget::Back => &mut self.back_pixels,
        }
    }

    pub fn init_back_buffer(&mut self) -> u16 {
        self.back_buffer_active = true;
        0
    }

    pub fn release_back_buffer(&mut self) {
        self.back_buffer_active = false;
        self.render_target = DisplayRenderTarget::Front;
        self.back_pixels.fill(0);
    }

    pub fn set_current_color(&mut self, color: u8) {
        self.current_color = color & 0x0f;
    }

    pub fn read_pixel(&self, x: usize, y: usize) -> Option<u8> {
        if x >= DISPLAY_SURFACE_WIDTH || y >= DISPLAY_SURFACE_HEIGHT {
            return None;
        }
        self.front_pixels
            .get(y * DISPLAY_SURFACE_WIDTH + x)
            .copied()
    }

    pub fn plot_pixel(&mut self, x: usize, y: usize) {
        if x >= DISPLAY_SURFACE_WIDTH || y >= DISPLAY_SURFACE_HEIGHT {
            panic!(
                "display pixel plot at ({x}, {y}) exceeds {DISPLAY_SURFACE_WIDTH}x{DISPLAY_SURFACE_HEIGHT}; clipping is a forbidden fallback"
            );
        }
        self.front_pixels[y * DISPLAY_SURFACE_WIDTH + x] = self.current_color;
    }

    pub fn draw_line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32) {
        assert_display_point_in_bounds(x0, y0, "display line start");
        assert_display_point_in_bounds(x1, y1, "display line end");
        let mut x = x0;
        let mut y = y0;
        let dx = (x1 - x0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let dy = -(y1 - y0).abs();
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        loop {
            self.front_pixels[y as usize * DISPLAY_SURFACE_WIDTH + x as usize] = self.current_color;
            if x == x1 && y == y1 {
                break;
            }
            let e2 = err.saturating_mul(2);
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    }

    /// `display-driver-abi.md §6` dispatch offset `0x3F`. The entry
    /// reads the render-target selector before filling and writes
    /// whichever surface it names; both row loops fill for real. The
    /// endgame fade and the map-viewport fades all point the selector at
    /// the hidden surface, fill through this entry, then dissolve
    /// forward, so a no-op here would dissolve stale pixels.
    pub fn fill_rect_current_color(&mut self, rect: DisplayPixelRect) {
        let color = self.current_color & 0x0f;
        fill_pixels_rect(self.render_pixels_mut(), rect, color);
    }

    pub fn fill_rect(&mut self, rect: DisplayPixelRect, color: u8) {
        let color = color & 0x0f;
        fill_pixels_rect(&mut self.front_pixels, rect, color);
    }

    pub fn fill_back_rect(&mut self, rect: DisplayPixelRect, color: u8) {
        let color = color & 0x0f;
        fill_pixels_rect(&mut self.back_pixels, rect, color);
    }

    pub fn clear_rect(&mut self, rect: DisplayPixelRect) {
        self.fill_rect(rect, 0);
    }

    /// `display-driver-abi.md §9.5` dispatch offset `0x27`, the vertical
    /// rectangle scroll. "**Correction: this is a general
    /// scroll-rectangle entry.** An earlier revision of this document
    /// said the entry checked that its primary argument named the message
    /// panel's left edge and that 'calls with any other left-edge value
    /// return without visible effect', concluding that the entry was
    /// 'strictly a right-side-text-panel scroll, not a general
    /// scroll-rectangle helper', and that any per-call distance argument
    /// was 'vestigial'. Those statements are withdrawn. The left-edge
    /// test selects between two live bodies; it does not gate the entry."
    ///
    /// `rows` is the signed scanline distance. The published text fixes
    /// both directions but not which sign names which — "positive moves
    /// the rectangle's contents one way, negative the other" — so this
    /// engine follows screen-space y: negative scrolls the contents
    /// upward, positive downward.
    pub fn scroll_rect_rows(&mut self, rect: DisplayPixelRect, rows: i32) {
        if rect.x0 == MESSAGE_PANEL_SCROLL_LEFT_EDGE {
            self.scroll_message_panel_fast_path();
            return;
        }
        if rows == 0 {
            return;
        }

        let height = rect.y1 - rect.y0 + 1;
        let distance = rows.unsigned_abs() as usize;
        let moved = height.saturating_sub(distance);
        let width = DISPLAY_SURFACE_WIDTH;
        let pixels = self.render_pixels_mut();

        // The sign is folded into the row-walk direction so the copy
        // never overlaps itself destructively.
        if moved > 0 {
            if rows < 0 {
                for y in rect.y0..(rect.y0 + moved) {
                    let src = (y + distance) * width;
                    let dst = y * width;
                    pixels.copy_within(src + rect.x0..=src + rect.x1, dst + rect.x0);
                }
            } else {
                for y in (rect.y0 + distance..=rect.y1).rev() {
                    let src = (y - distance) * width;
                    let dst = y * width;
                    pixels.copy_within(src + rect.x0..=src + rect.x1, dst + rect.x0);
                }
            }
        }

        // "When the copy finishes, the entry **blanks the vacated band**:
        // it saves the current drawing colour, fills the band the
        // contents moved out of with colour index `0`, and restores the
        // colour. The band is computed from the distance and its sign, so
        // it is the correct edge of the rectangle in either direction."
        let band_rows = distance.min(height);
        let band = if rows < 0 {
            DisplayPixelRect {
                x0: rect.x0,
                y0: rect.y1 + 1 - band_rows,
                x1: rect.x1,
                y1: rect.y1,
            }
        } else {
            DisplayPixelRect {
                x0: rect.x0,
                y0: rect.y0,
                x1: rect.x1,
                y1: rect.y0 + band_rows - 1,
            }
        };
        let saved_color = self.current_color;
        self.current_color = SCROLL_VACATED_BAND_COLOUR;
        self.fill_rect_current_color(band);
        self.current_color = saved_color;
    }

    /// `display-driver-abi.md §9.5`, "Message-panel fast path — left edge
    /// at pixel column 192": "This path is hardwired and ignores both the
    /// rest of the rectangle and the distance argument". The exposed band
    /// is *not* blanked — "After the scroll, the bottom eight scanlines of
    /// the panel inherit whatever pixels happened to lie immediately below
    /// the panel before the scroll", which on this 200-row surface is
    /// non-visible video memory the engine does not model, so those rows
    /// are left exactly as they were rather than cleared.
    fn scroll_message_panel_fast_path(&mut self) {
        let width = DISPLAY_SURFACE_WIDTH;
        let distance = MESSAGE_PANEL_SCROLL_SCANLINES;
        let pixels = self.render_pixels_mut();
        for y in MESSAGE_PANEL_SCROLL_TOP..=(MESSAGE_PANEL_SCROLL_BOTTOM - distance) {
            let src = (y + distance) * width;
            let dst = y * width;
            pixels.copy_within(
                src + MESSAGE_PANEL_SCROLL_LEFT_EDGE..=src + MESSAGE_PANEL_SCROLL_RIGHT_EDGE,
                dst + MESSAGE_PANEL_SCROLL_LEFT_EDGE,
            );
        }
    }

    /// Thin wrapper for the text layer's one-cell-row scroll:
    /// `display-driver-abi.md §9.5` notes that "every text scroll moves
    /// exactly one cell row regardless of the distance the resident
    /// helper computed".
    pub fn scroll_text_rect_up_one_row(&mut self, rect: DisplayPixelRect) {
        self.scroll_rect_rows(rect, -(CH_CELL_SIDE as i32));
    }

    pub fn copy_back_to_front_rect(&mut self, rect: DisplayPixelRect) {
        copy_rect_between_buffers(&self.back_pixels, &mut self.front_pixels, rect);
    }

    /// Copy one pixel from the hidden surface to the visible page.
    ///
    /// `display-driver-abi.md §9.6`: the rectangle dissolve always reads the
    /// hidden surface and always writes the visible page, whatever the
    /// render-target selector says. This is the per-pixel primitive
    /// [`crate::RectangleDissolve`] drives, so callers share one visit order
    /// instead of each carrying a transfer of their own.
    pub fn copy_back_pixel_to_front(&mut self, x: usize, y: usize) {
        assert!(
            x < DISPLAY_SURFACE_WIDTH && y < DISPLAY_SURFACE_HEIGHT,
            "dissolve pixel ({x}, {y}) exceeds {DISPLAY_SURFACE_WIDTH}x{DISPLAY_SURFACE_HEIGHT}"
        );
        let index = y * DISPLAY_SURFACE_WIDTH + x;
        self.front_pixels[index] = self.back_pixels[index];
    }

    pub fn dissolve_back_to_front_rect(&mut self, rect: DisplayPixelRect) {
        let mut state = EgaDissolveState::new(rect);
        while self.dissolve_back_to_front_step(&mut state, usize::MAX) != 0 {}
    }

    pub fn dissolve_back_to_front_step(
        &mut self,
        state: &mut EgaDissolveState,
        max_pixels: usize,
    ) -> usize {
        let mut copied = 0;
        while copied < max_pixels {
            let Some((x, y)) = state.next_pixel() else {
                break;
            };
            let index = y * DISPLAY_SURFACE_WIDTH + x;
            self.front_pixels[index] = self.back_pixels[index];
            copied += 1;
        }
        copied
    }

    pub fn blit_tile_at_pixel(
        &mut self,
        atlas: &TileAtlas,
        tile: usize,
        dst_x: i32,
        dst_y: i32,
    ) -> io::Result<()> {
        if dst_x < 0
            || dst_y < 0
            || dst_x + TILE_ATLAS_SIDE as i32 > DISPLAY_SURFACE_WIDTH as i32
            || dst_y + TILE_ATLAS_SIDE as i32 > DISPLAY_SURFACE_HEIGHT as i32
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "display tile blit at ({dst_x}, {dst_y}) with size {TILE_ATLAS_SIDE}x{TILE_ATLAS_SIDE} would clip against {DISPLAY_SURFACE_WIDTH}x{DISPLAY_SURFACE_HEIGHT}; clipping is a forbidden fallback"
                ),
            ));
        }
        let pixels = atlas.tile_pixels(tile).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("tile atlas is missing tile {tile}"),
            )
        })?;
        let mut staged = [0u8; TILE_ATLAS_SIDE * TILE_ATLAS_SIDE];
        for (index, slot) in staged.iter_mut().enumerate() {
            *slot = pixels[index] & 0x0f;
        }
        let target = self.render_pixels_mut();
        for row in 0..TILE_ATLAS_SIDE {
            let y = dst_y + row as i32;
            for col in 0..TILE_ATLAS_SIDE {
                let x = dst_x + col as i32;
                target[y as usize * DISPLAY_SURFACE_WIDTH + x as usize] =
                    staged[row * TILE_ATLAS_SIDE + col];
            }
        }
        Ok(())
    }

    pub fn blit_tile_cell(
        &mut self,
        atlas: &TileAtlas,
        tile: usize,
        cell_x: usize,
        cell_y: usize,
    ) -> io::Result<()> {
        self.blit_tile_at_pixel(
            atlas,
            tile,
            (cell_x * TILE_ATLAS_SIDE) as i32,
            (cell_y * TILE_ATLAS_SIDE) as i32,
        )
    }

    pub fn draw_fixed_glyph_cell(
        &mut self,
        font: &FixedCellFont,
        code: u8,
        cell_x: usize,
        cell_y: usize,
        foreground: u8,
        background: u8,
    ) -> io::Result<()> {
        let dst_x = cell_x * CH_CELL_SIDE;
        let dst_y = cell_y * CH_CELL_SIDE;
        if cell_x >= DISPLAY_TEXT_COLUMNS || cell_y >= DISPLAY_TEXT_ROWS {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "display fixed glyph at cell ({cell_x}, {cell_y}) would clip against {DISPLAY_TEXT_COLUMNS}x{DISPLAY_TEXT_ROWS} text cells; clipping is a forbidden fallback"
                ),
            ));
        }
        let mut staged = [0u8; CH_CELL_SIDE];
        for (glyph_y, slot) in staged.iter_mut().enumerate() {
            *slot = font.glyph_row(code & 0x7f, glyph_y).ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("fixed font glyph {} is missing row {glyph_y}", code & 0x7f),
                )
            })?;
        }
        let target = self.render_pixels_mut();
        for (glyph_y, row_bits) in staged.into_iter().enumerate() {
            for glyph_x in 0..CH_CELL_SIDE {
                let color = if row_bits & (1 << (7 - glyph_x)) != 0 {
                    foreground
                } else {
                    background
                } & 0x0f;
                let x = dst_x + glyph_x;
                let y = dst_y + glyph_y;
                target[y * DISPLAY_SURFACE_WIDTH + x] = color;
            }
        }
        Ok(())
    }

    pub fn advance_title_tick(&mut self) -> DisplayPixelRect {
        let rect = DisplayPixelRect {
            x0: TITLE_TICK_FRAME_X as usize,
            y0: TITLE_TICK_FRAME_Y as usize,
            x1: (TITLE_TICK_FRAME_X + TITLE_TICK_FRAME_WIDTH - 1) as usize,
            y1: (TITLE_TICK_FRAME_Y + TITLE_TICK_FRAME_HEIGHT - 1) as usize,
        };
        // `cleak/u5-spec#65`: every tick copies 49 rows at the full
        // 320-pixel width from the staged band to visible rows
        // 65..=113 — an opaque full-rectangle overwrite with no
        // transparency key and no preserved pixels. The staged frames
        // already carry the cleared flanks at columns 0..=15 and
        // 304..=319.
        let band_width = TITLE_TICK_FRAME_WIDTH as usize;
        for (row, src) in self
            .title_tick_frames
            .as_ref()
            .expect(
                "display title-tick operation requires the ULTIMA title-tick bands to be injected; generated clean-room frames are a forbidden fallback; see cleak/u5-spec#65",
            )
            .frame_pixels(self.title_tick_frame)
            .chunks_exact(band_width)
            .enumerate()
        {
            let dst_start = (rect.y0 + row) * DISPLAY_SURFACE_WIDTH + rect.x0;
            self.front_pixels[dst_start..dst_start + band_width].copy_from_slice(src);
        }
        self.title_tick_frame = title_tick_next_frame(self.title_tick_frame);
        rect
    }

    pub fn present_frame(&mut self) {
        self.presented_frames = self.presented_frames.saturating_add(1);
    }

    pub fn execute(&mut self, operation: EgaDisplayOperation<'_>) -> io::Result<EgaDispatchResult> {
        match operation {
            EgaDisplayOperation::ScreenHeight => Ok(EgaDispatchResult::ScreenHeight(
                DISPLAY_SURFACE_HEIGHT as u16,
            )),
            EgaDisplayOperation::EnterGraphicsMode => {
                self.render_target = DisplayRenderTarget::Front;
                Ok(EgaDispatchResult::None)
            }
            EgaDisplayOperation::InitBackBuffer => {
                let segment = self.init_back_buffer();
                Ok(EgaDispatchResult::BackBufferSegment(segment))
            }
            EgaDisplayOperation::ReleaseBackBuffer => {
                self.release_back_buffer();
                Ok(EgaDispatchResult::None)
            }
            EgaDisplayOperation::SetRenderTarget(target) => {
                self.set_render_target(target);
                Ok(EgaDispatchResult::None)
            }
            EgaDisplayOperation::PrepareBackBufferState => {
                self.back_buffer_active = true;
                Ok(EgaDispatchResult::None)
            }
            EgaDisplayOperation::SetCurrentColor(color) => {
                self.set_current_color(color);
                Ok(EgaDispatchResult::None)
            }
            EgaDisplayOperation::ReadPixel { x, y } => {
                let pixel = self.read_pixel(x, y).ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!(
                            "display pixel read at ({x}, {y}) exceeds {DISPLAY_SURFACE_WIDTH}x{DISPLAY_SURFACE_HEIGHT}; defaulting to black is a forbidden fallback"
                        ),
                    )
                })?;
                Ok(EgaDispatchResult::Pixel(pixel))
            }
            EgaDisplayOperation::PlotPixel { x, y } => {
                if x >= DISPLAY_SURFACE_WIDTH || y >= DISPLAY_SURFACE_HEIGHT {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!(
                            "display pixel plot at ({x}, {y}) exceeds {DISPLAY_SURFACE_WIDTH}x{DISPLAY_SURFACE_HEIGHT}; clipping is a forbidden fallback"
                        ),
                    ));
                }
                if self.render_target == DisplayRenderTarget::Front {
                    self.plot_pixel(x, y);
                } else {
                    self.back_pixels[y * DISPLAY_SURFACE_WIDTH + x] = self.current_color;
                }
                Ok(EgaDispatchResult::None)
            }
            EgaDisplayOperation::DrawLine { x0, y0, x1, y1 } => {
                // `display-driver-abi.md §9.2` dispatch offset `0x33`:
                // the line entry re-points the single-pixel writer at the
                // visible page before every pixel, so lines are
                // front-buffer-only *regardless of the render-target
                // selector*. That is a draw to the front buffer, not a
                // skipped draw.
                self.draw_line(x0, y0, x1, y1);
                Ok(EgaDispatchResult::None)
            }
            EgaDisplayOperation::ScrollRect { rect, rows } => {
                // `display-driver-abi.md §9.5`: the entry "reads the
                // descriptor's render-target selector and has a
                // **separate, complete body for the hidden surface**,
                // exactly like the fill, tile and glyph entries", so
                // there is no Front-only guard here — a hidden-surface
                // scroll is a real scroll, not a silent no-op.
                self.scroll_rect_rows(rect, rows);
                Ok(EgaDispatchResult::None)
            }
            EgaDisplayOperation::FillBackRect { rect, color }
            | EgaDisplayOperation::FillOldRect { rect, color } => {
                if self.render_target == DisplayRenderTarget::Back {
                    self.fill_back_rect(rect, color);
                } else {
                    self.fill_rect(rect, color);
                }
                Ok(EgaDispatchResult::Rect(rect))
            }
            EgaDisplayOperation::FillClippedRect(rect) => {
                // `display-driver-abi.md §6` dispatch offset `0x3F` is
                // render-target aware; both surfaces fill for real.
                self.fill_rect_current_color(rect);
                Ok(EgaDispatchResult::Rect(rect))
            }
            EgaDisplayOperation::CopyBackToFront(rect) => {
                self.copy_back_to_front_rect(rect);
                Ok(EgaDispatchResult::Rect(rect))
            }
            EgaDisplayOperation::DissolveBackToFront(rect) => {
                self.dissolve_back_to_front_rect(rect);
                Ok(EgaDispatchResult::Rect(rect))
            }
            EgaDisplayOperation::DrawTile {
                atlas,
                tile,
                dst_x,
                dst_y,
            } => {
                // `display-driver-abi.md §8` dispatch offset `0x51`
                // branches to a separate, complete back-buffer body when
                // the selector names the hidden surface.
                self.blit_tile_at_pixel(atlas, tile, dst_x, dst_y)?;
                Ok(EgaDispatchResult::None)
            }
            EgaDisplayOperation::DrawGlyph {
                font,
                code,
                cell_x,
                cell_y,
                foreground,
                background,
            } => {
                // `display-driver-abi.md §8` dispatch offset `0x5D` has a
                // real back-buffer body; selecting the hidden surface is
                // explicitly *not* a no-op.
                self.draw_fixed_glyph_cell(font, code, cell_x, cell_y, foreground, background)?;
                Ok(EgaDispatchResult::None)
            }
            EgaDisplayOperation::AdvanceTitleTick => {
                let rect = self.advance_title_tick();
                Ok(EgaDispatchResult::Rect(rect))
            }
            EgaDisplayOperation::SaveLoadedTileGraphics { atlas, saved } => {
                saved.save_from_atlas(atlas);
                Ok(EgaDispatchResult::None)
            }
            EgaDisplayOperation::RestoreLoadedTileGraphics { atlas, saved } => {
                saved.restore_to_atlas(atlas)?;
                Ok(EgaDispatchResult::None)
            }
            EgaDisplayOperation::SwapLoadedTileRedGreenPlanes { atlas, tile_range } => {
                swap_loaded_tile_red_green_planes(atlas, tile_range);
                Ok(EgaDispatchResult::None)
            }
            EgaDisplayOperation::PresentFrame => {
                self.present_frame();
                Ok(EgaDispatchResult::None)
            }
            EgaDisplayOperation::NoOp => Ok(EgaDispatchResult::None),
        }
    }
}

fn fill_pixels_rect(pixels: &mut [u8], rect: DisplayPixelRect, color: u8) {
    for y in rect.y0..=rect.y1 {
        let start = y * DISPLAY_SURFACE_WIDTH + rect.x0;
        pixels[start..=start + rect.width() - 1].fill(color);
    }
}

fn copy_rect_between_buffers(src: &[u8], dst: &mut [u8], rect: DisplayPixelRect) {
    for y in rect.y0..=rect.y1 {
        let start = y * DISPLAY_SURFACE_WIDTH + rect.x0;
        let end = start + rect.width();
        dst[start..end].copy_from_slice(&src[start..end]);
    }
}

fn assert_display_point_in_bounds(x: i32, y: i32, context: &str) {
    assert!(
        (0..DISPLAY_SURFACE_WIDTH as i32).contains(&x)
            && (0..DISPLAY_SURFACE_HEIGHT as i32).contains(&y),
        "{context} ({x}, {y}) exceeds {DISPLAY_SURFACE_WIDTH}x{DISPLAY_SURFACE_HEIGHT}; clipping is a forbidden fallback"
    );
}

#[cfg(test)]
mod red_green_plane_swap_tests {
    use super::*;

    /// `display-driver-abi.md §10` offset `0x6C` (`RETRACTIONS.md` R304):
    /// "The plane-swap mode covers tiles `0x05`, `0x1E`, `0x1F`, `0x4C`,
    /// `0xCA`, `0x20..0x26`, `0x30..0x37` and `0x60..0x6F`."
    #[test]
    fn the_plane_swap_mode_covers_exactly_the_published_tile_set() {
        let covered: Vec<u8> = (0u16..=0xFF)
            .map(|tile| tile as u8)
            .filter(|tile| ega_red_green_plane_swap_covers_tile(*tile))
            .collect();
        let mut published: Vec<u8> = EGA_RED_GREEN_PLANE_SWAP_TILES.to_vec();
        published.sort_unstable();
        assert_eq!(covered, published);
        assert_eq!(covered.len(), 36);

        // "It touches no water tile and no fire fixture directly."
        for water in crate::water_scroll::WATER_ROTATED_TILES {
            assert!(!ega_red_green_plane_swap_covers_tile(water));
        }
        for fire in [0xB0u8, 0xB1, 0xBF] {
            assert!(!ega_red_green_plane_swap_covers_tile(fire));
        }
        // "but `0x34..0x37` and `0x60..0x6F` are two of the three
        // destination groups of the per-step water composite … so the swap
        // and the water animation write to the same bitmaps and interact."
        for shared in [0x34u8, 0x35, 0x36, 0x37, 0x60, 0x6F] {
            assert!(ega_red_green_plane_swap_covers_tile(shared));
            assert!(crate::water_scroll::water_composite_mask(shared).is_some());
        }
    }

    #[test]
    fn the_published_set_swap_touches_only_covered_tiles() {
        let mut atlas = TileAtlas {
            depth: TileGraphicsDepth::Ega16,
            pixels: (0..256 * TILE_ATLAS_TILE_PIXELS)
                .map(|index| ((index % 16) as u8) & 0x0f)
                .collect(),
            dungeon_billboards: None,
            dungeon_sprites: None,
        };
        let before = atlas.pixels.clone();
        swap_loaded_tile_red_green_planes_over_published_set(&mut atlas);

        for tile in 0u16..=0xFF {
            let tile = tile as u8;
            let start = usize::from(tile) * TILE_ATLAS_TILE_PIXELS;
            let end = start + TILE_ATLAS_TILE_PIXELS;
            if end > atlas.pixels.len() {
                break;
            }
            let changed = atlas.pixels[start..end] != before[start..end];
            if ega_red_green_plane_swap_covers_tile(tile) {
                assert!(changed, "covered tile 0x{tile:02X} must be rewritten");
            } else {
                assert!(!changed, "tile 0x{tile:02X} is outside the published set");
            }
        }
    }
}
