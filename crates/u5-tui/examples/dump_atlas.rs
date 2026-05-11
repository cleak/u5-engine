//! Render every tile in the TILES.16 atlas into a single 16x32 grid PNG.
//! Each tile is 16x16 px; the output is a 256x512 atlas dump.
//! Usage: cargo run --release --example dump_atlas -- <game_dir>

use std::path::Path;

use image::{ImageBuffer, Rgba};
use u5_runtime::*;

fn main() {
    let mut args = std::env::args().skip(1);
    let dir = args
        .next()
        .unwrap_or_else(|| String::from(r"C:\Games\U5-Clean"));
    let dir = Path::new(&dir);

    let atlas = load_tile_atlas(dir, TileGraphicsDepth::Ega16).unwrap();
    let palette = &EGA_PALETTE_RGB;

    // Layout: 16 columns x 32 rows = 512 tiles, each 16x16 px.
    let cols = 16u32;
    let rows = (TILE_ATLAS_TILE_COUNT as u32).div_ceil(cols);
    let side = TILE_ATLAS_SIDE as u32;
    let w = cols * side;
    let h = rows * side;

    let mut img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::new(w, h);
    for tile_id in 0..TILE_ATLAS_TILE_COUNT {
        let col = (tile_id as u32) % cols;
        let row = (tile_id as u32) / cols;
        let base = tile_id * TILE_ATLAS_TILE_PIXELS;
        for ty in 0..side {
            for tx in 0..side {
                let px = base + (ty as usize) * (side as usize) + (tx as usize);
                let idx = atlas.pixels[px] as usize;
                let [r, g, b] = palette[idx.min(palette.len() - 1)];
                img.put_pixel(col * side + tx, row * side + ty, Rgba([r, g, b, 0xff]));
            }
        }
    }
    let out = Path::new("screenshots/atlas_ega.png");
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    img.save(out).unwrap();
    println!(
        "Saved {}x{} atlas dump ({} tiles, 16x16 each) to {}",
        w,
        h,
        TILE_ATLAS_TILE_COUNT,
        out.display()
    );
}
