// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! # ΔT (Delta T) — UT↔TT Correction Layer
//!
//! This module implements a piecewise model for **ΔT = TT − UT** combining:
//!
//! * **Pre-1620**: Stephenson & Houlden (1986) quadratic approximations.
//! * **1620–1992**: Biennial interpolation table (Meeus ch. 9).
//! * **1992–2025**: Annual observed ΔT values from IERS/USNO (Bulletin A).
//! * **Post-2025**: Linear extrapolation at the current observed rate
//!   (~+0.1 s/yr), far more accurate than the Meeus quadratic formula
//!   which diverges to ~120 s by 2020. The IERS-observed value for 2025
//!   is ~69.36 s.
//!
//! ## Integration with Time Scales
//!
//! The correction is applied **automatically** by the [`UT`](super::UT) time
//! scale marker.  When you convert from `Time<UT>` to any TT-based scale
//! (`.to::<JD>()`, `.to::<MJD>()`, etc.), `UT::to_jd_tt` adds ΔT.
//! The inverse (`UT::from_jd_tt`) uses a three-iteration fixed-point solver.
//!
//! [`Time::from_utc`](super::Time::from_utc) creates a `Time<UT>` internally
//! and then converts to the target scale, so external callers get the ΔT
//! correction without calling any function from this module.
//!
//! ## Quick Example
//! ```rust
//! # use tempoch_core as tempoch;
//! use tempoch::{UT, JD, Time};
//!
//! // UT-based Julian Day -> JD(TT) with ΔT applied
//! let ut = Time::<UT>::new(2_451_545.0);
//! let jd_tt = ut.to::<JD>();
//! println!("JD(TT) = {jd_tt}");
//!
//! // Query the raw ΔT value
//! let dt = ut.delta_t();
//! println!("ΔT = {dt}");
//! ```
//!
//! ## Scientific References
//! * Stephenson & Houlden (1986): *Atlas of Historical Eclipse Maps*.
//! * Morrison & Stephenson (2004): "Historical values of the Earth's clock error".
//! * IERS Conventions (2020): official ΔT data tables.
//! * IERS Bulletin A (2025): observed ΔT values.
//!
//! ## Valid Time Range
//! The algorithm is valid from ancient times through approximately 2035, with
//! typical uncertainties ≤ ±2 s before 1800 CE, ≤ ±0.5 s since 1900, and
//! ≤ ±0.1 s for 2000–2025 (observed data).

use super::instant::Time;
use super::scales::UT;
use super::JulianDate;
use qtty::{Days, Seconds, Simplify};

/// Total number of tabulated terms (biennial 1620–1992).
const TERMS: usize = 187;

/// Biennial ΔT table from 1620 to 1992 (in seconds), compiled by J. Meeus.
#[rustfmt::skip]
const DELTA_T: [Seconds; TERMS] = qtty::qtty_vec!(
    Seconds;
    124.0,115.0,106.0, 98.0, 91.0, 85.0, 79.0, 74.0, 70.0, 65.0,
     62.0, 58.0, 55.0, 53.0, 50.0, 48.0, 46.0, 44.0, 42.0, 40.0,
     37.0, 35.0, 33.0, 31.0, 28.0, 26.0, 24.0, 22.0, 20.0, 18.0,
     16.0, 14.0, 13.0, 12.0, 11.0, 10.0,  9.0,  9.0,  9.0,  9.0,
      9.0,  9.0,  9.0,  9.0, 10.0, 10.0, 10.0, 10.0, 10.0, 11.0,
     11.0, 11.0, 11.0, 11.0, 11.0, 11.0, 12.0, 12.0, 12.0, 12.0,
     12.0, 12.0, 13.0, 13.0, 13.0, 13.0, 14.0, 14.0, 14.0, 15.0,
     15.0, 15.0, 15.0, 16.0, 16.0, 16.0, 16.0, 16.0, 17.0, 17.0,
     17.0, 17.0, 17.0, 17.0, 17.0, 17.0, 16.0, 16.0, 15.0, 14.0,
     13.7, 13.1, 12.7, 12.5, 12.5, 12.5, 12.5, 12.5, 12.5, 12.3,
     12.0, 11.4, 10.6,  9.6,  8.6,  7.5,  6.6,  6.0,  5.7,  5.6,
      5.7,  5.9,  6.2,  6.5,  6.8,  7.1,  7.3,  7.5,  7.7,  7.8,
      7.9,  7.5,  6.4,  5.4,  2.9,  1.6, -1.0, -2.7, -3.6, -4.7,
     -5.4, -5.2, -5.5, -5.6, -5.8, -5.9, -6.2, -6.4, -6.1, -4.7,
     -2.7,  0.0,  2.6,  5.4,  7.7, 10.5, 13.4, 16.0, 18.2, 20.2,
     21.2, 22.4, 23.5, 23.9, 24.3, 24.0, 23.9, 23.9, 23.7, 24.0,
     24.3, 25.3, 26.2, 27.3, 28.2, 29.1, 30.0, 30.7, 31.4, 32.2,
     33.1, 34.0, 35.0, 36.5, 38.3, 40.2, 42.2, 44.5, 46.5, 48.5,
     50.5, 52.2, 53.8, 54.9, 55.8, 56.9, 58.3,
);

// ------------------------------------------------------------------------------------
// Annual observed ΔT table 1992–2025 (IERS/USNO Bulletin A)
// ------------------------------------------------------------------------------------

/// Annual ΔT values (seconds) from IERS/USNO observations, 1992.0–2025.0.
/// Index 0 = year 1992, index 33 = year 2025.
/// Source: IERS Bulletin A, USNO finals2000A data.
const OBSERVED_TERMS: usize = 34;
const OBSERVED_START_YEAR: f64 = 1992.0;

#[rustfmt::skip]
const OBSERVED_DT: [Seconds; OBSERVED_TERMS] = qtty::qtty_vec!(
    Seconds;
    // 1992  1993   1994   1995   1996   1997   1998   1999
    58.31, 59.12, 59.98, 60.78, 61.63, 62.30, 62.97, 63.47,
    // 2000  2001   2002   2003   2004   2005   2006   2007
    63.83, 64.09, 64.30, 64.47, 64.57, 64.69, 64.85, 65.15,
    // 2008  2009   2010   2011   2012   2013   2014   2015
    65.46, 65.78, 66.07, 66.32, 66.60, 66.91, 67.28, 67.64,
    // 2016  2017   2018   2019   2020   2021   2022   2023
    68.10, 68.59, 68.97, 69.22, 69.36, 69.36, 69.29, 69.18,
    // 2024  2025
    69.09, 69.36,
);

/// The year after the last observed data point. Beyond this we extrapolate.
const OBSERVED_END_YEAR: f64 = OBSERVED_START_YEAR + OBSERVED_TERMS as f64;

/// Last observed ΔT rate (seconds/year). Computed from the last 5 years of
/// observed data. The rate has been nearly flat 2019–2025 (~+0.02 s/yr).
const EXTRAPOLATION_RATE: f64 = 0.02;

// ------------------------------------------------------------------------------------
// ΔT Approximation Sections by Time Interval
// ------------------------------------------------------------------------------------

/// **Years < 948 CE**
/// Quadratic formula from Stephenson & Houlden (1986).
#[inline]
fn delta_t_ancient(jd: JulianDate) -> Seconds {
    const DT_A0_S: Seconds = Seconds::new(1_830.0);
    const DT_A1_S: Seconds = Seconds::new(-405.0);
    const DT_A2_S: Seconds = Seconds::new(46.5);
    const JD_EPOCH_948_UT: JulianDate = JulianDate::new(2_067_314.5);
    let c = days_ratio(jd - JD_EPOCH_948_UT, JulianDate::JULIAN_CENTURY);
    DT_A0_S + DT_A1_S * c + DT_A2_S * c * c
}

/// **Years 948–1600 CE**
/// Second polynomial from Stephenson & Houlden (1986).
#[inline]
fn delta_t_medieval(jd: JulianDate) -> Seconds {
    const JD_EPOCH_1850_UT: JulianDate = JulianDate::new(2_396_758.5);
    const DT_A2_S: Seconds = Seconds::new(22.5);

    let c = days_ratio(jd - JD_EPOCH_1850_UT, JulianDate::JULIAN_CENTURY);
    DT_A2_S * c * c
}

/// **Years 1600–1992**
/// Bicubic interpolation from the biennial `DELTA_T` table.
#[inline]
fn delta_t_table(jd: JulianDate) -> Seconds {
    const JD_TABLE_START_1620: JulianDate = JulianDate::new(2_312_752.5);
    const BIENNIAL_STEP_D: Days = Days::new(730.5);

    let mut i = days_ratio(jd - JD_TABLE_START_1620, BIENNIAL_STEP_D) as usize;
    if i > TERMS - 3 {
        i = TERMS - 3;
    }
    let a: Seconds = DELTA_T[i + 1] - DELTA_T[i];
    let b: Seconds = DELTA_T[i + 2] - DELTA_T[i + 1];
    let c: Seconds = a - b;
    let n = days_ratio(
        jd - (JD_TABLE_START_1620 + BIENNIAL_STEP_D * i as f64),
        BIENNIAL_STEP_D,
    );
    DELTA_T[i + 1] + n / 2.0 * (a + b + n * c)
}

/// **Years 1992–2026**
/// Linear interpolation from annual IERS/USNO observed ΔT values.
#[inline]
fn delta_t_observed(jd: JulianDate) -> Seconds {
    // Convert JD to fractional year
    let year = 2000.0 + (jd - JulianDate::J2000).value() / 365.25;
    let idx_f = year - OBSERVED_START_YEAR;
    let idx = idx_f as usize;

    if idx + 1 >= OBSERVED_TERMS {
        // At the very end of the table, return the last value
        return OBSERVED_DT[OBSERVED_TERMS - 1];
    }

    // Linear interpolation between annual values
    let frac = idx_f - idx as f64;
    OBSERVED_DT[idx] + frac * (OBSERVED_DT[idx + 1] - OBSERVED_DT[idx])
}

/// **Years > 2026**
/// Linear extrapolation from the last observed value at the current rate.
///
/// The observed ΔT trend 2019–2025 is nearly flat (~+0.02 s/yr), which is
/// far more accurate than the Meeus quadratic that predicted ~121 s for 2020
/// vs the observed ~69.36 s.
#[inline]
fn delta_t_extrapolated(jd: JulianDate) -> Seconds {
    let year = 2000.0 + (jd - JulianDate::J2000).value() / 365.25;
    let dt_last = OBSERVED_DT[OBSERVED_TERMS - 1];
    let years_past = year - OBSERVED_END_YEAR;
    dt_last + Seconds::new(EXTRAPOLATION_RATE * years_past)
}

#[inline]
fn days_ratio(num: Days, den: Days) -> f64 {
    (num / den).simplify().value()
}

/// JD boundary: start of year 1992.0
const JD_1992: JulianDate = JulianDate::new(2_448_622.5);

/// JD boundary: start of year 2026.0
const JD_2026: JulianDate = JulianDate::new(2_461_041.5);

/// Returns **ΔT** in seconds for a Julian Day on the **UT** axis.
#[inline]
pub(crate) fn delta_t_seconds_from_ut(jd_ut: JulianDate) -> Seconds {
    match jd_ut {
        jd if jd < JulianDate::new(2_067_314.5) => delta_t_ancient(jd),
        jd if jd < JulianDate::new(2_305_447.5) => delta_t_medieval(jd),
        jd if jd < JD_1992 => delta_t_table(jd),
        jd if jd < JD_2026 => delta_t_observed(jd),
        _ => delta_t_extrapolated(jd_ut),
    }
}

// ── Time<UT> convenience method ───────────────────────────────────────────

impl Time<UT> {
    /// Returns **ΔT = TT − UT** in seconds for this UT epoch.
    ///
    /// This is a convenience accessor; the same correction is applied
    /// automatically when converting to any TT-based scale (`.to::<JD>()`).
    #[inline]
    pub fn delta_t(&self) -> Seconds {
        delta_t_seconds_from_ut(JulianDate::from_days(self.quantity()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qtty::{Day, Days};

    #[test]
    fn delta_t_ancient_sample() {
        let dt = delta_t_seconds_from_ut(JulianDate::new(2_000_000.0));
        assert!((dt - Seconds::new(2_734.342_214_024_879_5)).abs() < Seconds::new(1e-6));
    }

    #[test]
    fn delta_t_medieval_sample() {
        let dt = delta_t_seconds_from_ut(JulianDate::new(2_100_000.0));
        assert!((dt - Seconds::new(1_485.280_240_204_242_3)).abs() < Seconds::new(1e-6));
    }

    #[test]
    fn delta_t_table_sample() {
        let dt = delta_t_seconds_from_ut(JulianDate::new(2_312_752.5));
        assert!((dt - Seconds::new(115.0)).abs() < Seconds::new(1e-6));
    }

    #[test]
    fn delta_t_table_upper_clip() {
        let dt = delta_t_table(JulianDate::new(2_449_356.0));
        assert!((dt - Seconds::new(59.3)).abs() < Seconds::new(1e-6));
    }

    #[test]
    fn delta_t_2000() {
        // IERS observed value: 63.83 s
        let dt = delta_t_seconds_from_ut(JulianDate::J2000);
        assert!(
            (dt - Seconds::new(63.83)).abs() < Seconds::new(0.1),
            "ΔT at J2000 = {dt}, expected 63.83 s"
        );
    }

    #[test]
    fn delta_t_2010() {
        // IERS observed value for 2010.0: ~66.07 s
        // JD 2455197.5 ≈ 2010-01-01
        let dt = delta_t_seconds_from_ut(JulianDate::new(2_455_197.5));
        assert!(
            (dt - Seconds::new(66.07)).abs() < Seconds::new(0.5),
            "ΔT at 2010. = {dt}, expected ~66.07 s"
        );
    }

    #[test]
    fn delta_t_2020() {
        // IERS observed value for 2020.0: ~69.36 s
        // The old Meeus extrapolation gave ~121 s here — way off.
        // JD for 2020-01-01 ≈ 2458849.5
        let dt = delta_t_seconds_from_ut(JulianDate::new(2_458_849.5));
        assert!(
            (dt - Seconds::new(69.36)).abs() < Seconds::new(0.5),
            "ΔT at 2020.0 = {dt}, expected ~69.36 s"
        );
    }

    #[test]
    fn delta_t_2025() {
        // IERS observed value for 2025.0: ~69.36 s
        // JD for 2025-01-01 ≈ 2460676.5
        let dt = delta_t_seconds_from_ut(JulianDate::new(2_460_676.5));
        assert!(
            (dt - Seconds::new(69.36)).abs() < Seconds::new(0.5),
            "ΔT at 2025.0 = {dt}, expected ~69.36 s"
        );
    }

    #[test]
    fn delta_t_extrapolated_near_future() {
        // Beyond 2026, linear extrapolation at ~0.02 s/yr
        // At 2030.0 (4 yr past end), ΔT ≈ 69.36 + 0.02*4 ≈ 69.44
        let jd_2030 = JulianDate::new(2_462_502.5);
        let dt = delta_t_seconds_from_ut(jd_2030);
        assert!(
            (dt - Seconds::new(69.44)).abs() < Seconds::new(1.0),
            "ΔT at 2030. = {dt}, expected ~69.44 s"
        );
        // Must NOT be the old ~135+ s value
        assert!(dt < Seconds::new(75.0), "ΔT at 2030 is too large: {dt}");
    }

    #[test]
    fn ut_scale_applies_delta_t() {
        let ut = Time::<UT>::new(2_451_545.0);
        let jd_tt = ut.to::<crate::JD>();
        let offset = jd_tt - JulianDate::new(2_451_545.0);
        let expected = delta_t_seconds_from_ut(JulianDate::new(2_451_545.0)).to::<Day>();
        assert!((offset - expected).abs() < Days::new(1e-9));
    }

    #[test]
    fn ut_scale_roundtrip() {
        let jd_tt = JulianDate::new(2_451_545.0);
        let ut: Time<UT> = jd_tt.to::<UT>();
        let back: JulianDate = ut.to::<crate::JD>();
        assert!((back - jd_tt).abs() < Days::new(1e-12));
    }

    #[test]
    fn delta_t_convenience_method() {
        let ut = Time::<UT>::new(2_451_545.0);
        let dt = ut.delta_t();
        assert!((dt - Seconds::new(63.83)).abs() < Seconds::new(0.5));
    }
}
