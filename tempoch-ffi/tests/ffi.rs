// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! FFI boundary tests — exercises every exported C function through Rust (vNext ABI).
//!
//! All functions now take typed carriers instead of bare f64.

use tempoch_ffi::*;

fn utc_j2000() -> TempochUtc {
    TempochUtc {
        year: 2000,
        month: 1,
        day: 1,
        hour: 12,
        minute: 0,
        second: 0,
        nanosecond: 0,
    }
}

// ─── Null-pointer guards ──────────────────────────────────────────────────────

#[test]
fn period_new_null_pointer() {
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
fn jd_to_utc_null_pointer() {
    let status = unsafe { tempoch_jd_to_utc(TempochJd::new(2_451_545.0), std::ptr::null_mut()) };
    assert_eq!(status, TempochStatus::NullPointer);
}

#[test]
fn mjd_to_utc_null_pointer() {
    let status = unsafe { tempoch_mjd_to_utc(TempochMjd::new(51_544.5), std::ptr::null_mut()) };
    assert_eq!(status, TempochStatus::NullPointer);
}

#[test]
fn jd_from_utc_null_pointer() {
    let status = unsafe { tempoch_jd_from_utc(utc_j2000(), std::ptr::null_mut()) };
    assert_eq!(status, TempochStatus::NullPointer);
}

#[test]
fn mjd_from_utc_null_pointer() {
    let status = unsafe { tempoch_mjd_from_utc(utc_j2000(), std::ptr::null_mut()) };
    assert_eq!(status, TempochStatus::NullPointer);
}

#[test]
fn period_intersection_null_pointer() {
    let a = TempochPeriodMjd {
        start_mjd: TempochMjd::new(0.0),
        end_mjd: TempochMjd::new(5.0),
    };
    let b = TempochPeriodMjd {
        start_mjd: TempochMjd::new(3.0),
        end_mjd: TempochMjd::new(8.0),
    };
    let status = unsafe { tempoch_period_mjd_intersection(a, b, std::ptr::null_mut()) };
    assert_eq!(status, TempochStatus::NullPointer);
}

#[test]
fn jd_add_qty_null_pointer() {
    let jd = TempochJd::new(2_451_545.0);
    let dur = QttyQuantity::new(1.0, UnitId::Day);
    let status = unsafe { tempoch_jd_add_qty(jd, dur, std::ptr::null_mut()) };
    assert_eq!(status, TempochStatus::NullPointer);
}

#[test]
fn mjd_add_qty_null_pointer() {
    let mjd = TempochMjd::new(51_544.5);
    let dur = QttyQuantity::new(1.0, UnitId::Day);
    let status = unsafe { tempoch_mjd_add_qty(mjd, dur, std::ptr::null_mut()) };
    assert_eq!(status, TempochStatus::NullPointer);
}

// ─── Invalid input ────────────────────────────────────────────────────────────

#[test]
fn period_new_invalid_period() {
    let mut out = std::mem::MaybeUninit::<TempochPeriodMjd>::uninit();
    let status = unsafe {
        tempoch_period_mjd_new(TempochMjd::new(5.0), TempochMjd::new(1.0), out.as_mut_ptr())
    };
    assert_eq!(status, TempochStatus::InvalidPeriod);
}

#[test]
fn jd_from_utc_invalid_date() {
    let utc = TempochUtc {
        year: 2000,
        month: 13, // invalid month
        day: 1,
        hour: 0,
        minute: 0,
        second: 0,
        nanosecond: 0,
    };
    let mut out = TempochJd::new(0.0);
    let status = unsafe { tempoch_jd_from_utc(utc, &mut out) };
    assert_eq!(status, TempochStatus::UtcConversionFailed);
}

#[test]
fn jd_add_qty_invalid_unit() {
    let jd = TempochJd::new(2_451_545.0);
    let bad = QttyQuantity::new(1.0, UnitId::Meter); // not a time unit
    let mut out = TempochJd::new(0.0);
    let status = unsafe { tempoch_jd_add_qty(jd, bad, &mut out) };
    assert_eq!(status, TempochStatus::InvalidDurationUnit);
}

#[test]
fn mjd_add_qty_invalid_unit() {
    let mjd = TempochMjd::new(51_544.5);
    let bad = QttyQuantity::new(1.0, UnitId::Meter);
    let mut out = TempochMjd::new(0.0);
    let status = unsafe { tempoch_mjd_add_qty(mjd, bad, &mut out) };
    assert_eq!(status, TempochStatus::InvalidDurationUnit);
}

// ─── Invalid scale IDs ────────────────────────────────────────────────────────

#[test]
fn jd_to_scale_invalid_id() {
    let jd = TempochJd::new(2_451_545.0);
    for bad_id in [-1i32, 11, 100, i32::MAX] {
        let mut out: f64 = 0.0;
        let status = unsafe { tempoch_jd_to_scale(jd, bad_id, &mut out) };
        assert_eq!(
            status,
            TempochStatus::InvalidScaleId,
            "scale_id {} should be rejected",
            bad_id
        );
    }
}

#[test]
fn scale_to_jd_invalid_id() {
    let mut out = TempochJd::new(0.0);
    for bad_id in [-1i32, 11, 100, i32::MAX] {
        let status = unsafe { tempoch_scale_to_jd(1.0, bad_id, &mut out) };
        assert_eq!(
            status,
            TempochStatus::InvalidScaleId,
            "scale_id {} should be rejected",
            bad_id
        );
    }
}

// ─── Period intersection ──────────────────────────────────────────────────────

#[test]
fn period_no_intersection() {
    let a = TempochPeriodMjd {
        start_mjd: TempochMjd::new(0.0),
        end_mjd: TempochMjd::new(3.0),
    };
    let b = TempochPeriodMjd {
        start_mjd: TempochMjd::new(5.0),
        end_mjd: TempochMjd::new(8.0),
    };
    let mut out = std::mem::MaybeUninit::<TempochPeriodMjd>::uninit();
    let status = unsafe { tempoch_period_mjd_intersection(a, b, out.as_mut_ptr()) };
    assert_eq!(status, TempochStatus::NoIntersection);
}

#[test]
fn period_intersection_ok() {
    let a = TempochPeriodMjd {
        start_mjd: TempochMjd::new(0.0),
        end_mjd: TempochMjd::new(5.0),
    };
    let b = TempochPeriodMjd {
        start_mjd: TempochMjd::new(3.0),
        end_mjd: TempochMjd::new(8.0),
    };
    let mut out = std::mem::MaybeUninit::<TempochPeriodMjd>::uninit();
    let status = unsafe { tempoch_period_mjd_intersection(a, b, out.as_mut_ptr()) };
    assert_eq!(status, TempochStatus::Ok);
    let result = unsafe { out.assume_init() };
    assert!((result.start_mjd.value - 3.0).abs() < 1e-12);
    assert!((result.end_mjd.value - 5.0).abs() < 1e-12);
}

// ─── Period duration ──────────────────────────────────────────────────────────

#[test]
fn period_duration_days() {
    let p = TempochPeriodMjd {
        start_mjd: TempochMjd::new(59_000.0),
        end_mjd: TempochMjd::new(59_001.5),
    };
    let dur = tempoch_period_mjd_duration_days(p);
    assert!((dur - 1.5).abs() < 1e-12);
}

#[test]
fn period_duration_qty_unit_is_day() {
    let p = TempochPeriodMjd {
        start_mjd: TempochMjd::new(59_000.0),
        end_mjd: TempochMjd::new(59_002.0),
    };
    let qty = tempoch_period_mjd_duration_qty(p);
    assert_eq!(qty.unit, UnitId::Day);
    assert!((qty.value - 2.0).abs() < 1e-12);
}

// ─── JD / MJD roundtrips ──────────────────────────────────────────────────────

#[test]
fn jd_utc_roundtrip_j2000() {
    let utc = utc_j2000();
    let mut jd = TempochJd::new(0.0);
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
    assert!(utc_back.second <= 1);
}

#[test]
fn mjd_utc_roundtrip_j2000() {
    let utc = utc_j2000();
    let mut mjd = TempochMjd::new(0.0);
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

// ─── JD ↔ MJD conversion ──────────────────────────────────────────────────────

#[test]
fn jd_mjd_conversion() {
    let jd = TempochJd::new(2_451_545.0);
    let mjd = tempoch_jd_to_mjd(jd);
    assert!((mjd.value - 51_544.5).abs() < 1e-12);

    let jd_back = tempoch_mjd_to_jd(mjd);
    assert!((jd_back.value - jd.value).abs() < 1e-12);
}

// ─── Arithmetic ───────────────────────────────────────────────────────────────

#[test]
fn jd_arithmetic() {
    let jd = TempochJd::new(2_451_545.0);
    let diff = tempoch_jd_difference(jd, TempochJd::new(2_451_544.0));
    assert!((diff - 1.0).abs() < 1e-12);

    let jd2 = tempoch_jd_add_days(jd, 10.0);
    assert!((jd2.value - 2_451_555.0).abs() < 1e-12);
}

#[test]
fn mjd_arithmetic() {
    let mjd = TempochMjd::new(51_544.5);
    let diff = tempoch_mjd_difference(mjd, TempochMjd::new(51_543.5));
    assert!((diff - 1.0).abs() < 1e-12);

    let mjd2 = tempoch_mjd_add_days(mjd, 10.0);
    assert!((mjd2.value - 51_554.5).abs() < 1e-12);
}

#[test]
fn jd_add_qty_days() {
    let jd = TempochJd::new(2_451_545.0);
    let dur = QttyQuantity::new(10.0, UnitId::Day);
    let mut out = TempochJd::new(0.0);
    let status = unsafe { tempoch_jd_add_qty(jd, dur, &mut out) };
    assert_eq!(status, TempochStatus::Ok);
    assert!((out.value - 2_451_555.0).abs() < 1e-10);
}

#[test]
fn jd_add_qty_hours() {
    let jd = TempochJd::new(2_451_545.0);
    let dur = QttyQuantity::new(24.0, UnitId::Hour);
    let mut out = TempochJd::new(0.0);
    let status = unsafe { tempoch_jd_add_qty(jd, dur, &mut out) };
    assert_eq!(status, TempochStatus::Ok);
    assert!((out.value - 2_451_546.0).abs() < 1e-10);
}

// ─── Scale conversions (all scales) ──────────────────────────────────────────

#[test]
fn all_scale_roundtrips() {
    let jd = TempochJd::new(2_451_545.0);
    for scale_id in 0..=10i32 {
        let mut scale_val: f64 = 0.0;
        let s1 = unsafe { tempoch_jd_to_scale(jd, scale_id, &mut scale_val) };
        assert_eq!(s1, TempochStatus::Ok, "to_scale failed for id {}", scale_id);

        let mut jd_back = TempochJd::new(0.0);
        let s2 = unsafe { tempoch_scale_to_jd(scale_val, scale_id, &mut jd_back) };
        assert_eq!(
            s2,
            TempochStatus::Ok,
            "scale_to_jd failed for id {}",
            scale_id
        );

        assert!(
            (jd_back.value - jd.value).abs() < 1e-6,
            "scale_id {} roundtrip error",
            scale_id
        );
    }
}

#[test]
fn typed_scale_roundtrips() {
    let jd = TempochJd::new(2_451_545.0);

    let tdb = tempoch_jd_to_tdb(jd);
    assert!((tempoch_tdb_to_jd(tdb).value - jd.value).abs() < 1e-6);

    let tai = tempoch_jd_to_tai(jd);
    assert!((tempoch_tai_to_jd(tai).value - jd.value).abs() < 1e-10);

    let tcg = tempoch_jd_to_tcg(jd);
    assert!((tempoch_tcg_to_jd(tcg).value - jd.value).abs() < 1e-6);

    let tcb = tempoch_jd_to_tcb(jd);
    assert!((tempoch_tcb_to_jd(tcb).value - jd.value).abs() < 1e-6);

    let gps = tempoch_jd_to_gps(jd);
    assert!((tempoch_gps_to_jd(gps).value - jd.value).abs() < 1e-10);

    let ut = tempoch_jd_to_ut(jd);
    assert!((tempoch_ut_to_jd(ut).value - jd.value).abs() < 1e-6);

    let jde = tempoch_jd_to_jde(jd);
    assert!((tempoch_jde_to_jd(jde).value - jd.value).abs() < 1e-12);

    let unix = tempoch_jd_to_unix(jd);
    assert!((tempoch_unix_to_jd(unix).value - jd.value).abs() < 1e-10);

    let tt = tempoch_jd_to_tt(jd);
    assert!((tempoch_tt_to_jd(tt).value - jd.value).abs() < 1e-12);
}

// ─── Julian centuries ─────────────────────────────────────────────────────────

#[test]
fn julian_centuries_at_j2000_is_zero() {
    let jc = tempoch_jd_julian_centuries(TempochJd::new(2_451_545.0));
    assert!(jc.abs() < 1e-12);
}

#[test]
fn julian_centuries_qty_at_j2000_is_zero() {
    let qty = tempoch_jd_julian_centuries_qty(TempochJd::new(2_451_545.0));
    assert!(qty.value.abs() < 1e-12);
    assert_eq!(qty.unit, UnitId::JulianCentury);
}

// ─── Period creation ──────────────────────────────────────────────────────────

#[test]
fn period_new_ok() {
    let mut out = std::mem::MaybeUninit::<TempochPeriodMjd>::uninit();
    let status = unsafe {
        tempoch_period_mjd_new(
            TempochMjd::new(59_000.0),
            TempochMjd::new(59_001.0),
            out.as_mut_ptr(),
        )
    };
    assert_eq!(status, TempochStatus::Ok);
    let p = unsafe { out.assume_init() };
    assert!((p.start_mjd.value - 59_000.0).abs() < 1e-12);
    assert!((p.end_mjd.value - 59_001.0).abs() < 1e-12);
}

// ─── Version ──────────────────────────────────────────────────────────────────

#[test]
fn ffi_version() {
    // 0.3.0 → 300
    assert_eq!(tempoch_ffi_version(), 300);
}

// ─── Free ─────────────────────────────────────────────────────────────────────

#[test]
fn period_free_null_is_safe() {
    unsafe { tempoch_period_mjd_free(std::ptr::null_mut(), 0) };
}
