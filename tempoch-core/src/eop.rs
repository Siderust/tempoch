// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Daily IERS Earth Orientation Parameters from the compiled
//! `finals2000A.all` series.
//!
//! The series combines observed Bulletin C04 values (flag `I` in the
//! upstream file) with short-range Bulletin A predictions (flag `P`). The
//! boundary between the two sub-ranges is [`EOP_OBSERVED_END_MJD`].
//!
//! The compiled series is loaded at compile time from
//! [`crate::generated::eop_data`]; no runtime fetching occurs.

use crate::generated::eop_data::{EopPoint as RawEopPoint, EOP_END_MJD, EOP_POINTS, EOP_START_MJD};
use qtty::{Day, Second};

/// Interpolated IERS Earth Orientation Parameters at a UTC MJD.
///
/// Fields carry the units used by the upstream IERS `finals2000A.all` file:
///
/// - `pm_xp`, `pm_yp` are *arcseconds* of polar motion.
/// - `ut1_minus_utc` is *seconds of time* (DUT1).
/// - `lod` is *milliseconds of time* excess over 86 400 SI seconds. It is
///   `None` whenever the bracketing rows do not both supply a LOD value.
/// - `dx`, `dy` are IAU 2000A celestial pole offsets in *milliarcseconds*.
///
/// Optional fields stay `None` when either bracketing upstream row leaves the
/// source column blank; the API does not fabricate zero-valued PM or nutation
/// quantities.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EopValues {
    pub mjd_utc: Day,
    pub pm_xp_arcsec: Option<f64>,
    pub pm_yp_arcsec: Option<f64>,
    pub ut1_minus_utc: Second,
    pub lod_milliseconds: Option<f64>,
    pub dx_milliarcsec: Option<f64>,
    pub dy_milliarcsec: Option<f64>,
    /// `true` when both bracketing rows are flagged observed (`I`).
    pub ut1_observed: bool,
}

fn covered_range() -> (Day, Day) {
    (Day::new(EOP_START_MJD as f64), Day::new(EOP_END_MJD as f64))
}

/// Returns `true` when [`builtin_eop_at`] would return `Some` for `mjd_utc`.
#[inline]
pub fn builtin_eop_covers(mjd_utc: Day) -> bool {
    let (lo, hi) = covered_range();
    (lo..=hi).contains(&mjd_utc)
}

#[inline]
fn lookup_index(mjd_i32: i32) -> Option<usize> {
    // EOP_POINTS is strictly monotone in MJD with step = 1, so we can index
    // directly once we know the series is non-empty.
    let first = EOP_POINTS[0].mjd;
    let last = EOP_POINTS[EOP_POINTS.len() - 1].mjd;
    if mjd_i32 < first || mjd_i32 > last {
        return None;
    }
    Some((mjd_i32 - first) as usize)
}

#[inline]
fn bracket(mjd_utc: Day) -> Option<(RawEopPoint, RawEopPoint, f64)> {
    let mjd_f = mjd_utc.value();
    let lo_i = mjd_f.floor() as i32;
    let hi_i = lo_i + 1;
    let lo_idx = lookup_index(lo_i)?;
    // If mjd lands exactly on the last point, clamp hi to the same row so
    // interpolation yields the point itself with zero slope contribution.
    let hi_idx = lookup_index(hi_i).unwrap_or(lo_idx);
    let frac = if hi_idx == lo_idx {
        0.0
    } else {
        mjd_f - lo_i as f64
    };
    Some((EOP_POINTS[lo_idx], EOP_POINTS[hi_idx], frac))
}

/// Linearly interpolate compiled EOP at a UTC MJD.
///
/// Returns `None` when `mjd_utc` is outside the compiled `[EOP_START_MJD,
/// EOP_END_MJD]` range.  Within range the function always succeeds, falling
/// back to the nearest-bracketing row's value for LOD when either
/// neighbour is missing.
pub fn builtin_eop_at(mjd_utc: Day) -> Option<EopValues> {
    let (lo, hi, frac) = bracket(mjd_utc)?;

    let lerp = |a: f64, b: f64| a + frac * (b - a);
    let lerp_opt = |a: Option<f64>, b: Option<f64>| match (a, b) {
        (Some(a), Some(b)) => Some(lerp(a, b)),
        _ => None,
    };
    let lod_milliseconds = match (lo.lod_milliseconds, hi.lod_milliseconds) {
        (Some(a), Some(b)) => Some(lerp(a, b)),
        _ => None,
    };

    Some(EopValues {
        mjd_utc,
        pm_xp_arcsec: lerp_opt(lo.pm_xp_arcsec, hi.pm_xp_arcsec),
        pm_yp_arcsec: lerp_opt(lo.pm_yp_arcsec, hi.pm_yp_arcsec),
        ut1_minus_utc: Second::new(lerp(lo.ut1_minus_utc_seconds, hi.ut1_minus_utc_seconds)),
        lod_milliseconds,
        dx_milliarcsec: lerp_opt(lo.dx_milliarcsec, hi.dx_milliarcsec),
        dy_milliarcsec: lerp_opt(lo.dy_milliarcsec, hi.dy_milliarcsec),
        ut1_observed: lo.ut1_observed && hi.ut1_observed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn covers_start_and_end() {
        assert!(builtin_eop_covers(Day::new(EOP_START_MJD as f64)));
        assert!(builtin_eop_covers(Day::new(EOP_END_MJD as f64)));
        assert!(!builtin_eop_covers(Day::new(EOP_START_MJD as f64 - 1.0)));
        assert!(!builtin_eop_covers(Day::new(EOP_END_MJD as f64 + 1.0)));
    }

    #[test]
    fn exact_point_matches_source() {
        let mid = EOP_POINTS[EOP_POINTS.len() / 2];
        let got = builtin_eop_at(Day::new(mid.mjd as f64)).unwrap();
        assert_eq!(got.pm_xp_arcsec, mid.pm_xp_arcsec);
        assert_eq!(got.pm_yp_arcsec, mid.pm_yp_arcsec);
        assert!(
            (got.ut1_minus_utc.value() - mid.ut1_minus_utc_seconds).abs() < 1e-12,
            "ut1: {} vs {}",
            got.ut1_minus_utc.value(),
            mid.ut1_minus_utc_seconds
        );
        assert_eq!(got.dx_milliarcsec, mid.dx_milliarcsec);
        assert_eq!(got.dy_milliarcsec, mid.dy_milliarcsec);
    }

    #[test]
    fn midpoint_is_halfway() {
        let lo = EOP_POINTS[100];
        let hi = EOP_POINTS[101];
        let got = builtin_eop_at(Day::new(lo.mjd as f64 + 0.5)).unwrap();
        let expected = 0.5 * (lo.ut1_minus_utc_seconds + hi.ut1_minus_utc_seconds);
        assert!((got.ut1_minus_utc.value() - expected).abs() < 1e-12);
    }

    #[test]
    fn missing_optional_fields_remain_missing() {
        let idx = EOP_POINTS
            .windows(2)
            .position(|window| {
                window[0].dx_milliarcsec.is_none() && window[1].dx_milliarcsec.is_none()
            })
            .expect("generated EOP tail should include rows with blank nutation fields");
        let lo = EOP_POINTS[idx];
        let got = builtin_eop_at(Day::new(lo.mjd as f64 + 0.5)).unwrap();
        assert_eq!(got.dx_milliarcsec, None);
        assert_eq!(got.dy_milliarcsec, None);
    }

    #[test]
    fn out_of_range_returns_none() {
        assert!(builtin_eop_at(Day::new(EOP_START_MJD as f64 - 10.0)).is_none());
        assert!(builtin_eop_at(Day::new(EOP_END_MJD as f64 + 10.0)).is_none());
    }
}
