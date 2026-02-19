// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! FFI boundary tests — exercises every exported C function through Rust.

use tempoch_ffi::*;

// ─── Null-pointer guards ──────────────────────────────────────────────────

#[test]
fn period_new_null_pointer() {
    let status = unsafe { tempoch_period_mjd_new(0.0, 1.0, std::ptr::null_mut()) };
    assert_eq!(status, TempochStatus::NullPointer);
}

#[test]
fn jd_to_utc_null_pointer() {
    let status = unsafe { tempoch_jd_to_utc(2_451_545.0, std::ptr::null_mut()) };
    assert_eq!(status, TempochStatus::NullPointer);
}

#[test]
fn mjd_to_utc_null_pointer() {
    let status = unsafe { tempoch_mjd_to_utc(51_544.5, std::ptr::null_mut()) };
    assert_eq!(status, TempochStatus::NullPointer);
}

#[test]
fn jd_from_utc_null_pointer() {
    let utc = TempochUtc {
        year: 2000,
        month: 1,
        day: 1,
        hour: 12,
        minute: 0,
        second: 0,
        nanosecond: 0,
    };
    let status = unsafe { tempoch_jd_from_utc(utc, std::ptr::null_mut()) };
    assert_eq!(status, TempochStatus::NullPointer);
}

#[test]
fn mjd_from_utc_null_pointer() {
    let utc = TempochUtc {
        year: 2000,
        month: 1,
        day: 1,
        hour: 12,
        minute: 0,
        second: 0,
        nanosecond: 0,
    };
    let status = unsafe { tempoch_mjd_from_utc(utc, std::ptr::null_mut()) };
    assert_eq!(status, TempochStatus::NullPointer);
}

#[test]
fn period_intersection_null_pointer() {
    let a = TempochPeriodMjd {
        start_mjd: 0.0,
        end_mjd: 5.0,
    };
    let b = TempochPeriodMjd {
        start_mjd: 3.0,
        end_mjd: 8.0,
    };
    let status = unsafe { tempoch_period_mjd_intersection(a, b, std::ptr::null_mut()) };
    assert_eq!(status, TempochStatus::NullPointer);
}

// ─── Invalid input ────────────────────────────────────────────────────────

#[test]
fn period_new_invalid_period() {
    let mut out = std::mem::MaybeUninit::<TempochPeriodMjd>::uninit();
    let status = unsafe { tempoch_period_mjd_new(5.0, 1.0, out.as_mut_ptr()) };
    assert_eq!(status, TempochStatus::InvalidPeriod);
}

#[test]
fn jd_from_utc_invalid_date() {
    let utc = TempochUtc {
        year: 2000,
        month: 13,
        day: 1, // invalid month
        hour: 0,
        minute: 0,
        second: 0,
        nanosecond: 0,
    };
    let mut out: f64 = 0.0;
    let status = unsafe { tempoch_jd_from_utc(utc, &mut out) };
    assert_eq!(status, TempochStatus::UtcConversionFailed);
}

// ─── Period intersection ──────────────────────────────────────────────────

#[test]
fn period_no_intersection() {
    let a = TempochPeriodMjd {
        start_mjd: 0.0,
        end_mjd: 3.0,
    };
    let b = TempochPeriodMjd {
        start_mjd: 5.0,
        end_mjd: 8.0,
    };
    let mut out = std::mem::MaybeUninit::<TempochPeriodMjd>::uninit();
    let status = unsafe { tempoch_period_mjd_intersection(a, b, out.as_mut_ptr()) };
    assert_eq!(status, TempochStatus::NoIntersection);
}

#[test]
fn period_intersection_ok() {
    let a = TempochPeriodMjd {
        start_mjd: 0.0,
        end_mjd: 5.0,
    };
    let b = TempochPeriodMjd {
        start_mjd: 3.0,
        end_mjd: 8.0,
    };
    let mut out = std::mem::MaybeUninit::<TempochPeriodMjd>::uninit();
    let status = unsafe { tempoch_period_mjd_intersection(a, b, out.as_mut_ptr()) };
    assert_eq!(status, TempochStatus::Ok);
    let result = unsafe { out.assume_init() };
    assert!((result.start_mjd - 3.0).abs() < 1e-12);
    assert!((result.end_mjd - 5.0).abs() < 1e-12);
}

// ─── Period duration ──────────────────────────────────────────────────────

#[test]
fn period_duration_days() {
    let p = TempochPeriodMjd {
        start_mjd: 59_000.0,
        end_mjd: 59_001.5,
    };
    let dur = tempoch_period_mjd_duration_days(p);
    assert!((dur - 1.5).abs() < 1e-12);
}

// ─── JD / MJD roundtrips ──────────────────────────────────────────────────

#[test]
fn jd_utc_roundtrip_j2000() {
    // J2000.0 = 2000-01-01T12:00:00 UTC (approximately)
    let utc = TempochUtc {
        year: 2000,
        month: 1,
        day: 1,
        hour: 12,
        minute: 0,
        second: 0,
        nanosecond: 0,
    };
    let mut jd: f64 = 0.0;
    let s1 = unsafe { tempoch_jd_from_utc(utc, &mut jd) };
    assert_eq!(s1, TempochStatus::Ok);

    let mut utc_back = std::mem::MaybeUninit::<TempochUtc>::uninit();
    let s2 = unsafe { tempoch_jd_to_utc(jd, utc_back.as_mut_ptr()) };
    assert_eq!(s2, TempochStatus::Ok);
    let utc_back = unsafe { utc_back.assume_init() };

    assert_eq!(utc_back.year, 2000);
    assert_eq!(utc_back.month, 1);
    assert_eq!(utc_back.day, 1);
    assert_eq!(utc_back.hour, 12);
    assert_eq!(utc_back.minute, 0);
    // Allow ±1 second tolerance due to ΔT
    assert!(utc_back.second <= 1);
}

#[test]
fn mjd_utc_roundtrip_j2000() {
    let utc = TempochUtc {
        year: 2000,
        month: 1,
        day: 1,
        hour: 12,
        minute: 0,
        second: 0,
        nanosecond: 0,
    };
    let mut mjd: f64 = 0.0;
    let s1 = unsafe { tempoch_mjd_from_utc(utc, &mut mjd) };
    assert_eq!(s1, TempochStatus::Ok);

    let mut utc_back = std::mem::MaybeUninit::<TempochUtc>::uninit();
    let s2 = unsafe { tempoch_mjd_to_utc(mjd, utc_back.as_mut_ptr()) };
    assert_eq!(s2, TempochStatus::Ok);
    let utc_back = unsafe { utc_back.assume_init() };

    assert_eq!(utc_back.year, 2000);
    assert_eq!(utc_back.month, 1);
    assert_eq!(utc_back.day, 1);
    assert_eq!(utc_back.hour, 12);
    assert_eq!(utc_back.minute, 0);
    assert!(utc_back.second <= 1);
}

// ─── JD ↔ MJD conversion ─────────────────────────────────────────────────

#[test]
fn jd_mjd_conversion() {
    let jd = 2_451_545.0; // J2000.0
    let mjd = tempoch_jd_to_mjd(jd);
    assert!((mjd - 51_544.5).abs() < 1e-12);

    let jd_back = tempoch_mjd_to_jd(mjd);
    assert!((jd_back - jd).abs() < 1e-12);
}

// ─── Arithmetic ───────────────────────────────────────────────────────────

#[test]
fn jd_arithmetic() {
    let jd = 2_451_545.0;
    let diff = tempoch_jd_difference(jd, 2_451_544.0);
    assert!((diff - 1.0).abs() < 1e-12);

    let jd2 = tempoch_jd_add_days(jd, 10.0);
    assert!((jd2 - 2_451_555.0).abs() < 1e-12);
}

#[test]
fn mjd_arithmetic() {
    let mjd = 51_544.5;
    let diff = tempoch_mjd_difference(mjd, 51_543.5);
    assert!((diff - 1.0).abs() < 1e-12);

    let mjd2 = tempoch_mjd_add_days(mjd, 10.0);
    assert!((mjd2 - 51_554.5).abs() < 1e-12);
}

// ─── Version ──────────────────────────────────────────────────────────────

#[test]
fn ffi_version_matches_cargo() {
    let v = tempoch_ffi_version();
    // 0.1.0 → 0*10000 + 1*100 + 0 = 100
    assert_eq!(v, 100);
}

// ─── Julian centuries ─────────────────────────────────────────────────────

#[test]
fn julian_centuries_at_j2000_is_zero() {
    let jc = tempoch_jd_julian_centuries(2_451_545.0);
    assert!(jc.abs() < 1e-12);
}

// ─── Period creation ok ───────────────────────────────────────────────────

#[test]
fn period_new_ok() {
    let mut out = std::mem::MaybeUninit::<TempochPeriodMjd>::uninit();
    let status = unsafe { tempoch_period_mjd_new(59_000.0, 59_001.0, out.as_mut_ptr()) };
    assert_eq!(status, TempochStatus::Ok);
    let p = unsafe { out.assume_init() };
    assert!((p.start_mjd - 59_000.0).abs() < 1e-12);
    assert!((p.end_mjd - 59_001.0).abs() < 1e-12);
}
