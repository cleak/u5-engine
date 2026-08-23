//! The dungeon corridor's first-person billboard renderer geometry.
//!
//! # Provenance
//!
//! `systems/dungeon-mode.md` sections 6.1 to 6.5, published as
//! `cleak/u5-spec#81` and fetched from the public spec repository.
//!
//! An earlier revision of that section described the corridor as a
//! *sparse* renderer plotting precomputed pixel constellations from
//! four coordinate-pair tables. **That reading is withdrawn**, and the
//! wireframe this module replaces was built from it. The corridor is
//! drawn from billboard bitmaps held in the dungeon art files, which is
//! why the shipped game shows a textured brick corridor rather than an
//! outline of dots. The four sparse tables are real, but they are the
//! animated fountain water of section 6.7.
//!
//! It is not a raycaster either: no projection arithmetic and no depth
//! buffer. Each cell is classified, a bitmap is selected by role and
//! depth band, blitted at a fixed destination, and painter's-algorithm
//! ordering does the rest. Depth shading is baked into the artwork.

use crate::graphics::GraphicImageDirectory;
use crate::scene::DungeonPresentationFlavour;

/// Per-band half-aperture. Every placement constant falls out of this
/// one sequence: the view is a nest of concentric squares centred on
/// the vanishing point, and the opening at band `b` lies `hw[b]` pixels
/// either side of it in both axes.
///
/// Band 0 is the party's **own** cell, not the cell ahead.
pub const DUNGEON_HALF_APERTURE: [i32; 4] = [80, 56, 24, 8];

/// Number of depth bands the sweep covers.
pub const DUNGEON_BANDS: usize = 4;

/// Vanishing point, the centre of the visible art, in screen pixels.
pub const DUNGEON_VANISHING_X: i32 = 96;
/// See [`DUNGEON_VANISHING_X`].
pub const DUNGEON_VANISHING_Y: i32 = 96;

/// Fixed vertical origin every billboard is drawn at, in screen pixels.
/// Horizontal placement is the only per-image variable.
pub const DUNGEON_BILLBOARD_ORIGIN_Y: i32 = 14;
/// Every billboard is this many rows tall.
pub const DUNGEON_BILLBOARD_ROWS: usize = 164;

/// The reflection constant in the placement rule
/// `x_right = 192 - x_left - width`, which mirrors the left rectangle
/// about the vertical centre line at x = 95.5.
pub const DUNGEON_MIRROR_SPAN: i32 = 192;

/// Directory slot count in each corridor art bank.
pub const DUNGEON_BILLBOARD_SLOTS: usize = 28;

/// The role a corridor billboard plays.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DungeonBillboardRole {
    /// Plain wall to the side.
    SideWall,
    /// Door to the side.
    SideDoor,
    /// See-through cell to the side.
    SideOpening,
    /// Decorated wall to the side.
    SideFlavourWall,
    /// Plain wall ahead.
    ForwardWall,
    /// Door ahead. Its band-0 entry is the 80-wide point-blank image
    /// that stands in for every blocker family at that range.
    ForwardDoor,
    /// Decorated wall ahead.
    ForwardFlavourWall,
}

impl DungeonBillboardRole {
    /// Whether this role's width follows the side rule
    /// (`hw[b] - hw[b+1]`) rather than the forward rule (`hw[b]`).
    pub const fn is_side(self) -> bool {
        matches!(
            self,
            Self::SideWall | Self::SideDoor | Self::SideOpening | Self::SideFlavourWall
        )
    }

    /// First directory slot of this role's family.
    ///
    /// # Provenance
    ///
    /// Published in full by `cleak/u5-spec#84`. The addressing rule is
    /// `slot = family_base + band`, so the low two bits of a slot index
    /// *are* the depth band and the renderer builds every index by
    /// adding the band to a per-role constant - which is why this is
    /// arithmetic rather than a 28-entry lookup table.
    ///
    /// Depth is always the band index inside a family, never a separate
    /// family: there is **no far or end wall role**, and the deepest
    /// thing visible is just a family's 8-wide band-3 image. There is
    /// **no forward opening role** either, because a non-blocking
    /// forward cell paints nothing and the sweep simply advances. That
    /// is why forward has three families where side has four: a passage
    /// mouth is visible in the side panel and needs an explicit image.
    ///
    /// The role **names** are descriptive labels chosen for what the art
    /// shows and what selects it; the original carries no labels. The
    /// slot indices and the selection conditions in [`dungeon_side_role`]
    /// and [`dungeon_forward_outcome`] are the contract - do not let a
    /// name imply behaviour the contract does not state.
    ///
    /// These seven assignments were first derived here by decoding
    /// `DNG1.16` and reading the art, before the table was published;
    /// all seven matched, including the two the published data already
    /// pinned by width.
    pub const fn family_base_slot(self) -> usize {
        match self {
            Self::SideWall => 0,
            Self::SideDoor => 4,
            Self::ForwardWall => 8,
            Self::ForwardDoor => 12,
            Self::SideOpening => 16,
            // The decorated families are their plain counterparts with
            // scenery composited on top, which is measurable: see the
            // pixel-difference self-check in the tests.
            Self::SideFlavourWall => 20,
            Self::ForwardFlavourWall => 24,
        }
    }

    /// Directory slot for this role at `band`, or `None` for the two
    /// deliberately absent entries, the band-0 forward wall and forward
    /// flavour wall.
    ///
    /// Those entries were never *needed* rather than merely overridden:
    /// band 0 is the party's own cell, and the party cannot stand inside
    /// a plain or flavour wall, so the only blocker that ever occurs
    /// underfoot is a door. Slot 12 therefore only ever has to depict a
    /// doorway, which is what the art shows in all three files - a
    /// full-height frame around an unlit interior. Treat a band-0
    /// forward blocker as a single special case and never consult the
    /// family table there; [`dungeon_forward_outcome`] does.
    pub const fn slot(self, band: usize) -> Option<usize> {
        if band >= DUNGEON_BANDS {
            return None;
        }
        if band == 0 && matches!(self, Self::ForwardWall | Self::ForwardFlavourWall) {
            return None;
        }
        Some(self.family_base_slot() + band)
    }
}

/// Width of a billboard at `band`.
///
/// Two self-checking invariants: a side image's width is the thickness
/// of the ring between its own band's aperture and the next
/// (`hw[b] - hw[b+1]`), and a forward image's width is `hw[b]` itself,
/// because each forward billboard is a half wall drawn twice.
pub const fn dungeon_billboard_width(role: DungeonBillboardRole, band: usize) -> Option<i32> {
    if band >= DUNGEON_BANDS {
        return None;
    }
    let here = DUNGEON_HALF_APERTURE[band];
    if role.is_side() {
        let next = if band + 1 < DUNGEON_BANDS {
            DUNGEON_HALF_APERTURE[band + 1]
        } else {
            0
        };
        Some(here - next)
    } else {
        Some(here)
    }
}

/// Left-hand destination x for any billboard at `band`, in screen
/// pixels. Both families share it.
pub const fn dungeon_billboard_left_x(band: usize) -> i32 {
    DUNGEON_VANISHING_X - DUNGEON_HALF_APERTURE[band]
}

/// Mirrored destination x: `x_right = 192 - x_left - width`.
pub const fn dungeon_billboard_right_x(left_x: i32, width: i32) -> i32 {
    DUNGEON_MIRROR_SPAN - left_x - width
}

/// Image family for a side cell, chosen from its high nibble.
///
/// Every class below the door families selects the *opening* image -
/// including the unused `0x9?` class, which is not a wall.
pub fn dungeon_side_role(cell: u8) -> DungeonBillboardRole {
    match cell >> 4 {
        0x0..=0x9 => DungeonBillboardRole::SideOpening,
        0xa | 0xe | 0xf => DungeonBillboardRole::SideDoor,
        0xc => DungeonBillboardRole::SideFlavourWall,
        _ => DungeonBillboardRole::SideWall,
    }
}

/// What the forward test reports for a cell at `band`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DungeonForwardOutcome {
    /// Blocker to paint twice, if any.
    pub blocker: Option<DungeonBillboardRole>,
    /// Whether the sweep continues past this band.
    pub see_through: bool,
    /// Whether the band-0 side cells are suppressed, so a point-blank
    /// door frame is not boxed in.
    pub point_blank: bool,
}

/// Run the forward test for `cell` at `band`.
///
/// Any cell below the door families is see-through. At band 0 every
/// blocker family uses the single point-blank image whatever its class,
/// which is why the two band-0 forward entries do not exist. A `0xE?`
/// heavy door in the party's own cell paints that image and reports
/// see-through anyway, so the doorway shows depth behind it; a `0xF?`
/// room trigger has no such pass-through.
pub fn dungeon_forward_outcome(cell: u8, band: usize) -> DungeonForwardOutcome {
    let nibble = cell >> 4;
    if nibble < 0xa {
        return DungeonForwardOutcome {
            blocker: None,
            see_through: true,
            point_blank: false,
        };
    }
    if band == 0 {
        return DungeonForwardOutcome {
            blocker: Some(DungeonBillboardRole::ForwardDoor),
            see_through: nibble == 0xe,
            point_blank: nibble == 0xe,
        };
    }
    let blocker = match nibble {
        0xa | 0xe | 0xf => DungeonBillboardRole::ForwardDoor,
        0xc => DungeonBillboardRole::ForwardFlavourWall,
        _ => DungeonBillboardRole::ForwardWall,
    };
    DungeonForwardOutcome {
        blocker: Some(blocker),
        see_through: false,
        point_blank: false,
    }
}

/// Art file stem for a presentation flavour. The three corridor files
/// have byte-identical directories; only the pixels differ, which is
/// the whole of the "different dungeons look different" mechanism.
/// The table is indexed by the **flavour byte**, not by the order the
/// flavours happen to be declared in: byte 1 selects the first file,
/// byte 2 the second, byte 3 the third. `FlavourByte3` is named for its
/// byte and therefore takes `DNG3`; the mine flavour is byte 2 and takes
/// `DNG2`.
///
/// The art corroborates this binding independently of any table:
/// `DNG1` is natural rock cavern (Normal), `DNG2` is a timbered mine
/// shaft with support beams (Mine), and `DNG3` is dressed masonry -
/// which is the grey stone the lit Deceit corridor shows.
pub const fn dungeon_billboard_stem(flavour: DungeonPresentationFlavour) -> &'static str {
    match flavour {
        DungeonPresentationFlavour::Normal => "DNG1",
        DungeonPresentationFlavour::Mine => "DNG2",
        DungeonPresentationFlavour::FlavourByte3 => "DNG3",
    }
}

/// A loaded corridor art bank.
pub type DungeonBillboardBank = GraphicImageDirectory;

/// The three loaded corridor banks, indexed by flavour.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DungeonBillboardBanks {
    normal: DungeonBillboardBank,
    mine: DungeonBillboardBank,
    flavour_byte_3: DungeonBillboardBank,
}

impl DungeonBillboardBanks {
    /// Bank for a presentation flavour.
    pub fn bank(&self, flavour: DungeonPresentationFlavour) -> &DungeonBillboardBank {
        match flavour {
            DungeonPresentationFlavour::Normal => &self.normal,
            DungeonPresentationFlavour::Mine => &self.mine,
            DungeonPresentationFlavour::FlavourByte3 => &self.flavour_byte_3,
        }
    }
}

/// Load the three corridor art banks from `game_dir`.
pub fn load_dungeon_billboard_banks(
    game_dir: &std::path::Path,
    depth: crate::graphics::TileGraphicsDepth,
) -> std::io::Result<DungeonBillboardBanks> {
    let load = |stem: &str| crate::graphics_io::load_graphic_image_directory(game_dir, stem, depth);
    Ok(DungeonBillboardBanks {
        normal: load(dungeon_billboard_stem(DungeonPresentationFlavour::Normal))?,
        mine: load(dungeon_billboard_stem(DungeonPresentationFlavour::Mine))?,
        flavour_byte_3: load(dungeon_billboard_stem(
            DungeonPresentationFlavour::FlavourByte3,
        ))?,
    })
}

/// Blit one billboard into a viewport, optionally mirrored.
///
/// Placements are published in screen pixels; the viewport is the
/// 176x176 tile window the frame puts at screen `(8, 8)`, so both axes
/// shift by that origin. Index 0 is the billboards' transparent margin
/// and is never written.
pub fn blit_dungeon_billboard(
    viewport: &mut crate::graphics::TileViewport,
    image: &crate::graphics::GraphicImage,
    screen_x: i32,
    mirrored: bool,
) {
    let origin_x = crate::gameplay_chrome::VIEWPORT_ORIGIN_X as i32;
    let origin_y = crate::gameplay_chrome::VIEWPORT_ORIGIN_Y as i32;
    let base_x = screen_x - origin_x;
    let base_y = DUNGEON_BILLBOARD_ORIGIN_Y - origin_y;
    let width = image.width as i32;
    let height = image.height as i32;
    for row in 0..height {
        let y = base_y + row;
        if y < 0 || y >= viewport.height as i32 {
            continue;
        }
        for column in 0..width {
            let source_column = if mirrored { width - 1 - column } else { column };
            let index = image.pixels[(row * width + source_column) as usize];
            if index == 0 {
                continue;
            }
            let x = base_x + column;
            if x < 0 || x >= viewport.width as i32 {
                continue;
            }
            viewport.pixels[y as usize * viewport.width + x as usize] = index;
        }
    }
}
