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
