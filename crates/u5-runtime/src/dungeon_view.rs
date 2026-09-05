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

use crate::ActiveObject;
use crate::graphics::{GraphicImageDirectory, GraphicSprite, GraphicSpriteSheet};
use crate::scene::DungeonPresentationFlavour;
use std::io;
use std::path::Path;

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
/// `x_right = 191 - x_left - width`, which mirrors the left rectangle
/// about the vertical centre line at **x = 95**.
///
/// `dungeon-mode.md §6.3` publishes `192 - x_left - width` and a centre
/// line at x = 95.5, and tabulates the right-hand x for every band from
/// it. Measured against the shipped game that whole column is one pixel
/// too far right.
///
/// The measurement is not a judgement call. Captured at an exact 2x
/// window with square pixels and no shader, downscaled 2:1 to native
/// 320x200 and compared against the engine's own native frame, in Deceit
/// level 1 at `(1, 1)`:
///
/// | region | at the published x | one pixel left |
/// |---|---|---|
/// | band-0 side wall, left copy | **0.0%** differ | 35.8% |
/// | band-0 side wall, mirrored copy | 34.5% | **0.0%** |
/// | band-1 forward wall, left copy | **0.0%** | 22.5% |
/// | band-1 forward wall, mirrored copy | 23.8% | **0.0%** |
///
/// Every left copy is already pixel-exact, so this is not a global
/// alignment error; only the mirrored copies move. The corollary is
/// visible without any reference: reflecting about 95.5 makes the
/// corridor perfectly symmetric, and the shipped game's corridor is
/// **not** - 23.5% of its pixels break left-right symmetry, because
/// reflecting about 95 makes the two halves overlap in column 95 rather
/// than abut. The overlap is invisible, since the mirrored copy's first
/// column carries the same value as the left copy's last.
///
/// Reported to the spec as `cleak/u5-spec#199`.
pub const DUNGEON_MIRROR_SPAN: i32 = 191;

/// Directory slot count in each corridor art bank.
pub const DUNGEON_BILLBOARD_SLOTS: usize = 28;
pub const DUNGEON_BILLBOARD_ABSENT_SLOTS: [usize; 2] = [8, 24];

pub const DUNGEON_OBJECT_LEFT_X: [i32; 4] = [56, 72, 80, 88];
pub const DUNGEON_OBJECT_FLOOR_Y: [i32; 4] = [176, 152, 120, 104];

/// Object-family base slots selected by one normalized dungeon cell.
/// The first member rises above the horizon; the second stands below it.
pub const fn dungeon_object_family_slots(cell: u8) -> (Option<usize>, Option<usize>) {
    match cell >> 4 {
        0x1 => (Some(0), None),
        0x2 => (None, Some(0)),
        0x3 => (Some(0), Some(0)),
        0x4 => (None, Some(12)),
        0x5 => (None, Some(4)),
        0x6 => (if cell & 0x07 == 0 { Some(8) } else { None }, Some(8)),
        0x7 => (None, Some(16)),
        _ => (None, None),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DungeonFieldPaintSpec {
    pub pen: u8,
    pub minimum: u16,
    pub maximum: u16,
    pub strokes: usize,
    pub endpoint_delta: u16,
}

pub const fn dungeon_field_paint_spec(cell: u8, band: usize) -> Option<DungeonFieldPaintSpec> {
    if cell >> 4 != 0x8 || band >= DUNGEON_BANDS {
        return None;
    }
    let pen = match cell & 0x0f {
        0 => 13,
        1 => 10,
        2 => 12,
        3 => 9,
        _ => return None,
    };
    let (minimum, maximum, strokes, endpoint_delta) = match band {
        0 => (16, 167, 300, 7),
        1 => (56, 135, 100, 7),
        2 => (80, 111, 50, 5),
        3 => (92, 99, 15, 2),
        _ => return None,
    };
    Some(DungeonFieldPaintSpec {
        pen,
        minimum,
        maximum,
        strokes,
        endpoint_delta,
    })
}

pub const fn dungeon_fountain_points(band: usize, frame: usize) -> &'static [(i32, i32)] {
    match (band, frame % 3) {
        (0, 0) => &[
            (90, 115),
            (82, 119),
            (95, 123),
            (87, 125),
            (91, 131),
            (80, 133),
            (80, 133),
            (80, 133),
            (89, 145),
        ],
        (0, 1) => &[
            (87, 115),
            (94, 119),
            (81, 122),
            (93, 125),
            (95, 129),
            (85, 130),
            (89, 135),
            (80, 139),
            (80, 144),
        ],
        (0, 2) => &[
            (84, 117),
            (90, 117),
            (90, 124),
            (80, 126),
            (85, 136),
            (89, 138),
            (89, 138),
            (89, 138),
            (85, 143),
        ],
        (1, 0) => &[(94, 103), (91, 106), (85, 107), (95, 116), (85, 123)],
        (1, 1) => &[(90, 102), (89, 111), (95, 111), (84, 112), (87, 122)],
        (1, 2) => &[(87, 104), (94, 107), (88, 115), (83, 117), (90, 124)],
        (2, 0) => &[(91, 97), (95, 100), (89, 102), (91, 104)],
        (2, 1) => &[(90, 98), (94, 98), (92, 106), (92, 106)],
        (2, 2) => &[(93, 97), (89, 100), (89, 105), (89, 105)],
        (3, 0) => &[(94, 96)],
        (3, 1) => &[(93, 97)],
        (3, 2) => &[(95, 98)],
        _ => &[],
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DungeonDecorationPlacement {
    SideLeft,
    SideRight,
    Forward,
}

pub const fn dungeon_decoration_position(
    placement: DungeonDecorationPlacement,
    band: usize,
    stage: u8,
) -> Option<(i32, i32)> {
    if stage > 4 {
        return None;
    }
    let stage = stage as usize;
    match (placement, band) {
        (DungeonDecorationPlacement::SideLeft, 0) => Some((33, [28, 37, 64, 112, 173][stage])),
        (DungeonDecorationPlacement::SideRight, 0) => Some((157, [28, 37, 64, 112, 173][stage])),
        (DungeonDecorationPlacement::SideLeft, 1) => Some((67, [54, 59, 74, 98, 133][stage])),
        (DungeonDecorationPlacement::SideRight, 1) => Some((123, [54, 59, 74, 98, 133][stage])),
        (DungeonDecorationPlacement::Forward, 1) => Some((95, [54, 61, 80, 114, 160][stage])),
        (DungeonDecorationPlacement::Forward, 2) => Some((95, [60, 64, 76, 96, 123][stage])),
        _ => None,
    }
}

pub const DUNGEON_MONSTER_INITIAL_STATES: [u8; 8] =
    [0x60, 0xa0, 0x00, 0x90, 0x80, 0x60, 0x00, 0x00];
pub const DUNGEON_MONSTER_COMBAT_CLASSES: [u8; 8] = [20, 21, 22, 23, 24, 25, 28, 27];
pub const DUNGEON_MONSTER_INACTIVE_DEP1: u8 = u8::MAX;
pub const DUNGEON_MONSTER_FLOOR_DEP3: u8 = 0;
pub const DUNGEON_MONSTER_UPPER_DEP3: u8 = u8::MAX;
pub const DUNGEON_ACTIVE_MONSTER_SLOT: usize = 1;
pub const DUNGEON_MONSTER_LEFT_X: [i32; 3] = [72, 80, 88];
pub const DUNGEON_MONSTER_NORMAL_Y: [i32; 3] = [86, 96, 98];
pub const DUNGEON_MONSTER_UPPER_Y: [i32; 3] = [40, 70, 85];

pub const fn dungeon_monster_family_for_combat_class(class: u8) -> Option<usize> {
    match class {
        20 => Some(0),
        21 => Some(1),
        22 => Some(2),
        23 => Some(3),
        24 => Some(4),
        25 => Some(5),
        28 => Some(6),
        27 => Some(7),
        _ => None,
    }
}

pub const fn dungeon_monster_record_active(object: ActiveObject) -> bool {
    object.aux1 != DUNGEON_MONSTER_INACTIVE_DEP1
}

/// `dungeon-mode.md §6.9`: Negate Time bypasses both random pose draws.
pub const fn dungeon_monster_negate_poses(family: usize) -> (usize, usize) {
    (1, if family == 3 { 0 } else { 1 })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DungeonDecorationToneSweep {
    pub updates: u8,
    pub start_hz: u16,
    pub step_hz: u16,
    pub delay_units: u8,
}

impl DungeonDecorationToneSweep {
    pub const fn frequency(self, update: u8) -> Option<u16> {
        if update >= self.updates {
            return None;
        }
        Some(self.start_hz + self.step_hz * update as u16)
    }
}

/// `dungeon-mode.md §6.8`: stage-5 speaker sweep. Band 3 only stops
/// the speaker, represented by `None`; exact milliseconds are outside
/// the contract because a calibrated delay unit is CPU-dependent.
pub const fn dungeon_decoration_tone_sweep(band: usize) -> Option<DungeonDecorationToneSweep> {
    match band {
        0 => Some(DungeonDecorationToneSweep {
            updates: 20,
            start_hz: 3200,
            step_hz: 15,
            delay_units: 20,
        }),
        1 => Some(DungeonDecorationToneSweep {
            updates: 12,
            start_hz: 3200,
            step_hz: 25,
            delay_units: 12,
        }),
        2 => Some(DungeonDecorationToneSweep {
            updates: 4,
            start_hz: 3200,
            step_hz: 75,
            delay_units: 4,
        }),
        _ => None,
    }
}

/// Apply §6.9's state-byte pose rules to two provisional selectors.
/// Returns `(new_state, left_pose, right_pose)` where each pose is 0 or 1.
pub const fn dungeon_monster_pose_step(
    state: u8,
    mut left: bool,
    mut right: bool,
) -> (u8, usize, usize) {
    let symmetry = state & 0x90;
    let mode = state & 0x60;
    let mut phase = state & 0x0f;
    match mode {
        0x20 => {
            phase ^= 1;
            left = phase & 1 != 0;
            right = left;
        }
        0x40 => {}
        0x60 => {
            phase = phase.wrapping_sub(1) & 0x03;
            let pair = [1u8, 3, 2, 3][phase as usize];
            left = pair & 1 != 0;
            right = pair & 2 != 0;
        }
        _ => {}
    }
    if symmetry != 0 {
        right = if symmetry == 0x90 { !left } else { left };
    }
    ((state & 0xf0) | phase, left as usize, right as usize)
}

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
            // The decorated families are the published scenery-bearing
            // counterparts to the plain families. Issue #84 retracted its
            // proposed numeric pixel-difference discriminator, so the
            // engine deliberately does not infer this mapping from a ratio.
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

/// Published inverse of the seven contiguous billboard families. The low two
/// bits select the depth band and the remaining bits select the family.
pub const fn dungeon_billboard_role_for_slot(slot: usize) -> Option<DungeonBillboardRole> {
    Some(match slot / DUNGEON_BANDS {
        0 => DungeonBillboardRole::SideWall,
        1 => DungeonBillboardRole::SideDoor,
        2 => DungeonBillboardRole::ForwardWall,
        3 => DungeonBillboardRole::ForwardDoor,
        4 => DungeonBillboardRole::SideOpening,
        5 => DungeonBillboardRole::SideFlavourWall,
        6 => DungeonBillboardRole::ForwardFlavourWall,
        _ => return None,
    })
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

/// `dungeon-mode.md §§6.6, 6.9`: the masked object bank and the eight
/// wandering-monster family banks used by the corridor's backward painter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DungeonSpriteBanks {
    objects: Option<GraphicSpriteSheet>,
    monsters: [Option<GraphicSpriteSheet>; 8],
}

impl DungeonSpriteBanks {
    pub fn objects(&self) -> Option<&GraphicSpriteSheet> {
        self.objects.as_ref()
    }

    pub fn monster(&self, family: usize) -> Option<&GraphicSpriteSheet> {
        self.monsters.get(family)?.as_ref()
    }
}

/// Load any first-person sprite resources present in a graphics directory.
/// A deliberately minimal fixture may omit them all. A present resource is
/// always parsed and shape-validated; malformed art is not rendered as blank.
pub fn load_optional_dungeon_sprite_banks(
    game_dir: &Path,
    depth: crate::graphics::TileGraphicsDepth,
) -> io::Result<Option<DungeonSpriteBanks>> {
    let object_path = game_dir.join(crate::graphics_io::tile_graphics_file_name("ITEMS", depth));
    let monster_paths = std::array::from_fn::<_, 8, _>(|family| {
        game_dir.join(crate::graphics_io::tile_graphics_file_name(
            &format!("MON{family}"),
            depth,
        ))
    });
    if !object_path.is_file() && monster_paths.iter().all(|path| !path.is_file()) {
        return Ok(None);
    }

    let objects = if object_path.is_file() {
        let sheet = crate::graphics_io::load_graphic_sprite_sheet(game_dir, "ITEMS", depth)?;
        validate_dungeon_object_sprites(&sheet, "ITEMS")?;
        Some(sheet)
    } else {
        None
    };
    let mut monsters: [Option<GraphicSpriteSheet>; 8] = std::array::from_fn(|_| None);
    for family in 0..8 {
        if !monster_paths[family].is_file() {
            continue;
        }
        let stem = format!("MON{family}");
        let sheet = crate::graphics_io::load_graphic_sprite_sheet(game_dir, &stem, depth)?;
        validate_dungeon_monster_sprites(&sheet, &stem)?;
        monsters[family] = Some(sheet);
    }
    Ok(Some(DungeonSpriteBanks { objects, monsters }))
}

pub fn validate_dungeon_object_sprites(
    sheet: &GraphicSpriteSheet,
    resource_name: &str,
) -> io::Result<()> {
    const DIMS: [[(usize, usize); 4]; 5] = [
        [(40, 80), (24, 56), (16, 24), (8, 8)],
        [(40, 80), (24, 56), (16, 24), (8, 8)],
        [(40, 24), (24, 32), (16, 16), (8, 8)],
        [(40, 24), (24, 32), (16, 16), (8, 8)],
        [(40, 24), (24, 32), (16, 16), (16, 16)],
    ];
    if sheet.sprites.len() != 20 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{resource_name} must contain 20 object sprites, got {}",
                sheet.sprites.len()
            ),
        ));
    }
    for (index, expected) in DIMS.into_iter().flatten().enumerate() {
        validate_dungeon_sprite_slot(sheet, resource_name, index, expected)?;
    }
    Ok(())
}

pub fn validate_dungeon_monster_sprites(
    sheet: &GraphicSpriteSheet,
    resource_name: &str,
) -> io::Result<()> {
    const DIMS: [(usize, usize); 6] = [(24, 66), (16, 25), (8, 6), (24, 66), (16, 25), (8, 6)];
    if sheet.sprites.len() != DIMS.len() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{resource_name} must contain 6 monster sprites, got {}",
                sheet.sprites.len()
            ),
        ));
    }
    for (index, expected) in DIMS.into_iter().enumerate() {
        validate_dungeon_sprite_slot(sheet, resource_name, index, expected)?;
    }
    Ok(())
}

fn validate_dungeon_sprite_slot(
    sheet: &GraphicSpriteSheet,
    resource_name: &str,
    index: usize,
    expected: (usize, usize),
) -> io::Result<()> {
    let sprite = sheet
        .sprites
        .get(index)
        .and_then(Option::as_ref)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{resource_name} sprite {index} must be populated"),
            )
        })?;
    if (sprite.image.width, sprite.image.height) != expected {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{resource_name} sprite {index} must be {}x{}, got {}x{}",
                expected.0, expected.1, sprite.image.width, sprite.image.height
            ),
        ));
    }
    Ok(())
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
    game_dir: &Path,
    depth: crate::graphics::TileGraphicsDepth,
) -> std::io::Result<DungeonBillboardBanks> {
    let load = |stem: &str| crate::graphics_io::load_graphic_image_directory(game_dir, stem, depth);
    let banks = DungeonBillboardBanks {
        normal: load(dungeon_billboard_stem(DungeonPresentationFlavour::Normal))?,
        mine: load(dungeon_billboard_stem(DungeonPresentationFlavour::Mine))?,
        flavour_byte_3: load(dungeon_billboard_stem(
            DungeonPresentationFlavour::FlavourByte3,
        ))?,
    };
    for (stem, bank) in [
        ("DNG1", &banks.normal),
        ("DNG2", &banks.mine),
        ("DNG3", &banks.flavour_byte_3),
    ] {
        validate_dungeon_billboard_bank(bank, stem)?;
    }
    Ok(banks)
}

/// Load the corridor resources when a fixture directory deliberately omits
/// all three files, but reject partial or malformed installations. Previously
/// `load_tile_atlas` converted every error to `None`, making a corrupt shipped
/// bank indistinguishable from an intentionally graphics-free test fixture.
pub fn load_optional_dungeon_billboard_banks(
    game_dir: &Path,
    depth: crate::graphics::TileGraphicsDepth,
) -> io::Result<Option<DungeonBillboardBanks>> {
    let stems = ["DNG1", "DNG2", "DNG3"];
    let present = stems.map(|stem| {
        game_dir
            .join(crate::graphics_io::tile_graphics_file_name(stem, depth))
            .is_file()
    });
    if present.iter().all(|value| !value) {
        return Ok(None);
    }
    if !present.iter().all(|value| *value) {
        let missing = stems
            .iter()
            .zip(present)
            .filter_map(|(stem, is_present)| (!is_present).then_some(*stem))
            .collect::<Vec<_>>()
            .join(", ");
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("dungeon corridor bank set is incomplete; missing {missing}"),
        ));
    }
    load_dungeon_billboard_banks(game_dir, depth).map(Some)
}

/// Validate the runtime-visible shape contract rather than relying on an
/// asset-audit test to catch it after a silently blank frame has rendered.
pub fn validate_dungeon_billboard_bank(
    bank: &DungeonBillboardBank,
    resource_name: &str,
) -> io::Result<()> {
    if bank.images.len() != DUNGEON_BILLBOARD_SLOTS {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{resource_name} must contain {DUNGEON_BILLBOARD_SLOTS} billboard slots, got {}",
                bank.images.len()
            ),
        ));
    }
    for slot in 0..DUNGEON_BILLBOARD_SLOTS {
        let expected_absent = DUNGEON_BILLBOARD_ABSENT_SLOTS.contains(&slot);
        let image = bank.images[slot].as_ref();
        if expected_absent {
            if image.is_some() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("{resource_name} slot {slot} must be absent"),
                ));
            }
            continue;
        }
        let image = image.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{resource_name} slot {slot} must be populated"),
            )
        })?;
        let role = dungeon_billboard_role_for_slot(slot)
            .expect("the validated slot range always maps to a family");
        let band = slot % DUNGEON_BANDS;
        let expected_width = dungeon_billboard_width(role, band)
            .expect("the validated band range always has a width")
            as usize;
        if image.width != expected_width || image.height != DUNGEON_BILLBOARD_ROWS {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{resource_name} slot {slot} must be {expected_width}x{DUNGEON_BILLBOARD_ROWS}, got {}x{}",
                    image.width, image.height
                ),
            ));
        }
    }
    Ok(())
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

/// Composite one masked half-sprite at a published screen coordinate.
/// Set mask bits preserve the destination; clear bits write the image pixel.
pub fn blit_dungeon_sprite(
    viewport: &mut crate::graphics::TileViewport,
    sprite: &GraphicSprite,
    screen_x: i32,
    screen_y: i32,
    mirrored: bool,
) {
    let base_x = screen_x - crate::gameplay_chrome::VIEWPORT_ORIGIN_X as i32;
    let base_y = screen_y - crate::gameplay_chrome::VIEWPORT_ORIGIN_Y as i32;
    let width = sprite.image.width as i32;
    let height = sprite.image.height as i32;
    for row in 0..height {
        let y = base_y + row;
        if y < 0 || y >= viewport.height as i32 {
            continue;
        }
        for column in 0..width {
            let source_column = if mirrored { width - 1 - column } else { column };
            let source = (row * width + source_column) as usize;
            if sprite.transparent_mask[source] != 0 {
                continue;
            }
            let x = base_x + column;
            if x < 0 || x >= viewport.width as i32 {
                continue;
            }
            viewport.pixels[y as usize * viewport.width + x as usize] = sprite.image.pixels[source];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graphics::{GraphicImage, TileGraphicsDepth};
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn canonical_bank() -> DungeonBillboardBank {
        let images = (0..DUNGEON_BILLBOARD_SLOTS)
            .map(|slot| {
                if DUNGEON_BILLBOARD_ABSENT_SLOTS.contains(&slot) {
                    return None;
                }
                let role = dungeon_billboard_role_for_slot(slot).unwrap();
                let width = dungeon_billboard_width(role, slot % DUNGEON_BANDS).unwrap() as usize;
                Some(GraphicImage {
                    width,
                    height: DUNGEON_BILLBOARD_ROWS,
                    pixels: vec![0; width * DUNGEON_BILLBOARD_ROWS],
                })
            })
            .collect();
        GraphicImageDirectory {
            depth: TileGraphicsDepth::Ega16,
            images,
        }
    }

    fn sprite(width: usize, height: usize) -> GraphicSprite {
        GraphicSprite {
            image: GraphicImage {
                width,
                height,
                pixels: vec![3; width * height],
            },
            transparent_mask: vec![0; width * height],
        }
    }

    #[test]
    fn slot_inverse_matches_every_published_family_base() {
        use DungeonBillboardRole::*;
        for (base, role) in [
            (0, SideWall),
            (4, SideDoor),
            (8, ForwardWall),
            (12, ForwardDoor),
            (16, SideOpening),
            (20, SideFlavourWall),
            (24, ForwardFlavourWall),
        ] {
            for band in 0..DUNGEON_BANDS {
                assert_eq!(dungeon_billboard_role_for_slot(base + band), Some(role));
            }
        }
        assert_eq!(
            dungeon_billboard_role_for_slot(DUNGEON_BILLBOARD_SLOTS),
            None
        );
    }

    #[test]
    fn validator_accepts_only_the_published_directory_shape() {
        let canonical = canonical_bank();
        validate_dungeon_billboard_bank(&canonical, "synthetic").unwrap();

        let mut populated_absent = canonical.clone();
        populated_absent.images[8] = Some(GraphicImage {
            width: 80,
            height: DUNGEON_BILLBOARD_ROWS,
            pixels: vec![0; 80 * DUNGEON_BILLBOARD_ROWS],
        });
        assert!(
            validate_dungeon_billboard_bank(&populated_absent, "synthetic")
                .unwrap_err()
                .to_string()
                .contains("slot 8 must be absent")
        );

        let mut missing_required = canonical.clone();
        missing_required.images[13] = None;
        assert!(
            validate_dungeon_billboard_bank(&missing_required, "synthetic")
                .unwrap_err()
                .to_string()
                .contains("slot 13 must be populated")
        );

        let mut wrong_dimensions = canonical;
        wrong_dimensions.images[5].as_mut().unwrap().width += 1;
        assert!(
            validate_dungeon_billboard_bank(&wrong_dimensions, "synthetic")
                .unwrap_err()
                .to_string()
                .contains("slot 5 must be 32x164")
        );
    }

    #[test]
    fn optional_loader_distinguishes_absent_from_partial_bank_sets() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "u5-dungeon-billboard-loader-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        assert!(
            load_optional_dungeon_billboard_banks(&dir, TileGraphicsDepth::Ega16)
                .unwrap()
                .is_none()
        );

        fs::write(dir.join("DNG1.16"), []).unwrap();
        let err = load_optional_dungeon_billboard_banks(&dir, TileGraphicsDepth::Ega16)
            .unwrap_err()
            .to_string();
        assert!(err.contains("missing DNG2, DNG3"), "{err}");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn object_family_classifier_covers_both_ladder_halves_and_distinct_chests() {
        assert_eq!(dungeon_object_family_slots(0x10), (Some(0), None));
        assert_eq!(dungeon_object_family_slots(0x20), (None, Some(0)));
        assert_eq!(dungeon_object_family_slots(0x30), (Some(0), Some(0)));
        assert_eq!(dungeon_object_family_slots(0x40), (None, Some(12)));
        assert_eq!(dungeon_object_family_slots(0x50), (None, Some(4)));
        assert_eq!(dungeon_object_family_slots(0x60), (Some(8), Some(8)));
        assert_eq!(dungeon_object_family_slots(0x67), (None, Some(8)));
        assert_eq!(dungeon_object_family_slots(0x70), (None, Some(16)));
    }

    #[test]
    fn field_specs_pin_pens_ranges_counts_and_inclusive_lengths() {
        let sleep = dungeon_field_paint_spec(0x80, 0).unwrap();
        assert_eq!((sleep.pen, sleep.minimum, sleep.maximum), (13, 16, 167));
        assert_eq!((sleep.strokes, sleep.endpoint_delta + 1), (300, 8));
        let electric = dungeon_field_paint_spec(0x83, 3).unwrap();
        assert_eq!(
            (electric.pen, electric.minimum, electric.maximum),
            (9, 92, 99)
        );
        assert_eq!((electric.strokes, electric.endpoint_delta + 1), (15, 3));
        assert!(dungeon_field_paint_spec(0x84, 0).is_none());
    }

    #[test]
    fn decoration_positions_cover_only_the_published_bands() {
        assert_eq!(
            dungeon_decoration_position(DungeonDecorationPlacement::SideLeft, 0, 4),
            Some((33, 173))
        );
        assert_eq!(
            dungeon_decoration_position(DungeonDecorationPlacement::Forward, 2, 3),
            Some((95, 96))
        );
        assert_eq!(
            dungeon_decoration_position(DungeonDecorationPlacement::Forward, 0, 0),
            None
        );
        assert_eq!(
            dungeon_decoration_position(DungeonDecorationPlacement::SideRight, 1, 5),
            None
        );
    }

    #[test]
    fn monster_family_and_pose_state_rules_match_the_public_tables() {
        assert_eq!(dungeon_monster_family_for_combat_class(20), Some(0));
        assert_eq!(dungeon_monster_family_for_combat_class(28), Some(6));
        assert_eq!(dungeon_monster_family_for_combat_class(27), Some(7));
        assert_eq!(dungeon_monster_family_for_combat_class(26), None);

        assert_eq!(dungeon_monster_pose_step(0x20, false, true), (0x21, 1, 1));
        assert_eq!(dungeon_monster_pose_step(0x21, true, true), (0x20, 0, 0));
        assert_eq!(dungeon_monster_pose_step(0x90, true, true), (0x90, 1, 0));
        assert_eq!(dungeon_monster_pose_step(0x40, true, false), (0x40, 1, 0));

        for family in 0..8 {
            assert_eq!(
                dungeon_monster_negate_poses(family),
                (1, if family == 3 { 0 } else { 1 })
            );
        }
        let rat = ActiveObject {
            type_byte: 0,
            tile: 0,
            x: 2,
            y: 3,
            z: 0,
            phase: DUNGEON_MONSTER_INITIAL_STATES[0],
            aux1: DUNGEON_MONSTER_COMBAT_CLASSES[0],
            aux3: DUNGEON_MONSTER_FLOOR_DEP3,
        };
        assert!(dungeon_monster_record_active(rat));
        assert!(!dungeon_monster_record_active(ActiveObject {
            aux1: DUNGEON_MONSTER_INACTIVE_DEP1,
            ..rat
        }));
    }

    #[test]
    fn decoration_stage_five_tone_sweeps_match_each_depth_band() {
        let expected = [(20, 15, 3485), (12, 25, 3475), (4, 75, 3425)];
        for (band, (updates, step, last)) in expected.into_iter().enumerate() {
            let sweep = dungeon_decoration_tone_sweep(band).unwrap();
            assert_eq!(
                (
                    sweep.updates,
                    sweep.start_hz,
                    sweep.step_hz,
                    sweep.delay_units
                ),
                (updates, 3200, step, updates)
            );
            assert_eq!(sweep.frequency(0), Some(3200));
            assert_eq!(sweep.frequency(updates - 1), Some(last));
            assert_eq!(sweep.frequency(updates), None);
        }
        assert_eq!(dungeon_decoration_tone_sweep(3), None);
    }

    #[test]
    fn sprite_bank_validators_pin_all_published_dimensions() {
        let object_dims = [
            [(40, 80), (24, 56), (16, 24), (8, 8)],
            [(40, 80), (24, 56), (16, 24), (8, 8)],
            [(40, 24), (24, 32), (16, 16), (8, 8)],
            [(40, 24), (24, 32), (16, 16), (8, 8)],
            [(40, 24), (24, 32), (16, 16), (16, 16)],
        ];
        let objects = GraphicSpriteSheet {
            depth: TileGraphicsDepth::Ega16,
            sprites: object_dims
                .into_iter()
                .flatten()
                .map(|(width, height)| Some(sprite(width, height)))
                .collect(),
        };
        validate_dungeon_object_sprites(&objects, "fixture").unwrap();

        let monsters = GraphicSpriteSheet {
            depth: TileGraphicsDepth::Ega16,
            sprites: [(24, 66), (16, 25), (8, 6), (24, 66), (16, 25), (8, 6)]
                .into_iter()
                .map(|(width, height)| Some(sprite(width, height)))
                .collect(),
        };
        validate_dungeon_monster_sprites(&monsters, "fixture").unwrap();
    }
}
