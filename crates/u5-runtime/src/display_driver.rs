//! Clean semantic display-driver surface for the EGA-compatible v1 renderer.

use std::io;

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
pub struct EgaDisplaySurface {
    front_pixels: Vec<u8>,
    back_pixels: Vec<u8>,
    current_color: u8,
    title_tick_frame: u8,
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
            presented_frames: 0,
            render_target: DisplayRenderTarget::Front,
            back_buffer_active: false,
        }
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
            return;
        }
        self.front_pixels[y * DISPLAY_SURFACE_WIDTH + x] = self.current_color;
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

    pub fn dissolve_back_to_front_rect(&mut self, rect: DisplayPixelRect) {
        copy_rect_between_buffers(&self.back_pixels, &mut self.front_pixels, rect);
    }

    pub fn blit_tile_at_pixel(
        &mut self,
        atlas: &TileAtlas,
        tile: usize,
        dst_x: i32,
        dst_y: i32,
    ) -> io::Result<()> {
        let pixels = atlas.tile_pixels(tile).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("tile atlas is missing tile {tile}"),
            )
        })?;
        for row in 0..TILE_ATLAS_SIDE {
            let y = dst_y + row as i32;
            if !(0..DISPLAY_SURFACE_HEIGHT as i32).contains(&y) {
                continue;
            }
            for col in 0..TILE_ATLAS_SIDE {
                let x = dst_x + col as i32;
                if !(0..DISPLAY_SURFACE_WIDTH as i32).contains(&x) {
                    continue;
                }
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
                if x < DISPLAY_SURFACE_WIDTH && y < DISPLAY_SURFACE_HEIGHT {
                    self.front_pixels[y * DISPLAY_SURFACE_WIDTH + x] = color;
                }
            }
        }
        Ok(())
    }

    pub fn advance_title_tick(&mut self) -> DisplayPixelRect {
        let rect = normalize_clamp_pixel_rect(
            i32::from(TITLE_TICK_FRAME_X),
            i32::from(TITLE_TICK_FRAME_Y),
            i32::from(TITLE_TICK_FRAME_X + TITLE_TICK_FRAME_WIDTH - 1),
            i32::from(TITLE_TICK_FRAME_Y + TITLE_TICK_FRAME_HEIGHT - 1),
        )
        .expect("title tick rectangle is inside the display surface");
        for y in rect.y0..=rect.y1 {
            let local_y = y - rect.y0;
            for x in rect.x0..=rect.x1 {
                let index = y * DISPLAY_SURFACE_WIDTH + x;
                if self.front_pixels[index] != 0 {
                    continue;
                }
                if let Some(color) =
                    title_tick_flame_palette_index(x - rect.x0, local_y, self.title_tick_frame)
                {
                    self.front_pixels[index] = color;
                }
            }
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
                Ok(EgaDispatchResult::Pixel(self.read_pixel(x, y).unwrap_or(0)))
            }
            EgaDisplayOperation::PlotPixel { x, y } => {
                if self.render_target == DisplayRenderTarget::Front {
                    self.plot_pixel(x, y);
                } else if x < DISPLAY_SURFACE_WIDTH && y < DISPLAY_SURFACE_HEIGHT {
                    self.back_pixels[y * DISPLAY_SURFACE_WIDTH + x] = self.current_color;
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
