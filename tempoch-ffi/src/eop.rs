// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! FFI bindings for the IERS Earth Orientation Parameters (EOP) API.
//!
//! Exposes `builtin_eop_at` and `builtin_eop_covers` from `tempoch::eop`
//! through a C-compatible struct.  Optional fields in the Rust `EopValues`
//! type are encoded as `f64` with `NaN` representing absent values.

use crate::catch_panic;
use crate::error::TempochStatus;
use qtty::Day;
use tempoch::eop;

/// Interpolated IERS Earth Orientation Parameter values at a UTC MJD.
///
/// All scalar fields use the units from the upstream IERS `finals2000A.all`
/// file.  Fields that the upstream file leaves blank for the requested epoch
/// are encoded as `NaN`; check with `isnan()` before using them.
///
/// # Fields
/// - `mjd_utc` — query epoch rounded to the nearest data point boundary.
/// - `pm_xp_arcsec` / `pm_yp_arcsec` — polar-motion components (arcseconds).
/// - `ut1_minus_utc` — DUT1 = UT1 − UTC (seconds of time).
/// - `lod_milliseconds` — excess length-of-day (milliseconds of time).
/// - `dx_milliarcsec` / `dy_milliarcsec` — IAU 2000A celestial-pole offsets
///   (milliarcseconds).
/// - `ut1_observed` — `1` when both bracketing rows carry observed (`I`) data.
///
/// cbindgen:prefix-with-name
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TempochEopValues {
    /// UTC MJD of the interpolation result.
    pub mjd_utc: f64,
    /// Polar motion X component (arcseconds). `NaN` if absent.
    pub pm_xp_arcsec: f64,
    /// Polar motion Y component (arcseconds). `NaN` if absent.
    pub pm_yp_arcsec: f64,
    /// DUT1 = UT1 − UTC (seconds of time).
    pub ut1_minus_utc: f64,
    /// Excess LOD (milliseconds of time). `NaN` if absent.
    pub lod_milliseconds: f64,
    /// ΔX celestial-pole offset (milliarcseconds). `NaN` if absent.
    pub dx_milliarcsec: f64,
    /// ΔY celestial-pole offset (milliarcseconds). `NaN` if absent.
    pub dy_milliarcsec: f64,
    /// Non-zero when both bracketing rows carry observed (`I`) data.
    pub ut1_observed: u8,
}

/// Returns `true` when [`tempoch_eop_at`] would succeed for `mjd_utc`.
#[no_mangle]
pub extern "C" fn tempoch_eop_covers(mjd_utc: f64) -> bool {
    eop::builtin_eop_covers(Day::new(mjd_utc))
}

/// Interpolate IERS EOP values at `mjd_utc` (UTC Modified Julian Date).
///
/// Returns `Ok` and writes the result into `*out` on success.
/// Returns `Ut1HorizonExceeded` when `mjd_utc` is outside the compiled EOP
/// range.
///
/// # Safety
/// `out` must be a valid, writable pointer to `TempochEopValues`.
#[no_mangle]
pub unsafe extern "C" fn tempoch_eop_at(mjd_utc: f64, out: *mut TempochEopValues) -> TempochStatus {
    catch_panic!(TempochStatus::InternalPanic, {
        if out.is_null() {
            return TempochStatus::NullPointer;
        }
        match eop::builtin_eop_at(Day::new(mjd_utc)) {
            Some(v) => {
                // SAFETY: `out` was checked for null and the function safety
                // contract requires it to point to writable
                // `TempochEopValues` storage.
                unsafe {
                    *out = TempochEopValues {
                        mjd_utc: v.mjd_utc.value(),
                        pm_xp_arcsec: v.pm_xp.map(|q| q.value()).unwrap_or(f64::NAN),
                        pm_yp_arcsec: v.pm_yp.map(|q| q.value()).unwrap_or(f64::NAN),
                        ut1_minus_utc: v.ut1_minus_utc.value(),
                        lod_milliseconds: v.lod.map(|q| q.value()).unwrap_or(f64::NAN),
                        dx_milliarcsec: v.dx.map(|q| q.value()).unwrap_or(f64::NAN),
                        dy_milliarcsec: v.dy.map(|q| q.value()).unwrap_or(f64::NAN),
                        ut1_observed: v.ut1_observed as u8,
                    };
                }
                TempochStatus::Ok
            }
            None => TempochStatus::Ut1HorizonExceeded,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constats::{tempoch_const_eop_end_mjd, tempoch_const_eop_start_mjd};

    #[test]
    fn layout_tempoch_eop_values() {
        // 7 × f64 (56 bytes) + 1 × u8 + 7 padding = 64 bytes
        assert_eq!(std::mem::size_of::<TempochEopValues>(), 64);
        assert_eq!(std::mem::align_of::<TempochEopValues>(), 8);
    }

    #[test]
    fn eop_not_loaded_start_and_end_are_nan() {
        // The compiled bundle no longer embeds EOP data; operators must
        // explicitly fetch via TimeDataManager.  When no EOP is loaded the
        // start/end constants return NaN.
        let start = tempoch_const_eop_start_mjd();
        let end = tempoch_const_eop_end_mjd();
        assert!(
            start.is_nan(),
            "expected NaN for eop_start when no EOP loaded, got {start}"
        );
        assert!(
            end.is_nan(),
            "expected NaN for eop_end when no EOP loaded, got {end}"
        );
    }

    #[test]
    fn eop_not_loaded_covers_nothing() {
        // When no EOP data is loaded, covers() should return false for all MJDs.
        assert!(!tempoch_eop_covers(0.0));
        assert!(!tempoch_eop_covers(51_544.0)); // J2000
        assert!(!tempoch_eop_covers(60_000.0));
    }

    #[test]
    fn at_null_pointer_returns_error() {
        let status = unsafe { tempoch_eop_at(51_544.5, std::ptr::null_mut()) };
        assert_eq!(status, TempochStatus::NullPointer);
    }

    #[test]
    fn at_out_of_range_returns_horizon_exceeded() {
        let mut out = TempochEopValues {
            mjd_utc: 0.0,
            pm_xp_arcsec: 0.0,
            pm_yp_arcsec: 0.0,
            ut1_minus_utc: 0.0,
            lod_milliseconds: 0.0,
            dx_milliarcsec: 0.0,
            dy_milliarcsec: 0.0,
            ut1_observed: 0,
        };
        let status = unsafe { tempoch_eop_at(0.0, &mut out) };
        assert_eq!(status, TempochStatus::Ut1HorizonExceeded);
    }

    #[test]
    fn eop_not_loaded_at_returns_horizon_exceeded() {
        // Without EOP data loaded, any valid-looking MJD still returns
        // Ut1HorizonExceeded because the EOP series is empty.
        let mut out = TempochEopValues {
            mjd_utc: 0.0,
            pm_xp_arcsec: 0.0,
            pm_yp_arcsec: 0.0,
            ut1_minus_utc: 0.0,
            lod_milliseconds: 0.0,
            dx_milliarcsec: 0.0,
            dy_milliarcsec: 0.0,
            ut1_observed: 0,
        };
        let status = unsafe { tempoch_eop_at(51_544.0, &mut out) };
        assert_eq!(status, TempochStatus::Ut1HorizonExceeded);
    }
}
