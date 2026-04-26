// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Civil layer: `chrono::DateTime<Utc>` interop plus Unix and GPS
//! representations.

use super::constats::GPS_EPOCH_TAI;
use super::context::TimeContext;
use super::error::ConversionError;
use super::scale::{TAI, UTC};
use super::time::Time;
use crate::data::active::{
    time_data_tai_seconds_from_utc, time_data_tai_seconds_is_in_leap_window,
    time_data_try_tai_minus_utc_mjd, time_data_utc_from_tai_seconds,
};
use crate::encoding::{mjd_to_j2000_seconds, unix_seconds_to_mjd};
use chrono::{DateTime, Utc};
use qtty::Second;

impl Time<UTC> {
    /// Build a UTC instant from a `chrono::DateTime<Utc>` using the context's
    /// captured time-data bundle.
    #[inline]
    pub fn try_from_chrono_with(
        dt: DateTime<Utc>,
        ctx: &TimeContext,
    ) -> Result<Self, ConversionError> {
        let tai_secs =
            time_data_tai_seconds_from_utc(ctx.time_data(), dt, ctx.allows_pre_definition_utc())?;
        Self::try_new(tai_secs, Second::new(0.0))
    }

    /// Build a UTC instant from a `chrono::DateTime<Utc>`.
    ///
    /// Snapshots the active time-data bundle at call time via
    /// [`TimeContext::new`]. For reproducible pipelines, prefer
    /// [`try_from_chrono_with`](Self::try_from_chrono_with) with an explicit
    /// context.
    #[inline]
    pub fn try_from_chrono(dt: DateTime<Utc>) -> Result<Self, ConversionError> {
        Self::try_from_chrono_with(dt, &TimeContext::new())
    }

    /// Convenience panicking wrapper over
    /// [`try_from_chrono_with`](Self::try_from_chrono_with).
    #[track_caller]
    #[inline]
    pub fn from_chrono_with(dt: DateTime<Utc>, ctx: &TimeContext) -> Self {
        Self::try_from_chrono_with(dt, ctx)
            .expect("UTC conversion failed; use try_from_chrono_with")
    }

    /// Convenience panicking wrapper over [`try_from_chrono`](Self::try_from_chrono).
    ///
    /// Snapshots the active time-data bundle at call time via
    /// [`TimeContext::new`]. For reproducible pipelines, prefer
    /// [`from_chrono_with`](Self::from_chrono_with).
    #[track_caller]
    #[inline]
    pub fn from_chrono(dt: DateTime<Utc>) -> Self {
        Self::try_from_chrono(dt).expect("UTC conversion failed; use try_from_chrono")
    }

    /// Convert to a `chrono::DateTime<Utc>`, preserving leap-second labels,
    /// using the context's captured time-data bundle.
    #[inline]
    pub fn try_to_chrono_with(self, ctx: &TimeContext) -> Result<DateTime<Utc>, ConversionError> {
        time_data_utc_from_tai_seconds(
            ctx.time_data(),
            self.total_seconds(),
            ctx.allows_pre_definition_utc(),
        )
    }

    /// Convert to a `chrono::DateTime<Utc>`, preserving leap-second labels.
    ///
    /// Snapshots the active time-data bundle at call time via
    /// [`TimeContext::new`]. For reproducible pipelines, prefer
    /// [`try_to_chrono_with`](Self::try_to_chrono_with) with an explicit
    /// context.
    #[inline]
    pub fn try_to_chrono(self) -> Result<DateTime<Utc>, ConversionError> {
        self.try_to_chrono_with(&TimeContext::new())
    }

    /// Convenience non-fallible wrapper (returns `None` on error) using the
    /// context's captured time-data bundle.
    #[inline]
    pub fn to_chrono_with(self, ctx: &TimeContext) -> Option<DateTime<Utc>> {
        self.try_to_chrono_with(ctx).ok()
    }

    /// Convenience non-fallible wrapper (returns `None` on error).
    ///
    /// Snapshots the active time-data bundle at call time via
    /// [`TimeContext::new`]. For reproducible pipelines, prefer
    /// [`to_chrono_with`](Self::to_chrono_with).
    #[inline]
    pub fn to_chrono(self) -> Option<DateTime<Utc>> {
        self.try_to_chrono().ok()
    }

    /// Build a UTC instant from a POSIX timestamp in seconds using the
    /// context's captured time-data bundle.
    #[inline]
    pub(crate) fn from_raw_unix_seconds_with(
        seconds: Second,
        ctx: &TimeContext,
    ) -> Result<Self, ConversionError> {
        if !seconds.is_finite() {
            return Err(ConversionError::NonFinite);
        }
        let mjd_utc = unix_seconds_to_mjd(seconds);
        let tai_minus_utc = time_data_try_tai_minus_utc_mjd(
            ctx.time_data(),
            mjd_utc,
            ctx.allows_pre_definition_utc(),
        )?;
        let tai_secs = mjd_to_j2000_seconds(mjd_utc) + tai_minus_utc;
        Self::try_new(tai_secs, Second::new(0.0))
    }

    /// Return the POSIX timestamp in seconds for this UTC instant using the
    /// context's captured time-data bundle.
    #[inline]
    pub(crate) fn raw_unix_seconds_with(
        self,
        ctx: &TimeContext,
    ) -> Result<Second, ConversionError> {
        if self.is_leap_second_with(ctx) {
            return Err(ConversionError::InvalidLeapSecond);
        }
        let dt = self.try_to_chrono_with(ctx)?;
        let nanos = dt.timestamp_subsec_nanos();
        Ok(Second::new(dt.timestamp() as f64 + nanos as f64 / 1e9))
    }

    /// Returns `true` if this instant falls inside a positive leap second in
    /// UTC (e.g. 23:59:60) using the context's captured time-data bundle.
    #[inline]
    pub fn is_leap_second_with(self, ctx: &TimeContext) -> bool {
        time_data_tai_seconds_is_in_leap_window(ctx.time_data(), self.total_seconds())
    }

    /// Returns `true` if this instant falls inside a positive leap second
    /// in UTC (e.g. 23:59:60).
    ///
    /// Snapshots the active time-data bundle at call time via
    /// [`TimeContext::new`]. For reproducible pipelines, prefer
    /// [`is_leap_second_with`](Self::is_leap_second_with).
    #[inline]
    pub fn is_leap_second(self) -> bool {
        self.is_leap_second_with(&TimeContext::new())
    }
}

impl Time<TAI> {
    /// Build a TAI instant from GPS seconds since the GPS epoch.
    #[inline]
    pub(crate) fn from_raw_gps_seconds(seconds: Second) -> Result<Self, ConversionError> {
        if !seconds.is_finite() {
            return Err(ConversionError::NonFinite);
        }
        Self::try_new(seconds + GPS_EPOCH_TAI, Second::new(0.0))
    }

    /// Return GPS seconds since the GPS epoch for this instant.
    #[inline]
    pub(crate) fn raw_gps_seconds(self) -> Second {
        self.total_seconds() - GPS_EPOCH_TAI
    }
}
