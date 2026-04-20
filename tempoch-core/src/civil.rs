// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Civil layer: `chrono::DateTime<Utc>` interop plus Unix and GPS
//! representations.

use super::constats::GPS_EPOCH_TAI;
use super::error::ConversionError;
use super::scale::{TAI, UTC};
use super::time::Time;
use crate::data::active::{
    active_time_data, time_data_tai_seconds_from_utc, time_data_tai_seconds_is_in_leap_window,
    time_data_try_tai_minus_utc_mjd, time_data_utc_from_tai_seconds,
};
use crate::encoding::{mjd_to_j2000_seconds, unix_seconds_to_mjd};
use chrono::{DateTime, Utc};
use qtty::Second;

impl Time<UTC> {
    /// Build a UTC instant from a `chrono::DateTime<Utc>`.
    #[inline]
    pub fn try_from_chrono(dt: DateTime<Utc>) -> Result<Self, ConversionError> {
        let data = active_time_data();
        let tai_secs = time_data_tai_seconds_from_utc(data.as_ref(), dt)?;
        Self::try_new(tai_secs, Second::new(0.0))
    }

    /// Convenience panicking wrapper over [`try_from_chrono`](Self::try_from_chrono).
    #[track_caller]
    #[inline]
    pub fn from_chrono(dt: DateTime<Utc>) -> Self {
        Self::try_from_chrono(dt).expect("UTC conversion failed; use try_from_chrono")
    }

    /// Convert to a `chrono::DateTime<Utc>`, preserving leap-second labels.
    #[inline]
    pub fn try_to_chrono(self) -> Result<DateTime<Utc>, ConversionError> {
        let data = active_time_data();
        time_data_utc_from_tai_seconds(data.as_ref(), self.total_seconds())
    }

    /// Convenience non-fallible wrapper (returns `None` on error).
    #[inline]
    pub fn to_chrono(self) -> Option<DateTime<Utc>> {
        self.try_to_chrono().ok()
    }

    /// Build a UTC instant from a POSIX timestamp in seconds.
    #[inline]
    pub fn from_unix_seconds(seconds: Second) -> Result<Self, ConversionError> {
        if !seconds.is_finite() {
            return Err(ConversionError::NonFinite);
        }
        let mjd_utc = unix_seconds_to_mjd(seconds);
        let data = active_time_data();
        let tai_minus_utc = time_data_try_tai_minus_utc_mjd(data.as_ref(), mjd_utc)
            .ok_or(ConversionError::UtcHistoryUnsupported)?;
        let tai_secs = mjd_to_j2000_seconds(mjd_utc) + tai_minus_utc;
        Self::try_new(tai_secs, Second::new(0.0))
    }

    /// Return the POSIX timestamp in seconds for this UTC instant.
    #[inline]
    pub fn unix_seconds(self) -> Result<Second, ConversionError> {
        let dt = self.try_to_chrono()?;
        let nanos = dt.timestamp_subsec_nanos().min(999_999_999);
        Ok(Second::new(dt.timestamp() as f64 + nanos as f64 / 1e9))
    }

    /// Returns `true` if this instant falls inside a positive leap second
    /// in UTC (e.g. 23:59:60).
    #[inline]
    pub fn is_leap_second(self) -> bool {
        let data = active_time_data();
        time_data_tai_seconds_is_in_leap_window(data.as_ref(), self.total_seconds())
    }
}

impl Time<TAI> {
    /// Build a TAI instant from GPS seconds since the GPS epoch.
    #[inline]
    pub fn from_gps_seconds(seconds: Second) -> Result<Self, ConversionError> {
        if !seconds.is_finite() {
            return Err(ConversionError::NonFinite);
        }
        Self::try_new(seconds + GPS_EPOCH_TAI, Second::new(0.0))
    }

    /// Return GPS seconds since the GPS epoch for this instant.
    #[inline]
    pub fn gps_seconds(self) -> Second {
        self.total_seconds() - GPS_EPOCH_TAI
    }
}
