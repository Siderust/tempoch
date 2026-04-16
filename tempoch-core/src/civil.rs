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

use super::constats::{GPS_EPOCH_TAI, TT_MINUS_TAI, UTC_INTERVAL_EPS};
use super::encoding::{
    j2000_seconds_to_jd, jd_to_j2000_seconds, jd_to_mjd, mjd_to_j2000_seconds,
    mjd_to_unix_seconds, unix_seconds_to_jd, unix_seconds_to_mjd,
};
use super::error::ConversionError;
use super::format::Format;
use super::format_conversion::CanonicalRoundtrip;
use super::scale::{TAI, UTC};
use super::scale_conversion::{try_tai_minus_utc_mjd, tt_mjd_to_utc_mjd_in_segment, utc_mjd_to_tt_mjd_in_segment};
use super::time::Time;
use crate::generated::time_data::UTC_TAI_SEGMENTS;
use chrono::{DateTime, Utc};
use qtty::time::{Days, Nanoseconds, Seconds};
use qtty::unit::{Day, Nanosecond, Second as SecondUnit};
use qtty::Second;

const NANOS_PER_SECOND: Nanoseconds = Nanoseconds::new(1_000_000_000.0);

// ── Helpers ──────────────────────────────────────────────────────────────

#[inline]
fn datetime_from_seconds_since_epoch(seconds_since_epoch: Seconds) -> Option<DateTime<Utc>> {
    if !seconds_since_epoch.is_finite() {
        return None;
    }

    let secs_floor = seconds_since_epoch.floor();
    let frac = seconds_since_epoch - secs_floor;

    let mut secs = secs_floor;
    let mut nanos: Nanoseconds = frac.to::<Nanosecond>().round();

    // Normalize nanos into [0, NANOS_PER_SECOND)
    if nanos < Nanoseconds::zero() {
        secs -= Seconds::one();
        nanos += NANOS_PER_SECOND;
    } else if nanos >= NANOS_PER_SECOND {
        secs += Seconds::one();
        nanos -= NANOS_PER_SECOND;
    }

    DateTime::<Utc>::from_timestamp(
        (secs / Seconds::one()) as i64,
        (nanos / Nanoseconds::one()) as u32,
    )
}

#[inline]
fn datetime_from_utc_mjd(mjd_utc: Days) -> Option<DateTime<Utc>> {
    datetime_from_seconds_since_epoch(mjd_to_unix_seconds(mjd_utc))
}

/// Convert TAI-seconds-since-J2000-TT into a chrono DateTime<Utc>, preserving
/// leap-second labels when the instant falls inside a leap window.
fn utc_from_tai_seconds(tai_secs: Seconds) -> Result<DateTime<Utc>, ConversionError> {
    if !tai_secs.is_finite() {
        return Err(ConversionError::NonFinite);
    }

    // TAI seconds → JD(TT) proxy: add 32.184 then divide by 86400.
    let jd_tt: Days = j2000_seconds_to_jd(tai_secs + TT_MINUS_TAI);
    let mjd_tt = jd_to_mjd(jd_tt);

    let first_start_tt =
        utc_mjd_to_tt_mjd_in_segment(UTC_TAI_SEGMENTS[0].start_mjd_days(), UTC_TAI_SEGMENTS[0]);
    if mjd_tt < first_start_tt - UTC_INTERVAL_EPS {
        return Err(ConversionError::UtcHistoryUnsupported);
    }

    for window in UTC_TAI_SEGMENTS.windows(2) {
        let segment = window[0];
        let next = window[1];
        let end_mjd = segment
            .end_mjd_days()
            .expect("all non-terminal UTC-TAI segments must have an end");
        let end_tt = utc_mjd_to_tt_mjd_in_segment(end_mjd, segment);
        if mjd_tt < end_tt - UTC_INTERVAL_EPS {
            let mjd_utc = tt_mjd_to_utc_mjd_in_segment(mjd_tt, segment);
            return datetime_from_utc_mjd(mjd_utc).ok_or(ConversionError::OutOfRange);
        }

        let next_start_tt = utc_mjd_to_tt_mjd_in_segment(next.start_mjd_days(), next);
        if mjd_tt < next_start_tt - UTC_INTERVAL_EPS {
            let boundary = datetime_from_utc_mjd(end_mjd).ok_or(ConversionError::OutOfRange)?;
            let base_secs = boundary.timestamp() - 1;
            let leap_nanos: Nanoseconds =
                NANOS_PER_SECOND + (mjd_tt - end_tt).to::<SecondUnit>().to::<Nanosecond>();
            let window_nanos: Nanoseconds = (next_start_tt - end_tt)
                .to::<SecondUnit>()
                .to::<Nanosecond>()
                .round()
                .max(Nanoseconds::one());
            let max_nanos: Nanoseconds = NANOS_PER_SECOND + window_nanos - Nanoseconds::one();
            let nanos: Nanoseconds = leap_nanos.round().clamp(NANOS_PER_SECOND, max_nanos);
            return DateTime::<Utc>::from_timestamp(base_secs, (nanos / Nanoseconds::one()) as u32)
                .ok_or(ConversionError::OutOfRange);
        }
    }

    let last = *UTC_TAI_SEGMENTS
        .last()
        .expect("UTC-TAI history must contain at least one segment");
    let mjd_utc = tt_mjd_to_utc_mjd_in_segment(mjd_tt, last);
    datetime_from_utc_mjd(mjd_utc).ok_or(ConversionError::OutOfRange)
}

/// Return `true` when `tai_secs` (TAI-seconds-since-J2000-TT) falls inside a
/// positive leap-second window — i.e. the UTC clock would show 23:59:60.x at
/// that instant.
///
/// This recomputes the answer from the compiled UTC-TAI segment table.
fn tai_seconds_is_in_leap_window(tai_secs: Second) -> bool {
    let jd_tt: Days = j2000_seconds_to_jd(tai_secs + TT_MINUS_TAI);
    let mjd_tt = jd_to_mjd(jd_tt);
    for window in UTC_TAI_SEGMENTS.windows(2) {
        let segment = window[0];
        let next = window[1];
        let end_mjd = match segment.end_mjd_days() {
            Some(d) => d,
            None => continue,
        };
        let end_tt = utc_mjd_to_tt_mjd_in_segment(end_mjd, segment);
        let next_start_tt = utc_mjd_to_tt_mjd_in_segment(next.start_mjd_days(), next);
        // The leap window is [end_tt, next_start_tt); any TT value inside it
        // maps to 23:59:60.x in UTC.
        if mjd_tt >= end_tt - UTC_INTERVAL_EPS && mjd_tt < next_start_tt - UTC_INTERVAL_EPS {
            return true;
        }
    }
    false
}

/// Convert a chrono DateTime<Utc> to TAI-seconds-since-J2000-TT.
fn tai_seconds_from_utc(dt: DateTime<Utc>) -> Result<Second, ConversionError> {
    let base_jd_utc = unix_seconds_to_jd(Seconds::new(dt.timestamp() as f64));
    let tai_minus_utc = try_tai_minus_utc_mjd(jd_to_mjd(base_jd_utc))
        .ok_or(ConversionError::UtcHistoryUnsupported)?;
    let subsec_nanos = dt.timestamp_subsec_nanos();
    if subsec_nanos >= 1_000_000_000 {
        let next =
            try_tai_minus_utc_mjd(jd_to_mjd(base_jd_utc) + Seconds::new(1.0).to::<Day>())
                .ok_or(ConversionError::InvalidLeapSecond)?;
        if next - tai_minus_utc < Seconds::new(0.5) {
            return Err(ConversionError::InvalidLeapSecond);
        }
    }

    // TAI secs = (JD_UTC - J2000_JD_TT) * 86400 + (TAI - UTC)
    let frac = Nanoseconds::new(subsec_nanos as f64).to::<SecondUnit>();
    let tai_secs = jd_to_j2000_seconds(base_jd_utc) + tai_minus_utc + frac;
    Ok(tai_secs)
}

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
    /// labels not present in the compiled history.
    #[inline]
    pub fn try_from_chrono(dt: DateTime<Utc>) -> Result<Self, ConversionError> {
        let tai_secs = tai_seconds_from_utc(dt)?;
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
        utc_from_tai_seconds(tai_secs)
    }

    /// Convenience non-fallible wrapper (returns `None` on error).
    #[inline]
    pub fn to_chrono(self) -> Option<DateTime<Utc>> {
        self.try_to_chrono().ok()
    }

    /// Returns `true` if this instant falls inside a positive leap second
    /// in UTC (e.g., 23:59:60).
    ///
    /// Recomputed from the UTC-TAI segment table, so the result is stable
    /// even after a scale round-trip (e.g. `UTC→TT→UTC`).
    #[inline]
    pub fn is_leap_second(self) -> bool {
        tai_seconds_is_in_leap_window(F::to_j2000s(self.value()))
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
        let tai_minus_utc =
            try_tai_minus_utc_mjd(mjd_utc).ok_or(ConversionError::UtcHistoryUnsupported)?;
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
        let dt = utc_from_tai_seconds(tai_secs)?;
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
    use super::super::scale::{TAI, TT};
    use super::super::context::TimeContext;
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
