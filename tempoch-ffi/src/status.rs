// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Active time-data status exposed over the C ABI.
//!
//! Mirrors [`tempoch::time_data_status`]: the validity horizons of the active
//! bundle plus the source it was loaded from. Optional EOP horizons are encoded
//! as `NaN` when no EOP data is loaded.

use crate::error::TempochStatus;
use tempoch::{time_data_status, ActiveTimeDataSource};

/// Origin of the currently active time-data bundle.
///
/// cbindgen:prefix-with-name
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TempochTimeDataSource {
    /// Compiled archive snapshot bundled at build time.
    Bundled = 0,
    /// Bundle loaded through the runtime fetch/cache path.
    RuntimeCache = 1,
    /// Test or caller-provided override is active.
    Override = 2,
}

impl From<ActiveTimeDataSource> for TempochTimeDataSource {
    #[inline]
    fn from(value: ActiveTimeDataSource) -> Self {
        match value {
            ActiveTimeDataSource::Bundled => Self::Bundled,
            ActiveTimeDataSource::RuntimeCache => Self::RuntimeCache,
            ActiveTimeDataSource::Override => Self::Override,
        }
    }
}

/// Validity horizons of the active time-data bundle, in MJD (UTC days).
///
/// EOP horizon fields are `NaN` when no EOP data is loaded.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TempochDataHorizons {
    /// First MJD covered by the EOP series, or `NaN` when no EOP is loaded.
    pub eop_start_mjd: f64,
    /// Last observed (non-predicted) EOP MJD, or `NaN` when no EOP is loaded.
    pub eop_observed_end_mjd: f64,
    /// Last EOP MJD including predictions, or `NaN` when no EOP is loaded.
    pub eop_end_mjd: f64,
    /// Last MJD with observed ΔT in the archive-provided modern table.
    pub modern_delta_t_observed_end_mjd: f64,
    /// Last MJD covered by the ΔT prediction table.
    pub delta_t_prediction_horizon_mjd: f64,
    /// Source of the active bundle as a [`TempochTimeDataSource`] discriminant.
    pub source: i32,
}

/// Capture the active time-data status into `out`.
///
/// # Safety
/// `out` must be a valid, writable pointer to `TempochDataHorizons`.
#[no_mangle]
pub unsafe extern "C" fn tempoch_time_data_status(
    out: *mut TempochDataHorizons,
) -> TempochStatus {
    if out.is_null() {
        return TempochStatus::NullPointer;
    }
    let status = time_data_status();
    let h = status.horizons;
    let horizons = TempochDataHorizons {
        eop_start_mjd: h.eop_start_mjd.unwrap_or(f64::NAN),
        eop_observed_end_mjd: h.eop_observed_end_mjd.unwrap_or(f64::NAN),
        eop_end_mjd: h.eop_end_mjd.unwrap_or(f64::NAN),
        modern_delta_t_observed_end_mjd: h.modern_delta_t_observed_end_mjd,
        delta_t_prediction_horizon_mjd: h.delta_t_prediction_horizon_mjd,
        source: TempochTimeDataSource::from(status.source) as i32,
    };
    // SAFETY: `out` was checked for null and the safety contract requires it to
    // point to writable `TempochDataHorizons` storage.
    unsafe { *out = horizons };
    TempochStatus::Ok
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_reports_finite_delta_t_horizons() {
        let mut out = TempochDataHorizons {
            eop_start_mjd: 0.0,
            eop_observed_end_mjd: 0.0,
            eop_end_mjd: 0.0,
            modern_delta_t_observed_end_mjd: 0.0,
            delta_t_prediction_horizon_mjd: 0.0,
            source: -1,
        };
        let status = unsafe { tempoch_time_data_status(&mut out) };
        assert_eq!(status, TempochStatus::Ok);
        assert!(out.modern_delta_t_observed_end_mjd.is_finite());
        assert!(out.delta_t_prediction_horizon_mjd.is_finite());
        assert!(out.source >= 0);
    }

    #[test]
    fn status_rejects_null_pointer() {
        let status = unsafe { tempoch_time_data_status(std::ptr::null_mut()) };
        assert_eq!(status, TempochStatus::NullPointer);
    }
}
