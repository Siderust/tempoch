// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Typed POD time carriers for each absolute time scale.
//!
//! Each carrier is a `#[repr(C)]` struct wrapping a single `f64 value` field.
//! Wrapping bare `f64`s this way makes time-scale mistakes a type error at the
//! C-ABI boundary rather than a silent wrong-answer.
//!
//! # UTC
//!
//! `TempochUtc` is the calendar breakdown struct; it is **not** a numeric
//! carrier.  Calendar fields remain raw integer types.
//!
//! # Durations
//!
//! Duration/unit-bearing outputs use `qtty_quantity_t` from qtty-ffi.  Typed
//! carriers are only for absolute instants.

use tempoch::{
    JulianDate, ModifiedJulianDate, Time, GPS, JD, JDE, MJD, TAI, TCB, TCG, TDB, TT, UT,
};

// ─── Macro to generate boilerplate for each scale carrier ────────────────────

macro_rules! define_carrier {
    (
        $(#[$attr:meta])*
        $name:ident, $scale:ty, $doc:literal
    ) => {
        $(#[$attr])*
        #[doc = $doc]
        #[repr(C)]
        #[derive(Debug, Clone, Copy, PartialEq)]
        pub struct $name {
            /// The numeric value in this time scale.
            pub value: f64,
        }

        impl $name {
            /// Construct a carrier from a raw numeric value.
            #[inline]
            pub const fn new(value: f64) -> Self {
                Self { value }
            }

            /// Construct from the corresponding tempoch `Time<S>` type.
            #[inline]
            pub fn from_time(t: &Time<$scale>) -> Self {
                Self { value: t.value() }
            }

            /// Convert to the corresponding tempoch `Time<S>` type.
            #[inline]
            pub fn to_time(&self) -> Time<$scale> {
                Time::<$scale>::new(self.value)
            }
        }
    };
}

// ─── Carrier definitions ─────────────────────────────────────────────────────

define_carrier!(
    /// Julian Date (TT) carrier.
    TempochJd, JD,
    "Julian Date (TT) — days since noon on 1 January 4713 BC."
);

define_carrier!(
    /// Modified Julian Date (TT) carrier.
    TempochMjd, MJD,
    "Modified Julian Date (TT) — JD minus 2 400 000.5."
);

define_carrier!(
    /// Terrestrial Time (TT) carrier.
    TempochTt, TT,
    "Terrestrial Time (TT) expressed as a Julian Date."
);

define_carrier!(
    /// Barycentric Dynamical Time (TDB) carrier.
    TempochTdb, TDB,
    "Barycentric Dynamical Time (TDB) expressed as a Julian Date."
);

define_carrier!(
    /// International Atomic Time (TAI) carrier.
    TempochTai, TAI,
    "International Atomic Time (TAI) expressed as a Julian Date."
);

define_carrier!(
    /// Geocentric Coordinate Time (TCG) carrier.
    TempochTcg, TCG,
    "Geocentric Coordinate Time (TCG) expressed as a Julian Date."
);

define_carrier!(
    /// Barycentric Coordinate Time (TCB) carrier.
    TempochTcb, TCB,
    "Barycentric Coordinate Time (TCB) expressed as a Julian Date."
);

define_carrier!(
    /// GPS Time carrier.
    TempochGps, GPS,
    "GPS Time expressed as a Julian Date."
);

define_carrier!(
    /// Universal Time UT1 carrier.
    TempochUt, UT,
    "Universal Time UT1 expressed as a Julian Date."
);

define_carrier!(
    /// Julian Ephemeris Date (JDE) carrier — semantic alias of JD(TT).
    TempochJde, JDE,
    "Julian Ephemeris Date (JDE) — semantic alias of JD(TT)."
);

/// Unix Time carrier.
///
/// Seconds since 1970-01-01T00:00:00 UTC, ignoring leap seconds.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TempochUnixTime {
    /// Seconds since the Unix epoch.
    pub value: f64,
}

impl TempochUnixTime {
    /// Construct from a raw Unix timestamp.
    #[inline]
    pub const fn new(value: f64) -> Self {
        Self { value }
    }

    /// Construct from a tempoch `Time<UnixTime>`.
    #[inline]
    pub fn from_time(t: &Time<tempoch::UnixTime>) -> Self {
        Self { value: t.value() }
    }

    /// Convert to a tempoch `Time<UnixTime>`.
    #[inline]
    pub fn to_time(&self) -> Time<tempoch::UnixTime> {
        Time::<tempoch::UnixTime>::new(self.value)
    }
}

// ─── Scale-dispatch enum (validated via raw i32 ID) ──────────────────────────

/// Time scale identifier for generic dispatch functions.
///
/// In the C ABI, callers pass raw `int32_t` values and must validate them
/// before dispatch.  Use `TempochScaleId_from_raw()` / matching against the
/// named constants in the header.
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
    /// Unix Time (seconds since 1970-01-01T00:00:00 UTC).
    UnixTime = 10,
}

impl TempochScaleId {
    /// Attempt to decode a raw `i32` into a `TempochScaleId`.
    ///
    /// Returns `None` if the value is not a recognized discriminant.
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

// ─── JD(TT) ↔ typed carrier helpers ─────────────────────────────────────────

/// Convert a `TempochJd` to the requested scale, returning its raw value.
///
/// This is used internally by the generic dispatch functions.
pub fn jd_to_scale_value(jd: TempochJd, scale: TempochScaleId) -> f64 {
    let t = JulianDate::new(jd.value);
    match scale {
        TempochScaleId::JD => jd.value,
        TempochScaleId::MJD => t.to::<MJD>().value(),
        TempochScaleId::TDB => t.to::<TDB>().value(),
        TempochScaleId::TT => t.to::<TT>().value(),
        TempochScaleId::TAI => t.to::<TAI>().value(),
        TempochScaleId::TCG => t.to::<TCG>().value(),
        TempochScaleId::TCB => t.to::<TCB>().value(),
        TempochScaleId::GPS => t.to::<GPS>().value(),
        TempochScaleId::UT => t.to::<UT>().value(),
        TempochScaleId::JDE => t.to::<JDE>().value(),
        TempochScaleId::UnixTime => t.to::<tempoch::UnixTime>().value(),
    }
}

/// Convert a raw value in the given scale to a `TempochJd`.
pub fn scale_value_to_jd(value: f64, scale: TempochScaleId) -> TempochJd {
    let jd_val = match scale {
        TempochScaleId::JD => value,
        TempochScaleId::MJD => ModifiedJulianDate::new(value).to::<JD>().value(),
        TempochScaleId::TDB => Time::<TDB>::new(value).to::<JD>().value(),
        TempochScaleId::TT => Time::<TT>::new(value).to::<JD>().value(),
        TempochScaleId::TAI => Time::<TAI>::new(value).to::<JD>().value(),
        TempochScaleId::TCG => Time::<TCG>::new(value).to::<JD>().value(),
        TempochScaleId::TCB => Time::<TCB>::new(value).to::<JD>().value(),
        TempochScaleId::GPS => Time::<GPS>::new(value).to::<JD>().value(),
        TempochScaleId::UT => Time::<UT>::new(value).to::<JD>().value(),
        TempochScaleId::JDE => Time::<JDE>::new(value).to::<JD>().value(),
        TempochScaleId::UnixTime => Time::<tempoch::UnixTime>::new(value).to::<JD>().value(),
    };
    TempochJd::new(jd_val)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_jd_carrier() {
        assert_eq!(std::mem::size_of::<TempochJd>(), 8);
        assert_eq!(std::mem::align_of::<TempochJd>(), 8);
    }

    #[test]
    fn layout_mjd_carrier() {
        assert_eq!(std::mem::size_of::<TempochMjd>(), 8);
        assert_eq!(std::mem::align_of::<TempochMjd>(), 8);
    }

    #[test]
    fn layout_tt_carrier() {
        assert_eq!(std::mem::size_of::<TempochTt>(), 8);
    }

    #[test]
    fn layout_scale_id() {
        assert_eq!(std::mem::size_of::<TempochScaleId>(), 4);
        assert_eq!(std::mem::align_of::<TempochScaleId>(), 4);
    }

    #[test]
    fn scale_id_from_raw_valid() {
        assert_eq!(TempochScaleId::from_raw(0), Some(TempochScaleId::JD));
        assert_eq!(TempochScaleId::from_raw(1), Some(TempochScaleId::MJD));
        assert_eq!(TempochScaleId::from_raw(10), Some(TempochScaleId::UnixTime));
    }

    #[test]
    fn scale_id_from_raw_invalid() {
        assert_eq!(TempochScaleId::from_raw(-1), None);
        assert_eq!(TempochScaleId::from_raw(11), None);
        assert_eq!(TempochScaleId::from_raw(999), None);
    }

    #[test]
    fn jd_to_mjd_roundtrip() {
        let jd = TempochJd::new(2451545.0); // J2000
        let mjd_val = jd_to_scale_value(jd, TempochScaleId::MJD);
        let back = scale_value_to_jd(mjd_val, TempochScaleId::MJD);
        assert!((back.value - jd.value).abs() < 1e-10);
    }

    #[test]
    fn carrier_new_and_value() {
        let c = TempochJd::new(2451545.0);
        assert_eq!(c.value, 2451545.0);
    }
}
