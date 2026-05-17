// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! FFI boundary tests for the split tempoch C ABI.

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

fn empty_time() -> TempochTime {
    TempochTime {
        hi_seconds: 0.0,
        lo_seconds: 0.0,
    }
}

#[test]
fn period_new_and_intersection_validate_inputs() {
    let status = unsafe { tempoch_period_mjd_new(0.0, 1.0, std::ptr::null_mut()) };
    assert_eq!(status, TempochStatus::NullPointer);

    let mut out = std::mem::MaybeUninit::<TempochPeriodMjd>::uninit();
    let status = unsafe { tempoch_period_mjd_new(5.0, 1.0, out.as_mut_ptr()) };
    assert_eq!(status, TempochStatus::InvalidPeriod);

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

    let mut intersection = std::mem::MaybeUninit::<TempochPeriodMjd>::uninit();
    let status = unsafe { tempoch_period_mjd_intersection(a, b, intersection.as_mut_ptr()) };
    assert_eq!(status, TempochStatus::Ok);
    let intersection = unsafe { intersection.assume_init() };
    assert!((intersection.start_mjd - 3.0).abs() < 1e-12);
    assert!((intersection.end_mjd - 5.0).abs() < 1e-12);
}

#[test]
fn period_duration_and_membership_are_stable() {
    let period = TempochPeriodMjd {
        start_mjd: 59_000.0,
        end_mjd: 59_001.5,
    };
    assert!((tempoch_period_mjd_duration_days(period) - 1.5).abs() < 1e-12);

    let qty = tempoch_period_mjd_duration_qty(period);
    assert_eq!(qty.unit, UnitId::Day as u32);
    assert!((qty.value - 1.5).abs() < 1e-12);

    assert!(tempoch_period_mjd_contains(period, 59_000.25));
    assert!(!tempoch_period_mjd_contains(period, 59_001.5));
}

#[test]
fn split_time_roundtrips_tt_jd() {
    let mut value = empty_time();
    let status = unsafe {
        tempoch_time_from_format(
            2_451_545.0,
            TempochScaleTag::TT as i32,
            TempochFormatTag::JD as i32,
            std::ptr::null(),
            &mut value,
        )
    };
    assert_eq!(status, TempochStatus::Ok);

    let mut jd = 0.0;
    let status = unsafe {
        tempoch_time_to_format(
            value,
            TempochScaleTag::TT as i32,
            TempochFormatTag::JD as i32,
            std::ptr::null(),
            &mut jd,
        )
    };
    assert_eq!(status, TempochStatus::Ok);
    assert!((jd - 2_451_545.0).abs() < 1e-12);
}

#[test]
fn split_time_rejects_invalid_scale_and_format_ids() {
    let mut value = empty_time();
    let status = unsafe {
        tempoch_time_from_format(
            0.0,
            99,
            TempochFormatTag::JD as i32,
            std::ptr::null(),
            &mut value,
        )
    };
    assert_eq!(status, TempochStatus::InvalidScaleId);

    let status = unsafe {
        tempoch_time_from_format(
            0.0,
            TempochScaleTag::TT as i32,
            99,
            std::ptr::null(),
            &mut value,
        )
    };
    assert_eq!(status, TempochStatus::InvalidFormatId);

    let mut raw = 0.0;
    let status = unsafe {
        tempoch_time_to_format(
            value,
            TempochScaleTag::TT as i32,
            99,
            std::ptr::null(),
            &mut raw,
        )
    };
    assert_eq!(status, TempochStatus::InvalidFormatId);
}

#[test]
fn civil_roundtrip_uses_split_utc_axis() {
    let mut utc_time = empty_time();
    let status = unsafe { tempoch_time_from_civil(utc_j2000(), std::ptr::null(), &mut utc_time) };
    assert_eq!(status, TempochStatus::Ok);

    let mut civil = TempochUtc {
        year: 0,
        month: 0,
        day: 0,
        hour: 0,
        minute: 0,
        second: 0,
        nanosecond: 0,
    };
    let status = unsafe { tempoch_time_to_civil(utc_time, std::ptr::null(), &mut civil) };
    assert_eq!(status, TempochStatus::Ok);
    assert_eq!(civil.year, 2000);
    assert_eq!(civil.month, 1);
    assert_eq!(civil.day, 1);
    assert_eq!(civil.hour, 12);
}

#[test]
fn unix_and_gps_formats_roundtrip() {
    let mut unix_time = empty_time();
    let status = unsafe {
        tempoch_time_from_format(
            0.0,
            TempochScaleTag::UTC as i32,
            TempochFormatTag::Unix as i32,
            std::ptr::null(),
            &mut unix_time,
        )
    };
    assert_eq!(status, TempochStatus::Ok);

    let mut unix_raw = 0.0;
    let status = unsafe {
        tempoch_time_to_format(
            unix_time,
            TempochScaleTag::UTC as i32,
            TempochFormatTag::Unix as i32,
            std::ptr::null(),
            &mut unix_raw,
        )
    };
    assert_eq!(status, TempochStatus::Ok);
    assert!(unix_raw.abs() < 1e-5);

    let mut gps_time = empty_time();
    let status = unsafe {
        tempoch_time_from_format(
            0.0,
            TempochScaleTag::TT as i32,
            TempochFormatTag::GPS as i32,
            std::ptr::null(),
            &mut gps_time,
        )
    };
    assert_eq!(status, TempochStatus::Ok);

    let mut gps_raw = 0.0;
    let status = unsafe {
        tempoch_time_to_format(
            gps_time,
            TempochScaleTag::TT as i32,
            TempochFormatTag::GPS as i32,
            std::ptr::null(),
            &mut gps_raw,
        )
    };
    assert_eq!(status, TempochStatus::Ok);
    assert!(gps_raw.abs() < 1e-5);
}

#[test]
fn scale_convert_reports_ut1_horizon() {
    let mut future_utc = empty_time();
    let status = unsafe {
        tempoch_time_from_format(
            2_465_000.0,
            TempochScaleTag::UTC as i32,
            TempochFormatTag::JD as i32,
            std::ptr::null(),
            &mut future_utc,
        )
    };
    assert_eq!(status, TempochStatus::Ok);

    let mut future_ut1 = empty_time();
    let status = unsafe {
        tempoch_time_scale_convert(
            future_utc,
            TempochScaleTag::UTC as i32,
            TempochScaleTag::UT1 as i32,
            std::ptr::null(),
            &mut future_ut1,
        )
    };
    assert_eq!(status, TempochStatus::Ut1HorizonExceeded);
}

#[test]
fn split_time_arithmetic_uses_seconds() {
    let mut value = empty_time();
    let status = unsafe { tempoch_time_new(10.0, 0.25, &mut value) };
    assert_eq!(status, TempochStatus::Ok);

    let mut shifted = empty_time();
    let status = unsafe {
        tempoch_time_add_seconds(value, QttyQuantity::new(2.0, UnitId::Second), &mut shifted)
    };
    assert_eq!(status, TempochStatus::Ok);
    assert!((shifted.hi_seconds + shifted.lo_seconds - 12.25).abs() < 1e-12);

    let mut diff = 0.0;
    let status = unsafe { tempoch_time_difference_seconds(shifted, value, &mut diff) };
    assert_eq!(status, TempochStatus::Ok);
    assert!((diff - 2.0).abs() < 1e-12);
}
