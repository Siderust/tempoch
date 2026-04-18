// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Format conversion layer.
//!
//! Two traits govern format-level transformations:
//!
//! * [`FormatConvertible`] — witnesses that format `F1` can be converted to
//!   `F2` within any scale (pure arithmetic: epoch offsets, unit scaling).
//! * [`CanonicalRoundtrip`] — witnesses that a format can round-trip through
//!   the canonical `J2000s` representation, enabling scale conversions.
//!
//! # Scale-conversion eligibility
//!
//! Not all formats implement [`CanonicalRoundtrip`]:
//!
//! | Format      | `CanonicalRoundtrip` | Note |
//! |-------------|----------------------|------|
//! | `J2000s`    | ✓                    | canonical |
//! | `JD`        | ✓                    | lossless f64 roundtrip |
//! | `MJD`       | ✓                    | lossless f64 roundtrip |
//! | `GpsSecs`   | ✗                    | GPS epoch is TAI-axis-specific; `.reformat::<J2000s>()` first |
//! | `UnixSecs`  | ✗                    | No cross-format conversions; use civil API only |
//! | `DayCount`  | ✗                    | i32 precision loss; `.reformat::<MJD>()` or `J2000s` first |
//!
//! `GpsSecs` intentionally does **not** implement `CanonicalRoundtrip`, for
//! the same reason as `UnixSecs`: its epoch offset (`GPS_EPOCH_TAI`) is only
//! physically meaningful on the TAI axis. Allowing `.to_scale::<S2>()` on
//! `Time<UTC, GpsSecs>` or `Time<TT, GpsSecs>` would silently produce
//! incorrect values. Use the civil API ([`Time::<TAI>::from_gps_seconds`])
//! or `.reformat::<J2000s>()` explicitly before calling `to_scale`.
//!
//! [`Time::<TAI>::from_gps_seconds`]: crate::Time::from_gps_seconds

use super::constats::{GPS_EPOCH_TAI, JD_MINUS_MJD};
use super::encoding::{
    j2000_seconds_to_jd, j2000_seconds_to_mjd, jd_to_j2000_seconds, mjd_to_j2000_seconds,
};
use super::format::{DayCount, GpsSecs, J2000s, JD, MJD};
use super::sealed::Sealed;
use qtty::time::{Days, Seconds};
use qtty::unit::Day;
use qtty::QuantityI32;

// ── CanonicalRoundtrip ───────────────────────────────────────────────────

/// Witness that a format can losslessly round-trip through J2000 SI seconds
/// (`Quantity<Second, f64>`). Required for scale conversions.
///
/// Integer-based formats (`UnixSecs`, `DayCount`) do **not** implement this
/// trait. `GpsSecs` also does **not** implement this trait because its epoch
/// offset (`GPS_EPOCH_TAI`) is calibrated on the TAI axis — allowing scale
/// conversions on `Time<UTC, GpsSecs>` etc. would silently produce incorrect
/// values. Users must `.reformat::<J2000s>()` first, making the precision
/// and scale-semantic trade-off explicit at the call site.
pub(crate) trait CanonicalRoundtrip: super::format::Format {
    fn to_j2000s(src: Self::Storage) -> Seconds;
    fn from_j2000s(secs: Seconds) -> Self::Storage;
}

impl CanonicalRoundtrip for J2000s {
    #[inline]
    fn to_j2000s(src: Seconds) -> Seconds {
        src
    }
    #[inline]
    fn from_j2000s(secs: Seconds) -> Seconds {
        secs
    }
}

impl CanonicalRoundtrip for JD {
    #[inline]
    fn to_j2000s(src: Days) -> Seconds {
        jd_to_j2000_seconds(src)
    }
    #[inline]
    fn from_j2000s(secs: Seconds) -> Days {
        j2000_seconds_to_jd(secs)
    }
}

impl CanonicalRoundtrip for MJD {
    #[inline]
    fn to_j2000s(src: Days) -> Seconds {
        mjd_to_j2000_seconds(src)
    }
    #[inline]
    fn from_j2000s(secs: Seconds) -> Days {
        j2000_seconds_to_mjd(secs)
    }
}

// ── FormatConvertible ────────────────────────────────────────────────────

/// Witness that format `Self` can be converted to format `F2` via pure
/// arithmetic (epoch offsets, unit scaling). Used by `Time::reformat()`.
pub(crate) trait FormatConvertible<F2: super::format::Format>:
    super::format::Format + Sealed
{
    fn convert(src: Self::Storage) -> F2::Storage;
}

// -- Identity conversions -------------------------------------------------

macro_rules! identity_format {
    ($($fmt:ty),+ $(,)?) => {
        $(
            impl FormatConvertible<$fmt> for $fmt {
                #[inline]
                fn convert(src: <$fmt as super::format::Format>::Storage) -> <$fmt as super::format::Format>::Storage {
                    src
                }
            }
        )+
    };
}
identity_format!(J2000s, JD, MJD, GpsSecs, DayCount);

// -- J2000s ↔ JD ----------------------------------------------------------

impl FormatConvertible<JD> for J2000s {
    #[inline]
    fn convert(src: Seconds) -> Days {
        j2000_seconds_to_jd(src)
    }
}

impl FormatConvertible<J2000s> for JD {
    #[inline]
    fn convert(src: Days) -> Seconds {
        jd_to_j2000_seconds(src)
    }
}

// -- J2000s ↔ MJD ---------------------------------------------------------

impl FormatConvertible<MJD> for J2000s {
    #[inline]
    fn convert(src: Seconds) -> Days {
        j2000_seconds_to_mjd(src)
    }
}

impl FormatConvertible<J2000s> for MJD {
    #[inline]
    fn convert(src: Days) -> Seconds {
        mjd_to_j2000_seconds(src)
    }
}

// -- JD ↔ MJD -------------------------------------------------------------

impl FormatConvertible<MJD> for JD {
    #[inline]
    fn convert(src: Days) -> Days {
        src - JD_MINUS_MJD
    }
}

impl FormatConvertible<JD> for MJD {
    #[inline]
    fn convert(src: Days) -> Days {
        src + JD_MINUS_MJD
    }
}

// -- J2000s ↔ GpsSecs -----------------------------------------------------

impl FormatConvertible<GpsSecs> for J2000s {
    #[inline]
    fn convert(src: Seconds) -> Seconds {
        src - GPS_EPOCH_TAI
    }
}

impl FormatConvertible<J2000s> for GpsSecs {
    #[inline]
    fn convert(src: Seconds) -> Seconds {
        src + GPS_EPOCH_TAI
    }
}

// -- JD ↔ DayCount (through MJD) -------------------------------------------

impl FormatConvertible<DayCount> for MJD {
    #[inline]
    fn convert(src: Days) -> QuantityI32<Day> {
        let floored = src.value().floor();
        debug_assert!(
            floored >= i32::MIN as f64 && floored <= i32::MAX as f64,
            "MJD {floored} out of DayCount (i32) range",
        );
        QuantityI32::<Day>::new(floored as i32)
    }
}

impl FormatConvertible<MJD> for DayCount {
    #[inline]
    fn convert(src: QuantityI32<Day>) -> Days {
        Days::new(src.value() as f64)
    }
}

// -- JD ↔ GpsSecs (through J2000s) ----------------------------------------

impl FormatConvertible<GpsSecs> for JD {
    #[inline]
    fn convert(src: Days) -> Seconds {
        <J2000s as FormatConvertible<GpsSecs>>::convert(jd_to_j2000_seconds(src))
    }
}

impl FormatConvertible<JD> for GpsSecs {
    #[inline]
    fn convert(src: Seconds) -> Days {
        j2000_seconds_to_jd(<GpsSecs as FormatConvertible<J2000s>>::convert(src))
    }
}

// -- MJD ↔ GpsSecs (through J2000s) ----------------------------------------

impl FormatConvertible<GpsSecs> for MJD {
    #[inline]
    fn convert(src: Days) -> Seconds {
        <J2000s as FormatConvertible<GpsSecs>>::convert(mjd_to_j2000_seconds(src))
    }
}

impl FormatConvertible<MJD> for GpsSecs {
    #[inline]
    fn convert(src: Seconds) -> Days {
        j2000_seconds_to_mjd(<GpsSecs as FormatConvertible<J2000s>>::convert(src))
    }
}

// -- GpsSecs ↔ DayCount (through J2000s → MJD → DayCount) -----------------

impl FormatConvertible<DayCount> for JD {
    #[inline]
    fn convert(src: Days) -> QuantityI32<Day> {
        <MJD as FormatConvertible<DayCount>>::convert(src - JD_MINUS_MJD)
    }
}

impl FormatConvertible<JD> for DayCount {
    #[inline]
    fn convert(src: QuantityI32<Day>) -> Days {
        <DayCount as FormatConvertible<MJD>>::convert(src) + JD_MINUS_MJD
    }
}

// -- J2000s ↔ DayCount (through MJD) ----------------------------------------

impl FormatConvertible<DayCount> for J2000s {
    #[inline]
    fn convert(src: Seconds) -> QuantityI32<Day> {
        <MJD as FormatConvertible<DayCount>>::convert(j2000_seconds_to_mjd(src))
    }
}

impl FormatConvertible<J2000s> for DayCount {
    #[inline]
    fn convert(src: QuantityI32<Day>) -> Seconds {
        mjd_to_j2000_seconds(<DayCount as FormatConvertible<MJD>>::convert(src))
    }
}

// -- GpsSecs ↔ DayCount (through J2000s → MJD → DayCount) -----------------

impl FormatConvertible<DayCount> for GpsSecs {
    #[inline]
    fn convert(src: Seconds) -> QuantityI32<Day> {
        <J2000s as FormatConvertible<DayCount>>::convert(
            <GpsSecs as FormatConvertible<J2000s>>::convert(src),
        )
    }
}

impl FormatConvertible<GpsSecs> for DayCount {
    #[inline]
    fn convert(src: QuantityI32<Day>) -> Seconds {
        <J2000s as FormatConvertible<GpsSecs>>::convert(
            <DayCount as FormatConvertible<J2000s>>::convert(src),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constats::J2000_JD_TT;

    const EPS_S: Seconds = Seconds::new(1e-9);
    const EPS_D: Days = Days::new(1e-12);

    #[test]
    fn j2000s_jd_round_trip() {
        let secs = Seconds::new(86_400.0); // 1 day after J2000
        let jd = <J2000s as FormatConvertible<JD>>::convert(secs);
        let back = <JD as FormatConvertible<J2000s>>::convert(jd);
        assert!((back - secs).abs() < EPS_S);
    }

    #[test]
    fn jd_mjd_offset() {
        let jd = J2000_JD_TT;
        let mjd = <JD as FormatConvertible<MJD>>::convert(jd);
        let expected = jd - JD_MINUS_MJD;
        assert!((mjd - expected).abs() < EPS_D);
    }

    #[test]
    fn j2000s_mjd_round_trip() {
        let secs = Seconds::new(0.0);
        let mjd = <J2000s as FormatConvertible<MJD>>::convert(secs);
        let back = <MJD as FormatConvertible<J2000s>>::convert(mjd);
        assert!((back - secs).abs() < EPS_S);
    }

    #[test]
    fn j2000s_gps_round_trip() {
        let secs = Seconds::new(1_000_000.0);
        let gps = <J2000s as FormatConvertible<GpsSecs>>::convert(secs);
        let back = <GpsSecs as FormatConvertible<J2000s>>::convert(gps);
        assert!((back - secs).abs() < EPS_S);
    }

    #[test]
    fn mjd_daycount_round_trip_truncation() {
        let mjd = Days::new(51_544.7);
        let dc = <MJD as FormatConvertible<DayCount>>::convert(mjd);
        assert_eq!(dc.value(), 51_544);
        let back = <DayCount as FormatConvertible<MJD>>::convert(dc);
        assert_eq!(back, Days::new(51_544.0));
    }

    #[test]
    fn mjd_canonical_roundtrip() {
        let mjd = Days::new(51_544.5);
        let secs = <MJD as CanonicalRoundtrip>::to_j2000s(mjd);
        let back = <MJD as CanonicalRoundtrip>::from_j2000s(secs);
        assert!((back - mjd).abs() < EPS_D);
    }

    #[test]
    fn identity_format_conversions() {
        let secs = Seconds::new(1_000_000.0);
        assert_eq!(<J2000s as FormatConvertible<J2000s>>::convert(secs), secs);
        let days = Days::new(2_451_545.0);
        assert_eq!(<JD as FormatConvertible<JD>>::convert(days), days);
        assert_eq!(<MJD as FormatConvertible<MJD>>::convert(days), days);
    }

    #[test]
    fn mjd_jd_round_trip() {
        let mjd = Days::new(51_544.5);
        let jd = <MJD as FormatConvertible<JD>>::convert(mjd);
        let back = <JD as FormatConvertible<MJD>>::convert(jd);
        assert!((back - mjd).abs() < EPS_D);
    }

    #[test]
    fn gps_jd_round_trip() {
        // JD values are ~2.4M days; f64 precision at that scale is ~50 µs
        let gps = Seconds::new(1_000_000.0);
        let jd = <GpsSecs as FormatConvertible<JD>>::convert(gps);
        let back = <JD as FormatConvertible<GpsSecs>>::convert(jd);
        assert!((back - gps).abs() < Seconds::new(1e-3));
    }

    #[test]
    fn gps_mjd_round_trip() {
        // MJD values are ~44k days; f64 precision at that scale is ~1e-4 s
        let gps = Seconds::new(1_000_000.0);
        let mjd = <GpsSecs as FormatConvertible<MJD>>::convert(gps);
        let back = <MJD as FormatConvertible<GpsSecs>>::convert(mjd);
        assert!((back - gps).abs() < Seconds::new(1e-3));
    }

    #[test]
    fn jd_daycount_and_back() {
        let jd = Days::new(2_451_545.0);
        let dc = <JD as FormatConvertible<DayCount>>::convert(jd);
        let back = <DayCount as FormatConvertible<JD>>::convert(dc);
        // DayCount truncates to whole days; back should be within 1 day of original
        assert!((back - jd).abs() < Days::new(1.0));
    }

    #[test]
    fn j2000s_daycount_and_back() {
        let secs = Seconds::new(86_400.0);
        let dc = <J2000s as FormatConvertible<DayCount>>::convert(secs);
        let back = <DayCount as FormatConvertible<J2000s>>::convert(dc);
        assert!((back - secs).abs() < Seconds::new(86_400.0));
    }

    #[test]
    fn gps_daycount_and_back() {
        let gps = Seconds::new(1_000_000.0);
        let dc = <GpsSecs as FormatConvertible<DayCount>>::convert(gps);
        let back = <DayCount as FormatConvertible<GpsSecs>>::convert(dc);
        assert!((back - gps).abs() < Seconds::new(86_400.0));
    }
}
