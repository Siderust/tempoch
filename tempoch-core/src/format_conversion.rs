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

use super::constats::{GPS_EPOCH_TAI, J2000_JD_TT, JD_MINUS_MJD};
use super::encoding::{
    j2000_seconds_to_jd, j2000_seconds_to_mjd, jd_to_j2000_seconds, mjd_to_j2000_seconds,
};
use super::format::{DayCount, GpsSecs, J2000s, Jd, Mjd, UnixSecs};
use super::sealed::Sealed;
use qtty::time::{Days, Seconds};
use qtty::unit::{Day, Second};
use qtty::{QuantityI32, QuantityI64};

// ── CanonicalRoundtrip ───────────────────────────────────────────────────

/// Witness that a format can losslessly round-trip through J2000 SI seconds
/// (`Quantity<Second, f64>`). Required for scale conversions.
///
/// Integer-based formats (`UnixSecs`, `DayCount`) do **not** implement this
/// trait. Users must `.reformat::<J2000s>()` first, making the precision
/// trade-off explicit at the call site.
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

impl CanonicalRoundtrip for GpsSecs {
    #[inline]
    fn to_j2000s(src: Seconds) -> Seconds {
        // GPS seconds are referenced to the GPS epoch on the TAI axis.
        // GPS_EPOCH_TAI is already expressed as J2000 TT seconds.
        src + GPS_EPOCH_TAI
    }
    #[inline]
    fn from_j2000s(secs: Seconds) -> Seconds {
        secs - GPS_EPOCH_TAI
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
identity_format!(J2000s, Jd, Mjd, UnixSecs, GpsSecs, DayCount);

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

// -- J2000s ↔ UnixSecs (lossy: f64 → i64) --------------------------------

impl FormatConvertible<UnixSecs> for J2000s {
    #[inline]
    fn convert(src: Seconds) -> QuantityI64<Second> {
        // J2000 TT is 2000-01-01T12:00:00 TT.
        // Unix epoch is 1970-01-01T00:00:00 UTC.
        // Offset: J2000_JD_TT - UNIX_EPOCH_JD in seconds.
        let unix_offset_secs: Seconds =
            (J2000_JD_TT - crate::constats::UNIX_EPOCH_JD).to::<Second>();
        let unix_secs = src + unix_offset_secs;
        QuantityI64::<Second>::new(unix_secs.value() as i64)
    }
}

impl FormatConvertible<J2000s> for UnixSecs {
    #[inline]
    fn convert(src: QuantityI64<Second>) -> Seconds {
        let unix_offset_secs: Seconds =
            (J2000_JD_TT - crate::constats::UNIX_EPOCH_JD).to::<Second>();
        Seconds::new(src.value() as f64) - unix_offset_secs
    }
}

// -- Mjd ↔ DayCount (lossy: f64 → i32) -----------------------------------

impl FormatConvertible<DayCount> for Mjd {
    #[inline]
    fn convert(src: Days) -> QuantityI32<Day> {
        QuantityI32::<Day>::new(src.value() as i32)
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

// -- Jd ↔ UnixSecs (through J2000s) ----------------------------------------

impl FormatConvertible<UnixSecs> for Jd {
    #[inline]
    fn convert(src: Days) -> QuantityI64<Second> {
        <J2000s as FormatConvertible<UnixSecs>>::convert(jd_to_j2000_seconds(src))
    }
}

impl FormatConvertible<Jd> for UnixSecs {
    #[inline]
    fn convert(src: QuantityI64<Second>) -> Days {
        j2000_seconds_to_jd(<UnixSecs as FormatConvertible<J2000s>>::convert(src))
    }
}

// -- Mjd ↔ UnixSecs (through J2000s) ----------------------------------------

impl FormatConvertible<UnixSecs> for Mjd {
    #[inline]
    fn convert(src: Days) -> QuantityI64<Second> {
        <J2000s as FormatConvertible<UnixSecs>>::convert(mjd_to_j2000_seconds(src))
    }
}

impl FormatConvertible<Mjd> for UnixSecs {
    #[inline]
    fn convert(src: QuantityI64<Second>) -> Days {
        j2000_seconds_to_mjd(<UnixSecs as FormatConvertible<J2000s>>::convert(src))
    }
}

// -- Jd ↔ DayCount (through Mjd) -------------------------------------------

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

// -- GpsSecs ↔ UnixSecs (through J2000s) -----------------------------------

impl FormatConvertible<UnixSecs> for GpsSecs {
    #[inline]
    fn convert(src: Seconds) -> QuantityI64<Second> {
        <J2000s as FormatConvertible<UnixSecs>>::convert(
            <GpsSecs as FormatConvertible<J2000s>>::convert(src),
        )
    }
}

impl FormatConvertible<GpsSecs> for UnixSecs {
    #[inline]
    fn convert(src: QuantityI64<Second>) -> Seconds {
        <J2000s as FormatConvertible<GpsSecs>>::convert(
            <UnixSecs as FormatConvertible<J2000s>>::convert(src),
        )
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

// -- UnixSecs ↔ DayCount (through J2000s → Mjd) ----------------------------

impl FormatConvertible<DayCount> for UnixSecs {
    #[inline]
    fn convert(src: QuantityI64<Second>) -> QuantityI32<Day> {
        <J2000s as FormatConvertible<DayCount>>::convert(
            <UnixSecs as FormatConvertible<J2000s>>::convert(src),
        )
    }
}

impl FormatConvertible<UnixSecs> for DayCount {
    #[inline]
    fn convert(src: QuantityI32<Day>) -> QuantityI64<Second> {
        <J2000s as FormatConvertible<UnixSecs>>::convert(
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
    fn j2000s_unix_round_trip_approximate() {
        // i64 truncation loses sub-second precision
        let secs = Seconds::new(1_000_000.0);
        let unix = <J2000s as FormatConvertible<UnixSecs>>::convert(secs);
        let back = <UnixSecs as FormatConvertible<J2000s>>::convert(unix);
        assert!((back - secs).abs() < Seconds::new(1.0));
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
