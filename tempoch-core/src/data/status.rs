// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Active time-data status and freshness diagnostics.
//!
//! `siderust-archive` owns IERS/USNO time-data provenance, source URLs,
//! checksums, parsing, download, and integrity validation.  `tempoch-core`
//! exposes a thin diagnostic view over whichever archive bundle is currently
//! active: the archive provenance record, validity horizons relevant to time
//! conversions, and the source of the active bundle.

use chrono::{DateTime, Utc};

use crate::archive::time::TimeDataProvenance;
use crate::data::runtime_data::{active_time_data, active_time_data_source};

/// Source of the currently active time-data bundle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveTimeDataSource {
    /// The compiled archive snapshot bundled into `siderust-archive`.
    Bundled,
    /// A bundle loaded through the runtime fetch/cache path.
    ///
    /// This value intentionally does not distinguish cache hit from fresh
    /// download; `siderust-archive` owns the fetch/cache mechanics and
    /// provenance timestamp.
    RuntimeCache,
    /// A test or caller-provided override is active.
    Override,
}

/// Active time-data status captured from the runtime store.
#[derive(Debug, Clone, PartialEq)]
pub struct TimeDataStatus {
    /// Archive-owned provenance and checksum metadata for the active bundle.
    pub provenance: TimeDataProvenance,
    /// Validity horizons relevant to `tempoch` conversions.
    pub horizons: DataHorizons,
    /// Where the active bundle came from.
    pub source: ActiveTimeDataSource,
}

/// Documented validity horizons of the currently active time-data bundle,
/// expressed in MJD UTC days.
///
/// EOP horizon fields are `None` when no EOP data has been loaded into the
/// active bundle. UTC-TAI and Delta T horizons are always present through the
/// bundled archive snapshot.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DataHorizons {
    /// First MJD covered by the EOP series, or `None` when no EOP is loaded.
    pub eop_start_mjd: Option<f64>,
    /// Last observed (non-predicted) EOP MJD, or `None` when no EOP is loaded.
    pub eop_observed_end_mjd: Option<f64>,
    /// Last EOP MJD including predictions, or `None` when no EOP is loaded.
    pub eop_end_mjd: Option<f64>,
    /// Last MJD with observed Delta T in the archive-provided modern table.
    pub modern_delta_t_observed_end_mjd: f64,
    /// Last MJD covered by the Delta T prediction table.
    pub delta_t_prediction_horizon_mjd: f64,
}

/// Errors raised by freshness checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FreshnessError {
    /// The active bundle has no parseable archive provenance timestamp.
    MissingTimestamp,
    /// The bundle is older than `max_age` relative to `now`. Carries the
    /// observed age in seconds.
    Stale {
        age_seconds: i64,
        max_age_seconds: i64,
    },
}

impl core::fmt::Display for FreshnessError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::MissingTimestamp => {
                f.write_str("time-data bundle has no parseable fetched_at timestamp")
            }
            Self::Stale {
                age_seconds,
                max_age_seconds,
            } => write!(
                f,
                "time-data bundle is {age_seconds}s old; max allowed is {max_age_seconds}s",
            ),
        }
    }
}

impl std::error::Error for FreshnessError {}

/// Capture status for the currently active time-data bundle.
///
/// The returned provenance is the archive-owned
/// [`TimeDataProvenance`](crate::archive::time::TimeDataProvenance), not a
/// second `tempoch` provenance model.
pub fn time_data_status() -> TimeDataStatus {
    let bundle = active_time_data();
    TimeDataStatus {
        provenance: bundle.provenance().clone(),
        horizons: DataHorizons {
            eop_start_mjd: bundle.eop_start_mjd().map(|v| v as f64),
            eop_observed_end_mjd: bundle.eop_observed_end_mjd().map(|v| v as f64),
            eop_end_mjd: bundle.eop_end_mjd().map(|v| v as f64),
            modern_delta_t_observed_end_mjd: crate::MODERN_DELTA_T_OBSERVED_END_MJD.value(),
            delta_t_prediction_horizon_mjd: crate::DELTA_T_PREDICTION_HORIZON_MJD.value(),
        },
        source: active_time_data_source(),
    }
}

/// Assert the active bundle is no older than `max_age` relative to `now`.
///
/// Freshness is based on `status.provenance.fetched_at()` from the archive
/// provenance record.
pub fn assert_fresh(now: DateTime<Utc>, max_age: chrono::Duration) -> Result<(), FreshnessError> {
    let status = time_data_status();
    let fetched = status
        .provenance
        .fetched_at()
        .ok_or(FreshnessError::MissingTimestamp)?;
    let age = now.signed_duration_since(fetched);
    if age > max_age {
        return Err(FreshnessError::Stale {
            age_seconds: age.num_seconds(),
            max_age_seconds: max_age.num_seconds(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::archive::time::{EopPoint, TimeDataBundle, TimeDataProvenance, UtcTaiSegment};
    use crate::data::runtime_data::{with_runtime_data_lock, with_test_time_data};
    use qtty::{Arcsecond, Millisecond, Second};

    fn eop_bundle_for_status_test(fetched_utc: &str) -> TimeDataBundle {
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
            vec![
                EopPoint {
                    mjd: 50000,
                    pm_observed: true,
                    ut1_observed: true,
                    nutation_observed: true,
                    pm_xp: Some(Arcsecond::new(0.1)),
                    pm_yp: Some(Arcsecond::new(0.1)),
                    ut1_minus_utc: Second::new(0.3),
                    lod: Some(Millisecond::new(1.0)),
                    dx: None,
                    dy: None,
                },
                EopPoint {
                    mjd: 50001,
                    pm_observed: false,
                    ut1_observed: false,
                    nutation_observed: false,
                    pm_xp: Some(Arcsecond::new(0.2)),
                    pm_yp: Some(Arcsecond::new(0.2)),
                    ut1_minus_utc: Second::new(0.4),
                    lod: None,
                    dx: None,
                    dy: None,
                },
            ],
            TimeDataProvenance::new(fetched_utc, "aaaa", "bbbb", "cccc", "dddd"),
        )
    }

    #[test]
    fn status_has_documented_horizons() {
        let bundle = eop_bundle_for_status_test("2024-01-01T00:00:00");
        with_test_time_data(bundle, || {
            let status = time_data_status();
            let eop_start = status
                .horizons
                .eop_start_mjd
                .expect("EOP start should be Some");
            let eop_end = status.horizons.eop_end_mjd.expect("EOP end should be Some");
            let eop_obs_end = status
                .horizons
                .eop_observed_end_mjd
                .expect("EOP observed end should be Some");
            assert!(eop_end > eop_start);
            assert!(eop_obs_end >= eop_start);
            assert!(eop_obs_end <= eop_end);
            assert!(status.horizons.delta_t_prediction_horizon_mjd > 0.0);
        });
    }

    #[test]
    fn compiled_bundle_eop_horizons_are_none() {
        with_runtime_data_lock(|| {
            // The compiled bundle intentionally has no EOP data.
            // Operators must explicitly fetch EOP via TimeDataManager.
            let status = time_data_status();
            assert_eq!(status.source, ActiveTimeDataSource::Bundled);
            assert!(status.horizons.eop_start_mjd.is_none());
            assert!(status.horizons.eop_observed_end_mjd.is_none());
            assert!(status.horizons.eop_end_mjd.is_none());
            assert!(status.horizons.delta_t_prediction_horizon_mjd > 0.0);
        });
    }

    #[test]
    fn status_exposes_archive_provenance_without_copy_fields() {
        let bundle = eop_bundle_for_status_test("2024-01-01T00:00:00");
        with_test_time_data(bundle, || {
            let status = time_data_status();
            assert_eq!(status.source, ActiveTimeDataSource::Override);
            assert_eq!(status.provenance.fetched_utc(), "2024-01-01T00:00:00");
            assert_eq!(status.provenance.utc_tai_sha256(), "aaaa");
            assert_eq!(status.provenance.delta_t_observed_sha256(), "bbbb");
            assert_eq!(status.provenance.delta_t_predictions_sha256(), "cccc");
            assert_eq!(status.provenance.eop_finals_sha256(), "dddd");
        });
    }

    #[test]
    fn assert_fresh_accepts_recent_archive_provenance() {
        let bundle = eop_bundle_for_status_test("2024-01-01T00:00:00");
        with_test_time_data(bundle, || {
            let now = DateTime::parse_from_rfc3339("2024-01-01T00:10:00Z")
                .unwrap()
                .with_timezone(&Utc);
            assert!(assert_fresh(now, chrono::Duration::minutes(15)).is_ok());
        });
    }

    #[test]
    fn assert_fresh_rejects_stale_archive_provenance() {
        let bundle = eop_bundle_for_status_test("2024-01-01T00:00:00");
        with_test_time_data(bundle, || {
            let now = DateTime::parse_from_rfc3339("2024-01-01T01:00:00Z")
                .unwrap()
                .with_timezone(&Utc);
            let res = assert_fresh(now, chrono::Duration::minutes(15));
            assert!(matches!(res, Err(FreshnessError::Stale { .. })));
        });
    }

    #[test]
    fn freshness_error_implements_display_and_error() {
        let e = FreshnessError::Stale {
            age_seconds: 100,
            max_age_seconds: 50,
        };
        let s = format!("{e}");
        assert!(s.contains("100"));
        let _: &dyn std::error::Error = &e;
    }
}
