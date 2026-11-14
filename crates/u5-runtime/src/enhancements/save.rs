//! The two read-only helpers the menu's slot save needs (Phase B design
//! 3.3.4 "Synthetic input", 3.3.5; build plan M4-10).
//!
//! - [`PlayState::save_command_available`] says whether a `Q` typed now
//!   would open the game's own `Save game?` prompt. The menu greys Save
//!   when it is false, so it never types a `Q` that would do anything
//!   else. A test feeds `Q` to a clone of every route-smoke state and
//!   checks the answer against the dispatcher.
//! - [`save_image_summary`] reads what a slot lists from a `SAVED.GAM`
//!   image: the Avatar's name, the date and time, and where the party
//!   stands. It is used for a slot made without a `Q` (an import, the
//!   command line) and to check a menu save.
//!
//! Neither writes anything, and neither adds a field to `PlayState`.

use std::io;

use crate::{
    Area, DungeonScene, GameClock, NATURAL_MOONGATE_TERRAIN_TILE, PlayState, PlayTarget,
    SAVE_AVATAR_NAME_OFFSET, SAVE_CHARACTER_NAME_LEN, SAVE_DAY_OFFSET, SAVE_HOUR_OFFSET,
    SAVE_MINUTE_OFFSET, SAVE_MONTH_OFFSET, SAVE_PARTY_SIZE_OFFSET, SAVE_SCENE_OFFSET,
    SAVE_YEAR_OFFSET, SAVE_Z_OFFSET, SAVED_GAM_LEN, Scene, WorldPlane,
    is_dungeon_room_helper_state, is_dungeon_room_trigger,
};

impl PlayState {
    /// Whether a `Q` typed now opens the `Save game?` prompt
    /// (`commands.md §4`, the `Q` row; `save-load.md §5.2`).
    ///
    /// It is answered at the idle exploration prompt, the only place the
    /// menu is open, and is false everywhere else: in combat
    /// (`save-load.md §5.1`), under any prompt or session, and while
    /// anything is pending. At the idle prompt it is false where the
    /// dispatcher would do something else with the key first:
    ///
    /// - in a town while the party is drunk, where each command takes an
    ///   even-odds gate that may turn it into a stagger
    ///   (`systems/shops.md`, tavern drunkenness);
    /// - on a live natural moongate, whose entry hook runs before the key
    ///   is dispatched (`overworld.md §9.2`);
    /// - on a dungeon room trigger, which the dungeon loop resolves before
    ///   the command (`dungeon-mode.md §5`).
    ///
    /// The drunk case is a coin toss, so a typed `Q` may still reach the
    /// prompt there; the menu does not take the chance.
    pub fn save_command_available(&self) -> bool {
        if !self.at_idle_command_prompt() {
            return false;
        }
        match self.area {
            Area::Town { .. } => self.town_drunkenness_counter == 0,
            Area::World { .. } => !self.live_moongate_hook_due(),
            Area::Dungeon { level, .. } => {
                let tile = self.dungeon_cell(level, self.player.x, self.player.y);
                !is_dungeon_room_trigger(tile) && !is_dungeon_room_helper_state(tile)
            }
        }
    }

    /// The natural-moongate entry hook would run before the next command.
    fn live_moongate_hook_due(&self) -> bool {
        !self.natural_moongate_hook_held_for_warp_arrival
            && self
                .current_live_natural_moongate_cell_index()
                .is_some_and(|idx| self.grid.get(idx) == Some(&NATURAL_MOONGATE_TERRAIN_TILE))
    }
}

/// What a slot lists about a save, read from its `SAVED.GAM` image.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaveImageSummary {
    /// The Avatar's name as the save holds it; `None` for an empty save
    /// (no active game, `save-load.md §4.2`).
    pub avatar: Option<String>,
    /// How many travel in the party (`formats/saved-gam.md`), as stored.
    pub party_size: u8,
    /// The game's date and time, as stored.
    pub clock: GameClock,
    /// The scene the party stands in.
    pub target: PlayTarget,
    /// The town floor or dungeon level (the save's `z`); the plane's floor
    /// outdoors.
    pub floor: i8,
}

/// Reads [`SaveImageSummary`] from a `SAVED.GAM` image. Fails on an image
/// of the wrong length or with a scene byte no save can hold, as the load
/// does. An empty save is not an error: its `avatar` is `None`.
pub fn save_image_summary(bytes: &[u8]) -> io::Result<SaveImageSummary> {
    if bytes.len() != SAVED_GAM_LEN {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("a save image is {SAVED_GAM_LEN} bytes, got {}", bytes.len()),
        ));
    }
    let name = &bytes[SAVE_AVATAR_NAME_OFFSET..SAVE_AVATAR_NAME_OFFSET + SAVE_CHARACTER_NAME_LEN];
    let name_len = name
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(name.len());
    let avatar = (name_len > 0).then(|| {
        name[..name_len]
            .iter()
            .map(|byte| {
                if byte.is_ascii_graphic() || *byte == b' ' {
                    char::from(*byte)
                } else {
                    '?'
                }
            })
            .collect::<String>()
    });
    let clock = GameClock {
        year: u16::from_le_bytes([bytes[SAVE_YEAR_OFFSET], bytes[SAVE_YEAR_OFFSET + 1]]),
        month: bytes[SAVE_MONTH_OFFSET],
        day: bytes[SAVE_DAY_OFFSET],
        hour: bytes[SAVE_HOUR_OFFSET],
        minute: bytes[SAVE_MINUTE_OFFSET],
    };
    let scene = bytes[SAVE_SCENE_OFFSET];
    let z = bytes[SAVE_Z_OFFSET];
    let (target, floor) = match scene {
        0 => {
            let plane = WorldPlane::from_save_z(z);
            (PlayTarget::World(plane), plane.save_floor())
        }
        1..=32 => (PlayTarget::Town(Scene::new(scene)?), z as i8),
        33..=40 if z <= 7 => (PlayTarget::Dungeon(DungeonScene::new(scene)?), z as i8),
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("the save's scene {scene} at level {z} is no place a save can hold"),
            ));
        }
    };
    Ok(SaveImageSummary {
        avatar,
        party_size: bytes[SAVE_PARTY_SIZE_OFFSET],
        clock,
        target,
        floor,
    })
}

#[cfg(test)]
mod tests;
