pub fn tile_get_message(
    prefix: String,
    replacement_tile: u8,
    grant: Option<ObjectPickupGrant>,
) -> String {
    match grant {
        Some(grant) => format!(
            "{prefix}; replaced with tile {replacement_tile}; added {} {}.",
            grant.amount,
            grant.kind.label()
        ),
        None => format!("{prefix}; replaced with tile {replacement_tile}."),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ObjectPickupEntry {
    pub target: PlayTarget,
    pub floor: i8,
    pub x: usize,
    pub y: usize,
    pub kind: ObjectPickupKind,
    pub amount: u8,
    pub expected_tile: Option<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorldWaterfallEntry {
    pub plane: WorldPlane,
    pub x: usize,
    pub y: usize,
    pub direction: Direction,
    pub steps: u8,
    pub expected_tile: Option<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorldWaterfallSweep {
    Settled {
        steps: u8,
    },
    PlaneTransition {
        steps: u8,
        entry: WorldPlaneTransitionEntry,
    },
    Moongate {
        steps: u8,
        entry: MoongateEntry,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorldDamageEffect {
    Lava,
    Drowning,
}

impl WorldDamageEffect {
    fn from_key(key: &str) -> Option<Self> {
        match key.to_ascii_uppercase().as_str() {
            "LAVA" => Some(Self::Lava),
            "DROWNING" | "WATER" => Some(Self::Drowning),
            _ => None,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Lava => "lava",
            Self::Drowning => "drowning",
        }
    }

    fn allows_transport(self, transport: TransportState) -> bool {
        match self {
            Self::Lava => matches!(
                transport,
                TransportState::Carpet { .. } | TransportState::Balloon { .. }
            ),
            Self::Drowning => matches!(
                transport,
                TransportState::Foot
                    | TransportState::Ship { .. }
                    | TransportState::Skiff { .. }
                    | TransportState::Carpet { .. }
                    | TransportState::Balloon { .. }
            ),
        }
    }

    fn damages_transport(self, transport: TransportState) -> bool {
        match self {
            Self::Lava => matches!(transport, TransportState::Carpet { .. }),
            Self::Drowning => matches!(transport, TransportState::Foot),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorldDamageTileEntry {
    pub plane: WorldPlane,
    pub x: usize,
    pub y: usize,
    pub effect: WorldDamageEffect,
    pub expected_tile: Option<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorldEncounterEntry {
    pub plane: WorldPlane,
    pub tile: u8,
    pub threshold: u8,
    pub type_byte: u8,
    pub dx: i8,
    pub dy: i8,
    pub phase: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DungeonDeeperTransitionEntry {
    pub scene: DungeonScene,
    pub level: u8,
    pub x: usize,
    pub y: usize,
    pub to_plane: WorldPlane,
    pub to_x: usize,
    pub to_y: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DungeonTeleportEntry {
    pub scene: DungeonScene,
    pub level: u8,
    pub x: usize,
    pub y: usize,
    pub to_level: u8,
    pub to_x: usize,
    pub to_y: usize,
    pub expected_cell: Option<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DungeonChestContentEntry {
    pub scene: DungeonScene,
    pub level: u8,
    pub x: usize,
    pub y: usize,
    pub expected_cell: Option<u8>,
    pub grants: Vec<ObjectPickupGrant>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DungeonWindTileEntry {
    pub scene: DungeonScene,
    pub level: u8,
    pub x: usize,
    pub y: usize,
    pub expected_cell: Option<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DungeonExitTileEntry {
    pub scene: DungeonScene,
    pub level: u8,
    pub x: usize,
    pub y: usize,
    pub expected_cell: Option<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DungeonDoorEntry {
    pub scene: DungeonScene,
    pub level: u8,
    pub x: usize,
    pub y: usize,
    pub open_cell: u8,
    pub expected_cell: Option<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SecretDoorEntry {
    Town {
        scene: Scene,
        floor: i8,
        x: usize,
        y: usize,
        reveal_tile: u8,
        expected_tile: Option<u8>,
    },
    Dungeon {
        scene: DungeonScene,
        level: u8,
        x: usize,
        y: usize,
        reveal_cell: u8,
        expected_cell: Option<u8>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TownFireSourceEntry {
    pub scene: Scene,
    pub floor: i8,
    pub x: usize,
    pub y: usize,
    pub direction: Direction,
    pub expected_tile: Option<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TownPushableEntry {
    pub scene: Scene,
    pub floor: i8,
    pub x: usize,
    pub y: usize,
    pub expected_tile: Option<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TownGetTileEntry {
    pub scene: Scene,
    pub floor: i8,
    pub x: usize,
    pub y: usize,
    pub replacement_tile: u8,
    pub expected_tile: Option<u8>,
    pub grant: Option<ObjectPickupGrant>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TownRestBedEntry {
    pub scene: Scene,
    pub floor: i8,
    pub x: usize,
    pub y: usize,
    pub expected_tile: Option<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TownStairKind {
    Up,
    Down,
    Both,
}

impl TownStairKind {
    fn allows(self, intent: ClimbIntent) -> bool {
        matches!(
            (self, intent),
            (Self::Up, ClimbIntent::Up) | (Self::Down, ClimbIntent::Down) | (Self::Both, _)
        )
    }

    fn intents(self) -> &'static [ClimbIntent] {
        match self {
            Self::Up => &[ClimbIntent::Up],
            Self::Down => &[ClimbIntent::Down],
            Self::Both => &[ClimbIntent::Up, ClimbIntent::Down],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TownStairEntry {
    pub scene: Scene,
    pub floor: i8,
    pub x: usize,
    pub y: usize,
    pub kind: TownStairKind,
    pub expected_tile: Option<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TownTrapDoorEntry {
    pub scene: Scene,
    pub floor: i8,
    pub x: usize,
    pub y: usize,
    pub to_floor: i8,
    pub expected_tile: Option<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TownExitTileEntry {
    pub scene: Scene,
    pub floor: i8,
    pub x: usize,
    pub y: usize,
    pub expected_tile: Option<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TownLockKind {
    Locked,
    Magic,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TownLockEntry {
    pub scene: Scene,
    pub floor: i8,
    pub x: usize,
    pub y: usize,
    pub locked_tile: u8,
    pub unlocked_tile: u8,
    pub kind: TownLockKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlinkTargetEntry {
    pub target: PlayTarget,
    pub floor: i8,
    pub from_x: usize,
    pub from_y: usize,
    pub direction: Direction,
    pub to_x: usize,
    pub to_y: usize,
    pub expected_from_tile: Option<u8>,
    pub expected_to_tile: Option<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TownFireTarget {
    Object { slot: usize, object: ActiveObject },
    Door { x: usize, y: usize, tile: u8 },
    Wall { x: usize, y: usize, tile: u8 },
    None,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MoongateEntry {
    pub x: usize,
    pub y: usize,
    pub destination_plane: WorldPlane,
    pub destination_x: usize,
    pub destination_y: usize,
    pub active_hours: Option<(u8, u8)>,
    pub expected_tile: Option<u8>,
}

impl MoongateEntry {
    fn is_active_at(self, hour: u8) -> bool {
        match self.active_hours {
            Some((start, end)) if start <= end => (start..=end).contains(&hour),
            Some((start, end)) => hour >= start || hour <= end,
            None => true,
        }
    }

    fn is_single_ended(self) -> bool {
        self.destination_x == u8::MAX as usize && self.destination_y == u8::MAX as usize
    }

    fn matches_origin_tile(self, tile: u8) -> bool {
        self.expected_tile
            .map_or(true, |expected_tile| expected_tile == tile)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LookTable {
    pub descriptions: Vec<String>,
}

impl LookTable {
    fn description(&self, tile: usize) -> Option<&str> {
        self.descriptions.get(tile).map(String::as_str)
    }

    fn is_sentinel(&self, description: &str) -> bool {
        self.description(0)
            .map(|sentinel| description == sentinel)
            .unwrap_or(false)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TileGraphicsDepth {
    Ega16,
    Cga4,
}

impl TileGraphicsDepth {
    fn from_key(value: &str) -> io::Result<Self> {
        match value.to_ascii_lowercase().as_str() {
            "e" | "ega" | "ega16" | "16" => Ok(Self::Ega16),
            "c" | "cga" | "cga4" | "4" => Ok(Self::Cga4),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("raster depth must be ega or cga, got `{value}`"),
            )),
        }
    }

    fn file_name(self) -> &'static str {
        match self {
            Self::Ega16 => TILES_EGA_FILE,
            Self::Cga4 => TILES_CGA_FILE,
        }
    }

    fn body_len(self) -> usize {
        match self {
            Self::Ega16 => TILE_ATLAS_EGA_BODY_LEN,
            Self::Cga4 => TILE_ATLAS_CGA_BODY_LEN,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Ega16 => "EGA tile atlas",
            Self::Cga4 => "CGA tile atlas",
        }
    }

    #[cfg(test)]
    fn file_suffix(self) -> &'static str {
        match self {
            Self::Ega16 => "16",
            Self::Cga4 => "4",
        }
    }

    #[cfg(test)]
    fn pixel_limit(self) -> u8 {
        match self {
            Self::Ega16 => 16,
            Self::Cga4 => 4,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TileAtlas {
    pub depth: TileGraphicsDepth,
    pub pixels: Vec<u8>,
}

impl TileAtlas {
    fn tile_pixels(&self, tile: usize) -> Option<&[u8]> {
        let start = tile.checked_mul(TILE_ATLAS_TILE_PIXELS)?;
        let end = start.checked_add(TILE_ATLAS_TILE_PIXELS)?;
        self.pixels.get(start..end)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TileViewport {
    pub depth: TileGraphicsDepth,
    pub cells_wide: usize,
    pub cells_high: usize,
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u8>,
}

impl TileViewport {
    #[cfg(test)]
    fn pixel(&self, x: usize, y: usize) -> Option<u8> {
        if x >= self.width || y >= self.height {
            return None;
        }
        self.pixels.get(y * self.width + x).copied()
    }

    pub fn to_rgba(&self) -> Vec<u8> {
        let palette: &[[u8; 3]] = match self.depth {
            TileGraphicsDepth::Ega16 => &EGA_PALETTE_RGB,
            TileGraphicsDepth::Cga4 => &CGA_PALETTE_RGB,
        };
        let mut rgba = Vec::with_capacity(self.pixels.len() * 4);
        let limit = palette.len();
        for &index in &self.pixels {
            let rgb = palette[(index as usize) % limit];
            rgba.extend_from_slice(&[rgb[0], rgb[1], rgb[2], 0xff]);
        }
        rgba
    }
}

pub const EGA_PALETTE_RGB: [[u8; 3]; 16] = [
    [0x00, 0x00, 0x00],
    [0x00, 0x00, 0xaa],
    [0x00, 0xaa, 0x00],
    [0x00, 0xaa, 0xaa],
    [0xaa, 0x00, 0x00],
    [0xaa, 0x00, 0xaa],
    [0xaa, 0x55, 0x00],
    [0xaa, 0xaa, 0xaa],
    [0x55, 0x55, 0x55],
    [0x55, 0x55, 0xff],
    [0x55, 0xff, 0x55],
    [0x55, 0xff, 0xff],
    [0xff, 0x55, 0x55],
    [0xff, 0x55, 0xff],
    [0xff, 0xff, 0x55],
    [0xff, 0xff, 0xff],
];

pub const CGA_PALETTE_RGB: [[u8; 3]; 4] = [
    [0x00, 0x00, 0x00],
    [0x55, 0xff, 0xff],
    [0xff, 0x55, 0xff],
    [0xff, 0xff, 0xff],
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TopDownRenderArea {
    Town,
    World(WorldPlane),
}

#[cfg(test)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphicImage {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u8>,
}

#[cfg(test)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphicImageDirectory {
    pub depth: TileGraphicsDepth,
    pub images: Vec<Option<GraphicImage>>,
}

#[cfg(test)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphicSprite {
    pub image: GraphicImage,
    pub transparent_mask: Vec<u8>,
}

#[cfg(test)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphicSpriteSheet {
    pub depth: TileGraphicsDepth,
    pub sprites: Vec<Option<GraphicSprite>>,
}

#[cfg(test)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MonochromeBitmap {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u8>,
}

#[cfg(test)]
impl MonochromeBitmap {
    fn pixel(&self, x: usize, y: usize) -> Option<u8> {
        if x >= self.width || y >= self.height {
            return None;
        }
        self.pixels.get(y * self.width + x).copied()
    }
}

#[cfg(test)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TitleBitImages {
    pub blocks: Vec<MonochromeBitmap>,
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TextCellStyle {
    pub underline: bool,
    pub inverse: bool,
}

#[cfg(test)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FixedFont {
    pub cell_width: usize,
    pub cell_height: usize,
    pub glyphs: Vec<MonochromeBitmap>,
}

#[cfg(test)]
impl FixedFont {
    fn glyph(&self, code: u8) -> Option<&MonochromeBitmap> {
        self.glyphs.get(code as usize)
    }
}

#[cfg(test)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProportionalGlyph {
    pub advance_width: u8,
    pub bitmap: MonochromeBitmap,
}

#[cfg(test)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProportionalFont {
    pub first_code: u8,
    pub glyphs: Vec<ProportionalGlyph>,
}

#[cfg(test)]
impl ProportionalFont {
    fn glyph_for_code(&self, code: u8) -> Option<&ProportionalGlyph> {
        code.checked_sub(self.first_code)
            .and_then(|slot| self.glyphs.get(slot as usize))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LocationFloorEntry {
    pub scene: Scene,
    pub base_page: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LocationEntryYEntry {
    pub scene: Scene,
    pub y: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TilePassability {
    pub bytes: [u8; TILE_PASSABILITY_LEN],
}

impl TilePassability {
    fn from_bytes(bytes: &[u8]) -> io::Result<Self> {
        if bytes.len() != TILE_PASSABILITY_LEN {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{TILE_PASSABILITY_FILE} must contain exactly {TILE_PASSABILITY_LEN} bytes, got {}",
                    bytes.len()
                ),
            ));
        }
        let mut out = [0; TILE_PASSABILITY_LEN];
        out.copy_from_slice(bytes);
        Ok(Self { bytes: out })
    }

    fn is_passable(&self, tile: u8) -> bool {
        let byte = self.bytes[(tile >> 3) as usize];
        let mask = 0x80u8 >> (tile & 7);
        byte & mask != 0
    }
}

