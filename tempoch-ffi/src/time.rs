// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! FFI bindings for tempoch time operations.
//!
//! The public C ABI uses scalar time values (`double`) plus an explicit
//! scale identifier for the generic entry points. Language-specific bindings
//! are expected to rebuild stronger typed wrappers on top of that substrate.

use crate::carriers::{
    jd_to_scale_value, scale_value_to_jd, time_add_days_value, time_difference_days_value,
    time_from_utc_value, time_to_utc_value, TempochScaleId,
};
use crate::catch_panic;
use crate::error::TempochStatus;
use chrono::{NaiveDate, Utc};
use qtty::Day;
use qtty_ffi::{QttyQuantity, UnitId};
use tempoch::{JulianDays, Time, TimeContext, TT, UT1};

const J2000_JD_TT: f64 = 2_451_545.0;
const JULIAN_CENTURY_DAYS: f64 = 36_525.0;

/// UTC date-time breakdown for C interop.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TempochUtc {
    /// Calendar year (e.g. 2026).
    pub year: i32,
    /// Month of the year (1–12).
    pub month: u8,
    /// Day of the month (1–31).
    pub day: u8,
    /// Hour of the day (0–23).
    pub hour: u8,
    /// Minute of the hour (0–59).
    pub minute: u8,
    /// Second of the minute (0–60). `60` denotes a positive leap second.
    pub second: u8,
    /// Sub-second component in nanoseconds (0–999_999_999).
    pub nanosecond: u32,
}

impl TempochUtc {
    pub(crate) fn into_chrono(self) -> Option<chrono::DateTime<Utc>> {
        let date = NaiveDate::from_ymd_opt(self.year, self.month as u32, self.day as u32)?;
        let (second, nanosecond) = if self.second == 60 {
            (59_u32, self.nanosecond.checked_add(1_000_000_000)?)
        } else {
            (self.second.into(), self.nanosecond)
        };
        let time =
            date.and_hms_nano_opt(self.hour.into(), self.minute.into(), second, nanosecond)?;
        Some(chrono::DateTime::<Utc>::from_naive_utc_and_offset(
            time, Utc,
        ))
    }

    pub(crate) fn from_chrono(dt: &chrono::DateTime<Utc>) -> Self {
        use chrono::{Datelike, Timelike};
        let (second, nanosecond) = if dt.nanosecond() >= 1_000_000_000 {
            (60_u8, dt.nanosecond() - 1_000_000_000)
        } else {
            (dt.second() as u8, dt.nanosecond())
        };
        Self {
            year: dt.year(),
            month: dt.month() as u8,
            day: dt.day() as u8,
            hour: dt.hour() as u8,
            minute: dt.minute() as u8,
            second,
            nanosecond,
        }
    }
}

#[inline]
fn days_from_qty(duration: QttyQuantity) -> Result<Day, TempochStatus> {
    duration
        .convert_to(UnitId::Day)
        .map(|q| Day::new(q.value))
        .ok_or(TempochStatus::InvalidDurationUnit)
}

#[inline]
fn decode_scale(scale_id: i32) -> Result<TempochScaleId, TempochStatus> {
    TempochScaleId::from_raw(scale_id).ok_or(TempochStatus::InvalidScaleId)
}

/// Create a Julian Date from a raw `double`.
#[no_mangle]
pub extern "C" fn tempoch_jd_new(value: f64) -> f64 {
    value
}

/// Return the J2000.0 epoch as a Julian Date (2451545.0).
#[no_mangle]
pub extern "C" fn tempoch_jd_j2000() -> f64 {
    J2000_JD_TT
}

/// Convert a Julian Date to a Modified Julian Date.
#[no_mangle]
pub extern "C" fn tempoch_jd_to_mjd(jd: f64) -> f64 {
    jd_to_scale_value(jd, TempochScaleId::MJD)
}

/// Create a Julian Date from a UTC date-time.
///
/// # Safety
/// `out` must be a valid, writable pointer to `double`.
#[no_mangle]
pub unsafe extern "C" fn tempoch_jd_from_utc(utc: TempochUtc, out: *mut f64) -> TempochStatus {
    catch_panic!(TempochStatus::InternalPanic, {
        if out.is_null() {
            return TempochStatus::NullPointer;
        }
        match utc.into_chrono() {
            Some(dt) => match time_from_utc_value(dt, TempochScaleId::JD) {
                Some(value) => {
                    unsafe { *out = value };
                    TempochStatus::Ok
                }
                None => TempochStatus::UtcConversionFailed,
            },
            None => TempochStatus::UtcConversionFailed,
        }
    })
}

/// Convert a Julian Date to a UTC breakdown.
///
/// # Safety
/// `out` must be a valid, writable pointer to `TempochUtc`.
#[no_mangle]
pub unsafe extern "C" fn tempoch_jd_to_utc(jd: f64, out: *mut TempochUtc) -> TempochStatus {
    catch_panic!(TempochStatus::InternalPanic, {
        if out.is_null() {
            return TempochStatus::NullPointer;
        }
        match time_to_utc_value(jd, TempochScaleId::JD) {
            Some(dt) => {
                unsafe { *out = TempochUtc::from_chrono(&dt) };
                TempochStatus::Ok
            }
            None => TempochStatus::UtcConversionFailed,
        }
    })
}

/// Create a Modified Julian Date from a raw `double`.
#[no_mangle]
pub extern "C" fn tempoch_mjd_new(value: f64) -> f64 {
    value
}

/// Convert a Modified Julian Date to a Julian Date.
#[no_mangle]
pub extern "C" fn tempoch_mjd_to_jd(mjd: f64) -> f64 {
    scale_value_to_jd(mjd, TempochScaleId::MJD)
}

/// Create a Modified Julian Date from a UTC date-time.
///
/// # Safety
/// `out` must be a valid, writable pointer to `double`.
#[no_mangle]
pub unsafe extern "C" fn tempoch_mjd_from_utc(utc: TempochUtc, out: *mut f64) -> TempochStatus {
    catch_panic!(TempochStatus::InternalPanic, {
        if out.is_null() {
            return TempochStatus::NullPointer;
        }
        match utc.into_chrono() {
            Some(dt) => match time_from_utc_value(dt, TempochScaleId::MJD) {
                Some(value) => {
                    unsafe { *out = value };
                    TempochStatus::Ok
                }
                None => TempochStatus::UtcConversionFailed,
            },
            None => TempochStatus::UtcConversionFailed,
        }
    })
}

/// Convert a Modified Julian Date to a UTC breakdown.
///
/// # Safety
/// `out` must be a valid, writable pointer to `TempochUtc`.
#[no_mangle]
pub unsafe extern "C" fn tempoch_mjd_to_utc(mjd: f64, out: *mut TempochUtc) -> TempochStatus {
    catch_panic!(TempochStatus::InternalPanic, {
        if out.is_null() {
            return TempochStatus::NullPointer;
        }
        match time_to_utc_value(mjd, TempochScaleId::MJD) {
            Some(dt) => {
                unsafe { *out = TempochUtc::from_chrono(&dt) };
                TempochStatus::Ok
            }
            None => TempochStatus::UtcConversionFailed,
        }
    })
}

/// Compute the difference between two Julian Dates in days (jd1 − jd2).
#[no_mangle]
pub extern "C" fn tempoch_jd_difference(jd1: f64, jd2: f64) -> f64 {
    time_difference_days_value(jd1, jd2, TempochScaleId::JD)
}

/// Add a duration in days to a Julian Date.
#[no_mangle]
pub extern "C" fn tempoch_jd_add_days(jd: f64, days: f64) -> f64 {
    time_add_days_value(jd, Day::new(days), TempochScaleId::JD)
}

/// Compute the difference between two Modified Julian Dates in days (mjd1 − mjd2).
#[no_mangle]
pub extern "C" fn tempoch_mjd_difference(mjd1: f64, mjd2: f64) -> f64 {
    time_difference_days_value(mjd1, mjd2, TempochScaleId::MJD)
}

/// Add a duration in days to a Modified Julian Date.
#[no_mangle]
pub extern "C" fn tempoch_mjd_add_days(mjd: f64, days: f64) -> f64 {
    time_add_days_value(mjd, Day::new(days), TempochScaleId::MJD)
}

/// Compute Julian centuries since J2000 for a given Julian Date.
#[no_mangle]
pub extern "C" fn tempoch_jd_julian_centuries(jd: f64) -> f64 {
    (jd - J2000_JD_TT) / JULIAN_CENTURY_DAYS
}

/// Compute the difference between two Julian Dates as a `QttyQuantity` in days.
#[no_mangle]
pub extern "C" fn tempoch_jd_difference_qty(jd1: f64, jd2: f64) -> QttyQuantity {
    QttyQuantity::new(tempoch_jd_difference(jd1, jd2), UnitId::Day)
}

/// Add a `QttyQuantity` duration (time-compatible) to a Julian Date.
///
/// Returns `InvalidDurationUnit` if the quantity cannot be converted to days.
///
/// # Safety
/// `out` must be a valid, writable pointer to `double`.
#[no_mangle]
pub unsafe extern "C" fn tempoch_jd_add_qty(
    jd: f64,
    duration: QttyQuantity,
    out: *mut f64,
) -> TempochStatus {
    catch_panic!(TempochStatus::InternalPanic, {
        if out.is_null() {
            return TempochStatus::NullPointer;
        }
        let days = match days_from_qty(duration) {
            Ok(days) => days,
            Err(status) => return status,
        };
        unsafe { *out = time_add_days_value(jd, days, TempochScaleId::JD) };
        TempochStatus::Ok
    })
}

/// Compute the difference between two Modified Julian Dates as a `QttyQuantity` in days.
#[no_mangle]
pub extern "C" fn tempoch_mjd_difference_qty(mjd1: f64, mjd2: f64) -> QttyQuantity {
    QttyQuantity::new(tempoch_mjd_difference(mjd1, mjd2), UnitId::Day)
}

/// Add a `QttyQuantity` duration (time-compatible) to a Modified Julian Date.
///
/// Returns `InvalidDurationUnit` if the quantity cannot be converted to days.
///
/// # Safety
/// `out` must be a valid, writable pointer to `double`.
#[no_mangle]
pub unsafe extern "C" fn tempoch_mjd_add_qty(
    mjd: f64,
    duration: QttyQuantity,
    out: *mut f64,
) -> TempochStatus {
    catch_panic!(TempochStatus::InternalPanic, {
        if out.is_null() {
            return TempochStatus::NullPointer;
        }
        let days = match days_from_qty(duration) {
            Ok(days) => days,
            Err(status) => return status,
        };
        unsafe { *out = time_add_days_value(mjd, days, TempochScaleId::MJD) };
        TempochStatus::Ok
    })
}

/// Compute Julian centuries since J2000 as a `QttyQuantity`.
#[no_mangle]
pub extern "C" fn tempoch_jd_julian_centuries_qty(jd: f64) -> QttyQuantity {
    QttyQuantity::new(tempoch_jd_julian_centuries(jd), UnitId::JulianCentury)
}

/// Convert a Julian Date (TT) to TDB.
#[no_mangle]
pub extern "C" fn tempoch_jd_to_tdb(jd: f64) -> f64 {
    jd_to_scale_value(jd, TempochScaleId::TDB)
}

/// Convert TDB back to Julian Date (TT).
#[no_mangle]
pub extern "C" fn tempoch_tdb_to_jd(tdb: f64) -> f64 {
    scale_value_to_jd(tdb, TempochScaleId::TDB)
}

/// Convert a Julian Date (TT) to TT (identity).
#[no_mangle]
pub extern "C" fn tempoch_jd_to_tt(jd: f64) -> f64 {
    jd_to_scale_value(jd, TempochScaleId::TT)
}

/// Convert TT back to Julian Date (TT).
#[no_mangle]
pub extern "C" fn tempoch_tt_to_jd(tt: f64) -> f64 {
    scale_value_to_jd(tt, TempochScaleId::TT)
}

/// Convert a Julian Date (TT) to TAI.
#[no_mangle]
pub extern "C" fn tempoch_jd_to_tai(jd: f64) -> f64 {
    jd_to_scale_value(jd, TempochScaleId::TAI)
}

/// Convert TAI back to Julian Date (TT).
#[no_mangle]
pub extern "C" fn tempoch_tai_to_jd(tai: f64) -> f64 {
    scale_value_to_jd(tai, TempochScaleId::TAI)
}

/// Convert a Julian Date (TT) to TCG.
#[no_mangle]
pub extern "C" fn tempoch_jd_to_tcg(jd: f64) -> f64 {
    jd_to_scale_value(jd, TempochScaleId::TCG)
}

/// Convert TCG back to Julian Date (TT).
#[no_mangle]
pub extern "C" fn tempoch_tcg_to_jd(tcg: f64) -> f64 {
    scale_value_to_jd(tcg, TempochScaleId::TCG)
}

/// Convert a Julian Date (TT) to TCB.
#[no_mangle]
pub extern "C" fn tempoch_jd_to_tcb(jd: f64) -> f64 {
    jd_to_scale_value(jd, TempochScaleId::TCB)
}

/// Convert TCB back to Julian Date (TT).
#[no_mangle]
pub extern "C" fn tempoch_tcb_to_jd(tcb: f64) -> f64 {
    scale_value_to_jd(tcb, TempochScaleId::TCB)
}

/// Convert a Julian Date (TT) to GPS Time.
#[no_mangle]
pub extern "C" fn tempoch_jd_to_gps(jd: f64) -> f64 {
    jd_to_scale_value(jd, TempochScaleId::GPS)
}

/// Convert GPS Time back to Julian Date (TT).
#[no_mangle]
pub extern "C" fn tempoch_gps_to_jd(gps: f64) -> f64 {
    scale_value_to_jd(gps, TempochScaleId::GPS)
}

/// Convert a Julian Date (TT) to Universal Time UT1.
#[no_mangle]
pub extern "C" fn tempoch_jd_to_ut(jd: f64) -> f64 {
    jd_to_scale_value(jd, TempochScaleId::UT)
}

/// Convert Universal Time UT1 back to Julian Date (TT).
#[no_mangle]
pub extern "C" fn tempoch_ut_to_jd(ut: f64) -> f64 {
    scale_value_to_jd(ut, TempochScaleId::UT)
}

/// Convert a Julian Date (TT) to Julian Ephemeris Date.
#[no_mangle]
pub extern "C" fn tempoch_jd_to_jde(jd: f64) -> f64 {
    jd_to_scale_value(jd, TempochScaleId::JDE)
}

/// Convert Julian Ephemeris Date back to Julian Date (TT).
#[no_mangle]
pub extern "C" fn tempoch_jde_to_jd(jde: f64) -> f64 {
    scale_value_to_jd(jde, TempochScaleId::JDE)
}

/// Convert a Julian Date (TT) to Unix time in **seconds** since 1970-01-01T00:00:00 UTC.
///
/// The result is a standard Unix timestamp suitable for passing to C `gmtime()`,
/// Python `datetime.fromtimestamp()`, etc. Internally the conversion routes
/// through the compiled UTC-TAI history.
#[no_mangle]
pub extern "C" fn tempoch_jd_to_unix(jd: f64) -> f64 {
    jd_to_scale_value(jd, TempochScaleId::UnixTime)
}

/// Convert Unix time in **seconds** since 1970-01-01T00:00:00 UTC back to Julian Date (TT).
///
/// Accepts a standard Unix timestamp (seconds, not days). The conversion
/// uses the compiled UTC-TAI history for leap-second handling.
#[no_mangle]
pub extern "C" fn tempoch_unix_to_jd(unix: f64) -> f64 {
    scale_value_to_jd(unix, TempochScaleId::UnixTime)
}

/// Create a Unix timestamp from seconds since 1970-01-01T00:00:00 UTC.
///
/// This is a convenience identity for the C ABI: the returned `double` is
/// the same value, confirming that the FFI Unix convention is **seconds**.
/// Use [`tempoch_unix_to_jd`] when you need the corresponding Julian Date.
#[no_mangle]
pub extern "C" fn tempoch_unix_from_seconds(seconds: f64) -> f64 {
    seconds
}

/// Extract the Unix timestamp in seconds from a value previously obtained
/// via [`tempoch_jd_to_unix`] or [`tempoch_unix_from_seconds`].
///
/// This is also a convenience identity confirming the seconds convention.
#[no_mangle]
pub extern "C" fn tempoch_unix_to_seconds(unix: f64) -> f64 {
    unix
}

/// Return ΔT = TT − UT1 in seconds for a given Julian Date.
#[no_mangle]
pub extern "C" fn tempoch_delta_t_seconds(jd: f64) -> f64 {
    let ctx = TimeContext::new();
    let tt = match Time::<TT, JulianDays>::from_julian_days(Day::new(jd)) {
        Ok(time) => time.repr(),
        Err(_) => return f64::NAN,
    };
    match tt.to_with::<UT1>(&ctx) {
        Ok(ut1) => (tt.si_seconds() - ut1.si_seconds()).erase_unit_raw(),
        Err(_) => f64::NAN,
    }
}

/// Convert a `double` time value from one scale to another.
///
/// # Safety
/// `out` must be a valid, writable pointer to `double`.
#[no_mangle]
pub unsafe extern "C" fn tempoch_time_convert(
    value: f64,
    from_scale_id: i32,
    to_scale_id: i32,
    out: *mut f64,
) -> TempochStatus {
    catch_panic!(TempochStatus::InternalPanic, {
        if out.is_null() {
            return TempochStatus::NullPointer;
        }
        let from = match decode_scale(from_scale_id) {
            Ok(scale) => scale,
            Err(status) => return status,
        };
        let to = match decode_scale(to_scale_id) {
            Ok(scale) => scale,
            Err(status) => return status,
        };
        let jd = scale_value_to_jd(value, from);
        unsafe { *out = jd_to_scale_value(jd, to) };
        TempochStatus::Ok
    })
}

/// Convert a UTC date-time to a value in any supported scale.
///
/// # Safety
/// `out` must be a valid, writable pointer to `double`.
#[no_mangle]
pub unsafe extern "C" fn tempoch_time_from_utc(
    utc: TempochUtc,
    scale_id: i32,
    out: *mut f64,
) -> TempochStatus {
    catch_panic!(TempochStatus::InternalPanic, {
        if out.is_null() {
            return TempochStatus::NullPointer;
        }
        let scale = match decode_scale(scale_id) {
            Ok(scale) => scale,
            Err(status) => return status,
        };
        match utc.into_chrono() {
            Some(dt) => match time_from_utc_value(dt, scale) {
                Some(value) => {
                    unsafe { *out = value };
                    TempochStatus::Ok
                }
                None => TempochStatus::UtcConversionFailed,
            },
            None => TempochStatus::UtcConversionFailed,
        }
    })
}

/// Convert a time value in any supported scale to UTC.
///
/// # Safety
/// `out` must be a valid, writable pointer to `TempochUtc`.
#[no_mangle]
pub unsafe extern "C" fn tempoch_time_to_utc(
    value: f64,
    scale_id: i32,
    out: *mut TempochUtc,
) -> TempochStatus {
    catch_panic!(TempochStatus::InternalPanic, {
        if out.is_null() {
            return TempochStatus::NullPointer;
        }
        let scale = match decode_scale(scale_id) {
            Ok(scale) => scale,
            Err(status) => return status,
        };
        match time_to_utc_value(value, scale) {
            Some(dt) => {
                unsafe { *out = TempochUtc::from_chrono(&dt) };
                TempochStatus::Ok
            }
            None => TempochStatus::UtcConversionFailed,
        }
    })
}

/// Compute a same-scale duration in days.
///
/// # Safety
/// `out` must be a valid, writable pointer to `double`.
#[no_mangle]
pub unsafe extern "C" fn tempoch_time_difference_days(
    lhs: f64,
    rhs: f64,
    scale_id: i32,
    out: *mut f64,
) -> TempochStatus {
    catch_panic!(TempochStatus::InternalPanic, {
        if out.is_null() {
            return TempochStatus::NullPointer;
        }
        let scale = match decode_scale(scale_id) {
            Ok(scale) => scale,
            Err(status) => return status,
        };
        unsafe { *out = time_difference_days_value(lhs, rhs, scale) };
        TempochStatus::Ok
    })
}

/// Compute a same-scale duration as a day-valued `QttyQuantity`.
///
/// # Safety
/// `out` must be a valid, writable pointer to `qtty_quantity_t`.
#[no_mangle]
pub unsafe extern "C" fn tempoch_time_difference_qty(
    lhs: f64,
    rhs: f64,
    scale_id: i32,
    out: *mut QttyQuantity,
) -> TempochStatus {
    catch_panic!(TempochStatus::InternalPanic, {
        if out.is_null() {
            return TempochStatus::NullPointer;
        }
        let scale = match decode_scale(scale_id) {
            Ok(scale) => scale,
            Err(status) => return status,
        };
        let days = time_difference_days_value(lhs, rhs, scale);
        unsafe { *out = QttyQuantity::new(days, UnitId::Day) };
        TempochStatus::Ok
    })
}

/// Add a same-scale duration in days.
///
/// # Safety
/// `out` must be a valid, writable pointer to `double`.
#[no_mangle]
pub unsafe extern "C" fn tempoch_time_add_days(
    value: f64,
    scale_id: i32,
    days: f64,
    out: *mut f64,
) -> TempochStatus {
    catch_panic!(TempochStatus::InternalPanic, {
        if out.is_null() {
            return TempochStatus::NullPointer;
        }
        let scale = match decode_scale(scale_id) {
            Ok(scale) => scale,
            Err(status) => return status,
        };
        unsafe { *out = time_add_days_value(value, Day::new(days), scale) };
        TempochStatus::Ok
    })
}

/// Add a same-scale duration from a time-compatible `QttyQuantity`.
///
/// # Safety
/// `out` must be a valid, writable pointer to `double`.
#[no_mangle]
pub unsafe extern "C" fn tempoch_time_add_qty(
    value: f64,
    scale_id: i32,
    duration: QttyQuantity,
    out: *mut f64,
) -> TempochStatus {
    catch_panic!(TempochStatus::InternalPanic, {
        if out.is_null() {
            return TempochStatus::NullPointer;
        }
        let scale = match decode_scale(scale_id) {
            Ok(scale) => scale,
            Err(status) => return status,
        };
        let days = match days_from_qty(duration) {
            Ok(days) => days,
            Err(status) => return status,
        };
        unsafe { *out = time_add_days_value(value, days, scale) };
        TempochStatus::Ok
    })
}

/// Convert a Julian Date to any scale, writing the result into `out`.
///
/// # Safety
/// `out` must be a valid, writable pointer to `double`.
#[no_mangle]
pub unsafe extern "C" fn tempoch_jd_to_scale(
    jd: f64,
    scale_id: i32,
    out: *mut f64,
) -> TempochStatus {
    unsafe { tempoch_time_convert(jd, TempochScaleId::JD as i32, scale_id, out) }
}

/// Convert a value in any scale to a Julian Date, writing the result into `out`.
///
/// # Safety
/// `out` must be a valid, writable pointer to `double`.
#[no_mangle]
pub unsafe extern "C" fn tempoch_scale_to_jd(
    value: f64,
    scale_id: i32,
    out: *mut f64,
) -> TempochStatus {
    unsafe { tempoch_time_convert(value, scale_id, TempochScaleId::JD as i32, out) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr;

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

    #[test]
    fn into_chrono_invalid_nanoseconds_returns_none() {
        let utc = TempochUtc {
            nanosecond: 1_500_000_000,
            ..utc_j2000()
        };
        assert!(utc.into_chrono().is_none());
    }

    #[test]
    fn into_chrono_accepts_leap_second() {
        let utc = TempochUtc {
            year: 2016,
            month: 12,
            day: 31,
            hour: 23,
            minute: 59,
            second: 60,
            nanosecond: 500_000_000,
        };
        let chrono = utc.into_chrono().expect("valid leap second encoding");
        assert_eq!(chrono.timestamp(), 1_483_228_799);
        assert_eq!(chrono.timestamp_subsec_nanos(), 1_500_000_000);
    }

    #[test]
    fn jd_new_carries_value() {
        assert_eq!(tempoch_jd_new(2_451_545.0), 2_451_545.0);
    }

    #[test]
    fn jd_j2000_value() {
        assert_eq!(tempoch_jd_j2000(), 2_451_545.0);
    }

    #[test]
    fn jd_to_mjd_roundtrip() {
        let jd = 2_451_545.0;
        let mjd = tempoch_jd_to_mjd(jd);
        assert!((mjd - 51_544.5).abs() < 1e-10);
        let back = tempoch_mjd_to_jd(mjd);
        assert!((back - jd).abs() < 1e-10);
    }

    #[test]
    fn jd_from_utc_null_pointer_returns_error() {
        let status = unsafe { tempoch_jd_from_utc(utc_j2000(), ptr::null_mut()) };
        assert_eq!(status, TempochStatus::NullPointer);
    }

    #[test]
    fn jd_from_utc_invalid_nanoseconds_returns_conversion_failed() {
        let bad = TempochUtc {
            nanosecond: 2_000_000_000,
            ..utc_j2000()
        };
        let mut out = 0.0;
        let status = unsafe { tempoch_jd_from_utc(bad, &mut out) };
        assert_eq!(status, TempochStatus::UtcConversionFailed);
    }

    #[test]
    fn jd_from_utc_pre_1961_returns_conversion_failed() {
        let before_history = TempochUtc {
            year: 1960,
            month: 12,
            day: 31,
            hour: 23,
            minute: 59,
            second: 59,
            nanosecond: 0,
        };
        let mut out = 0.0;
        let status = unsafe { tempoch_jd_from_utc(before_history, &mut out) };
        assert_eq!(status, TempochStatus::UtcConversionFailed);
    }

    #[test]
    fn jd_from_utc_success() {
        let mut out = 0.0;
        let status = unsafe { tempoch_jd_from_utc(utc_j2000(), &mut out) };
        assert_eq!(status, TempochStatus::Ok);
        assert!((out - 2_451_545.0).abs() < 0.01);
    }

    #[test]
    fn jd_to_utc_null_pointer_returns_error() {
        let status = unsafe { tempoch_jd_to_utc(2_451_545.0, ptr::null_mut()) };
        assert_eq!(status, TempochStatus::NullPointer);
    }

    #[test]
    fn jd_to_utc_success() {
        let mut out = TempochUtc {
            year: 0,
            month: 0,
            day: 0,
            hour: 0,
            minute: 0,
            second: 0,
            nanosecond: 0,
        };
        let status = unsafe { tempoch_jd_to_utc(2_451_545.0, &mut out) };
        assert_eq!(status, TempochStatus::Ok);
        assert_eq!(out.year, 2000);
        assert_eq!(out.month, 1);
        assert_eq!(out.day, 1);
    }

    #[test]
    fn jd_utc_leap_second_roundtrip() {
        let leap = TempochUtc {
            year: 2016,
            month: 12,
            day: 31,
            hour: 23,
            minute: 59,
            second: 60,
            nanosecond: 500_000_000,
        };
        let mut jd = 0.0;
        let from_status = unsafe { tempoch_jd_from_utc(leap, &mut jd) };
        assert_eq!(from_status, TempochStatus::Ok);

        let mut back = TempochUtc {
            year: 0,
            month: 0,
            day: 0,
            hour: 0,
            minute: 0,
            second: 0,
            nanosecond: 0,
        };
        let to_status = unsafe { tempoch_jd_to_utc(jd, &mut back) };
        assert_eq!(to_status, TempochStatus::Ok);
        assert_eq!(back.year, 2016);
        assert_eq!(back.month, 12);
        assert_eq!(back.day, 31);
        assert_eq!(back.hour, 23);
        assert_eq!(back.minute, 59);
        assert_eq!(back.second, 60);
        assert!((back.nanosecond as i64 - 500_000_000).abs() < 50_000);
    }

    #[test]
    fn mjd_from_utc_success() {
        let mut out = 0.0;
        let status = unsafe { tempoch_mjd_from_utc(utc_j2000(), &mut out) };
        assert_eq!(status, TempochStatus::Ok);
        assert!((out - 51_544.5).abs() < 0.01);
    }

    #[test]
    fn mjd_to_utc_success() {
        let mut out = TempochUtc {
            year: 0,
            month: 0,
            day: 0,
            hour: 0,
            minute: 0,
            second: 0,
            nanosecond: 0,
        };
        let status = unsafe { tempoch_mjd_to_utc(51_544.5, &mut out) };
        assert_eq!(status, TempochStatus::Ok);
        assert_eq!(out.year, 2000);
        assert_eq!(out.month, 1);
        assert_eq!(out.day, 1);
    }

    #[test]
    fn jd_add_days_is_correct() {
        assert!((tempoch_jd_add_days(2_451_545.0, 10.0) - 2_451_555.0).abs() < 1e-12);
    }

    #[test]
    fn jd_difference_is_correct() {
        assert!((tempoch_jd_difference(2_451_545.0, 2_451_544.0) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn mjd_difference_is_correct() {
        assert!((tempoch_mjd_difference(51_544.5, 51_543.5) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn jd_add_qty_hours() {
        let mut out = 0.0;
        let status = unsafe {
            tempoch_jd_add_qty(2_451_545.0, QttyQuantity::new(24.0, UnitId::Hour), &mut out)
        };
        assert_eq!(status, TempochStatus::Ok);
        assert!((out - 2_451_546.0).abs() < 1e-10);
    }

    #[test]
    fn jd_add_qty_invalid_unit_returns_error() {
        let mut out = 0.0;
        let status = unsafe {
            tempoch_jd_add_qty(2_451_545.0, QttyQuantity::new(1.0, UnitId::Meter), &mut out)
        };
        assert_eq!(status, TempochStatus::InvalidDurationUnit);
    }

    #[test]
    fn jd_add_qty_null_returns_error() {
        let status = unsafe {
            tempoch_jd_add_qty(
                2_451_545.0,
                QttyQuantity::new(1.0, UnitId::Day),
                ptr::null_mut(),
            )
        };
        assert_eq!(status, TempochStatus::NullPointer);
    }

    #[test]
    fn jd_tdb_roundtrip() {
        let jd = 2_451_545.0;
        let tdb = tempoch_jd_to_tdb(jd);
        let back = tempoch_tdb_to_jd(tdb);
        assert!((back - jd).abs() < 1e-6);
    }

    #[test]
    fn jd_tai_roundtrip() {
        let jd = 2_451_545.0;
        let tai = tempoch_jd_to_tai(jd);
        let back = tempoch_tai_to_jd(tai);
        assert!((back - jd).abs() < 1e-10);
    }

    #[test]
    fn jd_tcg_roundtrip() {
        let jd = 2_451_545.0;
        let tcg = tempoch_jd_to_tcg(jd);
        let back = tempoch_tcg_to_jd(tcg);
        assert!((back - jd).abs() < 1e-6);
    }

    #[test]
    fn jd_tcb_roundtrip() {
        let jd = 2_451_545.0;
        let tcb = tempoch_jd_to_tcb(jd);
        let back = tempoch_tcb_to_jd(tcb);
        assert!((back - jd).abs() < 1e-6);
    }

    #[test]
    fn jd_gps_roundtrip() {
        let jd = 2_451_545.0;
        let gps = tempoch_jd_to_gps(jd);
        let back = tempoch_gps_to_jd(gps);
        assert!((back - jd).abs() < 1e-10);
    }

    #[test]
    fn jd_ut_roundtrip() {
        let jd = 2_451_545.0;
        let ut = tempoch_jd_to_ut(jd);
        let back = tempoch_ut_to_jd(ut);
        assert!((back - jd).abs() < 1e-6);
    }

    #[test]
    fn jd_jde_roundtrip() {
        let jd = 2_451_545.0;
        let jde = tempoch_jd_to_jde(jd);
        let back = tempoch_jde_to_jd(jde);
        assert!((back - jd).abs() < 1e-12);
    }

    #[test]
    fn jd_unix_roundtrip() {
        let jd = 2_451_545.0;
        let unix = tempoch_jd_to_unix(jd);
        let back = tempoch_unix_to_jd(unix);
        assert!((back - jd).abs() < 1e-10);
    }

    #[test]
    fn unix_epoch_is_zero_seconds() {
        let mut out = 1.0;
        let status = unsafe {
            tempoch_time_from_utc(
                TempochUtc {
                    year: 1970,
                    month: 1,
                    day: 1,
                    hour: 0,
                    minute: 0,
                    second: 0,
                    nanosecond: 0,
                },
                TempochScaleId::UnixTime as i32,
                &mut out,
            )
        };
        assert_eq!(status, TempochStatus::Ok);
        assert!(out.abs() < 1e-9, "unix={out}");
    }

    #[test]
    fn jd_to_scale_valid_ids() {
        let jd = 2_451_545.0;
        for scale_id in 0..=10i32 {
            let mut out = 0.0;
            let status = unsafe { tempoch_jd_to_scale(jd, scale_id, &mut out) };
            assert_eq!(status, TempochStatus::Ok, "scale_id {scale_id}");
            assert!(out.is_finite());
        }
    }

    #[test]
    fn jd_to_scale_invalid_id() {
        let mut out = 0.0;
        let status = unsafe { tempoch_jd_to_scale(2_451_545.0, -1, &mut out) };
        assert_eq!(status, TempochStatus::InvalidScaleId);
    }

    #[test]
    fn scale_to_jd_valid_roundtrip() {
        let jd_orig = 2_451_545.0;
        for scale_id in 0..=10i32 {
            let mut scale_val = 0.0;
            let s1 = unsafe { tempoch_jd_to_scale(jd_orig, scale_id, &mut scale_val) };
            assert_eq!(s1, TempochStatus::Ok);
            let mut out = 0.0;
            let s2 = unsafe { tempoch_scale_to_jd(scale_val, scale_id, &mut out) };
            assert_eq!(s2, TempochStatus::Ok);
            assert!((out - jd_orig).abs() < 1e-6, "scale_id={scale_id}");
        }
    }

    #[test]
    fn time_convert_roundtrip() {
        let mut unix = 0.0;
        let s1 = unsafe {
            tempoch_time_convert(
                2_451_545.0,
                TempochScaleId::JD as i32,
                TempochScaleId::UnixTime as i32,
                &mut unix,
            )
        };
        assert_eq!(s1, TempochStatus::Ok);
        let mut jd = 0.0;
        let s2 = unsafe {
            tempoch_time_convert(
                unix,
                TempochScaleId::UnixTime as i32,
                TempochScaleId::JD as i32,
                &mut jd,
            )
        };
        assert_eq!(s2, TempochStatus::Ok);
        assert!((jd - 2_451_545.0).abs() < 1e-6);
    }

    #[test]
    fn time_from_utc_and_to_utc_generic() {
        let mut unix = 0.0;
        let s1 = unsafe {
            tempoch_time_from_utc(utc_j2000(), TempochScaleId::UnixTime as i32, &mut unix)
        };
        assert_eq!(s1, TempochStatus::Ok);
        let mut utc = TempochUtc {
            year: 0,
            month: 0,
            day: 0,
            hour: 0,
            minute: 0,
            second: 0,
            nanosecond: 0,
        };
        let s2 = unsafe { tempoch_time_to_utc(unix, TempochScaleId::UnixTime as i32, &mut utc) };
        assert_eq!(s2, TempochStatus::Ok);
        assert_eq!(utc.year, 2000);
        assert_eq!(utc.month, 1);
        assert_eq!(utc.day, 1);
    }

    #[test]
    fn time_add_qty_generic() {
        let mut out = 0.0;
        let status = unsafe {
            tempoch_time_add_qty(
                2_451_545.0,
                TempochScaleId::JD as i32,
                QttyQuantity::new(24.0, UnitId::Hour),
                &mut out,
            )
        };
        assert_eq!(status, TempochStatus::Ok);
        assert!((out - 2_451_546.0).abs() < 1e-10);
    }

    #[test]
    fn time_difference_qty_generic() {
        let mut out = QttyQuantity::new(0.0, UnitId::Day);
        let status = unsafe {
            tempoch_time_difference_qty(
                2_451_546.0,
                2_451_545.0,
                TempochScaleId::JD as i32,
                &mut out,
            )
        };
        assert_eq!(status, TempochStatus::Ok);
        assert_eq!(out.unit, UnitId::Day as u32);
        assert!((out.value - 1.0).abs() < 1e-12);
    }

    #[test]
    fn time_difference_days_generic() {
        let mut out = 0.0;
        let status = unsafe {
            tempoch_time_difference_days(
                2_451_546.0,
                2_451_545.0,
                TempochScaleId::JD as i32,
                &mut out,
            )
        };
        assert_eq!(status, TempochStatus::Ok);
        assert!((out - 1.0).abs() < 1e-12);
    }

    #[test]
    fn time_add_days_generic() {
        let mut out = 0.0;
        let status =
            unsafe { tempoch_time_add_days(2_451_545.0, TempochScaleId::JD as i32, 1.5, &mut out) };
        assert_eq!(status, TempochStatus::Ok);
        assert!((out - 2_451_546.5).abs() < 1e-12);
    }

    #[test]
    fn time_generic_invalid_scale() {
        let mut out = 0.0;
        let status = unsafe { tempoch_time_add_days(1.0, i32::MAX, 1.0, &mut out) };
        assert_eq!(status, TempochStatus::InvalidScaleId);
    }

    #[test]
    fn time_generic_null_out() {
        let status = unsafe {
            tempoch_time_difference_days(1.0, 0.0, TempochScaleId::JD as i32, ptr::null_mut())
        };
        assert_eq!(status, TempochStatus::NullPointer);
    }
}
