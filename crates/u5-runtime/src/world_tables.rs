//! Data structures for the world TSV tables (locations, plane transitions, get-tiles, pickups, waterfalls, damage, encounters, shrines).

use crate::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorldLocationEntry {
    pub plane: WorldPlane,
    pub x: usize,
    pub y: usize,
    pub target: PlayTarget,
    /// Retired sidecar compatibility column. Public issue #94 established
    /// that town entry ignores this value and always writes row 30.
    pub town_entry_y: Option<usize>,
    pub expected_tile: Option<u8>,
    /// Clean sidecars must publish this independently from the storage target.
    /// Stock rows always carry it; a missing extension value makes E-Enter
    /// reject the row as unrecognized rather than inferring presentation.
    pub narration_class: Option<WorldEntryNarrationClass>,
    /// Uppercase stock name printed on its own centered line, when any.
    pub proper_name: Option<&'static str>,
    /// Zero-based column in the sixteen-cell message window.
    pub name_column: Option<u8>,
    /// The seven ordinary dungeon rows accept the same coordinate on both
    /// outdoor planes. Doom is Underworld-only.
    pub accepts_both_world_planes: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorldEntryHelper {
    Town,
    Dungeon,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorldEntryNarrationClass {
    Hut,
    Keep,
    Village,
    Towne,
    Castle,
    Lighthouse,
    LordBritish,
    Blackthorn,
    Cave,
    Mine,
    Dungeon,
}

impl WorldEntryNarrationClass {
    pub const fn from_live_tile(tile: u8) -> Option<Self> {
        match tile {
            0x10 => Some(Self::Hut),
            0x12 => Some(Self::Keep),
            0x13 => Some(Self::Village),
            0x14 => Some(Self::Towne),
            0x15 => Some(Self::Castle),
            0x1B => Some(Self::Lighthouse),
            0x39 => Some(Self::Blackthorn),
            0x3E => Some(Self::LordBritish),
            0x16 => Some(Self::Cave),
            0x17 => Some(Self::Mine),
            0x18 => Some(Self::Dungeon),
            _ => None,
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        match key.to_ascii_uppercase().as_str() {
            "HUT" => Some(Self::Hut),
            "KEEP" => Some(Self::Keep),
            "VILLAGE" => Some(Self::Village),
            "TOWNE" | "TOWN" => Some(Self::Towne),
            "CASTLE" => Some(Self::Castle),
            "LIGHTHOUSE" => Some(Self::Lighthouse),
            "LORD_BRITISH" | "LORDBRITISH" => Some(Self::LordBritish),
            "BLACKTHORN" => Some(Self::Blackthorn),
            "CAVE" => Some(Self::Cave),
            "MINE" => Some(Self::Mine),
            "DUNGEON" => Some(Self::Dungeon),
            _ => None,
        }
    }

    pub const fn helper(self) -> WorldEntryHelper {
        match self {
            Self::Hut
            | Self::Keep
            | Self::Village
            | Self::Towne
            | Self::Castle
            | Self::Lighthouse
            | Self::LordBritish
            | Self::Blackthorn => WorldEntryHelper::Town,
            Self::Cave | Self::Mine | Self::Dungeon => WorldEntryHelper::Dungeon,
        }
    }

    pub const fn text(self) -> &'static str {
        match self {
            Self::Hut => "hut",
            Self::Keep => "keep",
            Self::Village => "village",
            Self::Towne => "towne",
            Self::Castle => "castle",
            Self::Lighthouse => "lighthouse",
            Self::LordBritish => "the Castle of Lord British!",
            Self::Blackthorn => "the palace of Blackthorn!",
            Self::Cave => "cave",
            Self::Mine => "mine",
            Self::Dungeon => "dungeon",
        }
    }
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
    /// `overworld.md` Section 8: the two scripted plane writers - the falls
    /// chain and the whirlpool - both reach the transition with the party's
    /// original transport marker deliberately restored, and neither "forces
    /// the durable post-transition transport marker to foot". Sidecar-driven
    /// transitions keep the ordinary reset.
    pub preserves_transport: bool,
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

    /// `vehicles.md §11` "Balloon boundary": "Settled, not merely
    /// untraced. Balloon sprites are catalog assets only... Do not invent
    /// boarding, landing, or wind-driven balloon movement." §2 adds
    /// "**There is no balloon and no sixth vehicle family.**"
    ///
    /// Each of the three rows below previously listed a balloon
    /// alternative. With no balloon family those alternatives are deleted
    /// outright rather than transferred: §3's family table gives the
    /// horse "normal terrain restrictions"/mounted-horse passability, the
    /// skiff the "facing-sensitive skiff predicate", the ship the ship
    /// predicate and the carpet "the carpet predicate family", and none of
    /// those rows names a lava or drowning immunity that the balloon row
    /// was standing in for. The surviving alternatives are exactly the
    /// ones that were already there for their own reasons.
    pub fn allows_transport(self, transport: TransportState) -> bool {
        match self {
            Self::Lava => matches!(transport, TransportState::Carpet { .. }),
            Self::NativeLava => matches!(
                transport,
                TransportState::Foot | TransportState::Carpet { .. }
            ),
            Self::Drowning => matches!(
                transport,
                TransportState::Foot
                    | TransportState::Ship { .. }
                    | TransportState::Skiff { .. }
                    | TransportState::Carpet { .. }
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
