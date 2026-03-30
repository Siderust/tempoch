// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! FFI bindings for tempoch period/interval types.

use crate::carriers::TempochMjd;
use crate::catch_panic;
use crate::error::TempochStatus;
use qtty_ffi::{QttyQuantity, UnitId};
use tempoch::{Interval, ModifiedJulianDate, Period, MJD};

// ═══════════════════════════════════════════════════════════════════════════
// C-repr type
// ═══════════════════════════════════════════════════════════════════════════

/// A time period expressed in Modified Julian Date, suitable for C interop.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TempochPeriodMjd {
    /// Start of the period (MJD).
    pub start_mjd: TempochMjd,
    /// End of the period (MJD).
    pub end_mjd: TempochMjd,
}

impl TempochPeriodMjd {
    /// Convert from a Rust `Period<MJD>` to the C-repr struct.
    pub fn from_period(p: &Period<MJD>) -> Self {
        Self {
            start_mjd: TempochMjd::new(p.start.value()),
            end_mjd: TempochMjd::new(p.end.value()),
        }
    }

    /// Convert to a Rust `Period<MJD>`.
    pub fn to_period(&self) -> Period<MJD> {
        Interval::new(
            ModifiedJulianDate::new(self.start_mjd.value),
            ModifiedJulianDate::new(self.end_mjd.value),
        )
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Period functions
// ═══════════════════════════════════════════════════════════════════════════

/// Create a new MJD period. Returns `InvalidPeriod` if `start_mjd > end_mjd`.
///
/// # Safety
/// `out` must be a valid, writable pointer to `TempochPeriodMjd`.
#[no_mangle]
pub unsafe extern "C" fn tempoch_period_mjd_new(
    start_mjd: TempochMjd,
    end_mjd: TempochMjd,
    out: *mut TempochPeriodMjd,
) -> TempochStatus {
    catch_panic!(TempochStatus::InternalPanic, {
        if out.is_null() {
            return TempochStatus::NullPointer;
        }
        if start_mjd.value > end_mjd.value {
            return TempochStatus::InvalidPeriod;
        }
        unsafe { *out = TempochPeriodMjd { start_mjd, end_mjd } };
        TempochStatus::Ok
    })
}

/// Compute the duration of a period in days (end − start).
#[no_mangle]
pub extern "C" fn tempoch_period_mjd_duration_days(period: TempochPeriodMjd) -> f64 {
    period.end_mjd.value - period.start_mjd.value
}

/// Compute the duration of a period as a `QttyQuantity` in days.
#[no_mangle]
pub extern "C" fn tempoch_period_mjd_duration_qty(period: TempochPeriodMjd) -> QttyQuantity {
    QttyQuantity::new(period.end_mjd.value - period.start_mjd.value, UnitId::Day)
}

/// Compute the intersection of two MJD periods.
///
/// Returns `NoIntersection` if the periods do not overlap, `Ok` if `out` is filled.
///
/// # Safety
/// `out` must be a valid, writable pointer to `TempochPeriodMjd`.
#[no_mangle]
pub unsafe extern "C" fn tempoch_period_mjd_intersection(
    a: TempochPeriodMjd,
    b: TempochPeriodMjd,
    out: *mut TempochPeriodMjd,
) -> TempochStatus {
    catch_panic!(TempochStatus::InternalPanic, {
        if out.is_null() {
            return TempochStatus::NullPointer;
        }
        let pa = a.to_period();
        let pb = b.to_period();
        match pa.intersection(&pb) {
            Some(result) => {
                unsafe { *out = TempochPeriodMjd::from_period(&result) };
                TempochStatus::Ok
            }
            None => TempochStatus::NoIntersection,
        }
    })
}

/// Free a `TempochPeriodMjd` array allocated by a tempoch-ffi function.
///
/// Passing a null pointer is safe (no-op).
///
/// # Safety
/// `ptr` and `count` must have been returned by the same function call and
/// must not be used after this call.
#[no_mangle]
pub unsafe extern "C" fn tempoch_period_mjd_free(ptr: *mut TempochPeriodMjd, count: usize) {
    if !ptr.is_null() && count > 0 {
        unsafe {
            let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, count));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::carriers::TempochMjd;

    #[test]
    fn period_new_valid() {
        let mut out = TempochPeriodMjd {
            start_mjd: TempochMjd::new(0.0),
            end_mjd: TempochMjd::new(0.0),
        };
        let status = unsafe {
            tempoch_period_mjd_new(
                TempochMjd::new(51_544.5),
                TempochMjd::new(51_545.5),
                &mut out,
            )
        };
        assert_eq!(status, TempochStatus::Ok);
        assert!((out.start_mjd.value - 51_544.5).abs() < 1e-12);
        assert!((out.end_mjd.value - 51_545.5).abs() < 1e-12);
    }

    #[test]
    fn period_new_invalid_start_gt_end() {
        let mut out = TempochPeriodMjd {
            start_mjd: TempochMjd::new(0.0),
            end_mjd: TempochMjd::new(0.0),
        };
        let status = unsafe {
            tempoch_period_mjd_new(
                TempochMjd::new(51_545.5),
                TempochMjd::new(51_544.5),
                &mut out,
            )
        };
        assert_eq!(status, TempochStatus::InvalidPeriod);
    }

    #[test]
    fn period_new_null_returns_error() {
        let status = unsafe {
            tempoch_period_mjd_new(
                TempochMjd::new(0.0),
                TempochMjd::new(1.0),
                std::ptr::null_mut(),
            )
        };
        assert_eq!(status, TempochStatus::NullPointer);
    }

    #[test]
    fn period_duration_days() {
        let p = TempochPeriodMjd {
            start_mjd: TempochMjd::new(51_544.5),
            end_mjd: TempochMjd::new(51_546.5),
        };
        let dur = tempoch_period_mjd_duration_days(p);
        assert!((dur - 2.0).abs() < 1e-12);
    }

    #[test]
    fn period_duration_qty_unit_is_day() {
        let p = TempochPeriodMjd {
            start_mjd: TempochMjd::new(51_544.5),
            end_mjd: TempochMjd::new(51_546.5),
        };
        let qty = tempoch_period_mjd_duration_qty(p);
        assert_eq!(qty.unit, UnitId::Day);
        assert!((qty.value - 2.0).abs() < 1e-12);
    }

    #[test]
    fn period_intersection_overlapping() {
        let a = TempochPeriodMjd {
            start_mjd: TempochMjd::new(0.0),
            end_mjd: TempochMjd::new(10.0),
        };
        let b = TempochPeriodMjd {
            start_mjd: TempochMjd::new(5.0),
            end_mjd: TempochMjd::new(15.0),
        };
        let mut out = TempochPeriodMjd {
            start_mjd: TempochMjd::new(0.0),
            end_mjd: TempochMjd::new(0.0),
        };
        let status = unsafe { tempoch_period_mjd_intersection(a, b, &mut out) };
        assert_eq!(status, TempochStatus::Ok);
        assert!((out.start_mjd.value - 5.0).abs() < 1e-12);
        assert!((out.end_mjd.value - 10.0).abs() < 1e-12);
    }

    #[test]
    fn period_intersection_non_overlapping() {
        let a = TempochPeriodMjd {
            start_mjd: TempochMjd::new(0.0),
            end_mjd: TempochMjd::new(5.0),
        };
        let b = TempochPeriodMjd {
            start_mjd: TempochMjd::new(10.0),
            end_mjd: TempochMjd::new(15.0),
        };
        let mut out = TempochPeriodMjd {
            start_mjd: TempochMjd::new(0.0),
            end_mjd: TempochMjd::new(0.0),
        };
        let status = unsafe { tempoch_period_mjd_intersection(a, b, &mut out) };
        assert_eq!(status, TempochStatus::NoIntersection);
    }

    #[test]
    fn period_intersection_null_out() {
        let a = TempochPeriodMjd {
            start_mjd: TempochMjd::new(0.0),
            end_mjd: TempochMjd::new(5.0),
        };
        let b = a;
        let status = unsafe { tempoch_period_mjd_intersection(a, b, std::ptr::null_mut()) };
        assert_eq!(status, TempochStatus::NullPointer);
    }

    #[test]
    fn period_free_null_is_safe() {
        unsafe { tempoch_period_mjd_free(std::ptr::null_mut(), 0) };
    }
}
