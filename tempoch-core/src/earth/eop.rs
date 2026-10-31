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
//! [`crate::archive::time::TimeDataManager`].  Until a bundle is loaded,
//! [`builtin_eop_at`] always returns `None`.

use crate::data::runtime_data::{active_time_data, time_data_eop_at};
use crate::qtty::{Day, Second};

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
    pub pm_xp: Option<crate::qtty::f64::Arcsecond>,
    pub pm_yp: Option<crate::qtty::f64::Arcsecond>,
    pub ut1_minus_utc: Second,
    pub lod: Option<crate::qtty::f64::Millisecond>,
    pub dx: Option<crate::qtty::f64::MilliArcsecond>,
    pub dy: Option<crate::qtty::f64::MilliArcsecond>,
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
/// EOP data has been loaded. First and last row MJDs are inclusive sample
/// epochs, not entire-day validity intervals. Within range lookup succeeds;
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
    use crate::archive::time::{EopPoint, TimeDataBundle, TimeDataProvenance, UtcTaiSegment};
    use crate::data::runtime_data::with_test_time_data;
    use crate::qtty::{Arcsecond, Day, MilliArcsecond, Millisecond, Second};

    fn make_test_eop_bundle(points: Vec<EopPoint>) -> TimeDataBundle {
        TimeDataBundle::new(
            vec![UtcTaiSegment {
                start_mjd: 41317,
                end_mjd: None,
                base: Second::new(37.0),
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
                pm_xp: Some(Arcsecond::new(0.1)),
                pm_yp: Some(Arcsecond::new(0.2)),
                ut1_minus_utc: Second::new(0.3),
                lod: Some(Millisecond::new(1.0)),
                dx: Some(MilliArcsecond::new(0.01)),
                dy: Some(MilliArcsecond::new(0.02)),
            },
            EopPoint {
                mjd: 50001,
                pm_observed: true,
                ut1_observed: true,
                nutation_observed: true,
                pm_xp: Some(Arcsecond::new(0.2)),
                pm_yp: Some(Arcsecond::new(0.4)),
                ut1_minus_utc: Second::new(0.5),
                lod: Some(Millisecond::new(2.0)),
                dx: None,
                dy: None,
            },
            EopPoint {
                mjd: 50002,
                pm_observed: false,
                ut1_observed: false,
                nutation_observed: false,
                pm_xp: Some(Arcsecond::new(0.3)),
                pm_yp: Some(Arcsecond::new(0.6)),
                ut1_minus_utc: Second::new(0.7),
                lod: None,
                dx: None,
                dy: None,
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
            assert_eq!(got.pm_xp.map(|v| v.value()), mid.pm_xp.map(|v| v.value()));
            assert_eq!(got.pm_yp.map(|v| v.value()), mid.pm_yp.map(|v| v.value()));
            assert!(
                (got.ut1_minus_utc.value() - mid.ut1_minus_utc.value()).abs() < 1e-12,
                "ut1: {} vs {}",
                got.ut1_minus_utc.value(),
                mid.ut1_minus_utc.value()
            );
            assert_eq!(got.dx.map(|v| v.value()), mid.dx.map(|v| v.value()));
            assert_eq!(got.dy.map(|v| v.value()), mid.dy.map(|v| v.value()));
        });
    }

    #[test]
    fn midpoint_is_halfway() {
        let points = three_point_fixture();
        let bundle = make_test_eop_bundle(points.clone());
        with_test_time_data(bundle, || {
            let got = builtin_eop_at(Day::new(50000.5)).unwrap();
            let expected =
                0.5 * (points[0].ut1_minus_utc.value() + points[1].ut1_minus_utc.value());
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
    fn exact_endpoints_and_fractional_bounds() {
        let points = three_point_fixture();
        with_test_time_data(make_test_eop_bundle(points.clone()), || {
            assert_eq!(eop_start(), Some(Day::new(50_000.0)));
            assert_eq!(eop_end(), Some(Day::new(50_002.0)));
            for point in [points[0], points[2]] {
                let mjd = Day::new(point.mjd as f64);
                let got = builtin_eop_at(mjd).expect("EOP sample epoch must be valid");
                assert!(builtin_eop_covers(mjd));
                assert_eq!(got.mjd_utc, mjd);
                assert_eq!(got.pm_xp, point.pm_xp);
                assert_eq!(got.pm_yp, point.pm_yp);
                assert_eq!(got.lod, point.lod);
                assert_eq!(got.dx, point.dx);
                assert_eq!(got.dy, point.dy);
                assert_eq!(got.ut1_observed, point.ut1_observed);
                assert!((got.ut1_minus_utc.value() - point.ut1_minus_utc.value()).abs() < 1e-12);
            }
            let just_before_first = f64::from_bits(50_000.0_f64.to_bits() - 1);
            let just_after_last = f64::from_bits(50_002.0_f64.to_bits() + 1);
            for mjd in [
                just_before_first,
                49_999.9999,
                just_after_last,
                50_002.0001,
                50_002.5,
            ] {
                assert!(
                    builtin_eop_at(Day::new(mjd)).is_none(),
                    "unexpected EOP extrapolation at MJD {mjd}"
                );
                assert!(!builtin_eop_covers(Day::new(mjd)));
            }
        });
    }

    #[test]
    fn interior_fractional_interpolation_and_optional_fields() {
        with_test_time_data(make_test_eop_bundle(three_point_fixture()), || {
            let quarter = builtin_eop_at(Day::new(50_000.25)).unwrap();
            assert!((quarter.pm_xp.unwrap().value() - 0.125).abs() < 1e-12);
            assert!((quarter.pm_yp.unwrap().value() - 0.25).abs() < 1e-12);
            assert!((quarter.ut1_minus_utc.value() - 0.35).abs() < 1e-12);
            assert!((quarter.lod.unwrap().value() - 1.25).abs() < 1e-12);
            assert_eq!(quarter.dx, None);
            assert_eq!(quarter.dy, None);
            assert!(quarter.ut1_observed);
            let later = builtin_eop_at(Day::new(50_001.75)).unwrap();
            assert!((later.pm_xp.unwrap().value() - 0.275).abs() < 1e-12);
            assert!((later.pm_yp.unwrap().value() - 0.55).abs() < 1e-12);
            assert!((later.ut1_minus_utc.value() - 0.65).abs() < 1e-12);
            assert_eq!(later.lod, None);
            assert!(!later.ut1_observed);
        });
    }

    #[test]
    fn context_coverage_agrees_and_nonfinite_epochs_are_rejected() {
        with_test_time_data(make_test_eop_bundle(three_point_fixture()), || {
            let ctx = crate::TimeContext::with_builtin_eop();
            let without_eop = crate::TimeContext::new();
            for mjd in [
                49_999.9999,
                50_000.0,
                50_000.5,
                50_001.75,
                50_002.0,
                50_002.0001,
                f64::NAN,
                f64::INFINITY,
                f64::NEG_INFINITY,
            ] {
                let epoch = Day::new(mjd);
                let direct = builtin_eop_at(epoch);
                assert_eq!(builtin_eop_covers(epoch), direct.is_some());
                assert_eq!(ctx.eop_at(epoch), direct);
                assert!(without_eop.eop_at(epoch).is_none());
            }
        });
    }

    #[test]
    fn ut1_interpolation_preserves_leap_second_discontinuity() {
        // Synthetic 2016-12-31 leap: DUT1 jumps by one second at MJD 57754,
        // but the continuous UT1-TAI interpolation does not.
        let mut points = three_point_fixture();
        for (point, (mjd, dut1)) in points
            .iter_mut()
            .zip([(57_753, -0.4), (57_754, 0.6), (57_755, 0.7)])
        {
            point.mjd = mjd;
            point.ut1_minus_utc = Second::new(dut1);
        }
        let bundle = TimeDataBundle::new(
            vec![
                UtcTaiSegment {
                    start_mjd: 57_753,
                    end_mjd: Some(57_754),
                    base: Second::new(36.0),
                    reference_mjd: 57_753.0,
                    slope_seconds_per_day: 0.0,
                },
                UtcTaiSegment {
                    start_mjd: 57_754,
                    end_mjd: None,
                    base: Second::new(37.0),
                    reference_mjd: 57_754.0,
                    slope_seconds_per_day: 0.0,
                },
            ],
            vec![(41_714.0, 42.184), (42_369.0, 45.0)],
            41_714.0,
            points,
            TimeDataProvenance::new("test", "x", "x", "x", "x"),
        );
        with_test_time_data(bundle, || {
            let before = builtin_eop_at(Day::new(57_753.5)).unwrap();
            let boundary = builtin_eop_at(Day::new(57_754.0)).unwrap();
            let after = builtin_eop_at(Day::new(57_754.5)).unwrap();
            assert!((before.ut1_minus_utc.value() + 0.4).abs() < 1e-12);
            assert!((boundary.ut1_minus_utc.value() - 0.6).abs() < 1e-12);
            assert!((after.ut1_minus_utc.value() - 0.65).abs() < 1e-12);
        });
    }

    #[test]
    fn no_eop_data_returns_none() {
        let bundle = make_test_eop_bundle(Vec::new());
        with_test_time_data(bundle, || {
            assert!(builtin_eop_at(Day::new(50000.0)).is_none());
            assert!(!builtin_eop_covers(Day::new(50000.0)));
            let context = crate::TimeContext::with_builtin_eop();
            assert!(context.eop_at(Day::new(50000.0)).is_none());
            assert!(context.eop_at(Day::new(f64::NAN)).is_none());
        });
    }
}
