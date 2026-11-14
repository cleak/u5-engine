//! A slot's thumbnail (Phase B design 3.3.5 step 4; plan M4-10).
//!
//! The picture is the game's own 320x200 frame at the save moment
//! (`VisualState.image_handle`), which no enhancement ever draws on: never
//! the grow-left canvas, never the menu or a toast, so it shows only what
//! the player saw (1.4.7). It is box filtered 4x, each 4x4 block of pixels
//! averaged to one, to 80x50, and PNG-encoded.

use bevy::prelude::*;

use crate::{VISUAL_PLAY_FRAME_HEIGHT, VISUAL_PLAY_FRAME_WIDTH};

/// The filter's factor.
pub const THUMB_FACTOR: u32 = 4;
/// The thumbnail's size: the frame's over [`THUMB_FACTOR`].
pub const THUMB_WIDTH: u32 = VISUAL_PLAY_FRAME_WIDTH / THUMB_FACTOR;
pub const THUMB_HEIGHT: u32 = VISUAL_PLAY_FRAME_HEIGHT / THUMB_FACTOR;

/// Box filters a 320x200 RGBA frame to 80x50: each channel of each output
/// pixel is the rounded mean of its 4x4 block. Alpha is opaque. `None` for
/// a buffer of another size.
pub fn box_filter(rgba: &[u8]) -> Option<Vec<u8>> {
    let (w, h) = (
        VISUAL_PLAY_FRAME_WIDTH as usize,
        VISUAL_PLAY_FRAME_HEIGHT as usize,
    );
    if rgba.len() != w * h * 4 {
        return None;
    }
    let f = THUMB_FACTOR as usize;
    let (tw, th) = (THUMB_WIDTH as usize, THUMB_HEIGHT as usize);
    let block = (f * f) as u32;
    let mut out = Vec::with_capacity(tw * th * 4);
    for ty in 0..th {
        for tx in 0..tw {
            let mut sum = [0u32; 3];
            for y in ty * f..ty * f + f {
                let row = y * w * 4;
                for x in tx * f..tx * f + f {
                    let at = row + x * 4;
                    for (channel, total) in sum.iter_mut().enumerate() {
                        *total += u32::from(rgba[at + channel]);
                    }
                }
            }
            for total in sum {
                out.push(((total + block / 2) / block) as u8);
            }
            out.push(0xff);
        }
    }
    Some(out)
}

/// The PNG bytes of an 80x50 RGBA thumbnail.
pub fn encode_png(thumb: &[u8]) -> Option<Vec<u8>> {
    let image = image::RgbaImage::from_raw(THUMB_WIDTH, THUMB_HEIGHT, thumb.to_vec())?;
    let mut bytes = std::io::Cursor::new(Vec::new());
    image.write_to(&mut bytes, image::ImageFormat::Png).ok()?;
    Some(bytes.into_inner())
}

/// The thumbnail of the game's frame `frame`, PNG-encoded: `None` if the
/// image holds no 320x200 RGBA data.
pub fn frame_thumbnail(frame: &Image) -> Option<Vec<u8>> {
    let size = frame.size();
    if size != UVec2::new(VISUAL_PLAY_FRAME_WIDTH, VISUAL_PLAY_FRAME_HEIGHT) {
        return None;
    }
    encode_png(&box_filter(frame.data.as_deref()?)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_block_is_its_rounded_mean() {
        let (w, h) = (
            VISUAL_PLAY_FRAME_WIDTH as usize,
            VISUAL_PLAY_FRAME_HEIGHT as usize,
        );
        let mut rgba = vec![0u8; w * h * 4];
        // The first block: one white pixel of sixteen, 255/16 = 15.94.
        rgba[..4].copy_from_slice(&[255, 255, 255, 255]);
        // The second block: a half-and-half brown and black split.
        for y in 0..4 {
            for x in 4..6 {
                let at = (y * w + x) * 4;
                rgba[at..at + 4].copy_from_slice(&[0xaa, 0x55, 0x00, 0xff]);
            }
        }
        let thumb = box_filter(&rgba).unwrap();
        assert_eq!(thumb.len(), (THUMB_WIDTH * THUMB_HEIGHT * 4) as usize);
        assert_eq!(&thumb[..4], &[16, 16, 16, 0xff]);
        assert_eq!(&thumb[4..8], &[0x55, 0x2b, 0x00, 0xff]);
        assert!(thumb[8..].chunks(4).all(|px| px == [0, 0, 0, 0xff]));
        assert_eq!(box_filter(&rgba[4..]), None, "wrong size");
    }

    #[test]
    fn the_png_decodes_to_the_filtered_pixels() {
        let rgba: Vec<u8> = (0..VISUAL_PLAY_FRAME_WIDTH * VISUAL_PLAY_FRAME_HEIGHT * 4)
            .map(|i| (i % 251) as u8)
            .collect();
        let thumb = box_filter(&rgba).unwrap();
        let png = encode_png(&thumb).unwrap();
        let decoded = image::load_from_memory_with_format(&png, image::ImageFormat::Png)
            .unwrap()
            .to_rgba8();
        assert_eq!(decoded.dimensions(), (THUMB_WIDTH, THUMB_HEIGHT));
        assert_eq!(decoded.into_raw(), thumb);
    }
}
