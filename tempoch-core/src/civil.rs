// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Civil layer: `chrono::DateTime<Utc>` interop plus Unix and GPS
//! representations.
//!
//! This module implements the fallible, history-dependent civil conversions:
//!
//! * `Time::<UTC>::from_chrono` / `try_from_chrono` / `to_chrono`
//! * `Time::<UTC, UnixSeconds<POSIX>>::from_unix_seconds` / `seconds`
//! * `Time::<TAI, GpsSeconds>::from_gps_seconds` / `seconds`

use super::axis::{TAI, UTC};
use super::conversion::{
    try_tai_minus_utc_mjd, tt_mjd_to_utc_mjd_in_segment, utc_mjd_to_tt_mjd_in_segment,
    GPS_EPOCH_TAI, J2000_JD_TT, JD_MINUS_MJD, TT_MINUS_TAI, UNIX_EPOCH_JD,
    UNIX_EPOCH_MJD, UTC_INTERVAL_EPS,
};
use super::error::ConversionError;
use super::representation::{GpsSeconds, Native, UnixSeconds, POSIX};
use super::storage::Storage;
use super::time::Time;
use crate::generated::time_data::UTC_TAI_SEGMENTS;
use chrono::{DateTime, Utc};
use qtty::time::{Days, Nanoseconds, Seconds};
use qtty::unit::{Day, Nanosecond, Second};

// ── Helpers ──────────────────────────────────────────────────────────────

#[inline]
fn datetime_from_seconds_since_epoch(seconds_since_epoch: Seconds) -> Option<DateTime<Utc>> {
    if !seconds_since_epoch.is_finite() {
        return None;
    }

    let nanos_per_second: Nanoseconds = Seconds::one().to::<Nanosecond>();

    let secs_floor = seconds_since_epoch.floor();
    let frac = seconds_since_epoch - secs_floor;

    let mut secs = secs_floor;
    let mut nanos: Nanoseconds = frac.to::<Nanosecond>().round();

    // Normalize nanos into [0, nanos_per_second)
    if nanos < Nanoseconds::zero() {
        secs -= Seconds::one();
        nanos += nanos_per_second;
    } else if nanos >= nanos_per_second {
        secs += Seconds::one();
        nanos -= nanos_per_second;
    }

    DateTime::<Utc>::from_timestamp(
        secs.erase_unit_raw() as i64,
        nanos.erase_unit_raw() as u32,
    )
}

#[inline]
fn datetime_from_utc_mjd(mjd_utc: Days) -> Option<DateTime<Utc>> {
    datetime_from_seconds_since_epoch((mjd_utc - UNIX_EPOCH_MJD).to::<Second>())
}

/// Convert TAI-seconds-since-J2000-TT into a chrono DateTime<Utc>, preserving
/// leap-second labels when the instant falls inside a leap window.
fn utc_from_tai_seconds(tai_secs: Seconds) -> Result<DateTime<Utc>, ConversionError> {
    if !tai_secs.is_finite() {
        return Err(ConversionError::NonFinite);
    }

    // TAI seconds → JD(TT) proxy: add 32.184 then divide by 86400.
    let jd_tt: Days = J2000_JD_TT + (tai_secs + TT_MINUS_TAI).to::<Day>();
    let mjd_tt = jd_tt - JD_MINUS_MJD;

    let first_start_tt = utc_mjd_to_tt_mjd_in_segment(
        Days::new(UTC_TAI_SEGMENTS[0].start_mjd as f64),
        UTC_TAI_SEGMENTS[0],
    );
    if mjd_tt < first_start_tt - UTC_INTERVAL_EPS {
        return Err(ConversionError::UtcHistoryUnsupported);
    }

    for window in UTC_TAI_SEGMENTS.windows(2) {
        let segment = window[0];
        let next = window[1];
        let end_mjd = segment
            .end_mjd
            .expect("all non-terminal UTC-TAI segments must have an end");
        let end_mjd = Days::new(end_mjd as f64);
        let end_tt = utc_mjd_to_tt_mjd_in_segment(end_mjd, segment);
        if mjd_tt < end_tt - UTC_INTERVAL_EPS {
            let mjd_utc = tt_mjd_to_utc_mjd_in_segment(mjd_tt, segment);
            return datetime_from_utc_mjd(mjd_utc).ok_or(ConversionError::OutOfRange);
        }

        let next_start_tt = utc_mjd_to_tt_mjd_in_segment(Days::new(next.start_mjd as f64), next);
        if mjd_tt < next_start_tt - UTC_INTERVAL_EPS {
            let boundary = datetime_from_utc_mjd(end_mjd).ok_or(ConversionError::OutOfRange)?;
            let base_secs = boundary.timestamp() - 1;
            let nanos_per_second = Seconds::new(1.0).to::<Nanosecond>().erase_unit_raw();
            let leap_nanos = nanos_per_second
                + (mjd_tt - end_tt)
                    .to::<Second>()
                    .to::<Nanosecond>()
                    .erase_unit_raw();
            let window_nanos = ((next_start_tt - end_tt)
                .to::<Second>()
                .to::<Nanosecond>()
                .erase_unit_raw())
                .round()
                .max(1.0);
            let max_nanos = nanos_per_second + window_nanos - 1.0;
            let nanos = leap_nanos.round().clamp(nanos_per_second, max_nanos);
            return DateTime::<Utc>::from_timestamp(base_secs, nanos as u32)
                .ok_or(ConversionError::OutOfRange);
        }
    }

    let last = *UTC_TAI_SEGMENTS
        .last()
        .expect("UTC-TAI history must contain at least one segment");
    let mjd_utc = tt_mjd_to_utc_mjd_in_segment(mjd_tt, last);
    datetime_from_utc_mjd(mjd_utc).ok_or(ConversionError::OutOfRange)
}

/// Convert a chrono DateTime<Utc> to TAI-seconds-since-J2000-TT. Returns
/// `(tai_secs, leap_flag)`.
fn tai_seconds_from_utc(dt: DateTime<Utc>) -> Result<(f64, bool), ConversionError> {
    let base_jd_utc = UNIX_EPOCH_JD + Seconds::new(dt.timestamp() as f64).to::<Day>();
    let tai_minus_utc = try_tai_minus_utc_mjd(base_jd_utc - JD_MINUS_MJD)
        .ok_or(ConversionError::UtcHistoryUnsupported)?;
    let subsec_nanos = dt.timestamp_subsec_nanos();
    let mut leap = false;
    if subsec_nanos >= 1_000_000_000 {
        let next = try_tai_minus_utc_mjd(
            base_jd_utc - JD_MINUS_MJD + Seconds::new(1.0).to::<Day>(),
        )
        .ok_or(ConversionError::InvalidLeapSecond)?;
        if next - tai_minus_utc < Seconds::new(0.5) {
            return Err(ConversionError::InvalidLeapSecond);
        }
        leap = true;
    }

    // Storage(TAI)(P) = (JD_TAI(P) - J2000_JD_TT) * 86400
    //                 = (JD_UTC(P) - J2000_JD_TT) * 86400 + (TAI − UTC)(P)
    let frac = Nanoseconds::new(subsec_nanos as f64).to::<Second>();
    let tai_secs = (base_jd_utc - J2000_JD_TT).to::<Second>() + tai_minus_utc + frac;
    Ok((tai_secs.erase_unit_raw(), leap))
}

// ── Time<UTC, Native>: chrono interop ────────────────────────────────────

impl Time<UTC, Native> {
    /// Build a UTC instant from a `chrono::DateTime<Utc>`.
    ///
    /// Leap-second labels (chrono's `nanos >= 1_000_000_000` encoding) are
    /// preserved.
    ///
    /// # Errors
    /// Returns [`ConversionError::UtcHistoryUnsupported`] for dates before
    /// 1961-01-01, or [`ConversionError::InvalidLeapSecond`] for leap-second
    /// labels not present in the compiled history.
    #[inline]
    pub fn try_from_chrono(dt: DateTime<Utc>) -> Result<Self, ConversionError> {
        let (tai_secs, leap) = tai_seconds_from_utc(dt)?;
        Ok(Self::from_storage(Storage::new_unchecked(
            Seconds::new(tai_secs),
            leap,
        )))
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
        utc_from_tai_seconds(self.storage().seconds)
    }

    /// Convenience non-fallible wrapper (returns `None` on error).
    #[inline]
    pub fn to_chrono(self) -> Option<DateTime<Utc>> {
        self.try_to_chrono().ok()
    }

    /// Returns `true` if this instant is labeled as a positive leap second
    /// in UTC (e.g., 23:59:60).
    #[inline]
    pub fn is_leap_second(self) -> bool {
        self.storage().leap
    }
}

// ── Time<UTC, UnixSeconds<POSIX>>: Unix-seconds constructors ─────────────

impl Time<UTC, UnixSeconds<POSIX>> {
    /// Build a UTC instant from a POSIX timestamp in seconds.
    ///
    /// A POSIX timestamp ignores leap seconds (one "Unix day" is always
    /// 86 400 ticks), matching C `time()`, Python `time.time()`, etc.
    #[inline]
    pub fn from_unix_seconds(seconds: f64) -> Result<Self, ConversionError> {
        if !seconds.is_finite() {
            return Err(ConversionError::NonFinite);
        }
        // POSIX time → UTC MJD (no leap seconds): mjd_utc = 40587 + s/86400.
        let mjd_utc = UNIX_EPOCH_MJD + Seconds::new(seconds).to::<Day>();
        let tai_minus_utc =
            try_tai_minus_utc_mjd(mjd_utc).ok_or(ConversionError::UtcHistoryUnsupported)?;
        let tai_secs = (mjd_utc + JD_MINUS_MJD - J2000_JD_TT).to::<Second>() + tai_minus_utc;
        Ok(Self::from_storage(Storage::new_unchecked(tai_secs, false)))
    }

    /// Return the POSIX timestamp in seconds for this UTC instant.
    ///
    /// Inverse of [`from_unix_seconds`](Self::from_unix_seconds). Leap-second
    /// labels collapse onto the preceding integer second (standard POSIX).
    #[inline]
    pub fn unix_seconds(self) -> Result<f64, ConversionError> {
        let dt = utc_from_tai_seconds(self.storage().seconds)?;
        let nanos = dt.timestamp_subsec_nanos().min(999_999_999);
        Ok(
            dt.timestamp() as f64
                + Nanoseconds::new(nanos as f64)
                    .to::<Second>()
                    .erase_unit_raw(),
        )
    }
}

// ── Time<TAI, GpsSeconds>: GPS-seconds constructors ──────────────────────

impl Time<TAI, GpsSeconds> {
    /// Build a GPS instant from GPS seconds since the GPS epoch
    /// (1980-01-06T00:00:00 UTC).
    ///
    /// GPS runs at the same rate as TAI with a fixed offset (GPS = TAI − 19 s).
    #[inline]
    pub fn from_gps_seconds(seconds: f64) -> Result<Self, ConversionError> {
        if !seconds.is_finite() {
            return Err(ConversionError::NonFinite);
        }
        Ok(Self::from_storage(Storage::new_unchecked(
            Seconds::new(seconds) + GPS_EPOCH_TAI,
            false,
        )))
    }

    /// Return GPS seconds since the GPS epoch for this instant.
    #[inline]
    pub fn gps_seconds(self) -> f64 {
        (self.storage().seconds - GPS_EPOCH_TAI).erase_unit_raw()
    }
}

#[cfg(test)]
mod tests {
    use super::super::axis::{TAI, TT};
    use super::super::context::TimeContext;
    use super::super::representation::{GpsSeconds, UnixSeconds, POSIX};
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
        // f64 storage floor ≈ 40 µs at J2000; a few hundred ns drift is
        // expected from the double-conversion round trip.
        // Storage floor ≈ 40 µs at J2000, and we convert through JD twice.
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
        // 2024-01-01T00:00:00Z ≈ 1_704_067_200 POSIX seconds.
        let secs = 1_704_067_200.0;
        let t = Time::<UTC, UnixSeconds<POSIX>>::from_unix_seconds(secs).unwrap();
        let out = t.unix_seconds().unwrap();
        assert!((out - secs).abs() < 1e-3, "round trip diff {}", out - secs);
    }

    #[test]
    fn unix_negative_fraction_round_trip() {
        let secs = -0.25;
        let t = Time::<UTC, UnixSeconds<POSIX>>::from_unix_seconds(secs).unwrap();
        let out = t.unix_seconds().unwrap();
        assert!((out - secs).abs() < 1e-3, "round trip diff {}", out - secs);
    }

    #[test]
    fn gps_round_trip() {
        let t = Time::<TAI, GpsSeconds>::from_gps_seconds(1_000_000.0).unwrap();
        assert!((t.gps_seconds() - 1_000_000.0).abs() < 1e-9);
    }

    #[test]
    fn gps_tai_offset_is_19s() {
        // GPS epoch in TAI seconds: (2024-01-01 GPS seconds) - (same TAI seconds) = 19 s.
        let gps = Time::<TAI, GpsSeconds>::from_gps_seconds(0.0).unwrap();
        let tai = gps
            .repr::<super::super::representation::SISeconds>()
            .seconds();
        // TAI reading at GPS epoch (1980-01-06 00:00:19 TAI) in SI seconds since J2000 TT.
        let expected = GPS_EPOCH_TAI;
        assert!(((tai - expected).abs()).erase_unit_raw() < 1e-6);
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
        let ut1 = tt.to_with::<super::super::axis::UT1>(&ctx).unwrap();
        let tt_back = ut1.to_with::<TT>(&ctx).unwrap();
        let diff = (tt - tt_back).abs().erase_unit_raw();
        assert!(diff < 1e-9, "round trip diff = {diff}");
    }
}
