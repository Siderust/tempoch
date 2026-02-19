// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Vallés Puig, Ramon

//! FFI bindings for tempoch period/interval types.

use crate::error::TempochStatus;
use tempoch::{Interval, ModifiedJulianDate, Period, MJD};

// ═══════════════════════════════════════════════════════════════════════════
// C-repr types
// ═══════════════════════════════════════════════════════════════════════════

/// A time period in Modified Julian Date, suitable for C interop.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TempochPeriodMjd {
    pub start_mjd: f64,
    pub end_mjd: f64,
}

impl TempochPeriodMjd {
    /// Convert from Rust `Period<MJD>` to the C-repr struct.
    pub fn from_period(p: &Period<MJD>) -> Self {
        Self {
            start_mjd: p.start.value(),
            end_mjd: p.end.value(),
        }
    }

    /// Convert to Rust `Period<MJD>`.
    pub fn to_period(&self) -> Period<MJD> {
        Interval::new(
            ModifiedJulianDate::new(self.start_mjd),
            ModifiedJulianDate::new(self.end_mjd),
        )
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Period functions
// ═══════════════════════════════════════════════════════════════════════════

/// Create a new MJD period. Returns InvalidPeriod if start > end.
#[no_mangle]
pub extern "C" fn tempoch_period_mjd_new(
    start_mjd: f64,
    end_mjd: f64,
    out: *mut TempochPeriodMjd,
) -> TempochStatus {
    if out.is_null() {
        return TempochStatus::NullPointer;
    }
    if start_mjd > end_mjd {
        return TempochStatus::InvalidPeriod;
    }
    unsafe {
        *out = TempochPeriodMjd {
            start_mjd,
            end_mjd,
        };
    }
    TempochStatus::Ok
}

/// Compute the duration of a period in days.
#[no_mangle]
pub extern "C" fn tempoch_period_mjd_duration_days(period: TempochPeriodMjd) -> f64 {
    period.end_mjd - period.start_mjd
}

/// Compute the intersection of two periods.
/// Returns NoIntersection if they don't overlap, Ok if `out` is filled.
#[no_mangle]
pub extern "C" fn tempoch_period_mjd_intersection(
    a: TempochPeriodMjd,
    b: TempochPeriodMjd,
    out: *mut TempochPeriodMjd,
) -> TempochStatus {
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
}

/// Free an array of MJD periods previously returned by siderust-ffi.
///
/// # Safety
/// `ptr` must have been allocated by this library, and `count` must match.
#[no_mangle]
pub unsafe extern "C" fn tempoch_periods_free(ptr: *mut TempochPeriodMjd, count: usize) {
    if !ptr.is_null() && count > 0 {
        let _ = Box::from_raw(std::slice::from_raw_parts_mut(ptr, count));
    }
}
