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
use tempoch::{ConversionError, ExactDuration, Time, BDT, GPST, GST, QZSST, TAI, UTC};
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
    utc_iso: String,
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
            utc_iso: cols[2].to_string(),
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

/// Convert UTC `Time` to GNSS week components, dispatching on the scale label.
fn to_gnss_week(utc: Time<UTC>, scale: &str) -> Result<tempoch::GnssWeek, ConversionError> {
    match scale {
        "GPST" => utc.to::<GPST>().to_gnss_week(),
        "GST" => utc.to::<GST>().to_gnss_week(),
        "QZSST" => utc.to::<QZSST>().to_gnss_week(),
        "BDT" => utc.to::<BDT>().to_gnss_week(),
        other => panic!("unknown scale {other}"),
    }
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

/// For each row that marks a constellation epoch (week 0 / second 0), verify
/// that parsing the UTC ISO timestamp and converting to the appropriate GNSS
/// scale yields exactly (week=0, sow=0, ns=0).
#[test]
fn epoch_utc_parses_to_week_zero_second_zero() {
    let rows = parse_rows();
    // Filter to epoch rows only (label contains "week_0_second_0").
    let epoch_rows: Vec<_> = rows
        .iter()
        .filter(|r| r.label.contains("week_0_second_0"))
        .collect();
    assert!(
        !epoch_rows.is_empty(),
        "no epoch rows found in gnss epochs.csv"
    );
    for row in epoch_rows {
        let utc = Time::<UTC>::parse_rfc3339(&row.utc_iso).unwrap_or_else(|e| {
            panic!(
                "failed to parse utc_iso '{}' for {}: {e:?}",
                row.utc_iso, row.label
            )
        });
        let gw = to_gnss_week(utc, &row.scale)
            .unwrap_or_else(|e| panic!("to_gnss_week failed for {}: {e:?}", row.label));
        assert_eq!(
            gw.week, 0,
            "{}: expected week=0, got {}",
            row.label, gw.week
        );
        assert_eq!(
            gw.seconds_of_week, 0,
            "{}: expected sow=0, got {}",
            row.label, gw.seconds_of_week
        );
        assert_eq!(
            gw.subsecond_nanos, 0,
            "{}: expected ns=0, got {}",
            row.label, gw.subsecond_nanos
        );
    }
}

/// For rollover rows, verify the UTC ISO timestamp yields the expected *full*
/// week number with no error.
///
/// [`to_gnss_week`] returns the full week count since the constellation epoch
/// (no modulo applied). GPS week boundaries do **not** coincide with UTC
/// midnight because GPST = TAI − 19 s and TAI−UTC ≠ 19 s outside the GPS
/// epoch; `sow ≠ 0` at UTC midnight at rollover dates.
#[test]
fn rollover_utc_parses_to_expected_week() {
    let rows = parse_rows();
    for row in &rows {
        let expected_week: Option<u32> = if row.label == "gps_week_1024_rollover" {
            Some(1024) // 1999-08-22T00:00:00 UTC → GPST full week 1024
        } else if row.label == "gps_week_2048_rollover" {
            Some(2048) // 2019-04-07T00:00:00 UTC → GPST full week 2048
        } else {
            None
        };
        let Some(expected_week) = expected_week else {
            continue;
        };
        let utc = Time::<UTC>::parse_rfc3339(&row.utc_iso).unwrap_or_else(|e| {
            panic!(
                "failed to parse utc_iso '{}' for {}: {e:?}",
                row.utc_iso, row.label
            )
        });
        let gw = to_gnss_week(utc, &row.scale)
            .unwrap_or_else(|e| panic!("to_gnss_week failed for {}: {e:?}", row.label));
        assert_eq!(
            gw.week, expected_week,
            "{}: expected full week={expected_week}, got {}",
            row.label, gw.week
        );
        // Sanity: sow must be strictly less than one full week.
        assert!(
            gw.seconds_of_week < 604_800,
            "{}: sow {} out of range",
            row.label,
            gw.seconds_of_week
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
