// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Civil layer: `chrono::DateTime<Utc>` interop plus Unix and GPS
//! representations.

use crate::data::runtime_data::{
    time_data_tai_seconds_from_utc, time_data_tai_seconds_is_in_leap_window,
    time_data_try_tai_minus_utc_mjd, time_data_utc_from_tai_seconds,
};
use crate::earth::context::TimeContext;
use crate::encoding::{day_to_j2000_seconds, unix_seconds_to_mjd};
use crate::format::TimeFormat;
use crate::format::MJD;
use crate::foundation::constats::GPS_EPOCH_TAI_SECONDS;
use crate::foundation::error::ConversionError;
use crate::model::scale::{TAI, UTC};
use crate::model::time::Time;
use chrono::{DateTime, Utc};
use qtty::Second;

impl<F: TimeFormat> Time<UTC, F> {
    /// Build a UTC instant from a `chrono::DateTime<Utc>` using the context's
    /// captured time-data bundle.
    #[inline]
    pub fn try_from_chrono_with(
        dt: DateTime<Utc>,
        ctx: &TimeContext,
    ) -> Result<Time<UTC, crate::format::J2000s>, ConversionError> {
        let tai_secs =
            time_data_tai_seconds_from_utc(ctx.time_data(), dt, ctx.allows_pre_definition_utc())?;
        Time::<UTC, crate::format::J2000s>::try_from_raw_j2000_seconds_split(
            tai_secs,
            Second::new(0.0),
        )
    }

    /// Build a UTC instant from a `chrono::DateTime<Utc>`.
    ///
    /// Snapshots the active time-data bundle at call time via
    /// [`TimeContext::new`]. For reproducible pipelines, prefer
    /// [`try_from_chrono_with`](Self::try_from_chrono_with) with an explicit
    /// context.
    #[inline]
    pub fn try_from_chrono(
        dt: DateTime<Utc>,
    ) -> Result<Time<UTC, crate::format::J2000s>, ConversionError> {
        Self::try_from_chrono_with(dt, &TimeContext::new())
    }

    /// Convenience panicking wrapper over
    /// [`try_from_chrono_with`](Self::try_from_chrono_with).
    #[track_caller]
    #[inline]
    pub fn from_chrono_with(
        dt: DateTime<Utc>,
        ctx: &TimeContext,
    ) -> Time<UTC, crate::format::J2000s> {
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
    pub fn from_chrono(dt: DateTime<Utc>) -> Time<UTC, crate::format::J2000s> {
        Self::try_from_chrono(dt).expect("UTC conversion failed; use try_from_chrono")
    }

    /// Convert to a `chrono::DateTime<Utc>`, preserving leap-second labels,
    /// using the context's captured time-data bundle.
    #[inline]
    pub fn try_to_chrono_with(self, ctx: &TimeContext) -> Result<DateTime<Utc>, ConversionError> {
        time_data_utc_from_tai_seconds(
            ctx.time_data(),
            self.to_j2000s().total_seconds(),
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
    ) -> Result<Time<UTC, crate::format::J2000s>, ConversionError> {
        if seconds.value().is_nan() {
            return Err(ConversionError::NonFinite);
        }
        let mjd_utc = unix_seconds_to_mjd(seconds);
        let tai_minus_utc = time_data_try_tai_minus_utc_mjd(
            ctx.time_data(),
            mjd_utc,
            ctx.allows_pre_definition_utc(),
        )?;
        let tai_secs = day_to_j2000_seconds::<MJD>(mjd_utc) + tai_minus_utc;
        Time::<UTC, crate::format::J2000s>::try_from_raw_j2000_seconds_split(
            tai_secs,
            Second::new(0.0),
        )
    }

    /// Return the POSIX timestamp in seconds for this UTC instant using the
    /// context's captured time-data bundle.
    #[inline]
    pub(crate) fn raw_unix_seconds_with(
        self,
        ctx: &TimeContext,
    ) -> Result<Second, ConversionError> {
        if self.to_j2000s().is_leap_second_with(ctx) {
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
        time_data_tai_seconds_is_in_leap_window(ctx.time_data(), self.to_j2000s().total_seconds())
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

impl<F: TimeFormat> Time<TAI, F> {
    /// Build a TAI instant from GPS seconds since the GPS epoch.
    #[inline]
    pub(crate) fn from_raw_gps_seconds(
        seconds: Second,
    ) -> Result<Time<TAI, crate::format::J2000s>, ConversionError> {
        if seconds.value().is_nan() {
            return Err(ConversionError::NonFinite);
        }
        Time::<TAI, crate::format::J2000s>::try_from_raw_j2000_seconds_split(
            seconds + GPS_EPOCH_TAI_SECONDS,
            Second::new(0.0),
        )
    }

    /// Return GPS seconds since the GPS epoch for this instant.
    #[inline]
    pub(crate) fn raw_gps_seconds(self) -> Second {
        self.to_j2000s().total_seconds() - GPS_EPOCH_TAI_SECONDS
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::runtime_data::{active_time_data, with_test_time_data};

    #[test]
    fn chrono_convenience_wrappers_roundtrip_with_context() {
        let bundle = active_time_data().as_ref().clone();
        with_test_time_data(bundle, || {
            let ctx = TimeContext::new();
            let dt = DateTime::from_timestamp(946_728_000, 125_000_000).unwrap();

            let with_ctx = Time::<UTC>::from_chrono_with(dt, &ctx);
            let default_ctx = Time::<UTC>::from_chrono(dt);
            assert_eq!(with_ctx, default_ctx);

            let back_with_ctx = with_ctx.to_chrono_with(&ctx).unwrap();
            let back_default = with_ctx.to_chrono().unwrap();
            let with_ctx_delta_ns =
                back_with_ctx.timestamp_nanos_opt().unwrap() - dt.timestamp_nanos_opt().unwrap();
            let default_delta_ns =
                back_default.timestamp_nanos_opt().unwrap() - dt.timestamp_nanos_opt().unwrap();

            assert!(with_ctx_delta_ns.abs() < 50_000);
            assert!(default_delta_ns.abs() < 50_000);
        });
    }

    #[test]
    fn gps_raw_seconds_reject_nan_and_roundtrip_finite() {
        assert!(matches!(
            Time::<TAI>::from_raw_gps_seconds(Second::new(f64::NAN)),
            Err(ConversionError::NonFinite)
        ));

        let tai = Time::<TAI>::from_raw_gps_seconds(Second::new(123.5)).unwrap();
        assert_eq!(tai.raw_gps_seconds(), Second::new(123.5));
    }
}
