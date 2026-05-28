// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Smoke tests for the new W3 scale markers (ET, GPST, GST, BDT, QZSST).
//!
//! These tests verify the fixed-offset relationships between GNSS system
//! times and TAI, and the ET-TDB identity, without depending on EOP/ΔT data.

use qtty::Second;
use tempoch::{ExactDuration, Time, BDT, ET, GPST, GST, QZSST, TAI, TDB, TT};

#[test]
fn gpst_offset_from_tai_is_19s() {
    let tai = Time::<TAI>::from_raw_j2000_seconds(Second::new(1_000_000.0)).unwrap();
    let gpst = tai.to::<GPST>();
    let d: ExactDuration = tai.diff_exact(gpst.to::<TAI>()).unwrap();
    assert_eq!(d.as_nanos_i128(), 0); // round trip exact at integer-s offset

    // Direct check: GPST = TAI - 19 s
    let tai_secs = tai.raw_seconds_pair();
    let gpst_secs = gpst.raw_seconds_pair();
    let total_tai = tai_secs.0 + tai_secs.1;
    let total_gpst = gpst_secs.0 + gpst_secs.1;
    assert!(((total_tai - total_gpst).value() - 19.0).abs() < 1e-9);
}

#[test]
fn gst_and_qzsst_match_gpst_nominally() {
    let tai = Time::<TAI>::from_raw_j2000_seconds(Second::new(0.0)).unwrap();
    let gpst = tai.to::<GPST>().to_j2000s();
    let gst = tai.to::<GST>().to_j2000s();
    let qzs = tai.to::<QZSST>().to_j2000s();
    assert_eq!(gpst.raw_seconds_pair(), gst.raw_seconds_pair());
    assert_eq!(gpst.raw_seconds_pair(), qzs.raw_seconds_pair());
}

#[test]
fn bdt_offset_from_gpst_is_14s() {
    let gpst = Time::<GPST>::from_raw_j2000_seconds(Second::new(0.0)).unwrap();
    let bdt = gpst.to::<BDT>();
    let g_secs = gpst.raw_seconds_pair();
    let b_secs = bdt.raw_seconds_pair();
    let total_g = (g_secs.0 + g_secs.1).value();
    let total_b = (b_secs.0 + b_secs.1).value();
    assert!(
        (total_g - total_b - 14.0).abs() < 1e-9,
        "BDT must lag GPST by 14 s"
    );
}

#[test]
fn bdt_offset_from_tai_is_33s() {
    let tai = Time::<TAI>::from_raw_j2000_seconds(Second::new(0.0)).unwrap();
    let bdt = tai.to::<BDT>();
    let total_t = (tai.raw_seconds_pair().0 + tai.raw_seconds_pair().1).value();
    let total_b = (bdt.raw_seconds_pair().0 + bdt.raw_seconds_pair().1).value();
    assert!((total_t - total_b - 33.0).abs() < 1e-9);
}

#[test]
fn gnss_round_trips_through_all_targets() {
    let original = Time::<TAI>::from_raw_j2000_seconds(Second::new(12_345_678.9)).unwrap();
    for label in ["GPST", "GST", "QZSST", "BDT"] {
        let round_trip = match label {
            "GPST" => original.to::<GPST>().to::<TAI>(),
            "GST" => original.to::<GST>().to::<TAI>(),
            "QZSST" => original.to::<QZSST>().to::<TAI>(),
            "BDT" => original.to::<BDT>().to::<TAI>(),
            _ => unreachable!(),
        };
        let d = original.diff_exact(round_trip).unwrap();
        assert!(
            d.as_nanos_i128().abs() < 1000,
            "{label}: round-trip drift > 1 µs: {d}"
        );
    }
}

#[test]
fn et_is_numerically_identical_to_tdb() {
    let tdb = Time::<TDB>::from_raw_j2000_seconds(Second::new(123_456.789)).unwrap();
    let et = tdb.to::<ET>();
    let d = tdb.diff_exact(et.to::<TDB>()).unwrap();
    assert_eq!(
        d.as_nanos_i128(),
        0,
        "ET ↔ TDB must be an identity at the storage layer"
    );
}

#[test]
fn et_routes_through_tdb_for_other_scales() {
    let et = Time::<ET>::from_raw_j2000_seconds(Second::new(0.0)).unwrap();
    let tt_via_et = et.to::<TT>();
    let tt_via_tdb = Time::<TDB>::from_raw_j2000_seconds(Second::new(0.0))
        .unwrap()
        .to::<TT>();
    let d = tt_via_et.diff_exact(tt_via_tdb).unwrap();
    assert_eq!(d.as_nanos_i128(), 0);
}

#[test]
fn gnss_cross_conversion_uses_integer_offsets() {
    // GPST -> BDT must lose 14 s exactly (nominal).
    let g = Time::<GPST>::from_raw_j2000_seconds(Second::new(500.0)).unwrap();
    let b = g.to::<BDT>();
    let total_g = (g.raw_seconds_pair().0 + g.raw_seconds_pair().1).value();
    let total_b = (b.raw_seconds_pair().0 + b.raw_seconds_pair().1).value();
    assert!((total_g - total_b - 14.0).abs() < 1e-9);
}

#[test]
fn diff_exact_returns_nanosecond_precision() {
    let a = Time::<TAI>::from_raw_j2000_seconds(Second::new(1_000_000.0)).unwrap();
    let b = Time::<TAI>::from_raw_j2000_seconds(Second::new(1_000_001.000_000_001)).unwrap();
    let d = b.diff_exact(a).unwrap();
    // Expect ~1.000000001 s = 1_000_000_001 ns; allow small f64 noise.
    let diff = (d.as_nanos_i128() - 1_000_000_001).abs();
    assert!(
        diff < 100,
        "diff_exact precision drift: {} ns from expected",
        diff
    );
}
