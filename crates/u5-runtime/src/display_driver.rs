//! Clean semantic display-driver surface for the EGA-compatible v1 renderer.

use std::io;
use std::ops::Range;

use crate::*;

pub const DISPLAY_SURFACE_WIDTH: usize = TITLE_SURFACE_WIDTH as usize;
pub const DISPLAY_SURFACE_HEIGHT: usize = TITLE_SURFACE_HEIGHT as usize;
pub const DISPLAY_SURFACE_PIXELS: usize = DISPLAY_SURFACE_WIDTH * DISPLAY_SURFACE_HEIGHT;
pub const DISPLAY_TEXT_COLUMNS: usize = TEXT_SCREEN_COLUMNS as usize;
pub const DISPLAY_TEXT_ROWS: usize = TEXT_SCREEN_ROWS as usize;
pub const EGA_DRIVER_SLOT_COUNT: u8 = 38;
pub const EGA_DRIVER_LAST_DISPATCH_OFFSET: u8 = (EGA_DRIVER_SLOT_COUNT - 1) * 3;

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
    ScrollTextUp {
        rect: DisplayPixelRect,
        blank_color: u8,
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EgaDissolveState {
    rect: DisplayPixelRect,
    cursor: usize,
    total: usize,
    stride: usize,
    offset: usize,
}

impl EgaDissolveState {
    pub fn new(rect: DisplayPixelRect) -> Self {
        let total = rect.width() * rect.height();
        Self {
            rect,
            cursor: 0,
            total,
            stride: dissolve_stride(total),
            offset: total / 2,
        }
    }

    pub const fn rect(&self) -> DisplayPixelRect {
        self.rect
    }

    pub const fn total_pixels(&self) -> usize {
        self.total
    }

    pub const fn copied_pixels(&self) -> usize {
        self.cursor
    }

    pub const fn remaining_pixels(&self) -> usize {
        self.total.saturating_sub(self.cursor)
    }

    pub const fn is_finished(&self) -> bool {
        self.cursor >= self.total
    }

    pub fn next_pixel(&mut self) -> Option<(usize, usize)> {
        if self.is_finished() {
            return None;
        }
        let visit = (self
            .cursor
            .saturating_mul(self.stride)
            .saturating_add(self.offset))
            % self.total;
        self.cursor += 1;
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

    pub fn fill_rect_current_color(&mut self, rect: DisplayPixelRect) {
        self.fill_rect(rect, self.current_color);
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

    pub fn scroll_rect(&mut self, rect: DisplayPixelRect, dx: i32, dy: i32, blank_color: u8) {
        let original = self.front_pixels.clone();
        self.fill_rect(rect, blank_color);
        for y in rect.y0..=rect.y1 {
            for x in rect.x0..=rect.x1 {
                let src_x = x as i32 - dx;
                let src_y = y as i32 - dy;
                if src_x < rect.x0 as i32
                    || src_x > rect.x1 as i32
                    || src_y < rect.y0 as i32
                    || src_y > rect.y1 as i32
                {
                    continue;
                }
                let src = src_y as usize * DISPLAY_SURFACE_WIDTH + src_x as usize;
                let dst = y * DISPLAY_SURFACE_WIDTH + x;
                self.front_pixels[dst] = original[src];
            }
        }
    }

    pub fn scroll_text_rect_up_one_row(&mut self, rect: DisplayPixelRect, blank_color: u8) {
        self.scroll_rect(rect, 0, -(CH_CELL_SIDE as i32), blank_color);
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
        for row in 0..TILE_ATLAS_SIDE {
            let y = dst_y + row as i32;
            for col in 0..TILE_ATLAS_SIDE {
                let x = dst_x + col as i32;
                self.front_pixels[y as usize * DISPLAY_SURFACE_WIDTH + x as usize] =
                    pixels[row * TILE_ATLAS_SIDE + col] & 0x0f;
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
        for glyph_y in 0..CH_CELL_SIDE {
            let row_bits = font.glyph_row(code & 0x7f, glyph_y).ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("fixed font glyph {} is missing row {glyph_y}", code & 0x7f),
                )
            })?;
            for glyph_x in 0..CH_CELL_SIDE {
                let color = if row_bits & (1 << (7 - glyph_x)) != 0 {
                    foreground
                } else {
                    background
                } & 0x0f;
                let x = dst_x + glyph_x;
                let y = dst_y + glyph_y;
                self.front_pixels[y * DISPLAY_SURFACE_WIDTH + x] = color;
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
        // `cleak/u5-spec#78`: the source band is 288 pixels wide and
        // lands at x = 16 inside the published 320-wide destination
        // rectangle. The 16 columns at each side are cleared to index
        // 0 so the tick still overwrites the whole rectangle opaquely.
        let source_x = rect.x0 + TITLE_TICK_SOURCE_X as usize;
        let source_width = TITLE_TICK_SOURCE_WIDTH as usize;
        for (row, src) in self
            .title_tick_frames
            .as_ref()
            .expect(
                "display title-tick operation requires the ULTIMA title-tick panels to be injected; generated clean-room frames are a forbidden fallback; see cleak/u5-spec#78",
            )
            .frame_pixels(self.title_tick_frame)
            .chunks_exact(source_width)
            .enumerate()
        {
            let row_start = (rect.y0 + row) * DISPLAY_SURFACE_WIDTH;
            self.front_pixels[row_start + rect.x0..row_start + source_x].fill(0);
            self.front_pixels[row_start + source_x..row_start + source_x + source_width]
                .copy_from_slice(src);
            self.front_pixels[row_start + source_x + source_width..row_start + rect.x1 + 1].fill(0);
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
                if self.render_target == DisplayRenderTarget::Front {
                    self.draw_line(x0, y0, x1, y1);
                }
                Ok(EgaDispatchResult::None)
            }
            EgaDisplayOperation::ScrollTextUp { rect, blank_color } => {
                if self.render_target == DisplayRenderTarget::Front {
                    self.scroll_text_rect_up_one_row(rect, blank_color);
                }
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
                if self.render_target == DisplayRenderTarget::Front {
                    self.fill_rect_current_color(rect);
                }
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
                if self.render_target == DisplayRenderTarget::Front {
                    self.blit_tile_at_pixel(atlas, tile, dst_x, dst_y)?;
                }
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
                if self.render_target == DisplayRenderTarget::Front {
                    self.draw_fixed_glyph_cell(font, code, cell_x, cell_y, foreground, background)?;
                }
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

fn dissolve_stride(total: usize) -> usize {
    [521, 257, 131, 73, 37, 17, 5, 1]
        .into_iter()
        .find(|candidate| *candidate < total && gcd(*candidate, total) == 1)
        .unwrap_or(1)
}

fn gcd(mut a: usize, mut b: usize) -> usize {
    while b != 0 {
        let rem = a % b;
        a = b;
        b = rem;
    }
    a
}
