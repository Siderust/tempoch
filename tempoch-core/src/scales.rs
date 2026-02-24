// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Time-scale marker types.
//!
//! Each zero-sized type identifies a specific time scale and encodes how
//! values in that scale relate to the canonical **Julian Date in TT**
//! (Terrestrial Time).
//!
//! # Epoch counters
//!
//! | Marker | Description | Epoch (JD) |
//! |--------|-------------|------------|
//! | [`JD`] | Julian Date | 0.0 |
//! | [`JDE`] | Julian Ephemeris Day | 0.0 |
//! | [`MJD`] | Modified Julian Date | 2 400 000.5 |
//! | [`UnixTime`] | Seconds since 1970-01-01 | 2 440 587.5 |
//! | [`GPS`] | GPS Time (days) | 2 444 244.5 |
//!
//! # Physical / relativistic scales
//!
//! | Marker | Description |
//! |--------|-------------|
//! | [`TDB`] | Barycentric Dynamical Time (canonical) |
//! | [`TT`]  | Terrestrial Time |
//! | [`TAI`] | International Atomic Time |
//! | [`TCG`] | Geocentric Coordinate Time (IAU 2000) |
//! | [`TCB`] | Barycentric Coordinate Time (IAU 2006) |

use super::instant::TimeScale;
use qtty::Days;

// ---------------------------------------------------------------------------
// Epoch counters
// ---------------------------------------------------------------------------

/// Julian Date — the identity scale.
///
/// `to_jd_tt(v) = v`, i.e. the quantity *is* a Julian Day number.
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub struct JD;

impl TimeScale for JD {
    const LABEL: &'static str = "Julian Day:";

    #[inline(always)]
    fn to_jd_tt(value: Days) -> Days {
        value
    }

    #[inline(always)]
    fn from_jd_tt(jd_tt: Days) -> Days {
        jd_tt
    }
}

/// Julian Ephemeris Day — dynamic Julian day used by ephemerides.
///
/// Numerically this is an absolute Julian day on the TT axis in this crate.
/// It is a semantic label for ephemeris formulas, not a UT→TT conversion.
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub struct JDE;

impl TimeScale for JDE {
    const LABEL: &'static str = "JDE";

    #[inline(always)]
    fn to_jd_tt(value: Days) -> Days {
        value
    }

    #[inline(always)]
    fn from_jd_tt(jd_tt: Days) -> Days {
        jd_tt
    }
}

/// Modified Julian Date — JD minus 2 400 000.5.
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub struct MJD;

/// The constant offset between JD and MJD: `JD = MJD + MJD_EPOCH`.
const MJD_EPOCH: Days = Days::new(2_400_000.5);

impl TimeScale for MJD {
    const LABEL: &'static str = "MJD";

    #[inline(always)]
    fn to_jd_tt(value: Days) -> Days {
        value + MJD_EPOCH
    }

    #[inline(always)]
    fn from_jd_tt(jd_tt: Days) -> Days {
        jd_tt - MJD_EPOCH
    }
}

// ---------------------------------------------------------------------------
// Physical / relativistic scales
// ---------------------------------------------------------------------------

/// Barycentric Dynamical Time.
///
/// TDB differs from TT by a periodic correction of ≈1.7 ms amplitude
/// (Fairhead & Bretagnon 1990).  This implementation applies the four
/// largest periodic terms automatically in `to_jd_tt` / `from_jd_tt`,
/// achieving accuracy better than 30 μs for dates within ±10 000 years
/// of J2000.
///
/// ## References
/// * Fairhead & Bretagnon (1990), A&A 229, 240
/// * USNO Circular 179, eq. 2.6
/// * IAU 2006 Resolution B3
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub struct TDB;

/// Compute TDB − TT in days using the Fairhead & Bretagnon (1990) 4-term
/// expression.  Accuracy: better than 30 μs for |t| < 100 centuries.
#[inline]
pub(crate) fn tdb_minus_tt_days(jd_tt: Days) -> Days {
    // Julian centuries from J2000.0 on the TT axis
    let t = (jd_tt.value() - 2_451_545.0) / 36_525.0;

    // Earth's mean anomaly (radians)
    let m_e = (357.5291092 + 35999.0502909 * t).to_radians();
    // Mean anomaly of Jupiter (radians)
    let m_j = (246.4512 + 3035.2335 * t).to_radians();
    // Mean elongation of the Moon from the Sun (radians)
    let d = (297.8502042 + 445267.1115168 * t).to_radians();
    // Mean longitude of lunar ascending node (radians)
    let om = (125.0445550 - 1934.1362091 * t).to_radians();

    // TDB − TT in seconds (Fairhead & Bretagnon largest terms):
    let dt_sec = 0.001_657 * (m_e + 0.01671 * m_e.sin()).sin()
        + 0.000_022 * (d - m_e).sin()
        + 0.000_014 * (2.0 * d).sin()
        + 0.000_005 * m_j.sin()
        + 0.000_005 * om.sin();

    Days::new(dt_sec / 86_400.0)
}

impl TimeScale for TDB {
    const LABEL: &'static str = "TDB";

    #[inline]
    fn to_jd_tt(tdb_value: Days) -> Days {
        // JD(TT) = JD(TDB) - (TDB - TT)
        // First approximation: use tdb_value as TT to compute the correction.
        // The correction is < 2 ms so one iteration is sufficient for f64.
        tdb_value - tdb_minus_tt_days(tdb_value)
    }

    #[inline]
    fn from_jd_tt(jd_tt: Days) -> Days {
        // JD(TDB) = JD(TT) + (TDB - TT)
        jd_tt + tdb_minus_tt_days(jd_tt)
    }
}

/// Terrestrial Time — the basis for astronomical ephemerides.
///
/// Numerically identical to JD when the Julian Day number already represents
/// TT (which is the convention used throughout this crate).
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub struct TT;

impl TimeScale for TT {
    const LABEL: &'static str = "TT";

    #[inline(always)]
    fn to_jd_tt(value: Days) -> Days {
        value // value is already JD(TT)
    }

    #[inline(always)]
    fn from_jd_tt(jd_tt: Days) -> Days {
        jd_tt
    }
}

/// International Atomic Time.
///
/// `TT = TAI + 32.184 s`.
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub struct TAI;

/// `TT = TAI + 32.184 s` expressed in days.
const TT_MINUS_TAI: Days = Days::new(32.184 / 86_400.0);

impl TimeScale for TAI {
    const LABEL: &'static str = "TAI";

    #[inline(always)]
    fn to_jd_tt(value: Days) -> Days {
        // TAI → TT: add 32.184 s
        value + TT_MINUS_TAI
    }

    #[inline(always)]
    fn from_jd_tt(jd_tt: Days) -> Days {
        // TT → TAI: subtract 32.184 s
        jd_tt - TT_MINUS_TAI
    }
}

// ---------------------------------------------------------------------------
// Coordinate time scales (IAU 2000 / 2006)
// ---------------------------------------------------------------------------

/// Geocentric Coordinate Time — the coordinate time for the GCRS.
///
/// TCG is the proper time experienced by a clock at the geocenter, free from
/// the gravitational time dilation of the Earth's potential.
///
/// The defining relation (IAU 2000 Resolution B1.9) is:
///
/// ```text
/// dTT / dTCG = 1 − L_G
/// ```
///
/// where **L_G = 6.969 290 134 × 10⁻¹⁰** is an IAU defining constant.
/// Integrating:
///
/// ```text
/// TT = TCG − L_G × (JD_TCG − T₀)
/// ```
///
/// with **T₀ = 2 443 144.500 372 5** (JD of 1977 January 1.0 TAI in the TCG scale).
///
/// ## References
/// * IAU 2000 Resolution B1.9
/// * IERS Conventions (2010), §1.2
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub struct TCG;

/// IAU defining constant L_G (IAU 2000 Resolution B1.9).
///
/// Defines the rate difference between TT and TCG:
/// `dTT/dTCG = 1 − L_G`.
const L_G: f64 = 6.969_290_134e-10;

/// TCG epoch T₀: JD(TCG) of 1977 January 1.0 TAI.
const TCG_EPOCH_T0: f64 = 2_443_144.500_372_5;

impl TimeScale for TCG {
    const LABEL: &'static str = "TCG";

    #[inline]
    fn to_jd_tt(tcg_value: Days) -> Days {
        // TT = TCG − L_G × (JD_TCG − T₀)
        let jd_tcg = tcg_value.value();
        Days::new(jd_tcg - L_G * (jd_tcg - TCG_EPOCH_T0))
    }

    #[inline]
    fn from_jd_tt(jd_tt: Days) -> Days {
        // JD_TCG = (JD_TT + L_G × T₀) / (1 − L_G)
        //        ≈ JD_TT + L_G × (JD_TT − T₀)   (first-order, adequate for f64)
        let tt = jd_tt.value();
        Days::new(tt + L_G * (tt - TCG_EPOCH_T0) / (1.0 - L_G))
    }
}

/// Barycentric Coordinate Time — the coordinate time for the BCRS.
///
/// TCB is the time coordinate complementary to barycentric spatial coordinates.
/// It relates to TDB via a linear drift:
///
/// ```text
/// TDB = TCB − L_B × (JD_TCB − T₀) + TDB₀
/// ```
///
/// where:
/// * **L_B = 1.550 519 768 × 10⁻⁸** (IAU 2006 Resolution B3, defining constant)
/// * **TDB₀ = −6.55 × 10⁻⁵ s** (IAU 2006 Resolution B3)
/// * **T₀ = 2 443 144.500 372 5** (JD of 1977 January 1.0 TAI)
///
/// Since TDB ≈ TT for route-through-JD(TT) purposes (the ≈1.7 ms periodic
/// difference is handled separately), we use the TDB relation directly.
///
/// ## References
/// * IAU 2006 Resolution B3
/// * IERS Conventions (2010), §1.2
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub struct TCB;

/// IAU defining constant L_B (IAU 2006 Resolution B3).
///
/// Defines the rate difference between TDB and TCB:
/// `TDB = TCB − L_B × (JD_TCB − T₀) + TDB₀`.
const L_B: f64 = 1.550_519_768e-8;

impl TimeScale for TCB {
    const LABEL: &'static str = "TCB";

    #[inline]
    fn to_jd_tt(tcb_value: Days) -> Days {
        // TDB = TCB − L_B × (JD_TCB − T₀)
        // Treating TDB ≈ TT (periodic ≈1.7 ms difference handled separately).
        // Matches SOFA iauTcbtdb.
        let jd_tcb = tcb_value.value();
        Days::new(jd_tcb - L_B * (jd_tcb - TCG_EPOCH_T0))
    }

    #[inline]
    fn from_jd_tt(jd_tt: Days) -> Days {
        // JD_TCB = JD_TDB + L_B × (JD_TDB − T₀)
        // Matches SOFA iauTdbtcb.
        let tt = jd_tt.value();
        Days::new(tt + L_B * (tt - TCG_EPOCH_T0))
    }
}

// ---------------------------------------------------------------------------
// Navigation counters
// ---------------------------------------------------------------------------

/// GPS Time — continuous day count since 1980-01-06T00:00:00 UTC.
///
/// GPS time has a fixed offset from TAI: `GPS = TAI − 19 s`.
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub struct GPS;

/// JD(TT) of the GPS epoch (1980-01-06T00:00:00 UTC).
/// GPS = TAI − 19s, and TT = TAI + 32.184s, so TT = GPS + 51.184s.
/// GPS epoch in JD(TT): JD 2444244.5 + 51.184/86400.
const GPS_EPOCH_JD_TT: Days = Days::new(2_444_244.5 + 51.184 / 86_400.0);

impl TimeScale for GPS {
    const LABEL: &'static str = "GPS";

    #[inline(always)]
    fn to_jd_tt(value: Days) -> Days {
        value + GPS_EPOCH_JD_TT
    }

    #[inline(always)]
    fn from_jd_tt(jd_tt: Days) -> Days {
        jd_tt - GPS_EPOCH_JD_TT
    }
}

/// Unix Time — seconds since 1970-01-01T00:00:00 UTC, stored as **days**.
///
/// This scale applies the cumulative leap-second offset from IERS Bulletin C
/// to convert between UTC-epoch Unix timestamps and the uniform TT axis.
/// The leap-second table covers 1972–2017 (28 insertions). Prior to 1972
/// (before the leap-second system) the offset is fixed at 10 s (the initial
/// TAI − UTC value adopted on 1972-01-01).
///
/// ## References
/// * IERS Bulletin C (leap second announcements)
/// * POSIX.1-2017 §4.16 (definition of Unix time)
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub struct UnixTime;

/// JD of the Unix epoch (1970-01-01T00:00:00Z).
const UNIX_EPOCH_JD: Days = Days::new(2_440_587.5);

/// Leap-second table: (JD of leap-second insertion, cumulative TAI−UTC after).
/// Source: IERS Bulletin C. Entries are the JD of 00:00:00 UTC on the day
/// the leap second takes effect (i.e. the second is inserted at the end
/// of the previous day).
///
/// TAI = UTC + leap_seconds (for times at or after each entry).
/// TT = TAI + 32.184 s.
const LEAP_SECONDS: [(f64, f64); 28] = [
    (2_441_317.5, 10.0), // 1972-01-01
    (2_441_499.5, 11.0), // 1972-07-01
    (2_441_683.5, 12.0), // 1973-01-01
    (2_442_048.5, 13.0), // 1974-01-01
    (2_442_413.5, 14.0), // 1975-01-01
    (2_442_778.5, 15.0), // 1976-01-01
    (2_443_144.5, 16.0), // 1977-01-01
    (2_443_509.5, 17.0), // 1978-01-01
    (2_443_874.5, 18.0), // 1979-01-01
    (2_444_239.5, 19.0), // 1980-01-01
    (2_444_786.5, 20.0), // 1981-07-01
    (2_445_151.5, 21.0), // 1982-07-01
    (2_445_516.5, 22.0), // 1983-07-01
    (2_446_247.5, 23.0), // 1985-07-01
    (2_447_161.5, 24.0), // 1988-01-01
    (2_447_892.5, 25.0), // 1990-01-01
    (2_448_257.5, 26.0), // 1991-01-01
    (2_448_804.5, 27.0), // 1992-07-01
    (2_449_169.5, 28.0), // 1993-07-01
    (2_449_534.5, 29.0), // 1994-07-01
    (2_450_083.5, 30.0), // 1996-01-01
    (2_450_630.5, 31.0), // 1997-07-01
    (2_451_179.5, 32.0), // 1999-01-01
    (2_453_736.5, 33.0), // 2006-01-01
    (2_454_832.5, 34.0), // 2009-01-01
    (2_456_109.5, 35.0), // 2012-07-01
    (2_457_204.5, 36.0), // 2015-07-01
    (2_457_754.5, 37.0), // 2017-01-01
];

/// Look up cumulative TAI−UTC (leap seconds) for a JD on the UTC axis.
///
/// Returns the number of leap seconds (TAI − UTC) in effect at the given
/// Julian Date on the UTC time axis.  The table covers 1972–2017
/// (28 insertions).  Before 1972 the conventional initial offset of 10 s
/// is returned.
///
/// # Arguments
///
/// * `jd_utc` - Julian Date number on the UTC axis.
#[inline]
pub fn tai_minus_utc(jd_utc: f64) -> f64 {
    // Binary search for the last entry <= jd_utc
    let mut lo = 0usize;
    let mut hi = LEAP_SECONDS.len();
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if LEAP_SECONDS[mid].0 <= jd_utc {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    if lo == 0 {
        // Before 1972-01-01: use the initial offset of 10 s
        // (this is an approximation; pre-1972 UTC had fractional offsets)
        10.0
    } else {
        LEAP_SECONDS[lo - 1].1
    }
}

/// TT = TAI + 32.184 s, and TAI = UTC + leap_seconds.
/// So TT = UTC + leap_seconds + 32.184 s.
const TT_MINUS_TAI_SECS: f64 = 32.184;

impl TimeScale for UnixTime {
    const LABEL: &'static str = "Unix";

    #[inline]
    fn to_jd_tt(value: Days) -> Days {
        // value is Unix days (days since 1970-01-01 on the UTC axis)
        let jd_utc = value.value() + UNIX_EPOCH_JD.value();
        let ls = tai_minus_utc(jd_utc);
        // JD(TT) = JD(UTC) + (TAI−UTC + 32.184) / 86400
        Days::new(jd_utc + (ls + TT_MINUS_TAI_SECS) / 86_400.0)
    }

    #[inline]
    fn from_jd_tt(jd_tt: Days) -> Days {
        // Approximate JD(UTC) by subtracting the largest plausible offset,
        // then refine with the correct leap-second count.
        let approx_utc = jd_tt.value() - (37.0 + TT_MINUS_TAI_SECS) / 86_400.0;
        let ls = tai_minus_utc(approx_utc);
        let jd_utc = jd_tt.value() - (ls + TT_MINUS_TAI_SECS) / 86_400.0;
        Days::new(jd_utc - UNIX_EPOCH_JD.value())
    }
}

// ---------------------------------------------------------------------------
// Universal Time (Earth-rotation based)
// ---------------------------------------------------------------------------

/// Universal Time — the civil time scale tied to Earth's rotation.
///
/// Unlike [`JD`], [`JDE`], and [`TT`] (which all live on the uniform TT
/// axis), `UT` encodes a Julian Day on the **UT** axis.  The conversion
/// to JD(TT) adds the epoch-dependent **ΔT** correction from Meeus (1998)
/// ch. 9, and the inverse uses a three-iteration fixed-point solver
/// with sub-microsecond accuracy.
///
/// [`Time::from_utc`](super::instant::Time::from_utc) routes through this
/// scale automatically, so callers rarely need to construct `Time<UT>` directly.
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub struct UT;

impl TimeScale for UT {
    const LABEL: &'static str = "UT";

    #[inline]
    fn to_jd_tt(ut_value: Days) -> Days {
        let jd_ut = super::instant::Time::<JD>::from_days(ut_value);
        let dt_secs = super::delta_t::delta_t_seconds_from_ut(jd_ut);
        ut_value + dt_secs.to::<qtty::Day>()
    }

    #[inline]
    fn from_jd_tt(jd_tt: Days) -> Days {
        // Solve ut + ΔT(ut)/86400 = tt via fixed-point iteration.
        // dΔT/dJD ≈ 3×10⁻⁸, so convergence is immediate.
        let mut ut = jd_tt;
        for _ in 0..3 {
            let jd_ut = super::instant::Time::<JD>::from_days(ut);
            let dt_days = super::delta_t::delta_t_seconds_from_ut(jd_ut).to::<qtty::Day>();
            ut = jd_tt - dt_days;
        }
        ut
    }
}

// ---------------------------------------------------------------------------
// Cross-scale From/Into and .to::<T>()  (generated by macro)
// ---------------------------------------------------------------------------

/// Generate pairwise `From<Time<A>> for Time<B>` implementations.
macro_rules! impl_time_conversions {
    // Base case: single scale, nothing left.
    ($single:ty) => {};

    // Recursive: generate pairs between $first and every $rest, then recurse.
    ($first:ty, $($rest:ty),+ $(,)?) => {
        $(
            impl From<super::instant::Time<$first>> for super::instant::Time<$rest> {
                #[inline]
                fn from(t: super::instant::Time<$first>) -> Self {
                    t.to::<$rest>()
                }
            }

            impl From<super::instant::Time<$rest>> for super::instant::Time<$first> {
                #[inline]
                fn from(t: super::instant::Time<$rest>) -> Self {
                    t.to::<$first>()
                }
            }
        )+

        impl_time_conversions!($($rest),+);
    };
}

impl_time_conversions!(JD, JDE, MJD, TDB, TT, TAI, TCG, TCB, GPS, UnixTime, UT);

#[cfg(test)]
mod tests {
    use super::super::instant::Time;
    use super::*;
    use qtty::{Day, Second, Seconds};

    #[test]
    fn jd_mjd_roundtrip() {
        let jd = Time::<JD>::new(2_451_545.0);
        let mjd: Time<MJD> = jd.to::<MJD>();
        assert!((mjd.quantity() - Days::new(51_544.5)).abs() < Days::new(1e-10));
        let back: Time<JD> = mjd.to::<JD>();
        assert!((back.quantity() - Days::new(2_451_545.0)).abs() < Days::new(1e-10));
    }

    #[test]
    fn jd_mjd_from_into() {
        let jd = Time::<JD>::new(2_451_545.0);
        let mjd: Time<MJD> = jd.into();
        assert!((mjd.quantity() - Days::new(51_544.5)).abs() < Days::new(1e-10));
        let back: Time<JD> = Time::from(mjd);
        assert!((back.quantity() - Days::new(2_451_545.0)).abs() < Days::new(1e-10));
    }

    #[test]
    fn tai_tt_offset() {
        // TT = TAI + 32.184s
        let tai = Time::<TAI>::new(2_451_545.0);
        let tt: Time<TT> = tai.to::<TT>();
        let expected_offset = Seconds::new(32.184).to::<Day>();
        assert!((tt.quantity() - (tai.quantity() + expected_offset)).abs() < Days::new(1e-15));
    }

    #[test]
    fn gps_epoch_roundtrip() {
        // GPS epoch is JD 2444244.5 (in UTC); in TT it is shifted by 51.184s
        let gps_zero = Time::<GPS>::new(0.0);
        let jd: Time<JD> = gps_zero.to::<JD>();
        let expected = Days::new(2_444_244.5) + Seconds::new(51.184).to::<Day>();
        assert!((jd.quantity() - expected).abs() < Days::new(1e-12));
    }

    #[test]
    fn unix_epoch_roundtrip() {
        // Unix epoch (1970-01-01) has 10 s TAI−UTC and 32.184 s TT−TAI = 42.184 s TT−UTC
        let unix_zero = Time::<UnixTime>::new(0.0);
        let jd: Time<JD> = unix_zero.to::<JD>();
        let expected = Days::new(2_440_587.5) + Seconds::new(42.184).to::<Day>();
        assert!(
            (jd.quantity() - expected).abs() < Days::new(1e-10),
            "Unix epoch JD(TT) = {}, expected {}",
            jd.quantity(),
            expected
        );
    }

    #[test]
    fn unix_2020_leap_seconds() {
        // 2020-01-01 00:00:00 UTC: TAI−UTC = 37 s, TT−UTC = 69.184 s
        // JD(UTC) of 2020-01-01 = 2458849.5
        // Unix days = 2458849.5 - 2440587.5 = 18262.0
        let unix_2020 = Time::<UnixTime>::new(18262.0);
        let jd: Time<JD> = unix_2020.to::<JD>();
        let expected = Days::new(2_458_849.5) + Seconds::new(69.184).to::<Day>();
        assert!(
            (jd.quantity() - expected).abs() < Days::new(1e-10),
            "2020 Unix JD(TT) = {}, expected {}",
            jd.quantity(),
            expected
        );
    }

    #[test]
    fn tdb_tt_offset() {
        // TDB − TT periodic correction is ~1.7 ms at maximum.
        // At J2000.0 t=0, the correction should be small but non-zero.
        let jd = Time::<JD>::new(2_451_545.0);
        let tdb: Time<TDB> = jd.to::<TDB>();
        let offset_secs = (tdb.quantity() - jd.quantity()).to::<Second>();
        // Should be within ±2 ms
        assert!(
            offset_secs.abs() < Seconds::new(0.002),
            "TDB−TT offset at J2000 = {} s, expected < 2 ms",
            offset_secs
        );
    }

    #[test]
    fn tdb_tt_roundtrip() {
        let jd = Time::<JD>::new(2_451_545.0);
        let tdb: Time<TDB> = jd.to::<TDB>();
        let back: Time<JD> = tdb.to::<JD>();
        assert!(
            (back.quantity() - jd.quantity()).abs() < Days::new(1e-14),
            "TDB→TT roundtrip error: {} days",
            (back.quantity() - jd.quantity()).abs()
        );
    }

    #[test]
    fn tcg_tt_offset_at_j2000() {
        // At J2000.0 (JD 2451545.0), TCG runs ahead of TT.
        // TT = TCG − L_G × (JD_TCG − T₀)
        // The offset at J2000 should be ~L_G × (2451545 − 2443144.5) × 86400 s
        //   ≈ 6.97e-10 × 8400.5 × 86400 ≈ 0.506 s
        let tt = Time::<TT>::new(2_451_545.0);
        let tcg: Time<TCG> = tt.to::<TCG>();
        let offset_days = tcg.quantity() - tt.quantity();
        let offset_secs = offset_days.to::<Second>();
        // TCG should be ahead of TT by ~0.506 s at J2000
        assert!(
            (offset_secs - Seconds::new(0.506)).abs() < Seconds::new(0.01),
            "TCG−TT offset at J2000 = {} s, expected ~0.506 s",
            offset_secs
        );
    }

    #[test]
    fn tcg_tt_roundtrip() {
        let tt = Time::<TT>::new(2_451_545.0);
        let tcg: Time<TCG> = tt.to::<TCG>();
        let back: Time<TT> = tcg.to::<TT>();
        assert!(
            (back.quantity() - tt.quantity()).abs() < Days::new(1e-12),
            "TCG→TT roundtrip error: {} days",
            (back.quantity() - tt.quantity()).abs()
        );
    }

    #[test]
    fn tcb_tdb_offset_at_j2000() {
        // TCB runs significantly ahead of TDB.
        // Offset ≈ L_B × (2451545 − 2443144.5) × 86400 s
        //        ≈ 1.55e-8 × 8400.5 × 86400 ≈ 11.25 s
        let tt = Time::<TT>::new(2_451_545.0);
        let tcb: Time<TCB> = tt.to::<TCB>();
        let offset_days = tcb.quantity() - tt.quantity();
        let offset_secs = offset_days.to::<Second>();
        // TCB should be ahead of TT/TDB by ~11.25 s at J2000
        assert!(
            (offset_secs - Seconds::new(11.25)).abs() < Seconds::new(0.5),
            "TCB−TT offset at J2000 = {} s, expected ~11.25 s",
            offset_secs
        );
    }

    #[test]
    fn tcb_tt_roundtrip() {
        let tt = Time::<TT>::new(2_458_000.0);
        let tcb: Time<TCB> = tt.to::<TCB>();
        let back: Time<TT> = tcb.to::<TT>();
        assert!(
            (back.quantity() - tt.quantity()).abs() < Days::new(1e-10),
            "TCB→TT roundtrip error: {} days",
            (back.quantity() - tt.quantity()).abs()
        );
    }

    #[test]
    fn ut_to_jd_applies_delta_t() {
        let ut = Time::<UT>::new(2_451_545.0);
        let jd: Time<JD> = ut.to::<JD>();
        // ΔT at J2000 ≈ 63.83 s
        let offset_secs = (jd.quantity() - ut.quantity()).to::<Second>();
        assert!(
            (offset_secs - Seconds::new(63.83)).abs() < Seconds::new(1.0),
            "UT→JD offset = {} s, expected ~63.83 s",
            offset_secs
        );
    }

    #[test]
    fn ut_jd_roundtrip() {
        let jd = Time::<JD>::new(2_451_545.0);
        let ut: Time<UT> = jd.to::<UT>();
        let back: Time<JD> = ut.to::<JD>();
        assert!(
            (back.quantity() - jd.quantity()).abs() < Days::new(1e-12),
            "roundtrip error: {} days",
            (back.quantity() - jd.quantity()).abs()
        );
    }

    #[test]
    fn ut_from_into() {
        let ut = Time::<UT>::new(2_451_545.0);
        let jd: Time<JD> = ut.into();
        let back: Time<UT> = jd.into();
        assert!((back.quantity() - ut.quantity()).abs() < Days::new(1e-12));
    }

    // ── New coverage tests ────────────────────────────────────────────

    #[test]
    fn jde_roundtrip() {
        // JDE is numerically identical to JD; test both conversion directions.
        let jde = Time::<JDE>::new(2_451_545.0);
        let jd: Time<JD> = jde.to::<JD>();
        assert!((jd.quantity() - Days::new(2_451_545.0)).abs() < Days::new(1e-10));
        let back: Time<JDE> = jd.to::<JDE>();
        assert!((back.quantity() - jde.quantity()).abs() < Days::new(1e-10));
    }

    #[test]
    fn tt_to_tai() {
        // TT = TAI + 32.184 s  ⟹  TAI = TT − 32.184 s.
        // This exercises TAI::from_jd_tt.
        let tt = Time::<TT>::new(2_451_545.0);
        let tai: Time<TAI> = tt.to::<TAI>();
        // Round-trip: TAI → TT should recover the original TT value.
        let back: Time<TT> = tai.to::<TT>();
        assert!(
            (back.quantity() - tt.quantity()).abs() < Days::new(1e-15),
            "TT → TAI → TT roundtrip error: {}",
            (back.quantity() - tt.quantity()).abs()
        );
    }

    #[test]
    fn gps_from_jd() {
        // Round-trip JD → GPS → JD; exercises GPS::from_jd_tt.
        let gps_zero = Time::<GPS>::new(0.0);
        let jd: Time<JD> = gps_zero.to::<JD>();
        let back: Time<GPS> = jd.to::<GPS>();
        assert!((back.quantity() - gps_zero.quantity()).abs() < Days::new(1e-12));
    }

    #[test]
    fn unix_from_jd() {
        // Round-trip UnixTime → JD → UnixTime; exercises UnixTime::from_jd_tt.
        let unix_2020 = Time::<UnixTime>::new(18262.0); // 2020-01-01 UTC
        let jd: Time<JD> = unix_2020.to::<JD>();
        let back: Time<UnixTime> = jd.to::<UnixTime>();
        assert!(
            (back.quantity() - unix_2020.quantity()).abs() < Days::new(1e-10),
            "UnixTime roundtrip error: {} days",
            (back.quantity() - unix_2020.quantity()).abs()
        );
    }

    #[test]
    fn tai_minus_utc_pre_1972_returns_10() {
        // Before 1972-01-01 (JD 2 441 317.5) the leap-second table has no
        // applicable entry, so the conventional initial 10 s offset is returned.
        assert_eq!(tai_minus_utc(2_400_000.0), 10.0);
    }
}
