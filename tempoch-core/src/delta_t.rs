// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! # ΔT (Delta T) — UT↔TT Correction Layer
//!
//! This module implements a piecewise model for **ΔT = TT − UT1** combining:
//!
//! * **Pre-1620**: Stephenson & Houlden (1986) quadratic approximations.
//! * **1620–1973**: Biennial interpolation table (Meeus ch. 9).
//! * **1973 onward**: generated modern data from USNO monthly determinations
//!   plus the published long-term prediction table compiled into the crate.
//! * **Beyond the last published prediction**: quadratic continuation of the
//!   official prediction tail using a least-squares fit over the final
//!   quarterly points.
//!
//! ## Integration with Time Scales
//!
//! The correction is applied **automatically** by the [`UT`](super::UT) time
//! scale marker.  When you convert from `Time<UT>` to any TT-based scale
//! (`.to::<JD>()`, `.to::<MJD>()`, etc.), `UT::to_jd_tt` adds ΔT.
//! The inverse (`UT::from_jd_tt`) uses a three-iteration fixed-point solver.
//!
//! Note: [`Time::from_utc`](super::Time::from_utc) uses the leap-second table
//! (`UTC → TAI → TT`), **not** the ΔT model.  The ΔT / UT scale is only used
//! when you explicitly construct `Time<UT>` values.
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
//! * USNO/IERS monthly Delta T determinations (`deltat.data`).
//! * USNO long-term Delta T predictions (`deltat.preds`).
//!
//! ## Valid Time Range
//! The historical model is valid from ancient times onward. Modern dates use
//! the generated USNO data compiled into the crate; the effective prediction
//! horizon is the last generated point in `deltat.preds`, after which a
//! quadratic continuation of the official prediction tail is used.

use super::instant::Time;
use super::scales::UT;
use super::JulianDate;
use crate::generated::time_data::{
    MODERN_DELTA_T_END_MJD, MODERN_DELTA_T_POINTS, MODERN_DELTA_T_START_MJD,
};
use qtty::{Day, Second};
use std::sync::OnceLock;

/// Total number of tabulated terms (biennial 1620–1992).
const TERMS: usize = 187;

/// Biennial ΔT table from 1620 to 1992 (in seconds), compiled by J. Meeus.
#[rustfmt::skip]
const DELTA_T: [Second; TERMS] = qtty::qtty_vec!(
    Second;
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
// ΔT Approximation Sections by Time Interval
// ------------------------------------------------------------------------------------

/// **Year < 948 CE**
/// Quadratic formula from Stephenson & Houlden (1986).
#[inline]
fn delta_t_ancient(jd: JulianDate) -> Second {
    const DT_A0_S: Second = Second::new(1_830.0);
    const DT_A1_S: Second = Second::new(-405.0);
    const DT_A2_S: Second = Second::new(46.5);
    const JD_EPOCH_948_UT: JulianDate = JulianDate::new(2_067_314.5);
    let c = (jd - JD_EPOCH_948_UT) / JulianDate::JULIAN_CENTURY;
    DT_A0_S + DT_A1_S * c + DT_A2_S * c * c
}

/// **Year 948–1600 CE**
/// Second polynomial from Stephenson & Houlden (1986).
#[inline]
fn delta_t_medieval(jd: JulianDate) -> Second {
    const JD_EPOCH_1850_UT: JulianDate = JulianDate::new(2_396_758.5);
    const DT_A2_S: Second = Second::new(22.5);

    let c = (jd - JD_EPOCH_1850_UT) / JulianDate::JULIAN_CENTURY;
    DT_A2_S * c * c
}

/// **Year 1600–1992**
/// Bicubic interpolation from the biennial `DELTA_T` table.
#[inline]
fn delta_t_table(jd: JulianDate) -> Second {
    const JD_TABLE_START_1620: JulianDate = JulianDate::new(2_312_752.5);
    const BIENNIAL_STEP_D: Day = Day::new(730.5);

    let mut i = ((jd - JD_TABLE_START_1620) / BIENNIAL_STEP_D) as usize;
    if i > TERMS - 3 {
        i = TERMS - 3;
    }
    let a: Second = DELTA_T[i + 1] - DELTA_T[i];
    let b: Second = DELTA_T[i + 2] - DELTA_T[i + 1];
    let c: Second = a - b;
    let n = (jd - (JD_TABLE_START_1620 + BIENNIAL_STEP_D * i as f64)) / BIENNIAL_STEP_D;
    DELTA_T[i + 1] + n / 2.0 * (a + b + n * c)
}

/// Linearly interpolate a generated modern Delta T series in MJD.
#[inline]
fn interpolate_modern_delta_t(mjd: f64) -> Option<Second> {
    if !(MODERN_DELTA_T_START_MJD..=MODERN_DELTA_T_END_MJD).contains(&mjd) {
        return None;
    }

    let mut lo = 0usize;
    let mut hi = MODERN_DELTA_T_POINTS.len() - 1;
    while lo + 1 < hi {
        let mid = lo + (hi - lo) / 2;
        if MODERN_DELTA_T_POINTS[mid].0 <= mjd {
            lo = mid;
        } else {
            hi = mid;
        }
    }

    let (mjd0, dt0) = MODERN_DELTA_T_POINTS[lo];
    let (mjd1, dt1) = MODERN_DELTA_T_POINTS[hi];

    Some(Second::new(
        dt0 + (mjd - mjd0) * (dt1 - dt0) / (mjd1 - mjd0),
    ))
}

/// **Year >= 1973**
/// Interpolation through the compiled modern Delta T series.
#[inline]
fn delta_t_modern_series(jd: JulianDate) -> Second {
    let mjd = jd.value() - 2_400_000.5;
    interpolate_modern_delta_t(mjd).expect("modern Delta T interpolation requires in-range MJD")
}

/// **Year > generated prediction horizon**
/// Quadratic continuation of the official prediction tail.
///
/// This is still an approximation, but it is materially more stable than a
/// last-two-points linear continuation because it preserves the smooth
/// curvature of the published prediction series near the extrapolation
/// boundary.
const DELTA_T_EXTRAPOLATION_TAIL_POINTS: usize = 12;

/// Quadratic coefficients `(a, b, c, origin)` for the prediction-tail fit.
/// Computed once from the compiled data; cached for all subsequent calls.
static TAIL_FIT: OnceLock<(f64, f64, f64, f64)> = OnceLock::new();

fn compute_tail_fit_coefficients() -> (f64, f64, f64, f64) {
    let tail_len = MODERN_DELTA_T_POINTS
        .len()
        .clamp(3, DELTA_T_EXTRAPOLATION_TAIL_POINTS);
    let tail = &MODERN_DELTA_T_POINTS[MODERN_DELTA_T_POINTS.len() - tail_len..];
    let origin = tail[tail.len() - 1].0;

    let (mut s0, mut s1, mut s2, mut s3, mut s4) = (0.0, 0.0, 0.0, 0.0, 0.0);
    let (mut t0, mut t1, mut t2) = (0.0, 0.0, 0.0);

    for &(sample_mjd, delta_t) in tail {
        let x = sample_mjd - origin;
        let x2 = x * x;
        s0 += 1.0;
        s1 += x;
        s2 += x2;
        s3 += x2 * x;
        s4 += x2 * x2;
        t0 += delta_t;
        t1 += x * delta_t;
        t2 += x2 * delta_t;
    }

    let mut system = [[s0, s1, s2, t0], [s1, s2, s3, t1], [s2, s3, s4, t2]];

    for pivot in 0..3 {
        let mut pivot_row = pivot;
        for row in (pivot + 1)..3 {
            if system[row][pivot].abs() > system[pivot_row][pivot].abs() {
                pivot_row = row;
            }
        }
        if pivot_row != pivot {
            system.swap(pivot, pivot_row);
        }

        let pivot_value = system[pivot][pivot];
        for value in system[pivot].iter_mut().skip(pivot) {
            *value /= pivot_value;
        }

        let pivot_row_values = system[pivot];
        for (row, row_values) in system.iter_mut().enumerate() {
            if row == pivot {
                continue;
            }
            let factor = row_values[pivot];
            for (column, value) in row_values.iter_mut().enumerate().skip(pivot) {
                *value -= factor * pivot_row_values[column];
            }
        }
    }

    (system[0][3], system[1][3], system[2][3], origin)
}

fn quadratic_tail_fit_delta_t_seconds(mjd: f64) -> f64 {
    let &(a, b, c, origin) = TAIL_FIT.get_or_init(compute_tail_fit_coefficients);
    let x = mjd - origin;
    a + b * x + c * x * x
}

#[inline]
fn delta_t_extrapolated(jd: JulianDate) -> Second {
    let mjd = jd.value() - 2_400_000.5;
    Second::new(quadratic_tail_fit_delta_t_seconds(mjd))
}

/// Returns **ΔT** in seconds for a Julian Day on the **UT** axis.
#[inline]
pub(crate) fn delta_t_seconds_from_ut(jd_ut: JulianDate) -> Second {
    match jd_ut {
        jd if jd < JulianDate::new(2_067_314.5) => delta_t_ancient(jd),
        jd if jd < JulianDate::new(2_305_447.5) => delta_t_medieval(jd),
        jd if jd < JulianDate::new(MODERN_DELTA_T_START_MJD + 2_400_000.5) => delta_t_table(jd),
        jd if jd <= JulianDate::new(MODERN_DELTA_T_END_MJD + 2_400_000.5) => {
            delta_t_modern_series(jd)
        }
        _ => delta_t_extrapolated(jd_ut),
    }
}

// ── Time<UT> convenience method ───────────────────────────────────────────

/// The MJD of the last compiled ΔT prediction point (currently 2033-10-01).
///
/// Beyond this date, [`Time::<UT>`] conversions fall back to a quadratic
/// continuation of the published prediction tail.  Use
/// [`Time::<UT>::is_within_delta_t_horizon`] to check whether a given epoch
/// is covered by compiled data.
pub const DELTA_T_PREDICTION_HORIZON_MJD: f64 = MODERN_DELTA_T_END_MJD;

impl Time<UT> {
    /// Returns **ΔT = TT − UT1** in seconds for this UT epoch.
    ///
    /// This is a convenience accessor; the same correction is applied
    /// automatically when converting to any TT-based scale (`.to::<JD>()`).
    #[inline]
    pub fn delta_t(&self) -> Second {
        delta_t_seconds_from_ut(JulianDate::from_days(self.quantity()))
    }

    /// Returns `true` if this epoch is within the compiled ΔT prediction data.
    ///
    /// The compiled data covers observed and official USNO-predicted ΔT values
    /// through MJD [`DELTA_T_PREDICTION_HORIZON_MJD`] (currently 2033-10-01).
    /// For epochs beyond this horizon, [`delta_t`](Self::delta_t) and all
    /// scale conversions use a quadratic extrapolation of the last 12
    /// prediction points; extrapolation uncertainty grows without bound past
    /// the horizon.
    #[inline]
    pub fn is_within_delta_t_horizon(&self) -> bool {
        self.quantity().value() <= MODERN_DELTA_T_END_MJD + 2_400_000.5
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generated::time_data::MODERN_DELTA_T_POINTS;
    use qtty::Day;

    fn quadratic_tail_fit_expected(mjd: f64) -> f64 {
        let tail_len = MODERN_DELTA_T_POINTS
            .len()
            .clamp(3, DELTA_T_EXTRAPOLATION_TAIL_POINTS);
        let tail = &MODERN_DELTA_T_POINTS[MODERN_DELTA_T_POINTS.len() - tail_len..];
        let origin = tail[tail.len() - 1].0;

        let (mut s0, mut s1, mut s2, mut s3, mut s4) = (0.0, 0.0, 0.0, 0.0, 0.0);
        let (mut t0, mut t1, mut t2) = (0.0, 0.0, 0.0);

        for &(sample_mjd, delta_t) in tail {
            let x = sample_mjd - origin;
            let x2 = x * x;
            s0 += 1.0;
            s1 += x;
            s2 += x2;
            s3 += x2 * x;
            s4 += x2 * x2;
            t0 += delta_t;
            t1 += x * delta_t;
            t2 += x2 * delta_t;
        }

        let mut system = [[s0, s1, s2, t0], [s1, s2, s3, t1], [s2, s3, s4, t2]];

        for pivot in 0..3 {
            let mut pivot_row = pivot;
            for row in (pivot + 1)..3 {
                if system[row][pivot].abs() > system[pivot_row][pivot].abs() {
                    pivot_row = row;
                }
            }
            if pivot_row != pivot {
                system.swap(pivot, pivot_row);
            }

            let pivot_value = system[pivot][pivot];
            for value in system[pivot].iter_mut().skip(pivot) {
                *value /= pivot_value;
            }

            let pivot_row_values = system[pivot];
            for (row, row_values) in system.iter_mut().enumerate() {
                if row == pivot {
                    continue;
                }
                let factor = row_values[pivot];
                for (column, value) in row_values.iter_mut().enumerate().skip(pivot) {
                    *value -= factor * pivot_row_values[column];
                }
            }
        }

        let x = mjd - origin;
        system[0][3] + system[1][3] * x + system[2][3] * x * x
    }

    #[test]
    fn delta_t_ancient_sample() {
        let dt = delta_t_seconds_from_ut(JulianDate::new(2_000_000.0));
        assert!((dt - Second::new(2_734.342_214_024_879_5)).abs() < Second::new(1e-6));
    }

    #[test]
    fn delta_t_medieval_sample() {
        let dt = delta_t_seconds_from_ut(JulianDate::new(2_100_000.0));
        assert!((dt - Second::new(1_485.280_240_204_242_3)).abs() < Second::new(1e-6));
    }

    #[test]
    fn delta_t_table_sample() {
        let dt = delta_t_seconds_from_ut(JulianDate::new(2_312_752.5));
        assert!((dt - Second::new(115.0)).abs() < Second::new(1e-6));
    }

    #[test]
    fn delta_t_table_upper_clip() {
        let dt = delta_t_table(JulianDate::new(2_449_356.0));
        assert!((dt - Second::new(59.3)).abs() < Second::new(1e-6));
    }

    #[test]
    fn delta_t_2000() {
        // IERS observed value: 63.83 s
        let dt = delta_t_seconds_from_ut(JulianDate::J2000);
        assert!(
            (dt - Second::new(63.83)).abs() < Second::new(0.1),
            "ΔT at J2000 = {dt}, expected 63.83 s"
        );
    }

    #[test]
    fn delta_t_2010() {
        // IERS observed value for 2010.0: ~66.07 s
        // JD 2455197.5 ≈ 2010-01-01
        let dt = delta_t_seconds_from_ut(JulianDate::new(2_455_197.5));
        assert!(
            (dt - Second::new(66.07)).abs() < Second::new(0.5),
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
            (dt - Second::new(69.36)).abs() < Second::new(0.5),
            "ΔT at 2020.0 = {dt}, expected ~69.36 s"
        );
    }

    #[test]
    fn delta_t_modern_series_matches_generated_endpoints() {
        let first = MODERN_DELTA_T_POINTS[0];
        let last = MODERN_DELTA_T_POINTS[MODERN_DELTA_T_POINTS.len() - 1];
        for (mjd, expected_seconds) in [first, last] {
            let jd = JulianDate::new(mjd + 2_400_000.5);
            let dt = delta_t_seconds_from_ut(jd);
            assert!((dt - Second::new(expected_seconds)).abs() < Second::new(1e-9));
        }
    }

    #[test]
    fn delta_t_extrapolated_matches_quadratic_tail_fit() {
        let future_mjd = MODERN_DELTA_T_POINTS[MODERN_DELTA_T_POINTS.len() - 1].0 + 365.25;
        let expected = quadratic_tail_fit_expected(future_mjd);
        let dt = delta_t_seconds_from_ut(JulianDate::new(future_mjd + 2_400_000.5));
        assert!(
            (dt - Second::new(expected)).abs() < Second::new(1e-9),
            "ΔT extrapolation = {dt}, expected {expected} s"
        );
    }

    #[test]
    fn ut_scale_applies_delta_t() {
        let ut = Time::<UT>::new(2_451_545.0);
        let jd_tt = ut.to::<crate::JD>();
        let offset = jd_tt - JulianDate::new(2_451_545.0);
        let expected =
            delta_t_seconds_from_ut(JulianDate::new(2_451_545.0)).to::<qtty::unit::Day>();
        assert!((offset - expected).abs() < Day::new(1e-9));
    }

    #[test]
    fn ut_scale_roundtrip() {
        let jd_tt = JulianDate::new(2_451_545.0);
        let ut: Time<UT> = jd_tt.to::<UT>();
        let back: JulianDate = ut.to::<crate::JD>();
        assert!((back - jd_tt).abs() < Day::new(1e-12));
    }

    #[test]
    fn delta_t_convenience_method() {
        let ut = Time::<UT>::new(2_451_545.0);
        let dt = ut.delta_t();
        assert!((dt - Second::new(63.83)).abs() < Second::new(0.5));
    }
}
