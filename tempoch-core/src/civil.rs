// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Civil layer: `chrono::DateTime<Utc>` interop plus Unix and GPS
//! representations.
//!
//! This module implements the fallible, history-dependent civil conversions:
//!
//! * `Time::<UTC, F>::from_chrono` / `try_from_chrono` / `to_chrono`
//! * `Time::<UTC, F>::from_unix_seconds` / `unix_seconds`
//! * `Time::<TAI, F>::from_gps_seconds` / `gps_seconds`
//!
//! # UTC storage invariant
//!
//! `Time<UTC, F>` and `Time<TAI, F>` store **the same numerical value** for
//! the same physical instant (see the [`UTC`](super::scale::UTC) scale doc).
//! In particular, `.si_seconds()` on a `Time<UTC>` returns a continuous
//! TAI-based J2000 TT second count — **not** a UTC coordinate value and
//! **not** a POSIX timestamp. Use this civil API for any operation that
//! requires the discontinuous UTC-TAI offset (i.e., leap seconds):
//!
//! | Need                          | Method                             |
//! |-------------------------------|------------------------------------|
//! | chrono `DateTime<Utc>`        | `try_from_chrono` / `try_to_chrono`|
//! | POSIX timestamp               | `from_unix_seconds` / `unix_seconds`|
//! | GPS timestamp                 | `Time::<TAI>::from_gps_seconds`    |

use super::constats::GPS_EPOCH_TAI;
use super::encoding::{mjd_to_j2000_seconds, unix_seconds_to_mjd};
use super::error::ConversionError;
use super::format::Format;
use super::format::conversion::CanonicalRoundtrip;
use super::scale::{TAI, UTC};
use super::time::Time;
use crate::data::active::{
    active_time_data, time_data_tai_seconds_from_utc, time_data_tai_seconds_is_in_leap_window,
    time_data_try_tai_minus_utc_mjd, time_data_utc_from_tai_seconds,
};
use chrono::{DateTime, Utc};
use qtty::time::{Nanoseconds, Seconds};
use qtty::unit::Second as SecondUnit;
use qtty::Second;

// ── Time<UTC, F>: chrono interop ─────────────────────────────────────────

#[allow(private_bounds)]
impl<F: Format + CanonicalRoundtrip> Time<UTC, F> {
    /// Build a UTC instant from a `chrono::DateTime<Utc>`.
    ///
    /// Leap-second labels (chrono's `nanos >= 1_000_000_000` encoding) are
    /// handled correctly.
    ///
    /// # Errors
    /// Returns [`ConversionError::UtcHistoryUnsupported`] for dates before
    /// 1961-01-01, or [`ConversionError::InvalidLeapSecond`] for leap-second
    /// labels not present in the active UTC history.
    #[inline]
    pub fn try_from_chrono(dt: DateTime<Utc>) -> Result<Self, ConversionError> {
        let data = active_time_data();
        let tai_secs = time_data_tai_seconds_from_utc(data.as_ref(), dt)?;
        Ok(Self::new(F::from_j2000s(tai_secs)))
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
        let tai_secs = F::to_j2000s(self.value());
        let data = active_time_data();
        time_data_utc_from_tai_seconds(data.as_ref(), tai_secs)
    }

    /// Convenience non-fallible wrapper (returns `None` on error).
    #[inline]
    pub fn to_chrono(self) -> Option<DateTime<Utc>> {
        self.try_to_chrono().ok()
    }

    /// Returns `true` if this instant falls inside a positive leap second
    /// in UTC (e.g., 23:59:60).
    ///
    /// Recomputed from the active UTC-TAI segment table, so the result is stable
    /// even after a scale round-trip (e.g. `UTC→TT→UTC`).
    #[inline]
    pub fn is_leap_second(self) -> bool {
        let data = active_time_data();
        time_data_tai_seconds_is_in_leap_window(data.as_ref(), F::to_j2000s(self.value()))
    }
}

// ── Time<UTC, F>: Unix-seconds constructors ──────────────────────────────

#[allow(private_bounds)]
impl<F: Format + CanonicalRoundtrip> Time<UTC, F> {
    /// Build a UTC instant from a POSIX timestamp in seconds.
    ///
    /// A POSIX timestamp ignores leap seconds (one "Unix day" is always
    /// 86 400 ticks), matching C `time()`, Python `time.time()`, etc.
    #[inline]
    pub fn from_unix_seconds(seconds: Second) -> Result<Self, ConversionError> {
        if !seconds.is_finite() {
            return Err(ConversionError::NonFinite);
        }
        // POSIX time → UTC MJD (no leap seconds): mjd_utc = 40587 + s/86400.
        let mjd_utc = unix_seconds_to_mjd(seconds);
        let data = active_time_data();
        let tai_minus_utc = time_data_try_tai_minus_utc_mjd(data.as_ref(), mjd_utc)
            .ok_or(ConversionError::UtcHistoryUnsupported)?;
        let tai_secs = mjd_to_j2000_seconds(mjd_utc) + tai_minus_utc;
        Ok(Self::new(F::from_j2000s(tai_secs)))
    }

    /// Return the POSIX timestamp in seconds for this UTC instant.
    ///
    /// Inverse of [`from_unix_seconds`](Self::from_unix_seconds). Leap-second
    /// labels collapse onto the preceding integer second (standard POSIX).
    #[inline]
    pub fn unix_seconds(self) -> Result<Second, ConversionError> {
        let tai_secs = F::to_j2000s(self.value());
        let data = active_time_data();
        let dt = time_data_utc_from_tai_seconds(data.as_ref(), tai_secs)?;
        let nanos = dt.timestamp_subsec_nanos().min(999_999_999);
        Ok(Seconds::new(dt.timestamp() as f64) + Nanoseconds::new(nanos as f64).to::<SecondUnit>())
    }
}

// ── Time<TAI>: GPS-seconds constructors ──────────────────────────────────

impl Time<TAI> {
    /// Build a TAI instant from GPS seconds since the GPS epoch
    /// (1980-01-06T00:00:00 UTC).
    ///
    /// GPS runs at the same rate as TAI with a fixed offset (GPS = TAI − 19 s).
    #[inline]
    pub fn from_gps_seconds(seconds: Second) -> Result<Self, ConversionError> {
        if !seconds.is_finite() {
            return Err(ConversionError::NonFinite);
        }
        Ok(Self::new(seconds + GPS_EPOCH_TAI))
    }

    /// Return GPS seconds since the GPS epoch for this instant.
    #[inline]
    pub fn gps_seconds(self) -> Second {
        self.value() - GPS_EPOCH_TAI
    }
}

#[cfg(test)]
mod tests {
    use super::super::context::TimeContext;
    use super::super::scale::{TAI, TT};
    use super::*;
    use chrono::{NaiveDate, TimeZone};

    fn utc(year: i32, month: u32, day: u32, hour: u32, min: u32, sec: u32) -> DateTime<Utc> {
        Utc.from_utc_datetime(
            &NaiveDate::from_ymd_opt(year, month, day)
                .unwrap()
                .and_hms_opt(hour, min, sec)
                .unwrap(),
        )
    }

    #[test]
    fn chrono_round_trip_current_epoch() {
        let dt = utc(2024, 3, 14, 12, 34, 56);
        let t = Time::<UTC>::from_chrono(dt);
        let back = t.to_chrono().unwrap();
        let drift = (back - dt).num_nanoseconds().unwrap().abs();
        assert!(drift < 100_000, "round-trip drift = {drift} ns");
    }

    #[test]
    fn chrono_pre_1961_rejected() {
        let dt = utc(1800, 1, 1, 0, 0, 0);
        assert_eq!(
            Time::<UTC>::try_from_chrono(dt).unwrap_err(),
            ConversionError::UtcHistoryUnsupported
        );
    }

    #[test]
    fn unix_round_trip() {
        let secs = Second::new(1_704_067_200.0);
        let t = Time::<UTC>::from_unix_seconds(secs).unwrap();
        let out = t.unix_seconds().unwrap();
        assert!(
            (out - secs).abs() < Second::new(1e-3),
            "round trip diff {:?}",
            out - secs
        );
    }

    #[test]
    fn unix_negative_fraction_round_trip() {
        let secs = Second::new(-0.25);
        let t = Time::<UTC>::from_unix_seconds(secs).unwrap();
        let out = t.unix_seconds().unwrap();
        assert!(
            (out - secs).abs() < Second::new(1e-3),
            "round trip diff {:?}",
            out - secs
        );
    }

    #[test]
    fn gps_round_trip() {
        let t = Time::<TAI>::from_gps_seconds(Second::new(1_000_000.0)).unwrap();
        assert!((t.gps_seconds() - Second::new(1_000_000.0)).abs() < Second::new(1e-9));
    }

    #[test]
    fn gps_tai_offset_is_19s() {
        let gps = Time::<TAI>::from_gps_seconds(Second::new(0.0)).unwrap();
        let tai = gps.si_seconds();
        let expected = GPS_EPOCH_TAI;
        assert!((tai - expected).abs() < Second::new(1e-6));
    }

    #[test]
    fn leap_second_label_preserved() {
        // 2016-12-31T23:59:60 UTC was a real leap second.
        let base = utc(2016, 12, 31, 23, 59, 59);
        let dt_leap =
            chrono::DateTime::<Utc>::from_timestamp(base.timestamp(), 1_500_000_000).unwrap();
        let t = Time::<UTC>::from_chrono(dt_leap);
        assert!(t.is_leap_second());
        let out = t.to_chrono().unwrap();
        let drift = (out - dt_leap).num_nanoseconds().unwrap().abs();
        assert!(drift < 10_000, "leap-second drift = {drift} ns");
    }

    #[test]
    fn ut1_tt_round_trip_through_context() {
        let ctx = TimeContext::new();
        let tt = Time::<TT>::from_si_seconds(qtty::Second::new(0.0)).unwrap();
        let ut1 = tt.to_scale_with::<super::super::scale::UT1>(&ctx).unwrap();
        let tt_back = ut1.to_scale_with::<TT>(&ctx).unwrap();
        let diff = (tt - tt_back).abs();
        assert!(diff < Second::new(1e-9), "round trip diff = {:?}", diff);
    }
}
