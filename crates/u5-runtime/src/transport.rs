//! Transport state (foot/horse/ship/skiff/carpet/balloon), pending-vehicle acquisitions, and board-vehicle candidates.

use crate::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TransportState {
    #[default]
    Foot,
    Horse {
        type_byte: u8,
        tile: u8,
    },
    Ship {
        type_byte: u8,
        tile: u8,
        sails_hoisted: bool,
        hull: u8,
        skiffs: u8,
    },
    Skiff {
        type_byte: u8,
        tile: u8,
    },
    Carpet {
        type_byte: u8,
        tile: u8,
    },
    Balloon {
        type_byte: u8,
        tile: u8,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PendingVehicleAcquisition {
    Frigate { x: usize, y: usize, skiffs: u8 },
    Skiff { x: usize, y: usize },
}

/// `vehicles.md §4` boardable-object byte ranges and their boarded
/// transport-marker results.
pub const HORSE_PARKED_FIRST: u8 = 0x10;
pub const HORSE_PARKED_LAST: u8 = 0x11;
pub const HORSE_MOUNTED_FIRST: u8 = 0x12;
pub const HORSE_MOUNTED_LAST: u8 = 0x13;
pub const CARPET_PARKED: u8 = 0x1B;
pub const CARPET_MOUNTED: u8 = 0x14;
pub const SHIP_PARKED_FIRST: u8 = 0x24;
pub const SHIP_PARKED_LAST: u8 = 0x27;
pub const SKIFF_PARKED_FIRST: u8 = 0x28;
pub const SKIFF_PARKED_LAST: u8 = 0x2B;

/// `vehicles.md §4` boardable family classification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoardableFamily {
    Horse,
    MagicCarpet,
    Ship,
    Skiff,
}

/// `vehicles.md §4`: classify a parked-object byte into its boardable
/// family, or `None` if the byte is not a boardable parked object. The
/// already-mounted ranges (`0x12..=0x13` mounted horses, `0x14` mounted
/// carpet) intentionally return `None`; they are caller live-state
/// markers, not parked objects to board.
pub const fn boardable_family(parked_byte: u8) -> Option<BoardableFamily> {
    Some(match parked_byte {
        HORSE_PARKED_FIRST..=HORSE_PARKED_LAST => BoardableFamily::Horse,
        CARPET_PARKED => BoardableFamily::MagicCarpet,
        SHIP_PARKED_FIRST..=SHIP_PARKED_LAST => BoardableFamily::Ship,
        SKIFF_PARKED_FIRST..=SKIFF_PARKED_LAST => BoardableFamily::Skiff,
        _ => return None,
    })
}

/// `vehicles.md §4`: rewrite a parked horse byte to its mounted-marker
/// counterpart by adding two (`0x10..=0x11` → `0x12..=0x13`). Returns
/// `None` if the input is not a parked horse byte.
pub const fn mount_horse_marker(parked_byte: u8) -> Option<u8> {
    if parked_byte == HORSE_PARKED_FIRST || parked_byte == HORSE_PARKED_LAST {
        Some(parked_byte + 2)
    } else {
        None
    }
}

/// `vehicles.md §4` shipwright frigate purchase initial state. A
/// freshly placed frigate carries full hull condition (100) and two
/// skiffs aboard; buying a standalone skiff while the frigate is
/// still queued increments the same carried-skiff payload instead
/// of placing a second active-object slot.
pub const FRIGATE_PURCHASE_HULL: u8 = 100;
pub const FRIGATE_PURCHASE_SKIFFS: u8 = 2;

/// `vehicles.md §3`: ship-boarding precondition — print a warning when
/// hull is below ten or no skiffs are aboard.
pub const SHIP_BOARDING_HULL_WARNING_THRESHOLD: u8 = 10;
pub const fn ship_boarding_warns(hull: u8, skiffs: u8) -> bool {
    hull < SHIP_BOARDING_HULL_WARNING_THRESHOLD || skiffs == 0
}

/// `vehicles.md §4` ship-boarding starting-state precondition. The
/// gate accepts the ordinary foot/avatar family `0x1C..=0x1F`, the
/// carpet north/east markers `0x14` and `0x15`, and the skiff family
/// `0x28..=0x2B`. Any other starting state produces the stock "On
/// foot" refusal with no state change.
pub const CARPET_BOARDING_NORTH_MARKER: u8 = 0x14;
pub const CARPET_BOARDING_EAST_MARKER: u8 = 0x15;
pub const fn ship_boarding_precondition_accepts(marker: u8) -> bool {
    matches!(
        marker,
        0x1C..=0x1F
            | CARPET_BOARDING_NORTH_MARKER
            | CARPET_BOARDING_EAST_MARKER
            | 0x28..=0x2B,
    )
}

/// `vehicles.md §4`: returns `true` for the two carpet-compatible
/// starting states that also bump the carried/stowed carpet counter
/// when boarding succeeds — only the north and east carpet markers.
/// South/west carpet markers do not stow on boarding.
pub const fn ship_boarding_stows_carpet(marker: u8) -> bool {
    matches!(
        marker,
        CARPET_BOARDING_NORTH_MARKER | CARPET_BOARDING_EAST_MARKER,
    )
}

/// `vehicles.md` section 5 / `doors-and-z-transitions.md` section 11:
/// selected nearby
/// active-object cells can support X-Xit without becoming the destination.
/// The support set is vehicle/party-like: carpets, riderless horses,
/// manually handled ships, skiffs, and party/avatar sentinels.
pub fn vehicle_exit_object_support(object: ActiveObject) -> bool {
    if object.is_player() {
        return true;
    }
    match transport_from_vehicle_object(object.type_byte, object.tile, object.aux1, object.aux3) {
        Some(TransportState::Horse { .. })
        | Some(TransportState::Skiff { .. })
        | Some(TransportState::Carpet { .. }) => true,
        Some(TransportState::Ship {
            sails_hoisted: false,
            ..
        }) => true,
        Some(TransportState::Ship {
            sails_hoisted: true,
            ..
        })
        | Some(TransportState::Balloon { .. })
        | Some(TransportState::Foot)
        | None => false,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BoardVehicleCandidate {
    pub slot: usize,
    pub transport: TransportState,
    pub blocked_by_occupant: bool,
}

impl PendingVehicleAcquisition {
    pub fn active_object(self, z: i8) -> ActiveObject {
        match self {
            Self::Frigate { x, y, skiffs } => ActiveObject {
                type_byte: SHIP_PARKED_FIRST,
                tile: FIRST_PLAYABLE_FRIGATE_TILE,
                x,
                y,
                z,
                phase: STEADY_PHASE,
                aux1: FIRST_PLAYABLE_FULL_SHIP_HULL,
                aux3: skiffs,
            },
            Self::Skiff { x, y } => ActiveObject {
                type_byte: SKIFF_PARKED_FIRST,
                tile: FIRST_PLAYABLE_SKIFF_TILE,
                x,
                y,
                z,
                phase: STEADY_PHASE,
                aux1: 0,
                aux3: 0,
            },
        }
    }
}

impl TransportState {
    pub fn is_foot(self) -> bool {
        matches!(self, Self::Foot)
    }

    pub fn is_horse(self) -> bool {
        matches!(self, Self::Horse { .. })
    }

    pub fn is_ship_under_sail(self) -> bool {
        matches!(
            self,
            Self::Ship {
                sails_hoisted: true,
                ..
            }
        )
    }

    pub fn is_balloon(self) -> bool {
        matches!(self, Self::Balloon { .. })
    }

    pub fn avatar_tile(self) -> u8 {
        match self {
            Self::Foot => PLAYER_TILE,
            Self::Horse { tile, .. }
            | Self::Ship { tile, .. }
            | Self::Skiff { tile, .. }
            | Self::Carpet { tile, .. }
            | Self::Balloon { tile, .. } => tile,
        }
    }

    pub fn kind_name(self) -> &'static str {
        match self {
            Self::Foot => "foot",
            Self::Horse { .. } => "horse",
            Self::Ship { .. } => "ship",
            Self::Skiff { .. } => "skiff",
            Self::Carpet { .. } => "carpet",
            Self::Balloon { .. } => "balloon",
        }
    }

    pub fn status_label(self) -> String {
        match self {
            Self::Foot => "foot".to_string(),
            Self::Horse { tile, .. } => format!("horse tile {tile}"),
            Self::Ship {
                tile,
                sails_hoisted,
                hull,
                skiffs,
                ..
            } => format!(
                "ship tile {tile} sails={} hull={hull} skiffs={skiffs}",
                if sails_hoisted { "hoisted" } else { "furled" }
            ),
            Self::Skiff { tile, .. } => format!("skiff tile {tile}"),
            Self::Carpet { tile, .. } => format!("magic carpet tile {tile}"),
            Self::Balloon { tile, .. } => format!("balloon tile {tile}"),
        }
    }

    pub fn can_board(self, target: Self) -> bool {
        match target {
            Self::Ship { .. } => ship_boarding_precondition_accepts(self.save_marker()),
            Self::Horse { .. } | Self::Skiff { .. } | Self::Carpet { .. } => self.is_foot(),
            Self::Balloon { .. } => false,
            Self::Foot => false,
        }
    }

    pub fn save_marker(self) -> u8 {
        match self {
            Self::Foot => FIRST_PLAYABLE_FOOT_TRANSPORT_MARKER,
            Self::Horse { type_byte, tile } => {
                transport_marker_for_vehicle_bytes(type_byte, tile, false)
                    .unwrap_or(FIRST_PLAYABLE_FOOT_TRANSPORT_MARKER)
            }
            Self::Ship {
                type_byte,
                tile,
                sails_hoisted,
                ..
            } => transport_marker_for_vehicle_bytes(type_byte, tile, sails_hoisted)
                .unwrap_or(FIRST_PLAYABLE_FOOT_TRANSPORT_MARKER),
            Self::Skiff { type_byte, tile } | Self::Carpet { type_byte, tile } => {
                transport_marker_for_vehicle_bytes(type_byte, tile, false)
                    .unwrap_or(FIRST_PLAYABLE_FOOT_TRANSPORT_MARKER)
            }
            Self::Balloon { .. } => FIRST_PLAYABLE_FOOT_TRANSPORT_MARKER,
        }
    }

    pub fn parked_object(self, x: usize, y: usize, z: i8) -> Option<ActiveObject> {
        let (type_byte, tile, aux1, aux3) = match self {
            Self::Foot => return None,
            Self::Horse { type_byte, tile } => {
                let parked_type = if (HORSE_MOUNTED_FIRST..=HORSE_MOUNTED_LAST).contains(&type_byte)
                {
                    type_byte - HORSE_BOARDING_BIAS
                } else {
                    type_byte
                };
                (parked_type, tile, 0, 0)
            }
            Self::Skiff { type_byte, tile } | Self::Balloon { type_byte, tile } => {
                (type_byte, tile, 0, 0)
            }
            Self::Carpet { type_byte, tile } => {
                let parked_type = if matches!(
                    transport_family(type_byte),
                    Some(TransportFamily::MagicCarpet)
                ) {
                    CARPET_PARKED
                } else {
                    type_byte
                };
                (parked_type, tile, 0, 0)
            }
            Self::Ship {
                type_byte,
                tile,
                hull,
                skiffs,
                ..
            } => {
                let parked_type = if matches!(
                    transport_family(type_byte),
                    Some(TransportFamily::ShipHoisted | TransportFamily::ShipFurled)
                ) {
                    transport_marker_for_vehicle_bytes(type_byte, tile, false).unwrap_or(type_byte)
                } else {
                    type_byte
                };
                (parked_type, tile, hull, skiffs)
            }
        };
        Some(ActiveObject {
            type_byte,
            tile,
            x,
            y,
            z,
            phase: STEADY_PHASE,
            aux1,
            aux3,
        })
    }

    pub fn append_ship_auxiliary_warnings(self, message: &mut String) {
        // vehicles.md §4: ship boarding warns when hull condition is below
        // ten and when no skiffs are aboard.
        if let Self::Ship { hull, .. } = self {
            if hull < 10 {
                message.push(' ');
                message.push_str(SHIP_BADLY_DAMAGED_WARNING);
            }
        }
        if let Self::Ship { skiffs: 0, .. } = self {
            message.push(' ');
            message.push_str(SHIP_NO_SKIFFS_WARNING);
        }
    }
}
