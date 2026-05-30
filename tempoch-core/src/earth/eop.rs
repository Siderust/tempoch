// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Daily IERS Earth Orientation Parameters.
//!
//! The series combines observed Bulletin C04 values (flag `I` in the
//! upstream file) with short-range Bulletin A predictions (flag `P`). The
//! boundary between the two sub-ranges is exposed by [`eop_observed_end`].
//!
//! EOP data is **not** compiled into the crate.  It must be loaded at runtime
//! via [`crate::data::runtime_data::update_runtime_time_data`] or
//! [`siderust_archive::time::TimeDataManager`].  Until a bundle is loaded,
//! [`builtin_eop_at`] always returns `None`.

use crate::data::runtime_data::{active_time_data, time_data_eop_at};
use qtty::{Day, Second};

/// First MJD present in the currently active EOP series, or `None` when no
/// EOP data has been loaded.
pub fn eop_start() -> Option<Day> {
    active_time_data()
        .eop_start_mjd()
        .map(|v| Day::new(v as f64))
}

/// Last observed (non-predicted) MJD in the currently active EOP series, or
/// `None` when no EOP data has been loaded.
pub fn eop_observed_end() -> Option<Day> {
    active_time_data()
        .eop_observed_end_mjd()
        .map(|v| Day::new(v as f64))
}

/// Last MJD (including predictions) in the currently active EOP series, or
/// `None` when no EOP data has been loaded.
pub fn eop_end() -> Option<Day> {
    active_time_data().eop_end_mjd().map(|v| Day::new(v as f64))
}

/// Interpolated IERS Earth Orientation Parameters at a UTC MJD.
///
/// All fields carry SI-coherent qtty typed quantities:
///
/// - `pm_xp`, `pm_yp` — polar motion in arcseconds.
/// - `ut1_minus_utc` — DUT1 in seconds of time.
/// - `lod` — length-of-day excess in milliseconds of time.
/// - `dx`, `dy` — IAU 2000A celestial pole offsets in milliarcseconds.
///
/// Optional fields stay `None` when either bracketing upstream row leaves the
/// source column blank; the API does not fabricate zero-valued PM or nutation
/// quantities.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EopValues {
    pub mjd_utc: Day,
    pub pm_xp: Option<qtty::f64::Arcsecond>,
    pub pm_yp: Option<qtty::f64::Arcsecond>,
    pub ut1_minus_utc: Second,
    pub lod: Option<qtty::f64::Millisecond>,
    pub dx: Option<qtty::f64::MilliArcsecond>,
    pub dy: Option<qtty::f64::MilliArcsecond>,
    /// `true` when both bracketing rows are flagged observed (`I`).
    pub ut1_observed: bool,
}

/// Returns `true` when [`builtin_eop_at`] would return `Some` for `mjd_utc`.
#[inline]
pub fn builtin_eop_covers(mjd_utc: Day) -> bool {
    let data = active_time_data();
    time_data_eop_at(data.as_ref(), mjd_utc).is_some()
}

/// Linearly interpolate EOP at a UTC MJD from the active bundle.
///
/// Returns `None` when `mjd_utc` is outside the loaded EOP range, or when no
/// EOP data has been loaded.  Within range the function always succeeds;
/// optional quantities remain `None` whenever either bracketing row leaves the
/// source field blank.
///
/// EOP data is not compiled into the crate.  Call
/// [`crate::data::runtime_data::update_runtime_time_data`] to load a cached
/// bundle before querying EOP values.
pub fn builtin_eop_at(mjd_utc: Day) -> Option<EopValues> {
    let data = active_time_data();
    time_data_eop_at(data.as_ref(), mjd_utc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::runtime_data::with_test_time_data;
    use siderust_archive::time::{EopPoint, TimeDataBundle, TimeDataProvenance, UtcTaiSegment};

    fn make_test_eop_bundle(points: Vec<EopPoint>) -> TimeDataBundle {
        TimeDataBundle::new(
            vec![UtcTaiSegment {
                start_mjd: 41317,
                end_mjd: None,
                base_seconds: 37.0,
                reference_mjd: 41317.0,
                slope_seconds_per_day: 0.0,
            }],
            vec![(41714.0, 42.184), (42369.0, 45.0)],
            41714.0,
            points,
            TimeDataProvenance::new("test", "x", "x", "x", "x"),
        )
    }

    fn three_point_fixture() -> Vec<EopPoint> {
        vec![
            EopPoint {
                mjd: 50000,
                pm_observed: true,
                ut1_observed: true,
                nutation_observed: true,
                pm_xp_arcsec: Some(0.1),
                pm_yp_arcsec: Some(0.2),
                ut1_minus_utc_seconds: 0.3,
                lod_milliseconds: Some(1.0),
                dx_milliarcsec: Some(0.01),
                dy_milliarcsec: Some(0.02),
            },
            EopPoint {
                mjd: 50001,
                pm_observed: true,
                ut1_observed: true,
                nutation_observed: true,
                pm_xp_arcsec: Some(0.2),
                pm_yp_arcsec: Some(0.4),
                ut1_minus_utc_seconds: 0.5,
                lod_milliseconds: Some(2.0),
                dx_milliarcsec: None,
                dy_milliarcsec: None,
            },
            EopPoint {
                mjd: 50002,
                pm_observed: false,
                ut1_observed: false,
                nutation_observed: false,
                pm_xp_arcsec: Some(0.3),
                pm_yp_arcsec: Some(0.6),
                ut1_minus_utc_seconds: 0.7,
                lod_milliseconds: None,
                dx_milliarcsec: None,
                dy_milliarcsec: None,
            },
        ]
    }

    #[test]
    fn covers_start_and_end() {
        let bundle = make_test_eop_bundle(three_point_fixture());
        with_test_time_data(bundle, || {
            assert!(builtin_eop_covers(Day::new(50000.0)));
            assert!(builtin_eop_covers(Day::new(50002.0)));
            assert!(!builtin_eop_covers(Day::new(49999.0)));
            assert!(!builtin_eop_covers(Day::new(50003.0)));
        });
    }

    #[test]
    fn exact_point_matches_source() {
        let points = three_point_fixture();
        let bundle = make_test_eop_bundle(points.clone());
        with_test_time_data(bundle, || {
            let mid = &points[1];
            let got = builtin_eop_at(Day::new(mid.mjd as f64)).unwrap();
            assert_eq!(got.pm_xp.map(|v| v.value()), mid.pm_xp_arcsec);
            assert_eq!(got.pm_yp.map(|v| v.value()), mid.pm_yp_arcsec);
            assert!(
                (got.ut1_minus_utc.value() - mid.ut1_minus_utc_seconds).abs() < 1e-12,
                "ut1: {} vs {}",
                got.ut1_minus_utc.value(),
                mid.ut1_minus_utc_seconds
            );
            assert_eq!(got.dx.map(|v| v.value()), mid.dx_milliarcsec);
            assert_eq!(got.dy.map(|v| v.value()), mid.dy_milliarcsec);
        });
    }

    #[test]
    fn midpoint_is_halfway() {
        let points = three_point_fixture();
        let bundle = make_test_eop_bundle(points.clone());
        with_test_time_data(bundle, || {
            let got = builtin_eop_at(Day::new(50000.5)).unwrap();
            let expected =
                0.5 * (points[0].ut1_minus_utc_seconds + points[1].ut1_minus_utc_seconds);
            assert!((got.ut1_minus_utc.value() - expected).abs() < 1e-12);
        });
    }

    #[test]
    fn missing_optional_fields_remain_missing() {
        let bundle = make_test_eop_bundle(three_point_fixture());
        with_test_time_data(bundle, || {
            // points[1] and points[2] both have dx=None, dy=None; interpolating between them keeps None
            let got = builtin_eop_at(Day::new(50001.5)).unwrap();
            assert_eq!(got.dx, None);
            assert_eq!(got.dy, None);
        });
    }

    #[test]
    fn out_of_range_returns_none() {
        let bundle = make_test_eop_bundle(three_point_fixture());
        with_test_time_data(bundle, || {
            assert!(builtin_eop_at(Day::new(49990.0)).is_none());
            assert!(builtin_eop_at(Day::new(50010.0)).is_none());
        });
    }

    #[test]
    fn no_eop_data_returns_none() {
        let bundle = make_test_eop_bundle(Vec::new());
        with_test_time_data(bundle, || {
            assert!(builtin_eop_at(Day::new(50000.0)).is_none());
            assert!(!builtin_eop_covers(Day::new(50000.0)));
        });
    }
}
