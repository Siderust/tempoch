// SPDX-License-Identifier: AGPL-3.0-only
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
///
/// # Safety
/// `out` must be a valid, writable pointer to `TempochPeriodMjd`.
#[no_mangle]
pub unsafe extern "C" fn tempoch_period_mjd_new(
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
    // SAFETY: `out` was checked for null and the caller guarantees it points to writable memory.
    unsafe {
        *out = TempochPeriodMjd { start_mjd, end_mjd };
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
///
/// # Safety
/// `out` must be a valid, writable pointer to `TempochPeriodMjd`.
#[no_mangle]
pub unsafe extern "C" fn tempoch_period_mjd_intersection(
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
            // SAFETY: `out` was checked for null and the caller guarantees it points to writable memory.
            unsafe { *out = TempochPeriodMjd::from_period(&result) };
            TempochStatus::Ok
        }
        None => TempochStatus::NoIntersection,
    }
}
