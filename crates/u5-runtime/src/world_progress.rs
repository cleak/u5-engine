//! Clean-engine sidecar for durable world-progress state without public save offsets.

use std::fs;
use std::io;
use std::path::Path;

use crate::*;

/// Clean-engine companion save file for semantic world state whose exact
/// original `SAVED.GAM` byte offsets are not yet public. The main save image
/// remains byte-preserving for unknown fields.
pub const WORLD_PROGRESS_STATE_FILE: &str = "SAVED.WPS";
pub const WORLD_PROGRESS_STATE_MAGIC: [u8; 4] = *b"WPS1";
pub const WORLD_PROGRESS_STATE_LEN: usize = 4
    + RARE_REAGENT_HARVEST_POINT_COUNT
    + FIXED_HIDDEN_TREASURE_FOUND_BYTES
    + 1
    + SHADOWLORD_COUNT
    + VIRTUE_COUNT;

const RARE_REAGENT_DAYS_OFFSET: usize = 4;
const FIXED_HIDDEN_FOUND_OFFSET: usize =
    RARE_REAGENT_DAYS_OFFSET + RARE_REAGENT_HARVEST_POINT_COUNT;
const FIXED_HIDDEN_DAILY_DAY_OFFSET: usize =
    FIXED_HIDDEN_FOUND_OFFSET + FIXED_HIDDEN_TREASURE_FOUND_BYTES;
const SHADOWLORD_HIDEOUTS_OFFSET: usize = FIXED_HIDDEN_DAILY_DAY_OFFSET + 1;
const SHRINE_STANDING_OFFSET: usize = SHADOWLORD_HIDEOUTS_OFFSET + SHADOWLORD_COUNT;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorldProgressState {
    pub rare_reagent_harvest_days: [u8; RARE_REAGENT_HARVEST_POINT_COUNT],
    pub fixed_hidden_treasure_found: [u8; FIXED_HIDDEN_TREASURE_FOUND_BYTES],
    pub fixed_hidden_treasure_daily_day: u8,
    pub shadowlord_hideouts: [u8; SHADOWLORD_COUNT],
    pub shrine_standing: [u8; VIRTUE_COUNT],
}

impl Default for WorldProgressState {
    fn default() -> Self {
        Self {
            rare_reagent_harvest_days: [RARE_REAGENT_HARVEST_UNSEEN_DAY;
                RARE_REAGENT_HARVEST_POINT_COUNT],
            fixed_hidden_treasure_found: [0; FIXED_HIDDEN_TREASURE_FOUND_BYTES],
            fixed_hidden_treasure_daily_day: FIXED_HIDDEN_TREASURE_DAILY_UNSEEN_DAY,
            shadowlord_hideouts: DEFAULT_SHADOWLORD_HIDEOUTS,
            shrine_standing: [0; VIRTUE_COUNT],
        }
    }
}

impl WorldProgressState {
    pub fn from_play_options(options: &PlayOptions) -> Self {
        Self {
            rare_reagent_harvest_days: options.rare_reagent_harvest_days,
            fixed_hidden_treasure_found: options.fixed_hidden_treasure_found,
            fixed_hidden_treasure_daily_day: options.fixed_hidden_treasure_daily_day,
            shadowlord_hideouts: options.shadowlord_hideouts,
            shrine_standing: options.shrine_standing,
        }
    }

    pub fn from_play_state(state: &PlayState) -> Self {
        Self {
            rare_reagent_harvest_days: state.rare_reagent_harvest_days,
            fixed_hidden_treasure_found: state.fixed_hidden_treasure_found,
            fixed_hidden_treasure_daily_day: state.fixed_hidden_treasure_daily_day,
            shadowlord_hideouts: state.shadowlord_hideouts,
            shrine_standing: state.shrine_standing,
        }
    }

    pub fn apply_to_play_options(self, options: &mut PlayOptions) {
        options.rare_reagent_harvest_days = self.rare_reagent_harvest_days;
        options.fixed_hidden_treasure_found = self.fixed_hidden_treasure_found;
        options.fixed_hidden_treasure_daily_day = self.fixed_hidden_treasure_daily_day;
        options.shadowlord_hideouts = self.shadowlord_hideouts;
        options.shrine_standing = self.shrine_standing;
    }

    pub fn encoded(self) -> [u8; WORLD_PROGRESS_STATE_LEN] {
        let mut bytes = [0; WORLD_PROGRESS_STATE_LEN];
        bytes[0..4].copy_from_slice(&WORLD_PROGRESS_STATE_MAGIC);
        bytes
            [RARE_REAGENT_DAYS_OFFSET..RARE_REAGENT_DAYS_OFFSET + RARE_REAGENT_HARVEST_POINT_COUNT]
            .copy_from_slice(&self.rare_reagent_harvest_days);
        bytes[FIXED_HIDDEN_FOUND_OFFSET
            ..FIXED_HIDDEN_FOUND_OFFSET + FIXED_HIDDEN_TREASURE_FOUND_BYTES]
            .copy_from_slice(&self.fixed_hidden_treasure_found);
        bytes[FIXED_HIDDEN_DAILY_DAY_OFFSET] = self.fixed_hidden_treasure_daily_day;
        bytes[SHADOWLORD_HIDEOUTS_OFFSET..SHADOWLORD_HIDEOUTS_OFFSET + SHADOWLORD_COUNT]
            .copy_from_slice(&self.shadowlord_hideouts);
        bytes[SHRINE_STANDING_OFFSET..SHRINE_STANDING_OFFSET + VIRTUE_COUNT]
            .copy_from_slice(&self.shrine_standing);
        bytes
    }

    pub fn decode(bytes: &[u8]) -> io::Result<Self> {
        if bytes.len() != WORLD_PROGRESS_STATE_LEN {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{WORLD_PROGRESS_STATE_FILE} must be {WORLD_PROGRESS_STATE_LEN} bytes, got {}",
                    bytes.len()
                ),
            ));
        }
        if bytes[0..4] != WORLD_PROGRESS_STATE_MAGIC[..] {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{WORLD_PROGRESS_STATE_FILE} has an invalid signature"),
            ));
        }

        let mut rare_reagent_harvest_days = [0; RARE_REAGENT_HARVEST_POINT_COUNT];
        rare_reagent_harvest_days.copy_from_slice(
            &bytes[RARE_REAGENT_DAYS_OFFSET
                ..RARE_REAGENT_DAYS_OFFSET + RARE_REAGENT_HARVEST_POINT_COUNT],
        );
        let mut fixed_hidden_treasure_found = [0; FIXED_HIDDEN_TREASURE_FOUND_BYTES];
        fixed_hidden_treasure_found.copy_from_slice(
            &bytes[FIXED_HIDDEN_FOUND_OFFSET
                ..FIXED_HIDDEN_FOUND_OFFSET + FIXED_HIDDEN_TREASURE_FOUND_BYTES],
        );
        let mut shadowlord_hideouts = [0; SHADOWLORD_COUNT];
        shadowlord_hideouts.copy_from_slice(
            &bytes[SHADOWLORD_HIDEOUTS_OFFSET..SHADOWLORD_HIDEOUTS_OFFSET + SHADOWLORD_COUNT],
        );
        let mut shrine_standing = [0; VIRTUE_COUNT];
        shrine_standing
            .copy_from_slice(&bytes[SHRINE_STANDING_OFFSET..SHRINE_STANDING_OFFSET + VIRTUE_COUNT]);

        Ok(Self {
            rare_reagent_harvest_days,
            fixed_hidden_treasure_found,
            fixed_hidden_treasure_daily_day: bytes[FIXED_HIDDEN_DAILY_DAY_OFFSET],
            shadowlord_hideouts,
            shrine_standing,
        })
    }
}

pub fn load_world_progress_state(game_dir: &Path) -> io::Result<WorldProgressState> {
    match fs::read(game_dir.join(WORLD_PROGRESS_STATE_FILE)) {
        Ok(bytes) => WorldProgressState::decode(&bytes),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(WorldProgressState::default()),
        Err(err) => Err(err),
    }
}

pub fn write_world_progress_state(game_dir: &Path, state: WorldProgressState) -> io::Result<()> {
    fs::write(game_dir.join(WORLD_PROGRESS_STATE_FILE), state.encoded())
}
