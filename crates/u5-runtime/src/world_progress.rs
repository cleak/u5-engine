//! Clean-engine sidecar for durable world-progress mirrors and state without public save offsets.

use std::fs;
use std::io;
use std::path::Path;

use crate::*;

/// Clean-engine companion save file for semantic world state. Publicly mapped
/// `SAVED.GAM` fields are mirrored for compatibility with older clean saves;
/// state without public original offsets still lives here.
pub const WORLD_PROGRESS_STATE_FILE: &str = "SAVED.WPS";
pub const WORLD_PROGRESS_STATE_MAGIC: [u8; 4] = *b"WPS1";
pub const WORLD_PROGRESS_STATE_LEGACY_LEN: usize =
    4 + RARE_REAGENT_HARVEST_POINT_COUNT + FIXED_HIDDEN_TREASURE_FOUND_BYTES + 1 + SHADOWLORD_COUNT;
pub const WORLD_PROGRESS_STATE_LEN: usize = WORLD_PROGRESS_STATE_LEGACY_LEN + 1;
pub const WORLD_PROGRESS_STATE_LEGACY_SHRINE_STANDING_LEN: usize =
    WORLD_PROGRESS_STATE_LEGACY_LEN + VIRTUE_COUNT;
const WORLD_PROGRESS_STATE_SHRINE_STANDING_LEN: usize = WORLD_PROGRESS_STATE_LEN + VIRTUE_COUNT;

const RARE_REAGENT_DAYS_OFFSET: usize = 4;
const FIXED_HIDDEN_FOUND_OFFSET: usize =
    RARE_REAGENT_DAYS_OFFSET + RARE_REAGENT_HARVEST_POINT_COUNT;
const FIXED_HIDDEN_DAILY_DAY_OFFSET: usize =
    FIXED_HIDDEN_FOUND_OFFSET + FIXED_HIDDEN_TREASURE_FOUND_BYTES;
const FIXED_HIDDEN_SINGLE_USE_COOKIE_OFFSET: usize = FIXED_HIDDEN_DAILY_DAY_OFFSET + 1;
const SHADOWLORD_HIDEOUTS_LEGACY_OFFSET: usize = FIXED_HIDDEN_DAILY_DAY_OFFSET + 1;
const SHADOWLORD_HIDEOUTS_OFFSET: usize = FIXED_HIDDEN_SINGLE_USE_COOKIE_OFFSET + 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorldProgressState {
    pub rare_reagent_harvest_days: [u8; RARE_REAGENT_HARVEST_POINT_COUNT],
    pub fixed_hidden_treasure_mirror_present: bool,
    pub fixed_hidden_treasure_found: [u8; FIXED_HIDDEN_TREASURE_FOUND_BYTES],
    pub fixed_hidden_treasure_daily_day: u8,
    pub fixed_hidden_treasure_single_use_cookie: Option<u8>,
    pub shadowlord_mirror_present: bool,
    pub shadowlord_hideouts: [u8; SHADOWLORD_COUNT],
}

impl Default for WorldProgressState {
    fn default() -> Self {
        Self {
            rare_reagent_harvest_days: [RARE_REAGENT_HARVEST_UNSEEN_DAY;
                RARE_REAGENT_HARVEST_POINT_COUNT],
            fixed_hidden_treasure_mirror_present: false,
            fixed_hidden_treasure_found: [0; FIXED_HIDDEN_TREASURE_FOUND_BYTES],
            fixed_hidden_treasure_daily_day: FIXED_HIDDEN_TREASURE_DAILY_UNSEEN_DAY,
            fixed_hidden_treasure_single_use_cookie: Some(
                FIXED_HIDDEN_TREASURE_SINGLE_USE_COOKIE_CLEAR,
            ),
            shadowlord_mirror_present: false,
            shadowlord_hideouts: DEFAULT_SHADOWLORD_HIDEOUTS,
        }
    }
}

impl WorldProgressState {
    pub fn from_play_options(options: &PlayOptions) -> Self {
        Self {
            rare_reagent_harvest_days: options.rare_reagent_harvest_days,
            fixed_hidden_treasure_mirror_present: true,
            fixed_hidden_treasure_found: options.fixed_hidden_treasure_found,
            fixed_hidden_treasure_daily_day: options.fixed_hidden_treasure_daily_day,
            fixed_hidden_treasure_single_use_cookie: Some(
                options.fixed_hidden_treasure_single_use_cookie,
            ),
            shadowlord_mirror_present: true,
            shadowlord_hideouts: options.shadowlord_hideouts,
        }
    }

    pub fn from_play_state(state: &PlayState) -> Self {
        Self {
            rare_reagent_harvest_days: state.rare_reagent_harvest_days,
            fixed_hidden_treasure_mirror_present: true,
            fixed_hidden_treasure_found: state.fixed_hidden_treasure_found,
            fixed_hidden_treasure_daily_day: state.fixed_hidden_treasure_daily_day,
            fixed_hidden_treasure_single_use_cookie: Some(
                state.fixed_hidden_treasure_single_use_cookie,
            ),
            shadowlord_mirror_present: true,
            shadowlord_hideouts: state.shadowlord_hideouts,
        }
    }

    pub fn apply_to_play_options(self, options: &mut PlayOptions) {
        options.rare_reagent_harvest_days = self.rare_reagent_harvest_days;
        if self.fixed_hidden_treasure_mirror_present {
            options.fixed_hidden_treasure_found = self.fixed_hidden_treasure_found;
            options.fixed_hidden_treasure_daily_day = self.fixed_hidden_treasure_daily_day;
            if let Some(cookie) = self.fixed_hidden_treasure_single_use_cookie {
                options.fixed_hidden_treasure_single_use_cookie = cookie;
            }
        }
        if self.shadowlord_mirror_present {
            options.shadowlord_hideouts = self.shadowlord_hideouts;
        }
    }

    pub fn apply_sidecar_only_to_play_options(self, options: &mut PlayOptions) {
        options.rare_reagent_harvest_days = self.rare_reagent_harvest_days;
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
        bytes[FIXED_HIDDEN_SINGLE_USE_COOKIE_OFFSET] = self
            .fixed_hidden_treasure_single_use_cookie
            .unwrap_or(FIXED_HIDDEN_TREASURE_SINGLE_USE_COOKIE_CLEAR);
        bytes[SHADOWLORD_HIDEOUTS_OFFSET..SHADOWLORD_HIDEOUTS_OFFSET + SHADOWLORD_COUNT]
            .copy_from_slice(&self.shadowlord_hideouts);
        bytes
    }

    pub fn decode(bytes: &[u8]) -> io::Result<Self> {
        if bytes.len() != WORLD_PROGRESS_STATE_LEN
            && bytes.len() != WORLD_PROGRESS_STATE_LEGACY_LEN
            && bytes.len() != WORLD_PROGRESS_STATE_LEGACY_SHRINE_STANDING_LEN
            && bytes.len() != WORLD_PROGRESS_STATE_SHRINE_STANDING_LEN
        {
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
        let has_single_use_cookie = bytes.len() == WORLD_PROGRESS_STATE_LEN
            || bytes.len() == WORLD_PROGRESS_STATE_SHRINE_STANDING_LEN;
        let shadowlord_hideouts_offset = if has_single_use_cookie {
            SHADOWLORD_HIDEOUTS_OFFSET
        } else {
            SHADOWLORD_HIDEOUTS_LEGACY_OFFSET
        };
        let mut shadowlord_hideouts = [0; SHADOWLORD_COUNT];
        shadowlord_hideouts.copy_from_slice(
            &bytes[shadowlord_hideouts_offset..shadowlord_hideouts_offset + SHADOWLORD_COUNT],
        );

        Ok(Self {
            rare_reagent_harvest_days,
            fixed_hidden_treasure_mirror_present: true,
            fixed_hidden_treasure_found,
            fixed_hidden_treasure_daily_day: bytes[FIXED_HIDDEN_DAILY_DAY_OFFSET],
            fixed_hidden_treasure_single_use_cookie: has_single_use_cookie
                .then_some(bytes[FIXED_HIDDEN_SINGLE_USE_COOKIE_OFFSET]),
            shadowlord_mirror_present: true,
            shadowlord_hideouts,
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
