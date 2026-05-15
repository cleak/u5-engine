//! Per-tile view-class lookup for the `V` View / Peer / gem-view overlay.
//! Spec: `systems/view.md` §4.
//!
//! The renderer sees the tile id after active-object/terrain lookup has
//! selected the cell to draw and reduces it to a small view-class byte.
//! Each class has a per-class renderer documented in §3 of the spec.
//!
//! This module produces only the class byte. The caller still picks a
//! concrete renderer based on whether peer/gem-view mode is active.

const VIEW_CLASS_DEFAULT: u8 = 0;

/// Per-tile view class per `view.md` §4. Tile ids not listed in the spec
/// table fall through to class 0 (empty/pass-through).
pub const fn tile_view_class(tile: u8) -> u8 {
    let t = tile as usize;
    match t {
        // Class 0
        0x00 => 0,
        0xC0..=0xC3 => 0,
        0xCC..=0xCF => 0,
        0xFF => 0,
        // Class 1
        0x05 => 1,
        0x30..=0x37 => 1,
        // Class 2
        0x09 | 0x0A => 2,
        0x2D => 2,
        // Class 3
        0x07 => 3,
        0x1C => 3,
        0x1E | 0x1F => 3,
        0x40 => 3,
        0x44 => 3,
        0x48 | 0x49 => 3,
        0x6A | 0x6B => 3,
        0x70..=0x7F => 3,
        0x87 => 3,
        0x8C => 3,
        0x8F => 3,
        0xAA => 3,
        0xBC => 3,
        0xDD => 3,
        // Class 4
        0x1D => 4,
        0x38 => 4,
        0x47 => 4,
        0x5A => 4,
        0x5C | 0x5D => 4,
        0x94..=0x96 => 4,
        0x9A..=0x9C => 4,
        0xAB | 0xAC => 4,
        0xBE => 4,
        // Class 5
        0x10..=0x1B => 5,
        0x29..=0x2B => 5,
        0x2E | 0x2F => 5,
        0x41..=0x43 => 5,
        0x4C => 5,
        0x58 | 0x59 => 5,
        0x5B => 5,
        0x5E | 0x5F => 5,
        0x80..=0x85 => 5,
        0x88..=0x8B => 5,
        0x8D | 0x8E => 5,
        0x90..=0x93 => 5,
        0x9D..=0xA9 => 5,
        0xAD..=0xB7 => 5,
        0xBD => 5,
        0xBF => 5,
        0xC8..=0xCB => 5,
        0xDE | 0xDF => 5,
        0xE8..=0xEB => 5,
        0xFA..=0xFD => 5,
        // Class 6
        0x0D => 6,
        0x45 => 6,
        0x4A | 0x4B => 6,
        0x86 => 6,
        0x97..=0x99 => 6,
        0xB8..=0xBB => 6,
        0xC4..=0xC7 => 6,
        0xEC..=0xF9 => 6,
        // Class 7
        0x0C => 7,
        0x27 | 0x28 => 7,
        0x39..=0x3F => 7,
        0x46 => 7,
        0x4D..=0x57 => 7,
        0xD0..=0xD3 => 7,
        0xFE => 7,
        // Class 8
        0x0B => 8,
        0x0E | 0x0F => 8,
        // Class 9
        0x06 => 9,
        0x08 => 9,
        0x2C => 9,
        // Class A
        0x03 => 0x0A,
        0x60..=0x69 => 0x0A,
        0x6C..=0x6F => 0x0A,
        0xE4..=0xE7 => 0x0A,
        // Class B
        0x02 => 0x0B,
        0xD4..=0xD7 => 0x0B,
        // Class C
        0x01 => 0x0C,
        // Class D
        0x04 => 0x0D,
        // Class E
        0xE0..=0xE3 => 0x0E,
        // Class F
        0xD8..=0xDC => 0x0F,
        // Class 0x10
        0x20..=0x26 => 0x10,
        _ => VIEW_CLASS_DEFAULT,
    }
}
