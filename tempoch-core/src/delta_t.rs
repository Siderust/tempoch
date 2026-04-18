// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! ΔT (TT − UT1) model.
//!
//! Piecewise model combining:
//!
//! * **Pre-948 CE**: Stephenson & Houlden (1986) quadratic with epoch 948.
//! * **948–1619**: Stephenson & Houlden (1986) quadratic with epoch 1850.
//! * **1620–1973**: Biennial interpolation table (Meeus ch. 9).
//! * **1973 onward**: Generated modern data (USNO monthly determinations).
//! * **Beyond the last published prediction**: Quadratic continuation of the
//!   last 12 prediction points.
//!
//! ## C0 continuity corrections
//!
//! The three pre-modern sub-models are independent historical approximations
//! (Stephenson & Houlden 1986; Meeus *Astronomical Algorithms* 2nd ed.) and do
//! not agree exactly at their boundary epochs.  To avoid non-physical
//! discontinuities, constant additive offsets are applied to each formula:
//!
//! | Offset | Value | Matched boundary |
//! |---|---|---|
//! | `MEDIEVAL_OFFSET` | +4.979 251 s | medieval formula → `DELTA_T[0]` at 1620 CE |
//! | `ANCIENT_OFFSET`  | +5.460 454 s | ancient formula → corrected medieval at 948 CE |
//!
//! Both corrections are well within the stated accuracy of their respective
//! sub-models (±15 s for 948–1620; ±hundreds of seconds before 948).

use crate::constats::DAYS_PER_JC;
use crate::encoding::jd_to_mjd;
use crate::error::ConversionError;
use crate::generated::time_data::MODERN_DELTA_T_POINTS;
use crate::generated::{MODERN_DELTA_T_END_MJD, MODERN_DELTA_T_START_MJD};
use qtty::{Day, Second};
use std::sync::OnceLock;

const JD_EPOCH_948_UT: Day = Day::new(2_067_314.5);
const JD_EPOCH_1850_UT: Day = Day::new(2_396_758.5);
const JD_TABLE_START_1620: Day = Day::new(2_312_752.5);
const BIENNIAL_STEP_D: Day = Day::new(730.5);

// C0 continuity offsets (see module doc).
// MEDIEVAL_OFFSET = DELTA_T[0] − medieval(JD_TABLE_START_1620) = 124.0 − 119.020750
const MEDIEVAL_OFFSET: f64 = 4.979_250_475_399_4;
// ANCIENT_OFFSET = (medieval(JD_EPOCH_948_UT) + MEDIEVAL_OFFSET) − ancient(JD_EPOCH_948_UT)
//               = 1835.460454 − 1830.0
const ANCIENT_OFFSET: f64 = 5.460_453_937_909_5;

const TERMS: usize = 187;

#[rustfmt::skip]
const DELTA_T: [f64; TERMS] = [
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
];

#[inline]
fn delta_t_ancient(jd_ut: Day) -> Second {
    const DT_A0: f64 = 1_830.0;
    const DT_A1: f64 = -405.0;
    const DT_A2: f64 = 46.5;
    let c = (jd_ut - JD_EPOCH_948_UT) / DAYS_PER_JC;
    Second::new(DT_A0 + ANCIENT_OFFSET + DT_A1 * c + DT_A2 * c * c)
}

#[inline]
fn delta_t_medieval(jd_ut: Day) -> Second {
    const DT_A2: f64 = 22.5;
    let c = (jd_ut - JD_EPOCH_1850_UT) / DAYS_PER_JC;
    Second::new(DT_A2 * c * c + MEDIEVAL_OFFSET)
}

#[inline]
fn delta_t_table(jd_ut: Day) -> Second {
    let mut i = ((jd_ut - JD_TABLE_START_1620) / BIENNIAL_STEP_D) as usize;
    if i > TERMS - 3 {
        i = TERMS - 3;
    }
    // Three-point Lagrange interpolation anchored at DELTA_T[i]:
    //
    //   a = Δy_i     = DELTA_T[i+1] − DELTA_T[i]    (first forward difference)
    //   b = Δy_{i+1} = DELTA_T[i+2] − DELTA_T[i+1]  (first forward difference at i+1)
    //   c = Δ²y_i    = b − a                          (second difference)
    //
    //   P(n) = DELTA_T[i] + n·a + n(n−1)/2 · c
    //
    // n ∈ [0, 1) is the fractional position within the interval starting at
    // knot i. Boundary invariants: P(0) = DELTA_T[i], P(1) = DELTA_T[i+1].
    // When i is clamped to TERMS−3, n may exceed 1, giving a smooth quadratic
    // extension through the last two knots.
    let a = DELTA_T[i + 1] - DELTA_T[i];
    let b = DELTA_T[i + 2] - DELTA_T[i + 1];
    let c = b - a; // second difference (sign intentional: b−a, not a−b)
    let step_start = JD_TABLE_START_1620 + BIENNIAL_STEP_D * i as f64;
    let n = (jd_ut - step_start) / BIENNIAL_STEP_D;
    Second::new(DELTA_T[i] + n * a + n * (n - 1.0) * c / 2.0)
}

#[inline]
fn modern_delta_t_point(index: usize) -> (Day, Second) {
    let (mjd, dt) = MODERN_DELTA_T_POINTS[index];
    (Day::new(mjd), Second::new(dt))
}

#[inline]
fn interpolate_modern_delta_t(mjd: Day) -> Option<Second> {
    if !(MODERN_DELTA_T_START_MJD..=MODERN_DELTA_T_END_MJD).contains(&mjd) {
        return None;
    }
    let mut lo = 0usize;
    let mut hi = MODERN_DELTA_T_POINTS.len() - 1;
    while lo + 1 < hi {
        let mid = lo + (hi - lo) / 2;
        if modern_delta_t_point(mid).0 <= mjd {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let (mjd0, dt0) = modern_delta_t_point(lo);
    let (mjd1, dt1) = modern_delta_t_point(hi);
    // Linear interpolation: dt0 + (mjd − mjd0) / (mjd1 − mjd0) * (dt1 − dt0)
    // Days / Days = f64 (ratio), Second * f64 = Second.
    let frac = (mjd - mjd0) / (mjd1 - mjd0);
    Some(dt0 + (dt1 - dt0) * frac)
}

#[inline]
fn delta_t_modern_series(jd_ut: Day) -> Second {
    // Source: USNO monthly determinations (MODERN_DELTA_T_POINTS).
    // Points up to MODERN_DELTA_T_OBSERVED_END_MJD are confirmed observations;
    // later points are C0-adjusted USNO predictions (see tempoch-time-data-updater).
    let mjd = jd_to_mjd(jd_ut);
    interpolate_modern_delta_t(mjd).expect("modern Delta T interpolation requires in-range MJD")
}

const DELTA_T_EXTRAPOLATION_TAIL_POINTS: usize = 12;
// Coefficients: (a: constant term in seconds, b: s/day, c: s/day², origin: MJD Day).
static TAIL_FIT: OnceLock<(Second, f64, f64, Day)> = OnceLock::new();

fn compute_tail_fit_coefficients() -> (Second, f64, f64, Day) {
    let tail_len = MODERN_DELTA_T_POINTS
        .len()
        .clamp(3, DELTA_T_EXTRAPOLATION_TAIL_POINTS);
    let tail = &MODERN_DELTA_T_POINTS[MODERN_DELTA_T_POINTS.len() - tail_len..];
    let origin = Day::new(tail[tail.len() - 1].0);

    let (mut s0, mut s1, mut s2, mut s3, mut s4) = (0.0_f64, 0.0, 0.0, 0.0, 0.0);
    let (mut t0, mut t1, mut t2) = (0.0_f64, 0.0, 0.0);
    for &(sample_mjd, delta_t) in tail {
        // x: number of days from origin (Day / Day = f64).
        let x = (Day::new(sample_mjd) - origin) / Day::new(1.0);
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
    (
        Second::new(system[0][3]),
        system[1][3],
        system[2][3],
        origin,
    )
}

fn quadratic_tail_fit_delta_t_seconds(mjd: Day) -> Second {
    let &(a, b, c, origin) = TAIL_FIT.get_or_init(compute_tail_fit_coefficients);
    // x: number of days from origin (Day / Day = f64).
    let x = (mjd - origin) / Day::new(1.0);
    a + Second::new(b * x + c * x * x)
}

#[inline]
fn delta_t_extrapolated(jd_ut: Day) -> Second {
    let mjd = jd_to_mjd(jd_ut);
    quadratic_tail_fit_delta_t_seconds(mjd)
}

/// ΔT = TT − UT1, in seconds, for a Julian Day on the UT1 axis.
///
/// Returns `Err(ConversionError::Ut1HorizonExceeded)` for any date beyond
/// [`DELTA_T_PREDICTION_HORIZON_MJD`], consistent with the behaviour of
/// [`Time::to_scale_with::<UT1>`] scale conversions.
///
/// For dates that require an unconstrained extrapolation beyond the horizon,
/// use [`delta_t_seconds_extrapolated`] instead — but note that those values
/// are scientifically unsupported.
///
/// Piecewise dispatch within the supported range:
/// * **Before 948 CE** — Stephenson & Houlden (1986) quadratic with epoch 948.
/// * **948 CE – 1619** — Stephenson & Houlden (1986) quadratic with epoch 1850
///   (`22.5 c²`). Extends to the start of the biennial table rather than
///   terminating at 1461, since the biennial table begins at 1620.
/// * **1620 – modern table end** — biennial interpolation (Meeus ch. 9) then
///   USNO monthly data.
#[inline]
pub fn delta_t_seconds(jd_ut: Day) -> Result<Second, ConversionError> {
    let mjd = jd_to_mjd(jd_ut);
    if mjd > DELTA_T_PREDICTION_HORIZON_MJD {
        return Err(ConversionError::Ut1HorizonExceeded);
    }
    Ok(delta_t_seconds_unconstrained(jd_ut))
}

/// ΔT = TT − UT1, in seconds, with quadratic tail-fit extrapolation beyond
/// the last published prediction point.
///
/// Unlike [`delta_t_seconds`], this function never returns an error: for
/// dates beyond [`DELTA_T_PREDICTION_HORIZON_MJD`] it applies a quadratic
/// continuation fit to the last 12 compiled prediction points.
///
/// # ⚠ Accuracy warning
///
/// The extrapolated values are **not from any official source**. Accuracy
/// degrades rapidly past the compiled horizon and is not bounded. Do **not**
/// use these values where scientific validity is required.
#[inline]
pub fn delta_t_seconds_extrapolated(jd_ut: Day) -> Second {
    delta_t_seconds_unconstrained(jd_ut)
}

/// Unconstrained dispatch — shared by the fallible and extrapolated APIs.
#[inline]
fn delta_t_seconds_unconstrained(jd_ut: Day) -> Second {
    let mjd = jd_to_mjd(jd_ut);
    if jd_ut < JD_EPOCH_948_UT {
        delta_t_ancient(jd_ut)
    } else if jd_ut < JD_TABLE_START_1620 {
        // Medieval model covers 948–1619; JD_TABLE_START_1620 is the first
        // valid biennial-table entry, so backward-extrapolating the table
        // into this range is wrong.
        delta_t_medieval(jd_ut)
    } else if mjd < MODERN_DELTA_T_START_MJD {
        delta_t_table(jd_ut)
    } else if mjd <= MODERN_DELTA_T_END_MJD {
        delta_t_modern_series(jd_ut)
    } else {
        delta_t_extrapolated(jd_ut)
    }
}

/// MJD of the last compiled ΔT prediction point.
pub const DELTA_T_PREDICTION_HORIZON_MJD: Day = MODERN_DELTA_T_END_MJD;

#[cfg(test)]
mod tests {
    use super::*;

    /// Convert a JD float to a `Day` for passing to `delta_t_seconds`.
    fn jd(jd: f64) -> Day {
        Day::new(jd)
    }

    /// `delta_t_table` anchor at 1620 CE: P(0) must equal DELTA_T[0] = 124.0 s.
    #[test]
    fn delta_t_table_knot_0_is_124() {
        let dt = delta_t_seconds_extrapolated(JD_TABLE_START_1620);
        assert!(
            (dt.value() - 124.0).abs() < 1e-9,
            "ΔT at 1620 CE (knot 0) expected 124.0 s, got {:.6} s",
            dt.value()
        );
    }

    /// `delta_t_table` anchor at 1622 CE: P(0) at the second knot must equal DELTA_T[1] = 115.0 s.
    #[test]
    fn delta_t_table_knot_1_is_115() {
        let dt = delta_t_seconds_extrapolated(JD_TABLE_START_1620 + BIENNIAL_STEP_D);
        assert!(
            (dt.value() - 115.0).abs() < 1e-9,
            "ΔT at 1622 CE (knot 1) expected 115.0 s, got {:.6} s",
            dt.value()
        );
    }

    /// `delta_t_table` anchor at 1624 CE: P(0) at the third knot must equal DELTA_T[2] = 106.0 s.
    #[test]
    fn delta_t_table_knot_2_is_106() {
        let dt = delta_t_seconds_extrapolated(JD_TABLE_START_1620 + BIENNIAL_STEP_D * 2.0);
        assert!(
            (dt.value() - 106.0).abs() < 1e-9,
            "ΔT at 1624 CE (knot 2) expected 106.0 s, got {:.6} s",
            dt.value()
        );
    }

    /// Midpoint of the first biennial interval (n = 0.5): pure quadratic interpolation
    /// between DELTA_T[0]=124, DELTA_T[1]=115, DELTA_T[2]=106 gives 119.5 s.
    ///
    /// Calculation: a=-9, b=-9, c=0 → P(0.5) = 124 + 0.5·(−9) = 119.5.
    #[test]
    fn delta_t_table_midpoint_interval_0_is_119_5() {
        let dt = delta_t_seconds_extrapolated(JD_TABLE_START_1620 + BIENNIAL_STEP_D * 0.5);
        assert!(
            (dt.value() - 119.5).abs() < 1e-9,
            "ΔT at midpoint of [1620,1622] expected 119.5 s, got {:.6} s",
            dt.value()
        );
    }

    /// Continuity at every biennial knot: the value approaching from the left
    /// (n → 1⁻) must equal the value at n = 0 from the right.
    #[test]
    fn delta_t_table_boundary_continuity() {
        // Test the first 10 internal knot boundaries.
        for k in 1..10usize {
            let jd_knot = JD_TABLE_START_1620 + BIENNIAL_STEP_D * k as f64;
            let left = delta_t_table(jd_knot - Day::new(1e-6)).value();
            let right = delta_t_table(jd_knot + Day::new(1e-6)).value();
            let exact = delta_t_table(jd_knot).value();
            assert!(
                (left - exact).abs() < 1e-3,
                "Discontinuity at knot {k}: left={left:.6}, exact={exact:.6}"
            );
            assert!(
                (right - exact).abs() < 1e-3,
                "Discontinuity at knot {k}: right={right:.6}, exact={exact:.6}"
            );
        }
    }

    /// Monotone early segment: ΔT should decrease from 1620 to 1900 as the
    /// table values go from 124 s down to ≈ 2.6 s.
    #[test]
    fn delta_t_table_decreases_1620_to_1900() {
        let dt_1620 = delta_t_seconds_extrapolated(JD_TABLE_START_1620).value();
        let jd_1900 = 2_415_020.5; // 1900 Jan 0.5 (standard epoch)
        let dt_1900 = delta_t_seconds_extrapolated(jd(jd_1900)).value();
        assert!(
            dt_1620 > dt_1900,
            "ΔT should decrease 1620→1900, got {dt_1620:.2} > {dt_1900:.2}"
        );
    }

    /// C0 continuity at the 1620 CE regime boundary: medieval→table.
    ///
    /// Immediately before 1620 the medieval formula (with offset) returns the
    /// same value as `DELTA_T[0] = 124.0 s` immediately after. The allowed
    /// tolerance is 1 ms (float arithmetic near the boundary).
    #[test]
    fn regime_boundary_1620_is_continuous() {
        let eps = Day::new(1e-4); // ~8.6 seconds before/after
        let before = delta_t_seconds_extrapolated(JD_TABLE_START_1620 - eps).value();
        let after = delta_t_seconds_extrapolated(JD_TABLE_START_1620 + eps).value();
        assert!(
            (before - after).abs() < 1e-3,
            "ΔT regime gap at 1620 CE: {before:.6} → {after:.6} s (gap {:.6} s)",
            (before - after).abs()
        );
    }

    /// C0 continuity at the 948 CE regime boundary: ancient→medieval.
    ///
    /// Both sub-models with their continuity offsets must agree to within 1 ms
    /// when evaluated just before and just after 948 CE.
    #[test]
    fn regime_boundary_948_is_continuous() {
        let eps = Day::new(1e-4);
        let before = delta_t_seconds_extrapolated(JD_EPOCH_948_UT - eps).value();
        let after = delta_t_seconds_extrapolated(JD_EPOCH_948_UT + eps).value();
        assert!(
            (before - after).abs() < 1e-3,
            "ΔT regime gap at 948 CE: {before:.6} → {after:.6} s (gap {:.6} s)",
            (before - after).abs()
        );
    }

    /// `delta_t_seconds` returns `Err(Ut1HorizonExceeded)` for dates beyond the
    /// compiled prediction horizon, and `Ok` for dates within it.
    #[test]
    fn delta_t_seconds_horizon_guard() {
        use crate::error::ConversionError;
        // JD 2_465_000 ≈ 2026 is well beyond any reasonable compiled horizon.
        let past = delta_t_seconds(Day::new(2_465_000.0));
        assert!(
            matches!(past, Err(ConversionError::Ut1HorizonExceeded)),
            "expected Ut1HorizonExceeded past horizon, got {past:?}"
        );
        // J2000 (2000-01-01) must be well within the supported range.
        let present = delta_t_seconds(Day::new(2_451_545.0));
        assert!(
            present.is_ok(),
            "expected Ok within horizon, got {present:?}"
        );
    }

    /// C0 continuity at the 1973 biennial→modern stitch.
    ///
    /// The biennial table ends near 1973 (MODERN_DELTA_T_START_MJD); the modern
    /// USNO series begins there.  The two must agree to within 0.01 s at that
    /// boundary.
    #[test]
    fn regime_boundary_1973_biennial_to_modern_is_continuous() {
        use crate::constats::JD_MINUS_MJD;
        use crate::generated::MODERN_DELTA_T_START_MJD;
        // Convert MJD start to JD.
        let jd_start = MODERN_DELTA_T_START_MJD + JD_MINUS_MJD;
        let eps = Day::new(1e-4); // ~8.6 s
        let before = delta_t_seconds_extrapolated(jd_start - eps).value();
        let after = delta_t_seconds_extrapolated(jd_start + eps).value();
        assert!(
            (before - after).abs() < 0.01,
            "ΔT gap at 1973 biennial→modern stitch: {before:.6} → {after:.6} s (gap {:.6} s)",
            (before - after).abs()
        );
    }

    /// `MODERN_DELTA_T_OBSERVED_END_MJD` must lie strictly inside the compiled
    /// point array (so delta_t lookup works at that boundary).
    #[test]
    fn modern_delta_t_observed_end_is_in_range() {
        use crate::generated::MODERN_DELTA_T_OBSERVED_END_MJD;
        assert!(
            MODERN_DELTA_T_OBSERVED_END_MJD > MODERN_DELTA_T_START_MJD,
            "observed end must be after series start"
        );
        assert!(
            MODERN_DELTA_T_OBSERVED_END_MJD < MODERN_DELTA_T_END_MJD,
            "observed end must be before prediction horizon"
        );
    }
}
