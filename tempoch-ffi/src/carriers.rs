// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Scale identifiers and scale-aware helpers for the scalar C ABI.
//!
//! The FFI exposes scalar time values as plain `double`s plus an explicit
//! [`TempochScaleId`] when generic dispatch is needed. This keeps the C ABI
//! small and regular while preserving the Rust crate's scale semantics in the
//! implementation.

use chrono::{DateTime, Utc};
use tempoch::{
    JulianDate, Time, TimeInstant, UnixTime, GPS, JD, JDE, MJD, TAI, TCB, TCG, TDB, TT, UT,
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
    /// Julian Date (TT).
    JD = 0,
    /// Modified Julian Date (TT).
    MJD = 1,
    /// Barycentric Dynamical Time.
    TDB = 2,
    /// Terrestrial Time.
    TT = 3,
    /// International Atomic Time.
    TAI = 4,
    /// Geocentric Coordinate Time.
    TCG = 5,
    /// Barycentric Coordinate Time.
    TCB = 6,
    /// GPS Time.
    GPS = 7,
    /// Universal Time UT1.
    UT = 8,
    /// Julian Ephemeris Date.
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
fn utc_to_unix_seconds(datetime: DateTime<Utc>) -> f64 {
    datetime.timestamp() as f64 + datetime.timestamp_subsec_nanos() as f64 / 1e9
}

#[inline]
fn unix_seconds_to_utc(seconds: f64) -> Option<DateTime<Utc>> {
    if !seconds.is_finite() {
        return None;
    }

    let mut whole = seconds.floor();
    let mut nanos = ((seconds - whole) * 1e9).round();
    if nanos >= 1e9 {
        whole += 1.0;
        nanos = 0.0;
    }

    DateTime::<Utc>::from_timestamp(whole as i64, nanos as u32)
}

/// Convert a JD(TT) value to the requested scale's native scalar.
pub(crate) fn jd_to_scale_value(jd: f64, scale: TempochScaleId) -> f64 {
    let t = JulianDate::new(jd);
    match scale {
        TempochScaleId::JD => t.to::<JD>().value(),
        TempochScaleId::MJD => t.to::<MJD>().value(),
        TempochScaleId::TDB => t.to::<TDB>().value(),
        TempochScaleId::TT => t.to::<TT>().value(),
        TempochScaleId::TAI => t.to::<TAI>().value(),
        TempochScaleId::TCG => t.to::<TCG>().value(),
        TempochScaleId::TCB => t.to::<TCB>().value(),
        TempochScaleId::GPS => t.to::<GPS>().value(),
        TempochScaleId::UT => t.to::<UT>().value(),
        TempochScaleId::JDE => t.to::<JDE>().value(),
        TempochScaleId::UnixTime => t.to::<UnixTime>().value() * 86_400.0,
    }
}

/// Convert a native scalar in the given scale to JD(TT).
pub(crate) fn scale_value_to_jd(value: f64, scale: TempochScaleId) -> f64 {
    match scale {
        TempochScaleId::JD => Time::<JD>::new(value).to::<JD>().value(),
        TempochScaleId::MJD => Time::<MJD>::new(value).to::<JD>().value(),
        TempochScaleId::TDB => Time::<TDB>::new(value).to::<JD>().value(),
        TempochScaleId::TT => Time::<TT>::new(value).to::<JD>().value(),
        TempochScaleId::TAI => Time::<TAI>::new(value).to::<JD>().value(),
        TempochScaleId::TCG => Time::<TCG>::new(value).to::<JD>().value(),
        TempochScaleId::TCB => Time::<TCB>::new(value).to::<JD>().value(),
        TempochScaleId::GPS => Time::<GPS>::new(value).to::<JD>().value(),
        TempochScaleId::UT => Time::<UT>::new(value).to::<JD>().value(),
        TempochScaleId::JDE => Time::<JDE>::new(value).to::<JD>().value(),
        TempochScaleId::UnixTime => {
            // value is Unix seconds; core UnixTime stores days
            Time::<UnixTime>::new(value / 86_400.0).to::<JD>().value()
        }
    }
}

/// Convert a UTC instant to a native scalar in the requested scale.
pub(crate) fn time_from_utc_value(datetime: DateTime<Utc>, scale: TempochScaleId) -> f64 {
    match scale {
        TempochScaleId::JD => Time::<JD>::from_utc(datetime).value(),
        TempochScaleId::MJD => Time::<MJD>::from_utc(datetime).value(),
        TempochScaleId::TDB => Time::<TDB>::from_utc(datetime).value(),
        TempochScaleId::TT => Time::<TT>::from_utc(datetime).value(),
        TempochScaleId::TAI => Time::<TAI>::from_utc(datetime).value(),
        TempochScaleId::TCG => Time::<TCG>::from_utc(datetime).value(),
        TempochScaleId::TCB => Time::<TCB>::from_utc(datetime).value(),
        TempochScaleId::GPS => Time::<GPS>::from_utc(datetime).value(),
        TempochScaleId::UT => Time::<UT>::from_utc(datetime).value(),
        TempochScaleId::JDE => Time::<JDE>::from_utc(datetime).value(),
        TempochScaleId::UnixTime => utc_to_unix_seconds(datetime),
    }
}

/// Convert a native scalar in the requested scale to UTC.
pub(crate) fn time_to_utc_value(value: f64, scale: TempochScaleId) -> Option<DateTime<Utc>> {
    match scale {
        TempochScaleId::JD => Time::<JD>::new(value).to_utc(),
        TempochScaleId::MJD => Time::<MJD>::new(value).to_utc(),
        TempochScaleId::TDB => Time::<TDB>::new(value).to_utc(),
        TempochScaleId::TT => Time::<TT>::new(value).to_utc(),
        TempochScaleId::TAI => Time::<TAI>::new(value).to_utc(),
        TempochScaleId::TCG => Time::<TCG>::new(value).to_utc(),
        TempochScaleId::TCB => Time::<TCB>::new(value).to_utc(),
        TempochScaleId::GPS => Time::<GPS>::new(value).to_utc(),
        TempochScaleId::UT => Time::<UT>::new(value).to_utc(),
        TempochScaleId::JDE => Time::<JDE>::new(value).to_utc(),
        TempochScaleId::UnixTime => unix_seconds_to_utc(value),
    }
}

/// Compute a same-scale duration in days.
pub(crate) fn time_difference_days_value(lhs: f64, rhs: f64, scale: TempochScaleId) -> f64 {
    match scale {
        TempochScaleId::JD => Time::<JD>::new(lhs)
            .difference(&Time::<JD>::new(rhs))
            .value(),
        TempochScaleId::MJD => Time::<MJD>::new(lhs)
            .difference(&Time::<MJD>::new(rhs))
            .value(),
        TempochScaleId::TDB => Time::<TDB>::new(lhs)
            .difference(&Time::<TDB>::new(rhs))
            .value(),
        TempochScaleId::TT => Time::<TT>::new(lhs)
            .difference(&Time::<TT>::new(rhs))
            .value(),
        TempochScaleId::TAI => Time::<TAI>::new(lhs)
            .difference(&Time::<TAI>::new(rhs))
            .value(),
        TempochScaleId::TCG => Time::<TCG>::new(lhs)
            .difference(&Time::<TCG>::new(rhs))
            .value(),
        TempochScaleId::TCB => Time::<TCB>::new(lhs)
            .difference(&Time::<TCB>::new(rhs))
            .value(),
        TempochScaleId::GPS => Time::<GPS>::new(lhs)
            .difference(&Time::<GPS>::new(rhs))
            .value(),
        TempochScaleId::UT => Time::<UT>::new(lhs)
            .difference(&Time::<UT>::new(rhs))
            .value(),
        TempochScaleId::JDE => Time::<JDE>::new(lhs)
            .difference(&Time::<JDE>::new(rhs))
            .value(),
        TempochScaleId::UnixTime => (lhs - rhs) / 86_400.0,
    }
}

/// Add a duration in days in the native scale and return the resulting scalar.
pub(crate) fn time_add_days_value(value: f64, days: qtty::Day, scale: TempochScaleId) -> f64 {
    match scale {
        TempochScaleId::JD => Time::<JD>::new(value).add_duration(days).value(),
        TempochScaleId::MJD => Time::<MJD>::new(value).add_duration(days).value(),
        TempochScaleId::TDB => Time::<TDB>::new(value).add_duration(days).value(),
        TempochScaleId::TT => Time::<TT>::new(value).add_duration(days).value(),
        TempochScaleId::TAI => Time::<TAI>::new(value).add_duration(days).value(),
        TempochScaleId::TCG => Time::<TCG>::new(value).add_duration(days).value(),
        TempochScaleId::TCB => Time::<TCB>::new(value).add_duration(days).value(),
        TempochScaleId::GPS => Time::<GPS>::new(value).add_duration(days).value(),
        TempochScaleId::UT => Time::<UT>::new(value).add_duration(days).value(),
        TempochScaleId::JDE => Time::<JDE>::new(value).add_duration(days).value(),
        TempochScaleId::UnixTime => value + days.value() * 86_400.0,
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
        assert!((back - unix).abs() < 1e-6);
    }
}
