// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! FFI bindings for tempoch period/interval types.

use crate::catch_panic;
use crate::error::TempochStatus;
use qtty::Day;
use qtty_ffi::{QttyQuantity, UnitId};
use tempoch::{Interval, ModifiedJulianDate, PeriodListError, Time, TT};

type MjdPeriod = Interval<Time<TT>>;

/// A time period expressed in Modified Julian Date, suitable for C interop.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TempochPeriodMjd {
    /// Start of the period (MJD).
    pub start_mjd: f64,
    /// End of the period (MJD).
    pub end_mjd: f64,
}

impl TempochPeriodMjd {
    /// Convert from a Rust MJD interval to the C-repr struct.
    pub fn from_period(p: &MjdPeriod) -> Self {
        use tempoch::MJD;
        Self {
            start_mjd: p.start.to::<MJD>().raw() / Day::new(1.0),
            end_mjd: p.end.to::<MJD>().raw() / Day::new(1.0),
        }
    }

    fn try_to_period(&self) -> Result<MjdPeriod, TempochStatus> {
        if self.start_mjd.is_nan() || self.end_mjd.is_nan() {
            return Err(TempochStatus::InvalidPeriod);
        }
        let start = ModifiedJulianDate::<TT>::new(self.start_mjd).to_j2000s();
        let end = ModifiedJulianDate::<TT>::new(self.end_mjd).to_j2000s();
        Interval::try_new(start, end).map_err(|_| TempochStatus::InvalidPeriod)
    }
}

/// Create a new MJD period. Returns `InvalidPeriod` when an endpoint is NaN or `start_mjd > end_mjd`.
///
/// # Safety
/// `out` must be a valid, writable pointer to `TempochPeriodMjd`.
#[no_mangle]
pub unsafe extern "C" fn tempoch_period_mjd_new(
    start_mjd: f64,
    end_mjd: f64,
    out: *mut TempochPeriodMjd,
) -> TempochStatus {
    catch_panic!(TempochStatus::InternalPanic, {
        if out.is_null() {
            return TempochStatus::NullPointer;
        }
        let candidate = TempochPeriodMjd { start_mjd, end_mjd };
        if candidate.try_to_period().is_err() {
            return TempochStatus::InvalidPeriod;
        }
        unsafe { *out = candidate };
        TempochStatus::Ok
    })
}

/// Compute the duration of a period in days (end − start).
#[no_mangle]
pub extern "C" fn tempoch_period_mjd_duration_days(period: TempochPeriodMjd) -> f64 {
    period.end_mjd - period.start_mjd
}

/// Compute the duration of a period as a `QttyQuantity` in days.
#[no_mangle]
pub extern "C" fn tempoch_period_mjd_duration_qty(period: TempochPeriodMjd) -> QttyQuantity {
    QttyQuantity::new(period.end_mjd - period.start_mjd, UnitId::Day)
}

/// Compute the intersection of two MJD periods.
///
/// Returns `InvalidPeriod` if either input period is malformed and
/// `NoIntersection` if the periods do not overlap.
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
        let pa = match a.try_to_period() {
            Ok(p) => p,
            Err(status) => return status,
        };
        let pb = match b.try_to_period() {
            Ok(p) => p,
            Err(status) => return status,
        };
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

// ── Helper utilities ──────────────────────────────────────────────────────────

/// Convert a `PeriodListError` to the appropriate `TempochStatus`.
#[inline]
fn period_list_error_to_status(e: PeriodListError) -> TempochStatus {
    match e {
        PeriodListError::InvalidInterval { .. } => TempochStatus::InvalidPeriod,
        PeriodListError::Unsorted { .. } => TempochStatus::PeriodListUnsorted,
        PeriodListError::Overlapping { .. } => TempochStatus::PeriodListOverlapping,
    }
}

/// Convert a C array of `TempochPeriodMjd` to a `Vec<MjdPeriod>`.
///
/// # Safety
/// `ptr` must point to `count` valid, initialized `TempochPeriodMjd` values.
unsafe fn slice_to_periods(
    ptr: *const TempochPeriodMjd,
    count: usize,
) -> Result<Vec<MjdPeriod>, TempochStatus> {
    let slice = unsafe { std::slice::from_raw_parts(ptr, count) };
    slice.iter().map(|p| p.try_to_period()).collect()
}

/// Convert a `Vec<MjdPeriod>` to a heap-allocated C array.
///
/// The caller is responsible for freeing the returned pointer with
/// `tempoch_period_mjd_free(*out, *out_count)`.
fn periods_to_heap(
    periods: Vec<MjdPeriod>,
    out: *mut *mut TempochPeriodMjd,
    out_count: *mut usize,
) {
    let ffi_periods: Vec<TempochPeriodMjd> =
        periods.iter().map(TempochPeriodMjd::from_period).collect();
    let mut boxed = ffi_periods.into_boxed_slice();
    let count = boxed.len();
    let ptr = boxed.as_mut_ptr();
    std::mem::forget(boxed);
    unsafe {
        *out = ptr;
        *out_count = count;
    }
}

// ── Point-in-period test ──────────────────────────────────────────────────────

/// Return `true` when `mjd` is within the half-open interval `[start, end)`.
#[no_mangle]
pub extern "C" fn tempoch_period_mjd_contains(period: TempochPeriodMjd, mjd: f64) -> bool {
    mjd >= period.start_mjd && mjd < period.end_mjd
}

// ── Pairwise union ────────────────────────────────────────────────────────────

/// Compute the union of two MJD periods.
///
/// Overlapping or touching periods are merged into one; disjoint periods
/// produce two entries.  The result is written into the caller-provided
/// two-element array `out[0..2]`; `*out_count` is set to 1 or 2.
///
/// Returns `InvalidPeriod` if either input period is malformed.
///
/// # Safety
/// `out` must point to at least two writable `TempochPeriodMjd` values.
/// `out_count` must be a valid writable pointer to `uintptr_t`.
#[no_mangle]
pub unsafe extern "C" fn tempoch_period_mjd_union(
    a: TempochPeriodMjd,
    b: TempochPeriodMjd,
    out: *mut TempochPeriodMjd,
    out_count: *mut usize,
) -> TempochStatus {
    catch_panic!(TempochStatus::InternalPanic, {
        if out.is_null() || out_count.is_null() {
            return TempochStatus::NullPointer;
        }
        let pa = match a.try_to_period() {
            Ok(p) => p,
            Err(status) => return status,
        };
        let pb = match b.try_to_period() {
            Ok(p) => p,
            Err(status) => return status,
        };
        let result = pa.union(&pb);
        unsafe {
            *out_count = result.len();
            for (i, p) in result.iter().enumerate() {
                *out.add(i) = TempochPeriodMjd::from_period(p);
            }
        }
        TempochStatus::Ok
    })
}

// ── Period list operations ────────────────────────────────────────────────────

/// Validate that a period list is sorted, non-overlapping, and each
/// `start <= end`.
///
/// Returns `Ok` if the list is valid, `InvalidPeriod` for malformed
/// intervals, `PeriodListUnsorted` for ordering violations, and
/// `PeriodListOverlapping` for overlapping intervals.
///
/// Passing a null pointer with `count == 0` is valid and returns `Ok`.
///
/// # Safety
/// `periods` must point to `count` valid, initialized `TempochPeriodMjd`
/// values (or be null when `count == 0`).
#[no_mangle]
pub unsafe extern "C" fn tempoch_period_list_validate(
    periods: *const TempochPeriodMjd,
    count: usize,
) -> TempochStatus {
    catch_panic!(TempochStatus::InternalPanic, {
        if count == 0 {
            return TempochStatus::Ok;
        }
        if periods.is_null() {
            return TempochStatus::NullPointer;
        }
        let rust_periods = match unsafe { slice_to_periods(periods, count) } {
            Ok(ps) => ps,
            Err(status) => return status,
        };
        match Interval::validate(&rust_periods) {
            Ok(()) => TempochStatus::Ok,
            Err(e) => period_list_error_to_status(e),
        }
    })
}

/// Compute the complement of `periods` within `outer`: the gaps inside
/// `outer` that are not covered by any period in the list.
///
/// `periods` must be sorted and non-overlapping.  The result is heap-
/// allocated; the caller must free it with `tempoch_period_mjd_free`.
///
/// Returns `InvalidPeriod` for malformed inputs, `PeriodListUnsorted` or
/// `PeriodListOverlapping` for invalid list structure.
///
/// # Safety
/// - `periods` must point to `count` valid `TempochPeriodMjd` values (or be
///   null when `count == 0`).
/// - `out` and `out_count` must be valid writable pointers.
#[no_mangle]
pub unsafe extern "C" fn tempoch_period_list_complement(
    outer: TempochPeriodMjd,
    periods: *const TempochPeriodMjd,
    count: usize,
    out: *mut *mut TempochPeriodMjd,
    out_count: *mut usize,
) -> TempochStatus {
    catch_panic!(TempochStatus::InternalPanic, {
        if out.is_null() || out_count.is_null() {
            return TempochStatus::NullPointer;
        }
        let outer_period = match outer.try_to_period() {
            Ok(p) => p,
            Err(status) => return status,
        };
        let input: Vec<MjdPeriod> = if count == 0 {
            Vec::new()
        } else {
            if periods.is_null() {
                return TempochStatus::NullPointer;
            }
            match unsafe { slice_to_periods(periods, count) } {
                Ok(ps) => ps,
                Err(status) => return status,
            }
        };
        match outer_period.try_complement(&input) {
            Ok(gaps) => {
                periods_to_heap(gaps, out, out_count);
                TempochStatus::Ok
            }
            Err(e) => period_list_error_to_status(e),
        }
    })
}

/// Intersect two sorted, non-overlapping period lists.
///
/// The result is heap-allocated; the caller must free it with
/// `tempoch_period_mjd_free`.
///
/// Returns `InvalidPeriod`, `PeriodListUnsorted`, or `PeriodListOverlapping`
/// when either input list is invalid.
///
/// # Safety
/// - `a` / `b` must point to `a_count` / `b_count` valid periods (or be null
///   when the corresponding count is 0).
/// - `out` and `out_count` must be valid writable pointers.
#[no_mangle]
pub unsafe extern "C" fn tempoch_period_list_intersect(
    a: *const TempochPeriodMjd,
    a_count: usize,
    b: *const TempochPeriodMjd,
    b_count: usize,
    out: *mut *mut TempochPeriodMjd,
    out_count: *mut usize,
) -> TempochStatus {
    catch_panic!(TempochStatus::InternalPanic, {
        if out.is_null() || out_count.is_null() {
            return TempochStatus::NullPointer;
        }
        let load = |ptr: *const TempochPeriodMjd, cnt: usize| {
            if cnt == 0 {
                Ok(Vec::new())
            } else if ptr.is_null() {
                Err(TempochStatus::NullPointer)
            } else {
                unsafe { slice_to_periods(ptr, cnt) }
            }
        };
        let ra = match load(a, a_count) {
            Ok(ps) => ps,
            Err(s) => return s,
        };
        let rb = match load(b, b_count) {
            Ok(ps) => ps,
            Err(s) => return s,
        };
        match Interval::try_intersect_many(&ra, &rb) {
            Ok(result) => {
                periods_to_heap(result, out, out_count);
                TempochStatus::Ok
            }
            Err(e) => period_list_error_to_status(e),
        }
    })
}

/// Merge two period lists, combining overlapping and adjacent intervals.
///
/// The inputs do not need to be pre-sorted or non-overlapping; the output
/// is always sorted and non-overlapping.  The result is heap-allocated; the
/// caller must free it with `tempoch_period_mjd_free`.
///
/// Returns `InvalidPeriod` if any input period is malformed.
///
/// # Safety
/// - `a` / `b` must point to `a_count` / `b_count` valid periods (or be null
///   when the corresponding count is 0).
/// - `out` and `out_count` must be valid writable pointers.
#[no_mangle]
pub unsafe extern "C" fn tempoch_period_list_union(
    a: *const TempochPeriodMjd,
    a_count: usize,
    b: *const TempochPeriodMjd,
    b_count: usize,
    out: *mut *mut TempochPeriodMjd,
    out_count: *mut usize,
) -> TempochStatus {
    catch_panic!(TempochStatus::InternalPanic, {
        if out.is_null() || out_count.is_null() {
            return TempochStatus::NullPointer;
        }
        let load = |ptr: *const TempochPeriodMjd, cnt: usize| {
            if cnt == 0 {
                Ok(Vec::new())
            } else if ptr.is_null() {
                Err(TempochStatus::NullPointer)
            } else {
                unsafe { slice_to_periods(ptr, cnt) }
            }
        };
        let ra = match load(a, a_count) {
            Ok(ps) => ps,
            Err(s) => return s,
        };
        let rb = match load(b, b_count) {
            Ok(ps) => ps,
            Err(s) => return s,
        };
        // Validate that each individual period is well-formed; union_many does
        // not check for internal correctness, only sorted order is assumed.
        if let Err(e) = Interval::validate(&ra) {
            return period_list_error_to_status(e);
        }
        if let Err(e) = Interval::validate(&rb) {
            return period_list_error_to_status(e);
        }
        let result = Interval::union_many(&ra, &rb);
        periods_to_heap(result, out, out_count);
        TempochStatus::Ok
    })
}

/// Sort and merge overlapping or adjacent intervals in a period list.
///
/// The result is heap-allocated; the caller must free it with
/// `tempoch_period_mjd_free`.
///
/// Returns `InvalidPeriod` if any input period is malformed.
///
/// # Safety
/// - `periods` must point to `count` valid periods (or be null when
///   `count == 0`).
/// - `out` and `out_count` must be valid writable pointers.
#[no_mangle]
pub unsafe extern "C" fn tempoch_period_list_normalize(
    periods: *const TempochPeriodMjd,
    count: usize,
    out: *mut *mut TempochPeriodMjd,
    out_count: *mut usize,
) -> TempochStatus {
    catch_panic!(TempochStatus::InternalPanic, {
        if out.is_null() || out_count.is_null() {
            return TempochStatus::NullPointer;
        }
        let input: Vec<MjdPeriod> = if count == 0 {
            Vec::new()
        } else {
            if periods.is_null() {
                return TempochStatus::NullPointer;
            }
            match unsafe { slice_to_periods(periods, count) } {
                Ok(ps) => ps,
                Err(status) => return status,
            }
        };
        let result = Interval::normalize(&input);
        periods_to_heap(result, out, out_count);
        TempochStatus::Ok
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn period_new_valid() {
        let mut out = TempochPeriodMjd {
            start_mjd: 0.0,
            end_mjd: 0.0,
        };
        let status = unsafe { tempoch_period_mjd_new(51_544.5, 51_545.5, &mut out) };
        assert_eq!(status, TempochStatus::Ok);
        assert!((out.start_mjd - 51_544.5).abs() < 1e-12);
        assert!((out.end_mjd - 51_545.5).abs() < 1e-12);
    }

    #[test]
    fn period_new_invalid_start_gt_end() {
        let mut out = TempochPeriodMjd {
            start_mjd: 0.0,
            end_mjd: 0.0,
        };
        let status = unsafe { tempoch_period_mjd_new(51_545.5, 51_544.5, &mut out) };
        assert_eq!(status, TempochStatus::InvalidPeriod);
    }

    #[test]
    fn period_new_invalid_nan() {
        let mut out = TempochPeriodMjd {
            start_mjd: 0.0,
            end_mjd: 0.0,
        };
        let status = unsafe { tempoch_period_mjd_new(f64::NAN, 1.0, &mut out) };
        assert_eq!(status, TempochStatus::InvalidPeriod);
    }

    #[test]
    fn period_new_null_returns_error() {
        let status = unsafe { tempoch_period_mjd_new(0.0, 1.0, std::ptr::null_mut()) };
        assert_eq!(status, TempochStatus::NullPointer);
    }

    #[test]
    fn period_duration_days() {
        let p = TempochPeriodMjd {
            start_mjd: 51_544.5,
            end_mjd: 51_546.5,
        };
        let dur = tempoch_period_mjd_duration_days(p);
        assert!((dur - 2.0).abs() < 1e-12);
    }

    #[test]
    fn period_duration_qty_unit_is_day() {
        let p = TempochPeriodMjd {
            start_mjd: 51_544.5,
            end_mjd: 51_546.5,
        };
        let qty = tempoch_period_mjd_duration_qty(p);
        assert_eq!(qty.unit, UnitId::Day as u32);
        assert!((qty.value - 2.0).abs() < 1e-12);
    }

    #[test]
    fn period_intersection_overlapping() {
        let a = TempochPeriodMjd {
            start_mjd: 0.0,
            end_mjd: 10.0,
        };
        let b = TempochPeriodMjd {
            start_mjd: 5.0,
            end_mjd: 15.0,
        };
        let mut out = TempochPeriodMjd {
            start_mjd: 0.0,
            end_mjd: 0.0,
        };
        let status = unsafe { tempoch_period_mjd_intersection(a, b, &mut out) };
        assert_eq!(status, TempochStatus::Ok);
        assert!((out.start_mjd - 5.0).abs() < 1e-12);
        assert!((out.end_mjd - 10.0).abs() < 1e-12);
    }

    #[test]
    fn period_intersection_non_overlapping() {
        let a = TempochPeriodMjd {
            start_mjd: 0.0,
            end_mjd: 5.0,
        };
        let b = TempochPeriodMjd {
            start_mjd: 10.0,
            end_mjd: 15.0,
        };
        let mut out = TempochPeriodMjd {
            start_mjd: 0.0,
            end_mjd: 0.0,
        };
        let status = unsafe { tempoch_period_mjd_intersection(a, b, &mut out) };
        assert_eq!(status, TempochStatus::NoIntersection);
    }

    #[test]
    fn period_intersection_invalid_period() {
        let a = TempochPeriodMjd {
            start_mjd: f64::NAN,
            end_mjd: 5.0,
        };
        let b = TempochPeriodMjd {
            start_mjd: 1.0,
            end_mjd: 2.0,
        };
        let mut out = TempochPeriodMjd {
            start_mjd: 0.0,
            end_mjd: 0.0,
        };
        let status = unsafe { tempoch_period_mjd_intersection(a, b, &mut out) };
        assert_eq!(status, TempochStatus::InvalidPeriod);
    }

    #[test]
    fn period_intersection_null_out() {
        let a = TempochPeriodMjd {
            start_mjd: 0.0,
            end_mjd: 5.0,
        };
        let b = a;
        let status = unsafe { tempoch_period_mjd_intersection(a, b, std::ptr::null_mut()) };
        assert_eq!(status, TempochStatus::NullPointer);
    }

    #[test]
    fn period_free_null_is_safe() {
        unsafe { tempoch_period_mjd_free(std::ptr::null_mut(), 0) };
    }

    // ── contains ────────────────────────────────────────────────────────────

    #[test]
    fn period_contains_inside() {
        let p = TempochPeriodMjd {
            start_mjd: 10.0,
            end_mjd: 20.0,
        };
        assert!(tempoch_period_mjd_contains(p, 15.0));
    }

    #[test]
    fn period_contains_at_start_is_inclusive() {
        let p = TempochPeriodMjd {
            start_mjd: 10.0,
            end_mjd: 20.0,
        };
        assert!(tempoch_period_mjd_contains(p, 10.0));
    }

    #[test]
    fn period_contains_at_end_is_exclusive() {
        let p = TempochPeriodMjd {
            start_mjd: 10.0,
            end_mjd: 20.0,
        };
        assert!(!tempoch_period_mjd_contains(p, 20.0));
    }

    #[test]
    fn period_contains_before_start_is_false() {
        let p = TempochPeriodMjd {
            start_mjd: 10.0,
            end_mjd: 20.0,
        };
        assert!(!tempoch_period_mjd_contains(p, 5.0));
    }

    #[test]
    fn period_contains_after_end_is_false() {
        let p = TempochPeriodMjd {
            start_mjd: 10.0,
            end_mjd: 20.0,
        };
        assert!(!tempoch_period_mjd_contains(p, 25.0));
    }

    // ── union ───────────────────────────────────────────────────────────────

    #[test]
    fn period_union_overlapping_merges_to_one() {
        let a = TempochPeriodMjd {
            start_mjd: 0.0,
            end_mjd: 10.0,
        };
        let b = TempochPeriodMjd {
            start_mjd: 5.0,
            end_mjd: 15.0,
        };
        let mut out = [
            TempochPeriodMjd {
                start_mjd: 0.0,
                end_mjd: 0.0,
            },
            TempochPeriodMjd {
                start_mjd: 0.0,
                end_mjd: 0.0,
            },
        ];
        let mut count: usize = 0;
        let status = unsafe { tempoch_period_mjd_union(a, b, out.as_mut_ptr(), &mut count) };
        assert_eq!(status, TempochStatus::Ok);
        assert_eq!(count, 1);
        assert!((out[0].start_mjd - 0.0).abs() < 1e-12);
        assert!((out[0].end_mjd - 15.0).abs() < 1e-12);
    }

    #[test]
    fn period_union_disjoint_gives_two_results() {
        let a = TempochPeriodMjd {
            start_mjd: 0.0,
            end_mjd: 5.0,
        };
        let b = TempochPeriodMjd {
            start_mjd: 10.0,
            end_mjd: 15.0,
        };
        let mut out = [
            TempochPeriodMjd {
                start_mjd: 0.0,
                end_mjd: 0.0,
            },
            TempochPeriodMjd {
                start_mjd: 0.0,
                end_mjd: 0.0,
            },
        ];
        let mut count: usize = 0;
        let status = unsafe { tempoch_period_mjd_union(a, b, out.as_mut_ptr(), &mut count) };
        assert_eq!(status, TempochStatus::Ok);
        assert_eq!(count, 2);
    }

    #[test]
    fn period_union_null_out_returns_null_pointer() {
        let a = TempochPeriodMjd {
            start_mjd: 0.0,
            end_mjd: 5.0,
        };
        let b = a;
        let mut count: usize = 0;
        let status = unsafe { tempoch_period_mjd_union(a, b, std::ptr::null_mut(), &mut count) };
        assert_eq!(status, TempochStatus::NullPointer);
    }

    #[test]
    fn period_union_null_count_returns_null_pointer() {
        let a = TempochPeriodMjd {
            start_mjd: 0.0,
            end_mjd: 5.0,
        };
        let b = a;
        let mut out = [
            TempochPeriodMjd {
                start_mjd: 0.0,
                end_mjd: 0.0,
            },
            TempochPeriodMjd {
                start_mjd: 0.0,
                end_mjd: 0.0,
            },
        ];
        let status =
            unsafe { tempoch_period_mjd_union(a, b, out.as_mut_ptr(), std::ptr::null_mut()) };
        assert_eq!(status, TempochStatus::NullPointer);
    }

    #[test]
    fn period_union_invalid_a_returns_invalid_period() {
        let a = TempochPeriodMjd {
            start_mjd: f64::NAN,
            end_mjd: 5.0,
        };
        let b = TempochPeriodMjd {
            start_mjd: 1.0,
            end_mjd: 5.0,
        };
        let mut out = [
            TempochPeriodMjd {
                start_mjd: 0.0,
                end_mjd: 0.0,
            },
            TempochPeriodMjd {
                start_mjd: 0.0,
                end_mjd: 0.0,
            },
        ];
        let mut count: usize = 0;
        let status = unsafe { tempoch_period_mjd_union(a, b, out.as_mut_ptr(), &mut count) };
        assert_eq!(status, TempochStatus::InvalidPeriod);
    }

    #[test]
    fn period_union_invalid_b_returns_invalid_period() {
        let a = TempochPeriodMjd {
            start_mjd: 1.0,
            end_mjd: 5.0,
        };
        let b = TempochPeriodMjd {
            start_mjd: f64::NAN,
            end_mjd: 5.0,
        };
        let mut out = [
            TempochPeriodMjd {
                start_mjd: 0.0,
                end_mjd: 0.0,
            },
            TempochPeriodMjd {
                start_mjd: 0.0,
                end_mjd: 0.0,
            },
        ];
        let mut count: usize = 0;
        let status = unsafe { tempoch_period_mjd_union(a, b, out.as_mut_ptr(), &mut count) };
        assert_eq!(status, TempochStatus::InvalidPeriod);
    }

    // ── period_intersection_invalid_b ───────────────────────────────────────

    #[test]
    fn period_intersection_invalid_b() {
        let a = TempochPeriodMjd {
            start_mjd: 1.0,
            end_mjd: 5.0,
        };
        let b = TempochPeriodMjd {
            start_mjd: f64::NAN,
            end_mjd: 5.0,
        };
        let mut out = TempochPeriodMjd {
            start_mjd: 0.0,
            end_mjd: 0.0,
        };
        let status = unsafe { tempoch_period_mjd_intersection(a, b, &mut out) };
        assert_eq!(status, TempochStatus::InvalidPeriod);
    }

    // ── list_validate ───────────────────────────────────────────────────────

    #[test]
    fn period_list_validate_empty_returns_ok() {
        let status = unsafe { tempoch_period_list_validate(std::ptr::null(), 0) };
        assert_eq!(status, TempochStatus::Ok);
    }

    #[test]
    fn period_list_validate_single_valid_returns_ok() {
        let p = TempochPeriodMjd {
            start_mjd: 1.0,
            end_mjd: 5.0,
        };
        let status = unsafe { tempoch_period_list_validate(&p, 1) };
        assert_eq!(status, TempochStatus::Ok);
    }

    #[test]
    fn period_list_validate_null_nonzero_returns_null_pointer() {
        let status = unsafe { tempoch_period_list_validate(std::ptr::null(), 1) };
        assert_eq!(status, TempochStatus::NullPointer);
    }

    #[test]
    fn period_list_validate_invalid_period_returns_invalid() {
        let p = TempochPeriodMjd {
            start_mjd: f64::NAN,
            end_mjd: 5.0,
        };
        let status = unsafe { tempoch_period_list_validate(&p, 1) };
        assert_eq!(status, TempochStatus::InvalidPeriod);
    }

    #[test]
    fn period_list_validate_unsorted_returns_unsorted() {
        let periods = [
            TempochPeriodMjd {
                start_mjd: 10.0,
                end_mjd: 15.0,
            },
            TempochPeriodMjd {
                start_mjd: 1.0,
                end_mjd: 5.0,
            },
        ];
        let status = unsafe { tempoch_period_list_validate(periods.as_ptr(), 2) };
        assert_eq!(status, TempochStatus::PeriodListUnsorted);
    }

    #[test]
    fn period_list_validate_overlapping_returns_overlapping() {
        let periods = [
            TempochPeriodMjd {
                start_mjd: 1.0,
                end_mjd: 10.0,
            },
            TempochPeriodMjd {
                start_mjd: 5.0,
                end_mjd: 15.0,
            },
        ];
        let status = unsafe { tempoch_period_list_validate(periods.as_ptr(), 2) };
        assert_eq!(status, TempochStatus::PeriodListOverlapping);
    }

    // ── list_complement ─────────────────────────────────────────────────────

    #[test]
    fn period_list_complement_empty_periods_returns_outer() {
        let outer = TempochPeriodMjd {
            start_mjd: 0.0,
            end_mjd: 10.0,
        };
        let mut out: *mut TempochPeriodMjd = std::ptr::null_mut();
        let mut out_count: usize = 0;
        let status = unsafe {
            tempoch_period_list_complement(outer, std::ptr::null(), 0, &mut out, &mut out_count)
        };
        assert_eq!(status, TempochStatus::Ok);
        assert_eq!(out_count, 1);
        if out_count > 0 {
            let result = unsafe { std::slice::from_raw_parts(out, out_count) };
            assert!((result[0].start_mjd - 0.0).abs() < 1e-12);
            assert!((result[0].end_mjd - 10.0).abs() < 1e-12);
            unsafe { tempoch_period_mjd_free(out, out_count) };
        }
    }

    #[test]
    fn period_list_complement_with_periods_returns_gaps() {
        let outer = TempochPeriodMjd {
            start_mjd: 0.0,
            end_mjd: 10.0,
        };
        let inner = TempochPeriodMjd {
            start_mjd: 3.0,
            end_mjd: 7.0,
        };
        let mut out: *mut TempochPeriodMjd = std::ptr::null_mut();
        let mut out_count: usize = 0;
        let status =
            unsafe { tempoch_period_list_complement(outer, &inner, 1, &mut out, &mut out_count) };
        assert_eq!(status, TempochStatus::Ok);
        assert_eq!(out_count, 2);
        if out_count > 0 {
            let result = unsafe { std::slice::from_raw_parts(out, out_count) };
            assert!((result[0].start_mjd - 0.0).abs() < 1e-12);
            assert!((result[0].end_mjd - 3.0).abs() < 1e-12);
            assert!((result[1].start_mjd - 7.0).abs() < 1e-12);
            assert!((result[1].end_mjd - 10.0).abs() < 1e-12);
            unsafe { tempoch_period_mjd_free(out, out_count) };
        }
    }

    #[test]
    fn period_list_complement_null_out_returns_null_pointer() {
        let outer = TempochPeriodMjd {
            start_mjd: 0.0,
            end_mjd: 10.0,
        };
        let mut out_count: usize = 0;
        let status = unsafe {
            tempoch_period_list_complement(
                outer,
                std::ptr::null(),
                0,
                std::ptr::null_mut(),
                &mut out_count,
            )
        };
        assert_eq!(status, TempochStatus::NullPointer);
    }

    #[test]
    fn period_list_complement_null_periods_nonzero_count_returns_null_pointer() {
        let outer = TempochPeriodMjd {
            start_mjd: 0.0,
            end_mjd: 10.0,
        };
        let mut out: *mut TempochPeriodMjd = std::ptr::null_mut();
        let mut out_count: usize = 0;
        let status = unsafe {
            tempoch_period_list_complement(outer, std::ptr::null(), 1, &mut out, &mut out_count)
        };
        assert_eq!(status, TempochStatus::NullPointer);
    }

    #[test]
    fn period_list_complement_invalid_outer_returns_invalid_period() {
        let outer = TempochPeriodMjd {
            start_mjd: f64::NAN,
            end_mjd: 10.0,
        };
        let mut out: *mut TempochPeriodMjd = std::ptr::null_mut();
        let mut out_count: usize = 0;
        let status = unsafe {
            tempoch_period_list_complement(outer, std::ptr::null(), 0, &mut out, &mut out_count)
        };
        assert_eq!(status, TempochStatus::InvalidPeriod);
    }

    // ── list_intersect ──────────────────────────────────────────────────────

    #[test]
    fn period_list_intersect_basic() {
        let a = [
            TempochPeriodMjd {
                start_mjd: 0.0,
                end_mjd: 5.0,
            },
            TempochPeriodMjd {
                start_mjd: 8.0,
                end_mjd: 12.0,
            },
        ];
        let b = [TempochPeriodMjd {
            start_mjd: 3.0,
            end_mjd: 10.0,
        }];
        let mut out: *mut TempochPeriodMjd = std::ptr::null_mut();
        let mut out_count: usize = 0;
        let status = unsafe {
            tempoch_period_list_intersect(
                a.as_ptr(),
                a.len(),
                b.as_ptr(),
                b.len(),
                &mut out,
                &mut out_count,
            )
        };
        assert_eq!(status, TempochStatus::Ok);
        assert_eq!(out_count, 2);
        if out_count > 0 {
            let result = unsafe { std::slice::from_raw_parts(out, out_count) };
            assert!((result[0].start_mjd - 3.0).abs() < 1e-12);
            assert!((result[0].end_mjd - 5.0).abs() < 1e-12);
            assert!((result[1].start_mjd - 8.0).abs() < 1e-12);
            assert!((result[1].end_mjd - 10.0).abs() < 1e-12);
            unsafe { tempoch_period_mjd_free(out, out_count) };
        }
    }

    #[test]
    fn period_list_intersect_empty_a_returns_empty() {
        let b = [TempochPeriodMjd {
            start_mjd: 3.0,
            end_mjd: 10.0,
        }];
        let mut out: *mut TempochPeriodMjd = std::ptr::null_mut();
        let mut out_count: usize = 0;
        let status = unsafe {
            tempoch_period_list_intersect(
                std::ptr::null(),
                0,
                b.as_ptr(),
                b.len(),
                &mut out,
                &mut out_count,
            )
        };
        assert_eq!(status, TempochStatus::Ok);
        assert_eq!(out_count, 0);
    }

    #[test]
    fn period_list_intersect_empty_b_returns_empty() {
        let a = [TempochPeriodMjd {
            start_mjd: 3.0,
            end_mjd: 10.0,
        }];
        let mut out: *mut TempochPeriodMjd = std::ptr::null_mut();
        let mut out_count: usize = 0;
        let status = unsafe {
            tempoch_period_list_intersect(
                a.as_ptr(),
                a.len(),
                std::ptr::null(),
                0,
                &mut out,
                &mut out_count,
            )
        };
        assert_eq!(status, TempochStatus::Ok);
        assert_eq!(out_count, 0);
    }

    #[test]
    fn period_list_intersect_null_out_returns_null_pointer() {
        let a = [TempochPeriodMjd {
            start_mjd: 0.0,
            end_mjd: 5.0,
        }];
        let b = [TempochPeriodMjd {
            start_mjd: 0.0,
            end_mjd: 5.0,
        }];
        let mut out_count: usize = 0;
        let status = unsafe {
            tempoch_period_list_intersect(
                a.as_ptr(),
                a.len(),
                b.as_ptr(),
                b.len(),
                std::ptr::null_mut(),
                &mut out_count,
            )
        };
        assert_eq!(status, TempochStatus::NullPointer);
    }

    #[test]
    fn period_list_intersect_null_a_nonzero_count_returns_null_pointer() {
        let b = [TempochPeriodMjd {
            start_mjd: 0.0,
            end_mjd: 5.0,
        }];
        let mut out: *mut TempochPeriodMjd = std::ptr::null_mut();
        let mut out_count: usize = 0;
        let status = unsafe {
            tempoch_period_list_intersect(
                std::ptr::null(),
                1,
                b.as_ptr(),
                b.len(),
                &mut out,
                &mut out_count,
            )
        };
        assert_eq!(status, TempochStatus::NullPointer);
    }

    #[test]
    fn period_list_intersect_null_b_nonzero_count_returns_null_pointer() {
        let a = [TempochPeriodMjd {
            start_mjd: 0.0,
            end_mjd: 5.0,
        }];
        let mut out: *mut TempochPeriodMjd = std::ptr::null_mut();
        let mut out_count: usize = 0;
        let status = unsafe {
            tempoch_period_list_intersect(
                a.as_ptr(),
                a.len(),
                std::ptr::null(),
                1,
                &mut out,
                &mut out_count,
            )
        };
        assert_eq!(status, TempochStatus::NullPointer);
    }

    // ── list_union ──────────────────────────────────────────────────────────

    #[test]
    fn period_list_union_basic_merges_adjacent() {
        let a = [TempochPeriodMjd {
            start_mjd: 0.0,
            end_mjd: 5.0,
        }];
        let b = [TempochPeriodMjd {
            start_mjd: 5.0,
            end_mjd: 10.0,
        }];
        let mut out: *mut TempochPeriodMjd = std::ptr::null_mut();
        let mut out_count: usize = 0;
        let status = unsafe {
            tempoch_period_list_union(
                a.as_ptr(),
                a.len(),
                b.as_ptr(),
                b.len(),
                &mut out,
                &mut out_count,
            )
        };
        assert_eq!(status, TempochStatus::Ok);
        assert!(out_count >= 1);
        if out_count > 0 {
            unsafe { tempoch_period_mjd_free(out, out_count) };
        }
    }

    #[test]
    fn period_list_union_both_empty_returns_ok() {
        let mut out: *mut TempochPeriodMjd = std::ptr::null_mut();
        let mut out_count: usize = 0;
        let status = unsafe {
            tempoch_period_list_union(
                std::ptr::null(),
                0,
                std::ptr::null(),
                0,
                &mut out,
                &mut out_count,
            )
        };
        assert_eq!(status, TempochStatus::Ok);
        assert_eq!(out_count, 0);
    }

    #[test]
    fn period_list_union_null_out_returns_null_pointer() {
        let mut out_count: usize = 0;
        let status = unsafe {
            tempoch_period_list_union(
                std::ptr::null(),
                0,
                std::ptr::null(),
                0,
                std::ptr::null_mut(),
                &mut out_count,
            )
        };
        assert_eq!(status, TempochStatus::NullPointer);
    }

    #[test]
    fn period_list_union_null_a_nonzero_returns_null_pointer() {
        let b = [TempochPeriodMjd {
            start_mjd: 0.0,
            end_mjd: 5.0,
        }];
        let mut out: *mut TempochPeriodMjd = std::ptr::null_mut();
        let mut out_count: usize = 0;
        let status = unsafe {
            tempoch_period_list_union(
                std::ptr::null(),
                1,
                b.as_ptr(),
                b.len(),
                &mut out,
                &mut out_count,
            )
        };
        assert_eq!(status, TempochStatus::NullPointer);
    }

    #[test]
    fn period_list_union_invalid_period_in_a_returns_invalid() {
        let a = [TempochPeriodMjd {
            start_mjd: f64::NAN,
            end_mjd: 5.0,
        }];
        let b = [TempochPeriodMjd {
            start_mjd: 0.0,
            end_mjd: 3.0,
        }];
        let mut out: *mut TempochPeriodMjd = std::ptr::null_mut();
        let mut out_count: usize = 0;
        let status = unsafe {
            tempoch_period_list_union(
                a.as_ptr(),
                a.len(),
                b.as_ptr(),
                b.len(),
                &mut out,
                &mut out_count,
            )
        };
        assert_eq!(status, TempochStatus::InvalidPeriod);
    }

    // ── list_normalize ──────────────────────────────────────────────────────

    #[test]
    fn period_list_normalize_merges_overlapping() {
        let periods = [
            TempochPeriodMjd {
                start_mjd: 0.0,
                end_mjd: 7.0,
            },
            TempochPeriodMjd {
                start_mjd: 5.0,
                end_mjd: 10.0,
            },
        ];
        let mut out: *mut TempochPeriodMjd = std::ptr::null_mut();
        let mut out_count: usize = 0;
        let status = unsafe {
            tempoch_period_list_normalize(periods.as_ptr(), periods.len(), &mut out, &mut out_count)
        };
        assert_eq!(status, TempochStatus::Ok);
        assert_eq!(out_count, 1);
        if out_count > 0 {
            let result = unsafe { std::slice::from_raw_parts(out, out_count) };
            assert!((result[0].start_mjd - 0.0).abs() < 1e-12);
            assert!((result[0].end_mjd - 10.0).abs() < 1e-12);
            unsafe { tempoch_period_mjd_free(out, out_count) };
        }
    }

    #[test]
    fn period_list_normalize_empty_input_returns_empty() {
        let mut out: *mut TempochPeriodMjd = std::ptr::null_mut();
        let mut out_count: usize = 0;
        let status =
            unsafe { tempoch_period_list_normalize(std::ptr::null(), 0, &mut out, &mut out_count) };
        assert_eq!(status, TempochStatus::Ok);
        assert_eq!(out_count, 0);
    }

    #[test]
    fn period_list_normalize_null_out_returns_null_pointer() {
        let mut out_count: usize = 0;
        let status = unsafe {
            tempoch_period_list_normalize(std::ptr::null(), 0, std::ptr::null_mut(), &mut out_count)
        };
        assert_eq!(status, TempochStatus::NullPointer);
    }

    #[test]
    fn period_list_normalize_invalid_period_returns_invalid() {
        let periods = [TempochPeriodMjd {
            start_mjd: f64::NAN,
            end_mjd: 5.0,
        }];
        let mut out: *mut TempochPeriodMjd = std::ptr::null_mut();
        let mut out_count: usize = 0;
        let status = unsafe {
            tempoch_period_list_normalize(periods.as_ptr(), periods.len(), &mut out, &mut out_count)
        };
        assert_eq!(status, TempochStatus::InvalidPeriod);
    }

    #[test]
    fn period_list_normalize_null_periods_nonzero_count_returns_null_pointer() {
        let mut out: *mut TempochPeriodMjd = std::ptr::null_mut();
        let mut out_count: usize = 0;
        let status =
            unsafe { tempoch_period_list_normalize(std::ptr::null(), 1, &mut out, &mut out_count) };
        assert_eq!(status, TempochStatus::NullPointer);
    }

    // ── free with nonzero count ─────────────────────────────────────────────

    #[test]
    fn period_free_heap_allocated_array() {
        let periods = [TempochPeriodMjd {
            start_mjd: 0.0,
            end_mjd: 5.0,
        }];
        let mut out: *mut TempochPeriodMjd = std::ptr::null_mut();
        let mut out_count: usize = 0;
        let status = unsafe {
            tempoch_period_list_normalize(periods.as_ptr(), periods.len(), &mut out, &mut out_count)
        };
        assert_eq!(status, TempochStatus::Ok);
        assert_eq!(out_count, 1);
        // Exercise the non-null, non-zero branch of tempoch_period_mjd_free
        unsafe { tempoch_period_mjd_free(out, out_count) };
    }
}
