// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Scale identifiers and scale-aware helpers for the scalar C ABI.
//!
//! The FFI exposes scalar time values as plain `double`s plus an explicit
//! [`TempochScaleId`] when generic dispatch is needed. This keeps the C ABI
//! small and regular. All conversion and arithmetic policy is centralized in
//! `tempoch::scalar` — this module only maps the C ABI discriminant to the
//! Rust [`ScaleKind`] and provides thin wrappers for the FFI entry points.

use chrono::{DateTime, Utc};
use qtty::time::Seconds;
use qtty::Day;
use tempoch::{
    scalar::{scalar_add_days, scalar_difference_in_days, time_tt_from_scalar, time_tt_to_scalar},
    ConversionError, ScaleKind, Time, TimeContext, TT, UTC,
};

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

impl From<TempochScaleId> for ScaleKind {
    #[inline]
    fn from(id: TempochScaleId) -> Self {
        match id {
            // JD, JDE, and TT are all JD on the TT axis in the scalar ABI.
            TempochScaleId::JD | TempochScaleId::JDE | TempochScaleId::TT => ScaleKind::JdTt,
            TempochScaleId::MJD => ScaleKind::MjdTt,
            TempochScaleId::TDB => ScaleKind::Tdb,
            TempochScaleId::TAI => ScaleKind::Tai,
            TempochScaleId::TCG => ScaleKind::Tcg,
            TempochScaleId::TCB => ScaleKind::Tcb,
            TempochScaleId::GPS => ScaleKind::GpsDays,
            TempochScaleId::UT => ScaleKind::Ut1,
            TempochScaleId::UnixTime => ScaleKind::Unix,
        }
    }
}

#[inline]
fn default_context() -> TimeContext {
    TimeContext::new()
}

/// Convert a JD(TT) value to the requested scale's native scalar.
pub(crate) fn jd_to_scale_value(jd: f64, scale: TempochScaleId) -> Result<f64, ConversionError> {
    let ctx = default_context();
    let tt = time_tt_from_scalar(jd, ScaleKind::JdTt, &ctx)?;
    time_tt_to_scalar(tt, ScaleKind::from(scale), &ctx)
}

/// Convert a native scalar in the given scale to JD(TT).
pub(crate) fn scale_value_to_jd(value: f64, scale: TempochScaleId) -> Result<f64, ConversionError> {
    let ctx = default_context();
    let tt = time_tt_from_scalar(value, ScaleKind::from(scale), &ctx)?;
    time_tt_to_scalar(tt, ScaleKind::JdTt, &ctx)
}

/// Convert a UTC instant to a native scalar in the requested scale.
pub(crate) fn time_from_utc_value(datetime: DateTime<Utc>, scale: TempochScaleId) -> Option<f64> {
    let ctx = default_context();
    let kind = ScaleKind::from(scale);
    if matches!(kind, ScaleKind::Unix) {
        // Validate via the civil API, then return the POSIX timestamp directly.
        // Routing Unix values through UTC→TT→UTC would silently accumulate
        // ~10 µs of error because
        // TT_MINUS_TAI (32.184 s) is not exactly representable in f64.
        Time::<UTC>::try_from_chrono_with(datetime, &ctx).ok()?;
        if datetime.timestamp_subsec_nanos() >= 1_000_000_000 {
            return None;
        }
        let nanos = datetime.timestamp_subsec_nanos();
        return Some(datetime.timestamp() as f64 + nanos as f64 / 1e9);
    }

    let tt = Time::<UTC>::try_from_chrono_with(datetime, &ctx)
        .ok()?
        .to_scale::<TT>();
    time_tt_to_scalar(tt, kind, &ctx).ok()
}

/// Convert a native scalar in the requested scale to UTC.
pub(crate) fn time_to_utc_value(value: f64, scale: TempochScaleId) -> Option<DateTime<Utc>> {
    let ctx = default_context();
    let kind = ScaleKind::from(scale);
    if matches!(kind, ScaleKind::Unix) {
        // Route through the civil API so that out-of-history-range Unix
        // timestamps and non-finite values are rejected consistently with
        // all other scales.
        return Time::<UTC>::from_unix_seconds_with(Seconds::new(value), &ctx)
            .ok()?
            .try_to_chrono_with(&ctx)
            .ok();
    }

    time_tt_from_scalar(value, kind, &ctx)
        .ok()?
        .to_scale::<UTC>()
        .try_to_chrono_with(&ctx)
        .ok()
}

/// Compute a same-scale duration in days.
pub(crate) fn time_difference_days_value(lhs: f64, rhs: f64, scale: TempochScaleId) -> f64 {
    scalar_difference_in_days(lhs, rhs, ScaleKind::from(scale))
}

/// Add a duration in days in the native scale and return the resulting scalar.
pub(crate) fn time_add_days_value(value: f64, days: Day, scale: TempochScaleId) -> f64 {
    scalar_add_days(value, days, ScaleKind::from(scale))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time::UNIX_ROUNDTRIP_TOLERANCE_SECONDS;

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
        assert!((back - unix).abs() <= UNIX_ROUNDTRIP_TOLERANCE_SECONDS);
    }

    #[test]
    fn gps_scale_uses_days_since_epoch() {
        // JD(TT) at the GPS epoch: JD(UTC) = 2_444_244.5, TT-UTC = 51.184 s
        let jd_tt_at_gps_epoch =
            2_444_244.5 + qtty::Second::new(51.184).to::<qtty::unit::Day>().value();
        let gps = jd_to_scale_value(jd_tt_at_gps_epoch, TempochScaleId::GPS).unwrap();
        assert!(gps.abs() < 1e-9);
    }
}
