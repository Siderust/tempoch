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
//! | `Jd`        | ✓                    | lossless f64 roundtrip |
//! | `Mjd`       | ✓                    | lossless f64 roundtrip |
//! | `GpsSecs`   | ✗                    | GPS epoch is TAI-axis-specific; `.reformat::<J2000s>()` first |
//! | `UnixSecs`  | ✗                    | No cross-format conversions; use civil API only |
//! | `DayCount`  | ✗                    | i32 precision loss; `.reformat::<Mjd>()` or `J2000s` first |
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
use super::format::{DayCount, GpsSecs, J2000s, Jd, Mjd};
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

impl CanonicalRoundtrip for Jd {
    #[inline]
    fn to_j2000s(src: Days) -> Seconds {
        jd_to_j2000_seconds(src)
    }
    #[inline]
    fn from_j2000s(secs: Seconds) -> Days {
        j2000_seconds_to_jd(secs)
    }
}

impl CanonicalRoundtrip for Mjd {
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
identity_format!(J2000s, Jd, Mjd, GpsSecs, DayCount);

// -- J2000s ↔ Jd ----------------------------------------------------------

impl FormatConvertible<Jd> for J2000s {
    #[inline]
    fn convert(src: Seconds) -> Days {
        j2000_seconds_to_jd(src)
    }
}

impl FormatConvertible<J2000s> for Jd {
    #[inline]
    fn convert(src: Days) -> Seconds {
        jd_to_j2000_seconds(src)
    }
}

// -- J2000s ↔ Mjd ---------------------------------------------------------

impl FormatConvertible<Mjd> for J2000s {
    #[inline]
    fn convert(src: Seconds) -> Days {
        j2000_seconds_to_mjd(src)
    }
}

impl FormatConvertible<J2000s> for Mjd {
    #[inline]
    fn convert(src: Days) -> Seconds {
        mjd_to_j2000_seconds(src)
    }
}

// -- Jd ↔ Mjd -------------------------------------------------------------

impl FormatConvertible<Mjd> for Jd {
    #[inline]
    fn convert(src: Days) -> Days {
        src - JD_MINUS_MJD
    }
}

impl FormatConvertible<Jd> for Mjd {
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

// -- Jd ↔ DayCount (through Mjd) -------------------------------------------

impl FormatConvertible<DayCount> for Mjd {
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

impl FormatConvertible<Mjd> for DayCount {
    #[inline]
    fn convert(src: QuantityI32<Day>) -> Days {
        Days::new(src.value() as f64)
    }
}

// -- Jd ↔ GpsSecs (through J2000s) ----------------------------------------

impl FormatConvertible<GpsSecs> for Jd {
    #[inline]
    fn convert(src: Days) -> Seconds {
        <J2000s as FormatConvertible<GpsSecs>>::convert(jd_to_j2000_seconds(src))
    }
}

impl FormatConvertible<Jd> for GpsSecs {
    #[inline]
    fn convert(src: Seconds) -> Days {
        j2000_seconds_to_jd(<GpsSecs as FormatConvertible<J2000s>>::convert(src))
    }
}

// -- Mjd ↔ GpsSecs (through J2000s) ----------------------------------------

impl FormatConvertible<GpsSecs> for Mjd {
    #[inline]
    fn convert(src: Days) -> Seconds {
        <J2000s as FormatConvertible<GpsSecs>>::convert(mjd_to_j2000_seconds(src))
    }
}

impl FormatConvertible<Mjd> for GpsSecs {
    #[inline]
    fn convert(src: Seconds) -> Days {
        j2000_seconds_to_mjd(<GpsSecs as FormatConvertible<J2000s>>::convert(src))
    }
}

// -- GpsSecs ↔ DayCount (through J2000s → Mjd → DayCount) -----------------

impl FormatConvertible<DayCount> for Jd {
    #[inline]
    fn convert(src: Days) -> QuantityI32<Day> {
        <Mjd as FormatConvertible<DayCount>>::convert(src - JD_MINUS_MJD)
    }
}

impl FormatConvertible<Jd> for DayCount {
    #[inline]
    fn convert(src: QuantityI32<Day>) -> Days {
        <DayCount as FormatConvertible<Mjd>>::convert(src) + JD_MINUS_MJD
    }
}

// -- J2000s ↔ DayCount (through Mjd) ----------------------------------------

impl FormatConvertible<DayCount> for J2000s {
    #[inline]
    fn convert(src: Seconds) -> QuantityI32<Day> {
        <Mjd as FormatConvertible<DayCount>>::convert(j2000_seconds_to_mjd(src))
    }
}

impl FormatConvertible<J2000s> for DayCount {
    #[inline]
    fn convert(src: QuantityI32<Day>) -> Seconds {
        mjd_to_j2000_seconds(<DayCount as FormatConvertible<Mjd>>::convert(src))
    }
}

// -- GpsSecs ↔ DayCount (through J2000s → Mjd → DayCount) -----------------

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
        let jd = <J2000s as FormatConvertible<Jd>>::convert(secs);
        let back = <Jd as FormatConvertible<J2000s>>::convert(jd);
        assert!((back - secs).abs() < EPS_S);
    }

    #[test]
    fn jd_mjd_offset() {
        let jd = J2000_JD_TT;
        let mjd = <Jd as FormatConvertible<Mjd>>::convert(jd);
        let expected = jd - JD_MINUS_MJD;
        assert!((mjd - expected).abs() < EPS_D);
    }

    #[test]
    fn j2000s_mjd_round_trip() {
        let secs = Seconds::new(0.0);
        let mjd = <J2000s as FormatConvertible<Mjd>>::convert(secs);
        let back = <Mjd as FormatConvertible<J2000s>>::convert(mjd);
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
        let dc = <Mjd as FormatConvertible<DayCount>>::convert(mjd);
        assert_eq!(dc.value(), 51_544);
        let back = <DayCount as FormatConvertible<Mjd>>::convert(dc);
        assert_eq!(back, Days::new(51_544.0));
    }
}
