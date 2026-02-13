// SPDX-License-Identifier: AGPL-3.0-or-later
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
/// For the purposes of this library **TDB is treated as numerically equal to
/// TT** on the Julian-day axis (offset = identity).  The ≈1.7 ms periodic
/// correction can be applied via `Time::<TT>::to_tdb()` when higher accuracy
/// is required, matching the convention used by VSOP87 and ELP2000.
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub struct TDB;

impl TimeScale for TDB {
    const LABEL: &'static str = "TDB";

    #[inline(always)]
    fn to_jd_tt(value: Days) -> Days {
        // TDB ≈ TT for our level of accuracy; the periodic term is
        // small enough that we treat them as equal on the JD axis.
        value
    }

    #[inline(always)]
    fn from_jd_tt(jd_tt: Days) -> Days {
        jd_tt
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

/// TDB₀ offset in days (IAU 2006 Resolution B3): −6.55 × 10⁻⁵ s.
const TDB0_DAYS: f64 = -6.55e-5 / 86_400.0;

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
/// Note: This scale ignores leap seconds (as POSIX does).
/// The relationship `UTC ≈ TT − ΔT` means there is a slowly-drifting offset
/// between Unix time and TT.  Here we use the fixed JD of the Unix epoch
/// as if UTC = TT for simplicity (consistent with the rest of the crate).
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub struct UnixTime;

/// JD of the Unix epoch (1970-01-01T00:00:00Z).
const UNIX_EPOCH_JD: Days = Days::new(2_440_587.5);

impl TimeScale for UnixTime {
    const LABEL: &'static str = "Unix";

    #[inline(always)]
    fn to_jd_tt(value: Days) -> Days {
        value + UNIX_EPOCH_JD
    }

    #[inline(always)]
    fn from_jd_tt(jd_tt: Days) -> Days {
        jd_tt - UNIX_EPOCH_JD
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
        let unix_zero = Time::<UnixTime>::new(0.0);
        let jd: Time<JD> = unix_zero.to::<JD>();
        assert!((jd.quantity() - Days::new(2_440_587.5)).abs() < Days::new(1e-12));
    }

    #[test]
    fn tdb_identity() {
        let jd = Time::<JD>::new(2_451_545.0);
        let tdb: Time<TDB> = jd.to::<TDB>();
        assert!((tdb.quantity() - jd.quantity()).abs() < Days::new(1e-15));
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
}
