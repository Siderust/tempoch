// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! GNSS ICD reference-point tests.
//!
//! Verifies that tempoch's GNSS scale markers reproduce the integer
//! TAI−scale offsets documented in each constellation's ICD at the
//! reference epochs given in `data/gnss/epochs.csv`.

use std::fs;
use std::path::PathBuf;

use qtty::Second;
use tempoch::{ExactDuration, Time, BDT, GPST, GST, QZSST, TAI};
use tempoch_validation::tolerance::GNSS_TAI_NS;

fn data_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("data/gnss/epochs.csv");
    p
}

#[derive(Debug)]
struct Row {
    label: String,
    scale: String,
    nominal_tai_minus_scale_s: f64,
}

fn parse_rows() -> Vec<Row> {
    let text = fs::read_to_string(data_path()).expect("read gnss epochs.csv");
    let mut rows = Vec::new();
    for (i, line) in text.lines().enumerate() {
        if i == 0 || line.trim().is_empty() {
            continue;
        }
        let cols: Vec<&str> = line.split(',').collect();
        assert_eq!(cols.len(), 5, "bad csv row {i}: {line}");
        rows.push(Row {
            label: cols[0].to_string(),
            scale: cols[1].to_string(),
            nominal_tai_minus_scale_s: cols[4].parse().expect("offset"),
        });
    }
    rows
}

/// Construct a `Time<TAI>` at an arbitrary J2000-second offset; the absolute
/// epoch does not matter for verifying that the *offset* between TAI and a
/// GNSS scale matches the ICD-documented integer constant. The conversion
/// matrix is the same at every instant for a fixed-offset scale.
fn sample_tai() -> Time<TAI> {
    Time::<TAI>::from_raw_j2000_seconds(Second::new(1_234_567.0)).unwrap()
}

fn offset_ns(scale: &str) -> i128 {
    let tai = sample_tai();
    let tai_secs = (tai.raw_seconds_pair().0 + tai.raw_seconds_pair().1).value();
    let scale_secs = match scale {
        "GPST" => {
            let s = tai.to::<GPST>();
            (s.raw_seconds_pair().0 + s.raw_seconds_pair().1).value()
        }
        "GST" => {
            let s = tai.to::<GST>();
            (s.raw_seconds_pair().0 + s.raw_seconds_pair().1).value()
        }
        "QZSST" => {
            let s = tai.to::<QZSST>();
            (s.raw_seconds_pair().0 + s.raw_seconds_pair().1).value()
        }
        "BDT" => {
            let s = tai.to::<BDT>();
            (s.raw_seconds_pair().0 + s.raw_seconds_pair().1).value()
        }
        other => panic!("unknown scale {other}"),
    };
    let delta_s = tai_secs - scale_secs;
    ExactDuration::from_seconds_f64_lossy(delta_s)
        .expect("finite delta")
        .as_nanos_i128()
}

#[test]
fn icd_integer_offsets_match() {
    let rows = parse_rows();
    assert!(!rows.is_empty(), "must have ICD reference rows");
    for row in &rows {
        let got_ns = offset_ns(&row.scale);
        let expected_ns = (row.nominal_tai_minus_scale_s * 1e9) as i128;
        let drift = (got_ns - expected_ns).abs();
        assert!(
            drift <= GNSS_TAI_NS,
            "{}: TAI - {} = {} ns, expected {} ns (drift {} ns)",
            row.label,
            row.scale,
            got_ns,
            expected_ns,
            drift
        );
    }
}

#[test]
fn bdt_minus_gpst_is_minus_14_s_per_icd() {
    let gpst = Time::<GPST>::from_raw_j2000_seconds(Second::new(0.0)).unwrap();
    let bdt = gpst.to::<BDT>();
    let delta_s = (gpst.raw_seconds_pair().0 + gpst.raw_seconds_pair().1).value()
        - (bdt.raw_seconds_pair().0 + bdt.raw_seconds_pair().1).value();
    let drift_ns = ExactDuration::from_seconds_f64_lossy(delta_s)
        .unwrap()
        .as_nanos_i128()
        - 14_000_000_000;
    assert!(
        drift_ns.abs() <= GNSS_TAI_NS,
        "BDT - GPST should be -14 s; drift {} ns",
        drift_ns
    );
}

#[test]
fn gpst_round_trip_to_tai_is_exact_within_tolerance() {
    let original = sample_tai();
    let round = original.to::<GPST>().to::<TAI>();
    let d = original.diff_exact(round).unwrap();
    assert!(
        d.as_nanos_i128().abs() <= GNSS_TAI_NS,
        "GPST round-trip drift > {} ns: {} ns",
        GNSS_TAI_NS,
        d.as_nanos_i128()
    );
}
