// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Programmatic provenance & freshness reporting for the compiled and
//! runtime time-data bundles.
//!
//! The bundled `UTC-TAI`, EOP, and ΔT tables ship with metadata captured at
//! build time (or at runtime refresh time when the `runtime-data-fetch`
//! feature is active). This module exposes that metadata as a versioned
//! `ProvenanceSnapshot` struct so downstream code can:
//!
//! * audit which upstream files were used,
//! * surface staleness diagnostics in operator UIs and pipelines,
//! * enforce a maximum acceptable age (`assert_fresh`),
//! * and refuse to silently extrapolate past documented horizons.
//!
//! The structured snapshot is *additive* over the existing
//! `TimeDataProvenance` accessor surface; existing callers continue to work.

use chrono::{DateTime, Utc};

use crate::data::runtime_data::active_time_data;

/// Programmatic snapshot of the currently active time-data bundle.
///
/// Captured at call time from the runtime store; safe to pass across
/// threads.
#[derive(Debug, Clone, PartialEq)]
pub struct ProvenanceSnapshot {
    /// When the bundle's source files were fetched, as recorded by the
    /// updater. May be `None` for the compiled-in bundle if the recorded
    /// timestamp is not parseable as ISO 8601.
    pub fetched_at: Option<DateTime<Utc>>,
    /// Raw stored timestamp string (the on-disk form).
    pub fetched_utc: String,
    /// SHA-256 of the upstream `UTC-TAI.history` file.
    pub utc_tai_sha256: String,
    /// SHA-256 of the upstream `deltat.data` file.
    pub delta_t_observed_sha256: String,
    /// SHA-256 of the upstream `deltat.preds` file.
    pub delta_t_predictions_sha256: String,
    /// SHA-256 of the upstream IERS `finals2000A.all` file.
    pub eop_finals_sha256: String,
    /// Documented validity horizons for the bundled tables.
    pub horizons: DataHorizons,
    /// Documented upstream source URLs (constant, listed for traceability).
    pub source_urls: SourceUrls,
}

/// Documented validity horizons of the currently active time-data bundle,
/// expressed in MJD UTC days.
///
/// EOP horizon fields are `None` when no EOP data has been loaded into the
/// active bundle.  UTC-TAI and ΔT horizons are always present (they come from
/// the compiled-in snapshot).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DataHorizons {
    /// First MJD covered by the EOP series, or `None` when no EOP is loaded.
    pub eop_start_mjd: Option<f64>,
    /// Last observed (non-predicted) EOP MJD, or `None` when no EOP is loaded.
    pub eop_observed_end_mjd: Option<f64>,
    /// Last EOP MJD (including predictions), or `None` when no EOP is loaded.
    pub eop_end_mjd: Option<f64>,
    pub modern_delta_t_observed_end_mjd: f64,
    pub delta_t_prediction_horizon_mjd: f64,
}

/// Documented upstream URLs for the bundled data products.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceUrls {
    pub utc_tai: &'static str,
    pub delta_t_observed: &'static str,
    pub delta_t_predictions: &'static str,
    pub eop_finals: &'static str,
}

impl SourceUrls {
    /// The constant URLs the updater fetches from.
    pub const DEFAULT: Self = Self {
        utc_tai: siderust_archive::time::UTC_TAI_HISTORY_URL,
        delta_t_observed: siderust_archive::time::DELTA_T_OBSERVED_URL,
        delta_t_predictions: siderust_archive::time::DELTA_T_PREDICTIONS_URL,
        eop_finals: siderust_archive::time::EOP_FINALS_URL,
    };
}

/// Errors raised by freshness checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FreshnessError {
    /// The bundle's `fetched_at` could not be parsed (compiled-in bundle
    /// may report a non-ISO 8601 timestamp).
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

/// Capture a `ProvenanceSnapshot` of the currently active bundle.
///
/// Safe to call at any time; reads the active runtime store under a lock.
pub fn provenance() -> ProvenanceSnapshot {
    let bundle = active_time_data();
    let raw = bundle.provenance();
    ProvenanceSnapshot {
        fetched_at: raw.fetched_at(),
        fetched_utc: raw.fetched_utc().to_string(),
        utc_tai_sha256: raw.utc_tai_sha256().to_string(),
        delta_t_observed_sha256: raw.delta_t_observed_sha256().to_string(),
        delta_t_predictions_sha256: raw.delta_t_predictions_sha256().to_string(),
        eop_finals_sha256: raw.eop_finals_sha256().to_string(),
        horizons: DataHorizons {
            eop_start_mjd: bundle.eop_start_mjd().map(|v| v as f64),
            eop_observed_end_mjd: bundle.eop_observed_end_mjd().map(|v| v as f64),
            eop_end_mjd: bundle.eop_end_mjd().map(|v| v as f64),
            modern_delta_t_observed_end_mjd: crate::MODERN_DELTA_T_OBSERVED_END_MJD.value(),
            delta_t_prediction_horizon_mjd: crate::DELTA_T_PREDICTION_HORIZON_MJD.value(),
        },
        source_urls: SourceUrls::DEFAULT,
    }
}

/// Assert the active bundle is no older than `max_age` relative to `now`.
///
/// Use this at pipeline start-up to surface a clear error before any
/// conversions silently use a stale UTC-TAI table.
pub fn assert_fresh(now: DateTime<Utc>, max_age: chrono::Duration) -> Result<(), FreshnessError> {
    let snap = provenance();
    let fetched = snap.fetched_at.ok_or(FreshnessError::MissingTimestamp)?;
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
    use crate::data::runtime_data::with_test_time_data;
    use siderust_archive::time::{EopPoint, TimeDataBundle, TimeDataProvenance, UtcTaiSegment};

    fn eop_bundle_for_provenance_test() -> TimeDataBundle {
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
            vec![
                EopPoint {
                    mjd: 50000,
                    pm_observed: true,
                    ut1_observed: true,
                    nutation_observed: true,
                    pm_xp_arcsec: Some(0.1),
                    pm_yp_arcsec: Some(0.1),
                    ut1_minus_utc_seconds: 0.3,
                    lod_milliseconds: Some(1.0),
                    dx_milliarcsec: None,
                    dy_milliarcsec: None,
                },
                EopPoint {
                    mjd: 50001,
                    pm_observed: false,
                    ut1_observed: false,
                    nutation_observed: false,
                    pm_xp_arcsec: Some(0.2),
                    pm_yp_arcsec: Some(0.2),
                    ut1_minus_utc_seconds: 0.4,
                    lod_milliseconds: None,
                    dx_milliarcsec: None,
                    dy_milliarcsec: None,
                },
            ],
            TimeDataProvenance::new("2024-01-01T00:00:00Z", "aaaa", "bbbb", "cccc", "dddd"),
        )
    }

    #[test]
    fn snapshot_has_documented_horizons() {
        let bundle = eop_bundle_for_provenance_test();
        with_test_time_data(bundle, || {
            let s = provenance();
            let eop_start = s.horizons.eop_start_mjd.expect("EOP start should be Some");
            let eop_end = s.horizons.eop_end_mjd.expect("EOP end should be Some");
            let eop_obs_end = s
                .horizons
                .eop_observed_end_mjd
                .expect("EOP observed end should be Some");
            assert!(eop_end > eop_start);
            assert!(eop_obs_end >= eop_start);
            assert!(eop_obs_end <= eop_end);
            assert!(s.horizons.delta_t_prediction_horizon_mjd > 0.0);
        });
    }

    #[test]
    fn compiled_bundle_eop_horizons_are_none() {
        // The compiled bundle intentionally has no EOP data.
        // Operators must explicitly fetch EOP via TimeDataManager.
        let s = provenance();
        assert!(s.horizons.eop_start_mjd.is_none());
        assert!(s.horizons.eop_observed_end_mjd.is_none());
        assert!(s.horizons.eop_end_mjd.is_none());
        assert!(s.horizons.delta_t_prediction_horizon_mjd > 0.0);
    }

    #[test]
    fn snapshot_exposes_source_urls() {
        let s = provenance();
        assert!(s.source_urls.utc_tai.starts_with("https://"));
        assert!(s.source_urls.eop_finals.contains("finals"));
    }

    #[test]
    fn snapshot_carries_sha256_strings() {
        let s = provenance();
        // Compiled-in bundle has placeholder shas; runtime bundles have 64-hex.
        // Just check the fields are non-empty.
        assert!(!s.utc_tai_sha256.is_empty());
        assert!(!s.eop_finals_sha256.is_empty());
    }

    #[test]
    fn assert_fresh_rejects_extremely_old_bundle() {
        // Force "now" so far ahead that even a recently-fetched bundle is stale.
        let far_future = DateTime::<Utc>::from_timestamp(10_000_000_000, 0).unwrap();
        let max_age = chrono::Duration::seconds(1);
        let res = assert_fresh(far_future, max_age);
        // Either Stale or MissingTimestamp is acceptable; both surface staleness.
        assert!(matches!(
            res,
            Err(FreshnessError::Stale { .. }) | Err(FreshnessError::MissingTimestamp)
        ));
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
