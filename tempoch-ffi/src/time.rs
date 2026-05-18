// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Shared UTC civil-time carrier used by the split tempoch C ABI.

use chrono::{NaiveDate, Utc};

/// UTC date-time breakdown for C interop.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TempochUtc {
    /// Calendar year (e.g. 2026).
    pub year: i32,
    /// Month of the year (1–12).
    pub month: u8,
    /// Day of the month (1–31).
    pub day: u8,
    /// Hour of the day (0–23).
    pub hour: u8,
    /// Minute of the hour (0–59).
    pub minute: u8,
    /// Second of the minute (0–60). `60` denotes a positive leap second.
    pub second: u8,
    /// Sub-second component in nanoseconds (0–999_999_999).
    pub nanosecond: u32,
}

impl TempochUtc {
    pub(crate) fn into_chrono(self) -> Option<chrono::DateTime<Utc>> {
        let date = NaiveDate::from_ymd_opt(self.year, self.month as u32, self.day as u32)?;
        let (second, nanosecond) = if self.second == 60 {
            (59_u32, self.nanosecond.checked_add(1_000_000_000)?)
        } else {
            (self.second.into(), self.nanosecond)
        };
        let time =
            date.and_hms_nano_opt(self.hour.into(), self.minute.into(), second, nanosecond)?;
        Some(chrono::DateTime::<Utc>::from_naive_utc_and_offset(
            time, Utc,
        ))
    }

    pub(crate) fn from_chrono(dt: &chrono::DateTime<Utc>) -> Self {
        use chrono::{Datelike, Timelike};
        let (second, nanosecond) = if dt.nanosecond() >= 1_000_000_000 {
            (60_u8, dt.nanosecond() - 1_000_000_000)
        } else {
            (dt.second() as u8, dt.nanosecond())
        };
        Self {
            year: dt.year(),
            month: dt.month() as u8,
            day: dt.day() as u8,
            hour: dt.hour() as u8,
            minute: dt.minute() as u8,
            second,
            nanosecond,
        }
    }
}
