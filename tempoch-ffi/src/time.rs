// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! FFI bindings for tempoch time types: JulianDate, ModifiedJulianDate,
//! and UTC conversions.

use crate::catch_panic;
use crate::error::TempochStatus;
use chrono::{DateTime, NaiveDate, Utc};
use qtty::Days;
use qtty_ffi::{QttyQuantity, UnitId};
use tempoch::{JulianDate, ModifiedJulianDate, TimeInstant, JD, MJD};

// ═══════════════════════════════════════════════════════════════════════════
// C-repr types
// ═══════════════════════════════════════════════════════════════════════════

/// UTC date-time breakdown for C interop.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TempochUtc {
    pub year: i32,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub nanosecond: u32,
}

impl TempochUtc {
    fn into_chrono(self) -> Option<DateTime<Utc>> {
        let date = NaiveDate::from_ymd_opt(self.year, self.month as u32, self.day as u32)?;
        let time = date.and_hms_nano_opt(
            self.hour.into(),
            self.minute.into(),
            self.second.into(),
            self.nanosecond,
        )?;
        Some(DateTime::<Utc>::from_naive_utc_and_offset(time, Utc))
    }

    fn from_chrono(dt: &DateTime<Utc>) -> Self {
        use chrono::{Datelike, Timelike};
        Self {
            year: dt.year(),
            month: dt.month() as u8,
            day: dt.day() as u8,
            hour: dt.hour() as u8,
            minute: dt.minute() as u8,
            second: dt.second() as u8,
            nanosecond: dt.nanosecond(),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Julian Date
// ═══════════════════════════════════════════════════════════════════════════

/// Create a Julian Date from a raw f64 value.
#[no_mangle]
pub extern "C" fn tempoch_jd_new(value: f64) -> f64 {
    value // JD is just a f64 — identity, but provides a typed entry point
}

/// Return the J2000.0 epoch as a Julian Date (2451545.0).
#[no_mangle]
pub extern "C" fn tempoch_jd_j2000() -> f64 {
    JulianDate::J2000.value()
}

/// Convert a Julian Date to a Modified Julian Date.
#[no_mangle]
pub extern "C" fn tempoch_jd_to_mjd(jd: f64) -> f64 {
    JulianDate::new(jd).to::<MJD>().value()
}

/// Create a Julian Date from a UTC date-time.
///
/// # Safety
/// `out` must be a valid, writable pointer to `f64`.
#[no_mangle]
pub unsafe extern "C" fn tempoch_jd_from_utc(utc: TempochUtc, out: *mut f64) -> TempochStatus {
    catch_panic!(TempochStatus::UtcConversionFailed, {
        if out.is_null() {
            return TempochStatus::NullPointer;
        }
        match utc.into_chrono() {
            Some(dt) => {
                let jd = JulianDate::from_utc(dt);
                unsafe { *out = jd.value() };
                TempochStatus::Ok
            }
            None => TempochStatus::UtcConversionFailed,
        }
    })
}

/// Convert a Julian Date to UTC. Returns Ok on success,
/// UtcConversionFailed if the date is out of representable range.
///
/// # Safety
/// `out` must be a valid, writable pointer to `TempochUtc`.
#[no_mangle]
pub unsafe extern "C" fn tempoch_jd_to_utc(jd: f64, out: *mut TempochUtc) -> TempochStatus {
    catch_panic!(TempochStatus::UtcConversionFailed, {
        if out.is_null() {
            return TempochStatus::NullPointer;
        }
        match JulianDate::new(jd).to_utc() {
            Some(dt) => {
                unsafe { *out = TempochUtc::from_chrono(&dt) };
                TempochStatus::Ok
            }
            None => TempochStatus::UtcConversionFailed,
        }
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// Modified Julian Date
// ═══════════════════════════════════════════════════════════════════════════

/// Create a Modified Julian Date from a raw f64 value.
#[no_mangle]
pub extern "C" fn tempoch_mjd_new(value: f64) -> f64 {
    value
}

/// Convert a Modified Julian Date to a Julian Date.
#[no_mangle]
pub extern "C" fn tempoch_mjd_to_jd(mjd: f64) -> f64 {
    ModifiedJulianDate::new(mjd).to::<JD>().value()
}

/// Create a Modified Julian Date from a UTC date-time.
///
/// # Safety
/// `out` must be a valid, writable pointer to `f64`.
#[no_mangle]
pub unsafe extern "C" fn tempoch_mjd_from_utc(utc: TempochUtc, out: *mut f64) -> TempochStatus {
    catch_panic!(TempochStatus::UtcConversionFailed, {
        if out.is_null() {
            return TempochStatus::NullPointer;
        }
        match utc.into_chrono() {
            Some(dt) => {
                let mjd = ModifiedJulianDate::from_utc(dt);
                unsafe { *out = mjd.value() };
                TempochStatus::Ok
            }
            None => TempochStatus::UtcConversionFailed,
        }
    })
}

/// Convert a Modified Julian Date to UTC.
///
/// # Safety
/// `out` must be a valid, writable pointer to `TempochUtc`.
#[no_mangle]
pub unsafe extern "C" fn tempoch_mjd_to_utc(mjd: f64, out: *mut TempochUtc) -> TempochStatus {
    catch_panic!(TempochStatus::UtcConversionFailed, {
        if out.is_null() {
            return TempochStatus::NullPointer;
        }
        match ModifiedJulianDate::new(mjd).to_utc() {
            Some(dt) => {
                unsafe { *out = TempochUtc::from_chrono(&dt) };
                TempochStatus::Ok
            }
            None => TempochStatus::UtcConversionFailed,
        }
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// Duration / Difference (raw f64 — backward compatible)
// ═══════════════════════════════════════════════════════════════════════════

/// Compute the difference between two Julian Dates in days.
#[no_mangle]
pub extern "C" fn tempoch_jd_difference(jd1: f64, jd2: f64) -> f64 {
    let t1 = JulianDate::new(jd1);
    let t2 = JulianDate::new(jd2);
    t1.difference(&t2).value()
}

/// Add a duration in days to a Julian Date.
#[no_mangle]
pub extern "C" fn tempoch_jd_add_days(jd: f64, days: f64) -> f64 {
    JulianDate::new(jd).add_duration(Days::new(days)).value()
}

/// Compute the difference between two Modified Julian Dates in days.
#[no_mangle]
pub extern "C" fn tempoch_mjd_difference(mjd1: f64, mjd2: f64) -> f64 {
    let t1 = ModifiedJulianDate::new(mjd1);
    let t2 = ModifiedJulianDate::new(mjd2);
    t1.difference(&t2).value()
}

/// Add a duration in days to a Modified Julian Date.
#[no_mangle]
pub extern "C" fn tempoch_mjd_add_days(mjd: f64, days: f64) -> f64 {
    ModifiedJulianDate::new(mjd)
        .add_duration(Days::new(days))
        .value()
}

/// Compute Julian centuries since J2000 for a given Julian Date.
#[no_mangle]
pub extern "C" fn tempoch_jd_julian_centuries(jd: f64) -> f64 {
    JulianDate::new(jd).julian_centuries().value()
}

// ═══════════════════════════════════════════════════════════════════════════
// Duration / Difference (QttyQuantity — typed)
// ═══════════════════════════════════════════════════════════════════════════
// These functions return `QttyQuantity` values with proper unit metadata,
// enabling type-safe conversions via the qtty-ffi API.

/// Compute the difference between two Julian Dates as a `QttyQuantity` in days.
#[no_mangle]
pub extern "C" fn tempoch_jd_difference_qty(jd1: f64, jd2: f64) -> QttyQuantity {
    let t1 = JulianDate::new(jd1);
    let t2 = JulianDate::new(jd2);
    QttyQuantity::new(t1.difference(&t2).value(), UnitId::Day)
}

/// Add a `QttyQuantity` duration (must be time-compatible) to a Julian Date.
/// The quantity is converted to days internally.
///
/// # Safety
/// `out` must be a valid, writable pointer to `f64`.
#[no_mangle]
pub unsafe extern "C" fn tempoch_jd_add_qty(
    jd: f64,
    duration: QttyQuantity,
    out: *mut f64,
) -> TempochStatus {
    catch_panic!(TempochStatus::UtcConversionFailed, {
        if out.is_null() {
            return TempochStatus::NullPointer;
        }
        // Convert quantity to days via qtty-ffi registry
        let days_val = match duration.convert_to(UnitId::Day) {
            Some(q) => q.value,
            None => return TempochStatus::UtcConversionFailed,
        };
        let result = JulianDate::new(jd)
            .add_duration(Days::new(days_val))
            .value();
        unsafe { *out = result };
        TempochStatus::Ok
    })
}

/// Compute the difference between two Modified Julian Dates as a `QttyQuantity` in days.
#[no_mangle]
pub extern "C" fn tempoch_mjd_difference_qty(mjd1: f64, mjd2: f64) -> QttyQuantity {
    let t1 = ModifiedJulianDate::new(mjd1);
    let t2 = ModifiedJulianDate::new(mjd2);
    QttyQuantity::new(t1.difference(&t2).value(), UnitId::Day)
}

/// Add a `QttyQuantity` duration (must be time-compatible) to a Modified Julian Date.
/// The quantity is converted to days internally.
///
/// # Safety
/// `out` must be a valid, writable pointer to `f64`.
#[no_mangle]
pub unsafe extern "C" fn tempoch_mjd_add_qty(
    mjd: f64,
    duration: QttyQuantity,
    out: *mut f64,
) -> TempochStatus {
    catch_panic!(TempochStatus::UtcConversionFailed, {
        if out.is_null() {
            return TempochStatus::NullPointer;
        }
        let days_val = match duration.convert_to(UnitId::Day) {
            Some(q) => q.value,
            None => return TempochStatus::UtcConversionFailed,
        };
        let result = ModifiedJulianDate::new(mjd)
            .add_duration(Days::new(days_val))
            .value();
        unsafe { *out = result };
        TempochStatus::Ok
    })
}

/// Compute Julian centuries since J2000 as a `QttyQuantity`.
#[no_mangle]
pub extern "C" fn tempoch_jd_julian_centuries_qty(jd: f64) -> QttyQuantity {
    QttyQuantity::new(
        JulianDate::new(jd).julian_centuries().value(),
        UnitId::JulianCentury,
    )
}

/// Compute the duration of a period as a `QttyQuantity` in days.
#[no_mangle]
pub extern "C" fn tempoch_period_mjd_duration_qty(period: crate::TempochPeriodMjd) -> QttyQuantity {
    QttyQuantity::new(period.end_mjd - period.start_mjd, UnitId::Day)
}
