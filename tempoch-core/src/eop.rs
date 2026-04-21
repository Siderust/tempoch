// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Daily IERS Earth Orientation Parameters from the compiled
//! `finals2000A.all` series.
//!
//! The series combines observed Bulletin C04 values (flag `I` in the
//! upstream file) with short-range Bulletin A predictions (flag `P`). The
//! boundary between the two sub-ranges is [`EOP_OBSERVED_END_MJD`].
//!
//! The baseline series is loaded at compile time from
//! [`crate::generated::eop_data`]. After
//! [`crate::update_runtime_time_data`] or [`crate::refresh_runtime_time_data`],
//! these helpers transparently use the active runtime bundle.

use crate::data::active::{active_time_data, time_data_eop_at};
use qtty::{Day, Second};

#[cfg(test)]
use crate::generated::eop_data::{EOP_END_MJD, EOP_POINTS, EOP_START_MJD};

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
    let data = active_time_data();
    (
        Day::new(data.eop_points()[0].mjd as f64),
        Day::new(data.eop_points()[data.eop_points().len() - 1].mjd as f64),
    )
}

/// Returns `true` when [`builtin_eop_at`] would return `Some` for `mjd_utc`.
#[inline]
pub fn builtin_eop_covers(mjd_utc: Day) -> bool {
    let (lo, hi) = covered_range();
    (lo..=hi).contains(&mjd_utc)
}

/// Linearly interpolate compiled EOP at a UTC MJD.
///
/// Returns `None` when `mjd_utc` is outside the compiled `[EOP_START_MJD,
/// EOP_END_MJD]` range. Within range the function always succeeds; optional
/// quantities remain `None` whenever either bracketing row leaves the source
/// field blank.
pub fn builtin_eop_at(mjd_utc: Day) -> Option<EopValues> {
    let data = active_time_data();
    time_data_eop_at(data.as_ref(), mjd_utc)
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
