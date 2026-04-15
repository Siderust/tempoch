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
    ConversionError, Time, TimeContext, TAI,
    TCB, TCG, TDB, TT, UT1, UTC,
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
fn utc_to_unix_seconds(datetime: DateTime<Utc>) -> Seconds {
    Seconds::new(datetime.timestamp() as f64)
        + Seconds::new(datetime.timestamp_subsec_nanos() as f64 / 1e9)
}

#[inline]
fn unix_seconds_to_utc(seconds: Seconds) -> Option<DateTime<Utc>> {
    if !seconds.is_finite() {
        return None;
    }

    let mut whole = seconds.floor();
    let mut nanos = (((seconds - whole) / Seconds::new(1.0)) * 1e9).round();
    if nanos >= 1e9 {
        whole += Seconds::new(1.0);
        nanos = 0.0;
    }

    DateTime::<Utc>::from_timestamp((whole / Seconds::new(1.0)) as i64, nanos as u32)
}

#[inline]
fn tt_from_jd(jd: f64) -> Result<Time<TT>, ConversionError> {
    Time::<TT>::from_julian_days(Day::new(jd))
}

#[inline]
fn tt_to_jd(tt: Time<TT>) -> f64 {
    tt.julian_days() / Day::new(1.0)
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
            Time::<TT>::from_modified_julian_days(Day::new(value))
        }
        TempochScaleId::TDB => {
            Time::<TDB>::from_julian_days(Day::new(value)).map(|t| t.to::<TT>())
        }
        TempochScaleId::TAI => {
            Time::<TAI>::from_julian_days(Day::new(value)).map(|t| t.to::<TT>())
        }
        TempochScaleId::TCG => {
            Time::<TCG>::from_julian_days(Day::new(value)).map(|t| t.to::<TT>())
        }
        TempochScaleId::TCB => {
            Time::<TCB>::from_julian_days(Day::new(value)).map(|t| t.to::<TT>())
        }
        TempochScaleId::GPS => {
            Time::<TAI>::from_julian_days(Day::new(value + GPS_EPOCH_TAI_JD))
                .map(|t| t.to::<TT>())
        }
        TempochScaleId::UT => Time::<UT1>::from_julian_days(Day::new(value))
            .and_then(|time| time.to_with::<TT>(ctx)),
        TempochScaleId::UnixTime => {
            Time::<UTC>::from_unix_seconds(Seconds::new(value))
                .map(|t| t.to::<TT>())
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
        TempochScaleId::MJD => Ok(tt.modified_julian_days() / Day::new(1.0)),
        TempochScaleId::TDB => Ok(tt.to::<TDB>().julian_days() / Day::new(1.0)),
        TempochScaleId::TAI => Ok(tt.to::<TAI>().julian_days() / Day::new(1.0)),
        TempochScaleId::TCG => Ok(tt.to::<TCG>().julian_days() / Day::new(1.0)),
        TempochScaleId::TCB => Ok(tt.to::<TCB>().julian_days() / Day::new(1.0)),
        TempochScaleId::GPS => {
            Ok(tt.to::<TAI>().julian_days() / Day::new(1.0) - GPS_EPOCH_TAI_JD)
        }
        TempochScaleId::UT => Ok(tt.to_with::<UT1>(ctx)?.julian_days() / Day::new(1.0)),
        TempochScaleId::UnixTime => Ok(
            utc_to_unix_seconds(tt.to::<UTC>().try_to_chrono()?) / Seconds::new(1.0),
        ),
    }
}

/// Convert a JD(TT) value to the requested scale's native scalar.
pub(crate) fn jd_to_scale_value(jd: f64, scale: TempochScaleId) -> f64 {
    let ctx = default_context();
    tt_from_jd(jd)
        .and_then(|tt| tt_to_scale_value(tt, scale, &ctx))
        .unwrap_or(f64::NAN)
}

/// Convert a native scalar in the given scale to JD(TT).
pub(crate) fn scale_value_to_jd(value: f64, scale: TempochScaleId) -> f64 {
    let ctx = default_context();
    scale_value_to_tt(value, scale, &ctx)
        .map(tt_to_jd)
        .unwrap_or(f64::NAN)
}

/// Convert a UTC instant to a native scalar in the requested scale.
pub(crate) fn time_from_utc_value(datetime: DateTime<Utc>, scale: TempochScaleId) -> Option<f64> {
    if matches!(scale, TempochScaleId::UnixTime) {
        return Some(utc_to_unix_seconds(datetime) / Seconds::new(1.0));
    }

    let ctx = default_context();
    let tt = Time::<UTC>::try_from_chrono(datetime).ok()?.to::<TT>();
    tt_to_scale_value(tt, scale, &ctx).ok()
}

/// Convert a native scalar in the requested scale to UTC.
pub(crate) fn time_to_utc_value(value: f64, scale: TempochScaleId) -> Option<DateTime<Utc>> {
    if matches!(scale, TempochScaleId::UnixTime) {
        return unix_seconds_to_utc(Seconds::new(value));
    }

    let ctx = default_context();
    scale_value_to_tt(value, scale, &ctx)
        .ok()?
        .to::<UTC>()
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
        TempochScaleId::UnixTime => value + days.to::<qtty::unit::Second>() / qtty::Second::new(1.0),
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
        let mjd = jd_to_scale_value(jd, TempochScaleId::MJD);
        let back = scale_value_to_jd(mjd, TempochScaleId::MJD);
        assert!((back - jd).abs() < 1e-10);
    }

    #[test]
    fn unix_time_generic_roundtrip_uses_seconds() {
        let unix = 946_728_000.0;
        let jd = scale_value_to_jd(unix, TempochScaleId::UnixTime);
        let back = jd_to_scale_value(jd, TempochScaleId::UnixTime);
        assert!((back - unix).abs() < 1e-3);
    }

    #[test]
    fn gps_scale_uses_days_since_epoch() {
        let gps = jd_to_scale_value(2_444_244.5 + 51.184 / SECONDS_PER_DAY, TempochScaleId::GPS);
        assert!(gps.abs() < 1e-9);
    }
}
