//! In-game wall clock: year, month, day, hour, minute.

use std::collections::{HashMap, VecDeque};
use std::fs;
use std::io;
use std::path::Path;

use crate::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GameClock {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
}

impl GameClock {
    pub fn new(hour: u8, minute: u8) -> io::Result<Self> {
        Self::with_date(
            PLAY_START_YEAR,
            PLAY_START_MONTH,
            PLAY_START_DAY,
            hour,
            minute,
        )
    }

    pub fn with_date(year: u16, month: u8, day: u8, hour: u8, minute: u8) -> io::Result<Self> {
        if !(1..=13).contains(&month) || !(1..=28).contains(&day) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("invalid Britannian date year {year}, month {month}, day {day}"),
            ));
        }
        if hour > 23 || minute > 59 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("invalid clock time {hour:02}:{minute:02}"),
            ));
        }
        Ok(Self {
            year,
            month,
            day,
            hour,
            minute,
        })
    }

    pub fn advance_minutes(&mut self, minutes: u8) {
        let total = self.minute as u16 + minutes as u16;
        self.minute = (total % 60) as u8;
        for _ in 0..(total / 60) {
            self.advance_hour();
        }
    }

    pub fn display_hour(self) -> u8 {
        match self.hour {
            0 => 12,
            1..=12 => self.hour,
            _ => self.hour - 12,
        }
    }

    pub fn am_pm_suffix(self) -> &'static str {
        if self.hour < 12 { "A.M." } else { "P.M." }
    }

    pub fn advance_hour(&mut self) {
        self.hour += 1;
        if self.hour >= 24 {
            self.hour = 0;
            self.advance_day();
        }
    }

    pub fn advance_day(&mut self) {
        self.day += 1;
        if self.day > 28 {
            self.day = 1;
            self.month += 1;
            if self.month > 13 {
                self.month = 1;
                self.year = self.year.saturating_add(1);
            }
        }
    }
}

/// One of the three sky-strip markers per `moons.md` §2.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkyStripMarker {
    FixedHour,
    Trammel,
    Felucca,
}

/// Width of the sky/status strip per `moons.md` §2.
pub const SKY_STRIP_CELL_COUNT: u8 = 12;

/// `shops.md` §4.1 substitution placeholder `@` (and any caller that wants
/// the same time-of-day word): returns `"morning"` for hours `0..12`,
/// `"afternoon"` for hours `12..18`, and `"evening"` for hours `18..24`.
pub const fn shop_time_of_day_word(hour: u8) -> &'static str {
    if hour < 12 {
        "morning"
    } else if hour < 18 {
        "afternoon"
    } else {
        "evening"
    }
}

/// Per `moons.md` §2: compute the cell index `0..11` where the given marker
/// is visible at the given hour. Returns `None` when the marker is below the
/// strip's visible horizon.
pub fn sky_strip_marker_position(hour: u8, marker: SkyStripMarker) -> Option<u8> {
    let position = match marker {
        SkyStripMarker::FixedHour => match hour {
            6..=17 => Some(17u8.wrapping_sub(hour)),
            _ => None,
        },
        SkyStripMarker::Trammel => match hour {
            0..=8 => Some(8u8.wrapping_sub(hour)),
            21..=23 => Some(32u8.wrapping_sub(hour)),
            _ => None,
        },
        SkyStripMarker::Felucca => match hour {
            0..=2 => Some(2u8.wrapping_sub(hour)),
            15..=23 => Some(26u8.wrapping_sub(hour)),
            _ => None,
        },
    };
    position.filter(|cell| *cell < SKY_STRIP_CELL_COUNT)
}

impl Default for GameClock {
    fn default() -> Self {
        Self {
            year: PLAY_START_YEAR,
            month: PLAY_START_MONTH,
            day: PLAY_START_DAY,
            hour: PLAY_START_HOUR,
            minute: 0,
        }
    }
}
