// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Scale identifiers and scale-aware helpers for the scalar C ABI.
//!
//! The FFI exposes scalar time values as plain `double`s plus an explicit
//! [`TempochScaleId`] when generic dispatch is needed. This keeps the C ABI
//! small and regular while preserving the Rust crate's scale semantics in the
//! implementation.

use chrono::{DateTime, Utc};
use qtty::time::Seconds;
use qtty::Day;
use tempoch::{
    ConversionError, J2000s, JD, MJD, Time, TimeContext, TAI, TCB, TCG, TDB, TT, UT1, UTC,
};

const SECONDS_PER_DAY: f64 = 86_400.0;
const GPS_EPOCH_TAI_JD: f64 = 2_444_244.5 + 19.0 / SECONDS_PER_DAY;

/// Time scale identifier for generic dispatch functions.
///
/// In the C ABI, callers pass raw `int32_t` values and must validate them
/// before dispatch.
///
/// cbindgen:prefix-with-name
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TempochScaleId {
    /// Julian Date (TT), expressed in days.
    JD = 0,
    /// Modified Julian Date (TT), expressed in days.
    MJD = 1,
    /// Barycentric Dynamical Time, expressed as Julian days on the TDB axis.
    TDB = 2,
    /// Terrestrial Time, expressed as Julian days on the TT axis.
    TT = 3,
    /// International Atomic Time, expressed as Julian days on the TAI axis.
    TAI = 4,
    /// Geocentric Coordinate Time, expressed as Julian days on the TCG axis.
    TCG = 5,
    /// Barycentric Coordinate Time, expressed as Julian days on the TCB axis.
    TCB = 6,
    /// GPS time, expressed in days since the GPS epoch.
    GPS = 7,
    /// Universal Time UT1, expressed as Julian days on the UT1 axis.
    UT = 8,
    /// Julian Ephemeris Date, numerically equal to JD(TT) in this ABI.
    JDE = 9,
    /// Unix / POSIX time in seconds since 1970-01-01T00:00:00 UTC.
    UnixTime = 10,
}

impl TempochScaleId {
    /// Attempt to decode a raw `i32` into a `TempochScaleId`.
    #[inline]
    pub fn from_raw(raw: i32) -> Option<Self> {
        match raw {
            0 => Some(Self::JD),
            1 => Some(Self::MJD),
            2 => Some(Self::TDB),
            3 => Some(Self::TT),
            4 => Some(Self::TAI),
            5 => Some(Self::TCG),
            6 => Some(Self::TCB),
            7 => Some(Self::GPS),
            8 => Some(Self::UT),
            9 => Some(Self::JDE),
            10 => Some(Self::UnixTime),
            _ => None,
        }
    }
}

#[inline]
fn default_context() -> TimeContext {
    TimeContext::new()
}

#[inline]
fn tt_from_jd(jd: f64) -> Result<Time<TT>, ConversionError> {
    Time::<TT, JD>::from_julian_days(Day::new(jd)).map(|t| t.reformat())
}

#[inline]
fn tt_to_jd(tt: Time<TT>) -> f64 {
    tt.reformat::<JD>().julian_days() / Day::new(1.0)
}

#[inline]
fn scale_value_to_tt(
    value: f64,
    scale: TempochScaleId,
    ctx: &TimeContext,
) -> Result<Time<TT>, ConversionError> {
    match scale {
        TempochScaleId::JD | TempochScaleId::JDE | TempochScaleId::TT => tt_from_jd(value),
        TempochScaleId::MJD => {
            Time::<TT, MJD>::from_modified_julian_days(Day::new(value)).map(|t| t.reformat())
        }
        TempochScaleId::TDB => Time::<TDB, JD>::from_julian_days(Day::new(value))
            .map(|t| t.reformat::<J2000s>().to_scale::<TT>()),
        TempochScaleId::TAI => Time::<TAI, JD>::from_julian_days(Day::new(value))
            .map(|t| t.reformat::<J2000s>().to_scale::<TT>()),
        TempochScaleId::TCG => Time::<TCG, JD>::from_julian_days(Day::new(value))
            .map(|t| t.reformat::<J2000s>().to_scale::<TT>()),
        TempochScaleId::TCB => Time::<TCB, JD>::from_julian_days(Day::new(value))
            .map(|t| t.reformat::<J2000s>().to_scale::<TT>()),
        TempochScaleId::GPS => {
            Time::<TAI, JD>::from_julian_days(Day::new(value + GPS_EPOCH_TAI_JD))
                .map(|t| t.reformat::<J2000s>().to_scale::<TT>())
        }
        TempochScaleId::UT => Time::<UT1, JD>::from_julian_days(Day::new(value))
            .map(|t| t.reformat())
            .and_then(|time: Time<UT1>| time.to_scale_with::<TT>(ctx)),
        TempochScaleId::UnixTime => {
            Time::<UTC>::from_unix_seconds(Seconds::new(value)).map(|t| t.to_scale::<TT>())
        }
    }
}

#[inline]
fn tt_to_scale_value(
    tt: Time<TT>,
    scale: TempochScaleId,
    ctx: &TimeContext,
) -> Result<f64, ConversionError> {
    match scale {
        TempochScaleId::JD | TempochScaleId::JDE | TempochScaleId::TT => Ok(tt_to_jd(tt)),
        TempochScaleId::MJD => Ok(tt.reformat::<MJD>().modified_julian_days() / Day::new(1.0)),
        TempochScaleId::TDB => {
            Ok(tt.to_scale::<TDB>().reformat::<JD>().julian_days() / Day::new(1.0))
        }
        TempochScaleId::TAI => {
            Ok(tt.to_scale::<TAI>().reformat::<JD>().julian_days() / Day::new(1.0))
        }
        TempochScaleId::TCG => {
            Ok(tt.to_scale::<TCG>().reformat::<JD>().julian_days() / Day::new(1.0))
        }
        TempochScaleId::TCB => {
            Ok(tt.to_scale::<TCB>().reformat::<JD>().julian_days() / Day::new(1.0))
        }
        TempochScaleId::GPS => Ok(tt.to_scale::<TAI>().reformat::<JD>().julian_days()
            / Day::new(1.0)
            - GPS_EPOCH_TAI_JD),
        TempochScaleId::UT => {
            Ok(tt.to_scale_with::<UT1>(ctx)?.reformat::<JD>().julian_days() / Day::new(1.0))
        }
        TempochScaleId::UnixTime => tt
            .to_scale::<UTC>()
            .unix_seconds()
            .map(|s| s / Seconds::new(1.0)),
    }
}

/// Convert a JD(TT) value to the requested scale's native scalar.
pub(crate) fn jd_to_scale_value(jd: f64, scale: TempochScaleId) -> Result<f64, ConversionError> {
    let ctx = default_context();
    tt_from_jd(jd).and_then(|tt| tt_to_scale_value(tt, scale, &ctx))
}

/// Convert a native scalar in the given scale to JD(TT).
pub(crate) fn scale_value_to_jd(value: f64, scale: TempochScaleId) -> Result<f64, ConversionError> {
    let ctx = default_context();
    scale_value_to_tt(value, scale, &ctx).map(tt_to_jd)
}

/// Convert a UTC instant to a native scalar in the requested scale.
pub(crate) fn time_from_utc_value(datetime: DateTime<Utc>, scale: TempochScaleId) -> Option<f64> {
    if matches!(scale, TempochScaleId::UnixTime) {
        // Validate against UTC-history bounds via the civil API (rejects pre-1961
        // dates), then return the POSIX timestamp directly.  A UTC→TT→UTC
        // round-trip would silently accumulate ~10 µs of error because
        // TT_MINUS_TAI (32.184 s) is not exactly representable in f64.
        Time::<UTC>::try_from_chrono(datetime).ok()?;
        let nanos = datetime.timestamp_subsec_nanos().min(999_999_999);
        return Some(datetime.timestamp() as f64 + nanos as f64 / 1e9);
    }

    let ctx = default_context();
    let tt = Time::<UTC>::try_from_chrono(datetime)
        .ok()?
        .to_scale::<TT>();
    tt_to_scale_value(tt, scale, &ctx).ok()
}

/// Convert a native scalar in the requested scale to UTC.
pub(crate) fn time_to_utc_value(value: f64, scale: TempochScaleId) -> Option<DateTime<Utc>> {
    if matches!(scale, TempochScaleId::UnixTime) {
        // Route through the civil API so that out-of-history-range Unix
        // timestamps and non-finite values are rejected consistently with
        // all other scales.
        return Time::<UTC>::from_unix_seconds(Seconds::new(value))
            .ok()?
            .try_to_chrono()
            .ok();
    }

    let ctx = default_context();
    scale_value_to_tt(value, scale, &ctx)
        .ok()?
        .to_scale::<UTC>()
        .try_to_chrono()
        .ok()
}

/// Compute a same-scale duration in days.
pub(crate) fn time_difference_days_value(lhs: f64, rhs: f64, scale: TempochScaleId) -> f64 {
    match scale {
        TempochScaleId::UnixTime => (lhs - rhs) / SECONDS_PER_DAY,
        _ => lhs - rhs,
    }
}

/// Add a duration in days in the native scale and return the resulting scalar.
pub(crate) fn time_add_days_value(value: f64, days: qtty::Day, scale: TempochScaleId) -> f64 {
    match scale {
        TempochScaleId::UnixTime => {
            value + days.to::<qtty::unit::Second>() / qtty::Second::new(1.0)
        }
        _ => value + days / Day::new(1.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_scale_id() {
        assert_eq!(std::mem::size_of::<TempochScaleId>(), 4);
        assert_eq!(std::mem::align_of::<TempochScaleId>(), 4);
    }

    #[test]
    fn scale_id_from_raw_valid() {
        for raw in 0..=10 {
            assert!(TempochScaleId::from_raw(raw).is_some(), "raw={raw}");
        }
    }

    #[test]
    fn scale_id_from_raw_invalid() {
        assert_eq!(TempochScaleId::from_raw(-1), None);
        assert_eq!(TempochScaleId::from_raw(999), None);
    }

    #[test]
    fn jd_to_mjd_roundtrip() {
        let jd = 2_451_545.0;
        let mjd = jd_to_scale_value(jd, TempochScaleId::MJD).unwrap();
        let back = scale_value_to_jd(mjd, TempochScaleId::MJD).unwrap();
        assert!((back - jd).abs() < 1e-10);
    }

    #[test]
    fn unix_time_generic_roundtrip_uses_seconds() {
        let unix = 946_728_000.0;
        let jd = scale_value_to_jd(unix, TempochScaleId::UnixTime).unwrap();
        let back = jd_to_scale_value(jd, TempochScaleId::UnixTime).unwrap();
        assert!((back - unix).abs() < 1e-3);
    }

    #[test]
    fn gps_scale_uses_days_since_epoch() {
        let gps =
            jd_to_scale_value(2_444_244.5 + 51.184 / SECONDS_PER_DAY, TempochScaleId::GPS).unwrap();
        assert!(gps.abs() < 1e-9);
    }
}
