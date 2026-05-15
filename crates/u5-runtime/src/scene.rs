//! Scene partitioning: which file/index a town/castle/dungeon scene maps to,
//! plus the world-plane (Britannia/Underworld) and the unified `PlayTarget` enum.

use std::io;

use crate::parse_u8_literal;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Family {
    Towne,
    Dwelling,
    Castle,
    Keep,
}

impl Family {
    pub fn from_scene(scene: u8) -> Option<Self> {
        if scene == 0 || scene > 32 {
            return None;
        }
        match (scene - 1) >> 3 {
            0 => Some(Self::Towne),
            1 => Some(Self::Dwelling),
            2 => Some(Self::Castle),
            3 => Some(Self::Keep),
            _ => None,
        }
    }

    pub fn stem(self) -> &'static str {
        match self {
            Self::Towne => "TOWNE",
            Self::Dwelling => "DWELLING",
            Self::Castle => "CASTLE",
            Self::Keep => "KEEP",
        }
    }

    pub fn from_stem(stem: &str) -> Option<Self> {
        match stem.to_ascii_uppercase().as_str() {
            "TOWNE" | "TOWN" => Some(Self::Towne),
            "DWELLING" => Some(Self::Dwelling),
            "CASTLE" => Some(Self::Castle),
            "KEEP" => Some(Self::Keep),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Scene {
    pub byte: u8,
    pub family: Family,
    pub block: usize,
}

impl Scene {
    pub fn new(byte: u8) -> io::Result<Self> {
        let family = Family::from_scene(byte).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("invalid scene byte {byte}"),
            )
        })?;
        Ok(Self {
            byte,
            family,
            block: ((byte - 1) & 7) as usize,
        })
    }

    pub fn key(self) -> String {
        format!("{}:{}", self.family.stem(), self.block)
    }

    pub fn from_key(value: &str) -> io::Result<Self> {
        if let Some((family, block)) = value.split_once(':') {
            let family = Family::from_stem(family).ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("unknown scene family `{family}`"),
                )
            })?;
            let block = block.parse::<u8>().map_err(|err| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("invalid scene block `{block}`: {err}"),
                )
            })?;
            if block > 7 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("scene block must be 0..7, got {block}"),
                ));
            }
            let byte = match family {
                Family::Towne => 1 + block,
                Family::Dwelling => 9 + block,
                Family::Castle => 17 + block,
                Family::Keep => 25 + block,
            };
            return Self::new(byte);
        }

        let byte = parse_u8_literal(value)?;
        Self::new(byte)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DungeonScene {
    pub byte: u8,
    pub record: usize,
}

impl DungeonScene {
    pub fn new(byte: u8) -> io::Result<Self> {
        if !(33..=40).contains(&byte) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("invalid dungeon scene byte {byte}"),
            ));
        }
        Ok(Self {
            byte,
            record: (byte - 33) as usize,
        })
    }

    pub fn from_record(record: u8) -> io::Result<Self> {
        if record > 7 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("dungeon record must be 0..7, got {record}"),
            ));
        }
        Self::new(33 + record)
    }

    pub fn key(self) -> String {
        format!("DUNGEON:{}", self.record)
    }

    pub fn name(self) -> &'static str {
        match self.record {
            0 => "Deceit",
            1 => "Despise",
            2 => "Destard",
            3 => "Wrong",
            4 => "Covetous",
            5 => "Shame",
            6 => "Hythloth",
            7 => "Doom",
            _ => "Unknown",
        }
    }

    /// `dungeon-mode.md §2` presentation flavour. Cosmetic divergences
    /// in corner glyphs, view resource selection, wall/corpse class-`0xC?`
    /// descriptions, normal-flavour wall decoration, and a Doom-flavour
    /// rare text easter egg branch on this. The flavour does not change
    /// geometry, tile semantics, or encounter selection.
    pub fn presentation_flavour(self) -> DungeonPresentationFlavour {
        match self.record {
            0 | 3 | 4 => DungeonPresentationFlavour::FlavourByte3,
            5 | 6 => DungeonPresentationFlavour::Mine,
            _ => DungeonPresentationFlavour::Normal,
        }
    }
}

/// `dungeon-mode.md §2` presentation flavour the dungeon view uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DungeonPresentationFlavour {
    /// Despise, Destard, Doom — ordinary presentation.
    Normal,
    /// Deceit, Wrong, Covetous — flavour-byte-3 variant.
    FlavourByte3,
    /// Shame, Hythloth — mine-style presentation.
    Mine,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayTarget {
    Town(Scene),
    Dungeon(DungeonScene),
    World(WorldPlane),
}

impl PlayTarget {
    pub fn from_key(value: &str) -> io::Result<Self> {
        if let Some(plane) = WorldPlane::from_key(value) {
            return Ok(Self::World(plane));
        }
        if let Some((family, record)) = value.split_once(':') {
            if family.eq_ignore_ascii_case("DUNGEON") || family.eq_ignore_ascii_case("DUNGEONS") {
                let record = record.parse::<u8>().map_err(|err| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!("invalid dungeon record `{record}`: {err}"),
                    )
                })?;
                return Ok(Self::Dungeon(DungeonScene::from_record(record)?));
            }
            return Ok(Self::Town(Scene::from_key(value)?));
        }

        let byte = parse_u8_literal(value)?;
        if byte == 0 {
            Ok(Self::World(WorldPlane::Britannia))
        } else if (1..=32).contains(&byte) {
            Ok(Self::Town(Scene::new(byte)?))
        } else if (33..=40).contains(&byte) {
            Ok(Self::Dungeon(DungeonScene::new(byte)?))
        } else {
            Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("play scene byte must be 1..40, got {byte}"),
            ))
        }
    }

    pub fn key(self) -> String {
        match self {
            Self::Town(scene) => scene.key(),
            Self::Dungeon(scene) => scene.key(),
            Self::World(plane) => plane.key().to_string(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorldPlane {
    Britannia,
    Underworld,
}

impl WorldPlane {
    pub fn from_key(value: &str) -> Option<Self> {
        if value.eq_ignore_ascii_case("UNDERWORLD") || value.eq_ignore_ascii_case("UNDER") {
            Some(Self::Underworld)
        } else if value.eq_ignore_ascii_case("OVERWORLD")
            || value.eq_ignore_ascii_case("BRITANNIA")
            || value.eq_ignore_ascii_case("BRIT")
            || value.eq_ignore_ascii_case("WORLD")
        {
            Some(Self::Britannia)
        } else {
            None
        }
    }

    pub fn from_save_z(z: u8) -> Self {
        if z == 0 {
            Self::Britannia
        } else {
            Self::Underworld
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::Britannia => "BRITANNIA",
            Self::Underworld => "UNDERWORLD",
        }
    }

    pub fn file_name(self) -> &'static str {
        match self {
            Self::Britannia => "BRIT.DAT",
            Self::Underworld => "UNDER.DAT",
        }
    }

    pub fn save_floor(self) -> i8 {
        match self {
            Self::Britannia => 0,
            Self::Underworld => -1,
        }
    }
}
