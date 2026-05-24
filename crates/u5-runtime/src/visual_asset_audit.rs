//! Sanitized aggregate audits for fixed visual resources.
//!
//! This intentionally covers tile atlases and fixed-cell fonts only. Sparse
//! `.BIT` resources and `PROPORT.PCS` stay out of this audit while the public
//! disk-envelope clarification is pending. Reports contain counts, masks, and
//! hashes only; they do not emit raw pixels or glyph rows.

use std::io;
use std::path::Path;

use crate::*;

pub const VISUAL_ASSET_AUDIT_KIND_COUNT: usize = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VisualAssetAuditKind {
    TileAtlas,
    FixedFont,
}

impl VisualAssetAuditKind {
    pub const fn index(self) -> usize {
        match self {
            Self::TileAtlas => 0,
            Self::FixedFont => 1,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::TileAtlas => "tile-atlas",
            Self::FixedFont => "fixed-font",
        }
    }

    pub const ALL: [Self; VISUAL_ASSET_AUDIT_KIND_COUNT] = [Self::TileAtlas, Self::FixedFont];
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VisualAssetAuditEntry {
    pub resource_name: &'static str,
    pub kind: VisualAssetAuditKind,
    pub depth: Option<TileGraphicsDepth>,
    pub item_count: usize,
    pub cell_width: usize,
    pub cell_height: usize,
    pub total_pixels: usize,
    pub nonzero_pixels: usize,
    pub max_value: u8,
    pub value_mask: u32,
    pub content_hash: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VisualAssetAuditReport {
    pub entries: Vec<VisualAssetAuditEntry>,
    pub kind_counts: [usize; VISUAL_ASSET_AUDIT_KIND_COUNT],
    pub total_items: usize,
    pub total_pixels: usize,
    pub nonzero_pixels: usize,
    pub content_hash: u64,
}

pub fn audit_visual_assets(game_dir: &Path) -> io::Result<VisualAssetAuditReport> {
    let mut entries = Vec::new();

    for depth in [TileGraphicsDepth::Ega16, TileGraphicsDepth::Cga4] {
        let atlas = load_tile_atlas(game_dir, depth)?;
        entries.push(audit_tile_atlas_resource(depth.file_name(), &atlas));
    }

    for file_name in [IBM_CH_FILE, RUNES_CH_FILE] {
        let font = load_ch_font(game_dir, file_name)?;
        entries.push(audit_fixed_font_resource(file_name, &font));
    }
    for file_name in [IBM_HCS_FILE, RUNES_HCS_FILE] {
        let font = load_hcs_font(game_dir, file_name)?;
        entries.push(audit_fixed_font_resource(file_name, &font));
    }

    Ok(visual_asset_audit_report_from_entries(entries))
}

pub fn visual_asset_audit_report_from_entries(
    entries: Vec<VisualAssetAuditEntry>,
) -> VisualAssetAuditReport {
    let mut report = VisualAssetAuditReport {
        entries: Vec::new(),
        kind_counts: [0; VISUAL_ASSET_AUDIT_KIND_COUNT],
        total_items: 0,
        total_pixels: 0,
        nonzero_pixels: 0,
        content_hash: 0xcbf29ce484222325,
    };
    for entry in entries {
        merge_visual_asset_entry(&mut report, entry);
    }
    report
}

pub fn audit_tile_atlas_resource(
    resource_name: &'static str,
    atlas: &TileAtlas,
) -> VisualAssetAuditEntry {
    let (nonzero_pixels, max_value, value_mask) = audit_pixel_values(&atlas.pixels);
    VisualAssetAuditEntry {
        resource_name,
        kind: VisualAssetAuditKind::TileAtlas,
        depth: Some(atlas.depth),
        item_count: TILE_ATLAS_TILE_COUNT,
        cell_width: TILE_ATLAS_SIDE,
        cell_height: TILE_ATLAS_SIDE,
        total_pixels: atlas.pixels.len(),
        nonzero_pixels,
        max_value,
        value_mask,
        content_hash: hash_bytes(&atlas.pixels),
    }
}

pub fn audit_fixed_font_resource(
    resource_name: &'static str,
    font: &FixedFont,
) -> VisualAssetAuditEntry {
    let mut pixels = Vec::with_capacity(
        font.glyphs
            .len()
            .saturating_mul(font.cell_width)
            .saturating_mul(font.cell_height),
    );
    for glyph in &font.glyphs {
        pixels.extend_from_slice(&glyph.pixels);
    }
    let (nonzero_pixels, max_value, value_mask) = audit_pixel_values(&pixels);
    VisualAssetAuditEntry {
        resource_name,
        kind: VisualAssetAuditKind::FixedFont,
        depth: None,
        item_count: font.glyphs.len(),
        cell_width: font.cell_width,
        cell_height: font.cell_height,
        total_pixels: pixels.len(),
        nonzero_pixels,
        max_value,
        value_mask,
        content_hash: hash_bytes(&pixels),
    }
}

pub fn visual_asset_audit_report_text(report: &VisualAssetAuditReport) -> String {
    let mut text = String::new();
    text.push_str("Ultima V visual asset audit\n");
    text.push_str(&format!(
        "resources={} items={} pixels={} nonzero={} hash={:016x}\n",
        report.entries.len(),
        report.total_items,
        report.total_pixels,
        report.nonzero_pixels,
        report.content_hash
    ));
    text.push_str("kinds:");
    for kind in VisualAssetAuditKind::ALL {
        let count = report.kind_counts[kind.index()];
        if count > 0 {
            text.push_str(&format!(" {}={count}", kind.label()));
        }
    }
    text.push('\n');
    text.push_str("resources:");
    for entry in &report.entries {
        let depth = entry.depth.map(TileGraphicsDepth::label).unwrap_or("mono");
        text.push_str(&format!(
            " {}({}:{}x{}x{} nz={} mask={:#x} hash={:016x})",
            entry.resource_name,
            depth,
            entry.item_count,
            entry.cell_width,
            entry.cell_height,
            entry.nonzero_pixels,
            entry.value_mask,
            entry.content_hash
        ));
    }
    text.push('\n');
    text
}

fn merge_visual_asset_entry(report: &mut VisualAssetAuditReport, entry: VisualAssetAuditEntry) {
    report.kind_counts[entry.kind.index()] += 1;
    report.total_items += entry.item_count;
    report.total_pixels += entry.total_pixels;
    report.nonzero_pixels += entry.nonzero_pixels;
    report.content_hash ^= entry.content_hash;
    report.content_hash = report.content_hash.wrapping_mul(0x100000001b3);
    report.entries.push(entry);
}

fn audit_pixel_values(pixels: &[u8]) -> (usize, u8, u32) {
    let mut nonzero_pixels = 0;
    let mut max_value = 0;
    let mut value_mask = 0u32;
    for pixel in pixels {
        if *pixel != 0 {
            nonzero_pixels += 1;
        }
        max_value = max_value.max(*pixel);
        if *pixel < 32 {
            value_mask |= 1u32 << *pixel;
        }
    }
    (nonzero_pixels, max_value, value_mask)
}
