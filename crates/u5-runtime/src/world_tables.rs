//! Data structures for the world TSV tables (locations, plane transitions, get-tiles, pickups, waterfalls, damage, encounters, shrines).

use crate::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorldLocationEntry {
    pub plane: WorldPlane,
    pub x: usize,
    pub y: usize,
    pub target: PlayTarget,
    pub town_entry_y: Option<usize>,
    pub expected_tile: Option<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShrineEntry {
    pub plane: WorldPlane,
    pub x: usize,
    pub y: usize,
    pub virtue: ShrineVirtue,
    pub expected_tile: Option<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CodexUrnEntry {
    pub plane: WorldPlane,
    pub x: usize,
    pub y: usize,
    pub expected_tile: Option<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EternalFlameEntry {
    pub target: PlayTarget,
    pub floor: i8,
    pub x: usize,
    pub y: usize,
    pub flame: EternalFlame,
    pub expected_tile: Option<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorldPlaneTransitionEntry {
    pub from_plane: WorldPlane,
    pub x: usize,
    pub y: usize,
    pub to_plane: WorldPlane,
    pub to_x: usize,
    pub to_y: usize,
    pub expected_tile: Option<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorldGetTileEntry {
    pub plane: WorldPlane,
    pub x: usize,
    pub y: usize,
    pub replacement_tile: u8,
    pub expected_tile: Option<u8>,
    pub grant: Option<ObjectPickupGrant>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObjectPickupKind {
    Food,
    Gold,
    Keys,
    Gems,
    Torches,
    SkullKeys,
    Potion(usize),
    Scroll(usize),
    Equipment(usize),
    Moonstone(usize),
    MagicCarpet,
    HmsCapePlans,
    SandalwoodBox,
    CrownOfLordBritish,
    SceptreOfLordBritish,
    AmuletOfLordBritish,
    ShadowlordShard(usize),
}

impl ObjectPickupKind {
    pub fn from_key(key: &str) -> Option<Self> {
        let key = key.to_ascii_lowercase();
        if let Some(index) = parse_indexed_pickup_key(&key, "potion", POTION_COUNT) {
            return Some(Self::Potion(index));
        }
        if let Some(index) = parse_indexed_pickup_key(&key, "scroll", SCROLL_COUNT) {
            return Some(Self::Scroll(index));
        }
        if let Some(index) = parse_indexed_pickup_key(&key, "equipment", EQUIPMENT_COUNT)
            .or_else(|| parse_indexed_pickup_key(&key, "equip", EQUIPMENT_COUNT))
        {
            return Some(Self::Equipment(index));
        }
        if let Some(index) = parse_indexed_pickup_key(&key, "shard", SHADOWLORD_COUNT) {
            return Some(Self::ShadowlordShard(index));
        }
        if let Some(index) = parse_indexed_pickup_key(&key, "moonstone", MOONSTONE_SLOT_COUNT) {
            return Some(Self::Moonstone(index));
        }
        match key.as_str() {
            "food" | "ration" | "rations" => Some(Self::Food),
            "gold" | "coin" | "coins" => Some(Self::Gold),
            "key" | "keys" => Some(Self::Keys),
            "skullkey" | "skullkeys" | "skull_key" | "skull_keys" | "specialkey"
            | "specialkeys" | "special_key" | "special_keys" => Some(Self::SkullKeys),
            "gem" | "gems" => Some(Self::Gems),
            "torch" | "torches" => Some(Self::Torches),
            "carpet" | "magiccarpet" | "magic_carpet" => Some(Self::MagicCarpet),
            "plans" | "hmscapeplans" | "hms_cape_plans" => Some(Self::HmsCapePlans),
            "box" | "woodenbox" | "wooden_box" | "sandalwoodbox" | "sandalwood_box" => {
                Some(Self::SandalwoodBox)
            }
            "crown" | "crown_lb" | "crown_of_lord_british" => Some(Self::CrownOfLordBritish),
            "sceptre"
            | "scepter"
            | "sceptre_lb"
            | "scepter_lb"
            | "sceptre_of_lord_british"
            | "scepter_of_lord_british" => Some(Self::SceptreOfLordBritish),
            "amulet" | "amulet_lb" | "amulet_of_lord_british" => Some(Self::AmuletOfLordBritish),
            _ => None,
        }
    }

    pub fn label(self) -> String {
        match self {
            Self::Food => "food".to_string(),
            Self::Gold => "gold".to_string(),
            Self::Keys => "keys".to_string(),
            Self::Gems => "gems".to_string(),
            Self::Torches => "torches".to_string(),
            Self::SkullKeys => "skull keys".to_string(),
            Self::Potion(index) => format!("{} potion", potion_label(index)),
            Self::Scroll(index) => format!(
                "{} scroll",
                SCROLL_SPELL_LABELS.get(index).copied().unwrap_or("unknown")
            ),
            Self::Equipment(index) => equipment_name(index).to_string(),
            Self::Moonstone(index) => format!("Moonstone phase {}", index + 1),
            Self::MagicCarpet => "magic carpet".to_string(),
            Self::HmsCapePlans => "HMS Cape plans".to_string(),
            Self::SandalwoodBox => "sandalwood box".to_string(),
            Self::CrownOfLordBritish => "Crown of Lord British".to_string(),
            Self::SceptreOfLordBritish => "Sceptre of Lord British".to_string(),
            Self::AmuletOfLordBritish => "Amulet of Lord British".to_string(),
            Self::ShadowlordShard(index) => match index {
                0 => "Shard of Falsehood".to_string(),
                1 => "Shard of Hatred".to_string(),
                2 => "Shard of Cowardice".to_string(),
                _ => "Shadowlord shard".to_string(),
            },
        }
    }
}

fn parse_indexed_pickup_key(key: &str, prefix: &str, limit: usize) -> Option<usize> {
    let suffix = key.strip_prefix(prefix)?;
    let suffix = suffix
        .strip_prefix(':')
        .or_else(|| suffix.strip_prefix('_'))
        .or_else(|| suffix.strip_prefix('-'))
        .unwrap_or(suffix);
    if suffix.is_empty() {
        return None;
    }
    suffix.parse::<usize>().ok().filter(|index| *index < limit)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ObjectPickupGrant {
    pub kind: ObjectPickupKind,
    pub amount: u8,
}

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
    NativeLava,
    Drowning,
}

impl WorldDamageEffect {
    pub fn from_key(key: &str) -> Option<Self> {
        match key.to_ascii_uppercase().as_str() {
            "LAVA" => Some(Self::Lava),
            "DROWNING" | "WATER" => Some(Self::Drowning),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Lava | Self::NativeLava => "lava",
            Self::Drowning => "drowning",
        }
    }

    pub fn allows_transport(self, transport: TransportState) -> bool {
        match self {
            Self::Lava => matches!(
                transport,
                TransportState::Carpet { .. } | TransportState::Balloon { .. }
            ),
            Self::NativeLava => matches!(
                transport,
                TransportState::Foot
                    | TransportState::Carpet { .. }
                    | TransportState::Balloon { .. }
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

    pub fn damages_transport(self, transport: TransportState) -> bool {
        match self {
            Self::Lava => matches!(transport, TransportState::Carpet { .. }),
            Self::NativeLava => {
                matches!(
                    transport,
                    TransportState::Foot | TransportState::Carpet { .. }
                )
            }
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

pub fn intrinsic_world_damage_tile_entry(
    plane: WorldPlane,
    x: usize,
    y: usize,
    tile: u8,
) -> Option<WorldDamageTileEntry> {
    is_lava_tile(tile).then_some(WorldDamageTileEntry {
        plane,
        x,
        y,
        effect: WorldDamageEffect::NativeLava,
        expected_tile: Some(tile),
    })
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
