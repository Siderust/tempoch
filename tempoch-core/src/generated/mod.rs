// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Valles Puig, Ramon

use qtty::{Day, Second};

#[allow(dead_code)]
pub(crate) mod time_data;

pub(crate) const PRE_1961_TAI_MINUS_UTC_APPROX: Second =
    Second::new(time_data::PRE_1961_TAI_MINUS_UTC_APPROX);
pub(crate) const UTC_TAI_HISTORY_START_MJD: Day =
    Day::new(time_data::UTC_TAI_HISTORY_START_MJD as f64);
pub(crate) const MODERN_DELTA_T_START_MJD: Day = Day::new(time_data::MODERN_DELTA_T_START_MJD);
pub(crate) const MODERN_DELTA_T_END_MJD: Day = Day::new(time_data::MODERN_DELTA_T_END_MJD);

impl time_data::UtcTaiSegment {
    #[inline]
    pub(crate) fn start_mjd_days(self) -> Day {
        Day::new(self.start_mjd as f64)
    }

    #[inline]
    pub(crate) fn end_mjd_days(self) -> Option<Day> {
        self.end_mjd.map(|mjd| Day::new(mjd as f64))
    }

    #[inline]
    pub(crate) fn reference_mjd_days(self) -> Day {
        Day::new(self.reference_mjd)
    }

    #[inline]
    pub(crate) fn offset_at(self, mjd_utc: Day) -> Second {
        let utc_offset = mjd_utc - self.reference_mjd_days();
        Second::new(self.base_seconds)
            + Second::new(self.slope_seconds_per_day) * (utc_offset / Day::new(1.0))
    }
}
