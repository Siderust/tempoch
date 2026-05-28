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

/// Documented validity horizons of the compiled-in / loaded time-data
/// tables, expressed in MJD UTC days.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DataHorizons {
    pub eop_start_mjd: f64,
    pub eop_observed_end_mjd: f64,
    pub eop_end_mjd: f64,
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
        utc_tai: tempoch_time_data::UTC_TAI_HISTORY_URL,
        delta_t_observed: tempoch_time_data::DELTA_T_OBSERVED_URL,
        delta_t_predictions: tempoch_time_data::DELTA_T_PREDICTIONS_URL,
        eop_finals: tempoch_time_data::EOP_FINALS_URL,
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
            eop_start_mjd: crate::EOP_START_MJD.value(),
            eop_observed_end_mjd: crate::EOP_OBSERVED_END_MJD.value(),
            eop_end_mjd: crate::EOP_END_MJD.value(),
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

    #[test]
    fn snapshot_has_documented_horizons() {
        let s = provenance();
        assert!(s.horizons.eop_end_mjd > s.horizons.eop_start_mjd);
        assert!(s.horizons.eop_observed_end_mjd >= s.horizons.eop_start_mjd);
        assert!(s.horizons.eop_observed_end_mjd <= s.horizons.eop_end_mjd);
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
