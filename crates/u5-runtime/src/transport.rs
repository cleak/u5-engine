//! Transport state (foot/horse/ship/skiff/carpet), pending-vehicle acquisitions, and board-vehicle candidates.

use crate::*;

/// `vehicles.md §3`: the exact overworld pier tile that automatically furls
/// an under-sail ship after a successful step onto it.
pub const OVERWORLD_PIER_TILE: u8 = 0x47;

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
    /// `vehicles.md §11` "Balloon boundary": "Settled, not merely
    /// untraced. Balloon sprites are catalog assets only. No value a
    /// balloon could occupy is written or read by any shipped binary, and
    /// Section 2 gives the arithmetic argument that closes the last route
    /// by which such a value could have been reached." §2 states it
    /// directly: "**There is no balloon and no sixth vehicle family.**"
    /// and "Do not model balloon art as a transport state." There is
    /// therefore deliberately no `Balloon` variant here; the balloon art
    /// band stays in the tile catalog only.
    ///
    /// `vehicles.md §2` marker `0x00`, "Party sprite suppressed": "The
    /// party is drawn as nothing. As a *persistent* state this is reached
    /// only by drowning when a ship is lost with no skiff and no carpet
    /// available; see Section 6."
    ///
    /// It is a real transport marker rather than an absence of one, and
    /// `systems/overworld.md` Section 6.2.4 lists it among the markers
    /// that take the whole-party damage pass, so it cannot be modelled as
    /// [`TransportState::Foot`].
    ///
    /// [`crate::transport_from_save_marker`] deliberately does **not**
    /// decode `0x00` into this variant: the shipped chargen template
    /// leaves the byte zero before the first overworld entry, and §2 names
    /// `0x1C` as "[t]he clean seed and default state". The variant is
    /// reached at runtime, by the ladder, and a save that carries `0x00`
    /// loads as foot.
    SpriteSuppressed,
}

/// `vehicles.md §6` loss-of-ship ladder: "When a frigate is destroyed, the
/// party is not simply killed. The engine walks a fixed fallback ladder
/// and takes the first option that is available."
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShipLossFallback {
    /// "**A skiff is aboard.** The party abandons into a skiff, keeping
    /// the ship's current facing, and the marker becomes the matching
    /// skiff value."
    Skiff,
    /// "**Otherwise, a carpet is in stock.** The party deploys a carried
    /// carpet, the carried-carpet count is decremented, and the marker
    /// becomes one of the two carpet frames (chosen at random, since the
    /// frame is cosmetic)."
    Carpet,
    /// "**Otherwise, the party drowns.** The marker is set to the
    /// sprite-suppressed value and the drowning outcome runs. This is the
    /// only way the suppressed value becomes persistent state."
    Drown,
}

/// `vehicles.md §6`: pick the ladder rung. The order is fixed and the
/// first available option wins.
pub const fn ship_loss_fallback(skiffs_aboard: u8, carried_carpets: u8) -> ShipLossFallback {
    if skiffs_aboard > 0 {
        ShipLossFallback::Skiff
    } else if carried_carpets > 0 {
        ShipLossFallback::Carpet
    } else {
        ShipLossFallback::Drown
    }
}

/// `vehicles.md §2`: the two magic-carpet marker frames. The loss-of-ship
/// ladder picks between them at random "since the frame is cosmetic".
pub const CARPET_MARKER_FRAMES: [u8; 2] = [0x14, 0x15];

/// `vehicles.md §2` marker `0x00`, the sprite-suppressed party.
pub const TRANSPORT_MARKER_SPRITE_SUPPRESSED: u8 = 0x00;

/// `vehicles.md §6` drowning-loop exit scan: the loop continues only while
/// at least one Good, Poisoned, or Sleeping member remains.
///
/// This is deliberately **not** the same test as
/// [`crate::outdoor_impact_damages_member`], which skips only the dead
/// marker. `vehicles.md §6` now resolves the difference: shipped gameplay
/// writes only G/P/S/D, but imported saves may preserve another status. Such
/// a member is damaged while a G/P/S member holds the loop open, but does not
/// itself keep the loop alive and can therefore survive the helper's return.
pub const fn party_member_counts_as_living(status: u8) -> bool {
    matches!(status, b'G' | b'P' | b'S')
}

/// `audio.md §8.9`: exact ship-loss presentation lines.
pub const SHIP_SUNK_MESSAGE: &str = "Ship sunk!";
pub const ABANDON_SHIP_MESSAGE: &str = "Abandon ship!";
pub const DROWNING_MESSAGE: &str = "DROWNING!!!";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PendingVehicleAcquisition {
    Frigate { x: usize, y: usize, skiffs: u8 },
    Skiff { x: usize, y: usize, aux3: u8 },
}

/// The three-byte queued shipwright-delivery state persisted exclusively in
/// `SAVED.GAM` (`formats/saved-gam.md §10`). The class byte is retained
/// verbatim so inactive and noncanonical packed values survive load/save.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PendingVehicleSaveState {
    pub x: u8,
    pub y: u8,
    pub class_byte: u8,
}

impl PendingVehicleSaveState {
    pub const fn acquisition(self) -> Option<PendingVehicleAcquisition> {
        if self.class_byte < 0x40 {
            None
        } else if self.class_byte < 0x80 {
            Some(PendingVehicleAcquisition::Skiff {
                x: self.x as usize,
                y: self.y as usize,
                aux3: self.class_byte & 0x3f,
            })
        } else {
            Some(PendingVehicleAcquisition::Frigate {
                x: self.x as usize,
                y: self.y as usize,
                // Retain bit 6 as well as the delivered low-six-bit count.
                // This lets a subsequent shipwright increment reproduce the
                // specified whole-class-byte increment for 0xBF..=0xFF.
                skiffs: self.class_byte & 0x7f,
            })
        }
    }

    pub const fn from_acquisition(pending: PendingVehicleAcquisition) -> Self {
        match pending {
            PendingVehicleAcquisition::Skiff { x, y, aux3 } => Self {
                x: x as u8,
                y: y as u8,
                class_byte: 0x40 | (aux3 & 0x3f),
            },
            PendingVehicleAcquisition::Frigate { x, y, skiffs } => Self {
                x: x as u8,
                y: y as u8,
                class_byte: 0x80 | (skiffs & 0x7f),
            },
        }
    }

    /// Delivery clears only the packed class byte; coordinates remain as
    /// opaque saved state.
    pub const fn clear_class(self) -> Self {
        Self {
            class_byte: 0,
            ..self
        }
    }
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

/// `vehicles.md` shipwright frigate purchase initial state: hull
/// condition **99** and two skiffs aboard. Buying a standalone skiff
/// while the frigate is still queued increments the same carried-skiff
/// payload instead of placing a second active-object slot.
///
/// This was `100`, described in the doc as "full hull condition". The
/// spec says 99 in two separate places, and "full" was our inference
/// from the round number rather than anything published - 99 is simply
/// the value the purchase writes.
pub const FRIGATE_PURCHASE_HULL: u8 = 99;
pub const FRIGATE_PURCHASE_SKIFFS: u8 = 2;

/// `vehicles.md §4`: on a successful ship board the handler copies the
/// selected ship object's byte `+5` hull condition and byte `+7` skiff
/// count into the active vehicle state, then "warns if hull condition
/// is below ten, warns if no skiffs are aboard" — **two independent
/// warnings issued by the one boarding path**. Both are presentation
/// only; the ship boards either way.
pub const SHIP_BOARDING_HULL_WARNING_THRESHOLD: u8 = 10;

/// `vehicles.md §4`: which of the two ship-boarding warnings fired.
/// Both can fire on the same board (a battered hull with no skiffs
/// aboard warns twice), so this is a pair of flags rather than one
/// bool — collapsing them with `||` would tell a caller that the
/// player was warned but not what about.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ShipBoardingWarnings {
    /// Hull condition is strictly below
    /// [`SHIP_BOARDING_HULL_WARNING_THRESHOLD`].
    pub low_hull: bool,
    /// No skiffs are aboard.
    pub no_skiffs: bool,
}

impl ShipBoardingWarnings {
    /// `true` when the board issues at least one warning.
    pub const fn any(self) -> bool {
        self.low_hull || self.no_skiffs
    }
}

/// `vehicles.md §4`: classify the two ship-boarding warnings for the
/// boarded ship's hull condition and carried-skiff count.
pub const fn ship_boarding_warnings(hull: u8, skiffs: u8) -> ShipBoardingWarnings {
    ShipBoardingWarnings {
        low_hull: hull < SHIP_BOARDING_HULL_WARNING_THRESHOLD,
        no_skiffs: skiffs == 0,
    }
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
/// The support set is transport-like: saved transport-marker families,
/// carpets, riderless horses, manually handled ships, and skiffs.
pub fn vehicle_exit_object_support(object: ActiveObject) -> bool {
    if transport_family(object.type_byte).is_some() {
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
        | Some(TransportState::Foot)
        | Some(TransportState::SpriteSuppressed)
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
                aux3: skiffs & 0x3f,
            },
            Self::Skiff { x, y, aux3 } => ActiveObject {
                type_byte: SKIFF_PARKED_FIRST,
                tile: FIRST_PLAYABLE_SKIFF_TILE,
                x,
                y,
                z,
                phase: STEADY_PHASE,
                aux1: 0,
                aux3,
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

    /// Return the same transport with ship sails furled. Other transport
    /// families are unchanged so callers can apply the pier rule directly.
    pub fn with_ship_sails_furled(self) -> Self {
        match self {
            Self::Ship {
                type_byte,
                tile,
                hull,
                skiffs,
                ..
            } => Self::Ship {
                type_byte,
                tile,
                sails_hoisted: false,
                hull,
                skiffs,
            },
            other => other,
        }
    }

    /// `vehicles.md §11` "Balloon boundary": balloon sprites are
    /// "catalog assets only" and there is no balloon transport family, so
    /// this is always `false`.
    ///
    /// Retained only because `play_state_impl/chunk_07.rs` still calls it
    /// from `vehicle_can_park_at_current_cell`; that call site (and this
    /// shim with it) should be deleted by that file's owner.
    pub fn is_balloon(self) -> bool {
        false
    }

    pub fn avatar_tile(self) -> u8 {
        match self {
            Self::Foot => PLAYER_TILE,
            Self::Horse { tile, .. }
            | Self::Ship { tile, .. }
            | Self::Skiff { tile, .. }
            | Self::Carpet { tile, .. } => tile,
            // vehicles.md §2: "The party is drawn as nothing."
            Self::SpriteSuppressed => TRANSPORT_MARKER_SPRITE_SUPPRESSED,
        }
    }

    pub fn avatar_tile_with_facing(self, facing: Direction) -> u8 {
        transport_visual_tile_for_marker(self.save_marker_with_facing(facing))
            .unwrap_or_else(|| self.avatar_tile())
    }

    pub fn kind_name(self) -> &'static str {
        match self {
            Self::Foot => "foot",
            Self::Horse { .. } => "horse",
            Self::Ship { .. } => "ship",
            Self::Skiff { .. } => "skiff",
            Self::Carpet { .. } => "carpet",
            Self::SpriteSuppressed => "drowned",
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
            Self::SpriteSuppressed => "sprite suppressed".to_string(),
        }
    }

    pub fn can_board(self, target: Self) -> bool {
        match target {
            Self::Ship { .. } => ship_boarding_precondition_accepts(self.save_marker()),
            Self::Horse { .. } | Self::Skiff { .. } | Self::Carpet { .. } => self.is_foot(),
            // `vehicles.md §4`: "No balloon object byte is accepted by the
            // traced B-Board handler" -- and §2 removes the family
            // entirely, so there is no balloon target to reject here.
            Self::Foot | Self::SpriteSuppressed => false,
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
            Self::SpriteSuppressed => TRANSPORT_MARKER_SPRITE_SUPPRESSED,
        }
    }

    pub fn save_marker_with_facing(self, facing: Direction) -> u8 {
        let marker = self.save_marker();
        transport_marker_with_facing(marker, facing).unwrap_or(marker)
    }

    pub fn with_facing(self, facing: Direction) -> Self {
        let marker = self.save_marker_with_facing(facing);
        let tile = transport_visual_tile_for_marker(marker);
        match self {
            Self::Foot => Self::Foot,
            Self::Horse { tile: old_tile, .. } => Self::Horse {
                type_byte: marker,
                tile: tile.unwrap_or(old_tile),
            },
            Self::Ship {
                tile: old_tile,
                sails_hoisted,
                hull,
                skiffs,
                ..
            } => Self::Ship {
                type_byte: marker,
                tile: tile.unwrap_or(old_tile),
                sails_hoisted,
                hull,
                skiffs,
            },
            Self::Skiff { tile: old_tile, .. } => Self::Skiff {
                type_byte: marker,
                tile: tile.unwrap_or(old_tile),
            },
            Self::Carpet { tile: old_tile, .. } => Self::Carpet {
                type_byte: marker,
                tile: tile.unwrap_or(old_tile),
            },
            Self::SpriteSuppressed => Self::SpriteSuppressed,
        }
    }

    pub fn parked_object(self, x: usize, y: usize, z: i8) -> Option<ActiveObject> {
        let (type_byte, tile, aux1, aux3) = match self {
            Self::Foot | Self::SpriteSuppressed => return None,
            Self::Horse { type_byte, tile } => {
                let parked_type = if (HORSE_MOUNTED_FIRST..=HORSE_MOUNTED_LAST).contains(&type_byte)
                {
                    type_byte - HORSE_BOARDING_BIAS
                } else {
                    type_byte
                };
                (parked_type, tile, 0, 0)
            }
            Self::Skiff { type_byte, tile } => (type_byte, tile, 0, 0),
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
