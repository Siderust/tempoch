// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! FFI bindings for tempoch time types: JulianDate, ModifiedJulianDate,
//! and UTC conversions.

use crate::catch_panic;
use crate::error::TempochStatus;
use chrono::{DateTime, NaiveDate, Utc};
use qtty::Days;
use qtty_ffi::{QttyQuantity, UnitId};
use tempoch::{
    JulianDate, ModifiedJulianDate, Time, TimeInstant, UniversalTime, GPS, JD, JDE, MJD, TAI, TCB,
    TCG, TDB, TT, UT,
};

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

// ═══════════════════════════════════════════════════════════════════════════
// Time scale conversions  (JD ↔ TDB, TT, TAI, TCG, TCB, GPS, UT, JDE, UnixTime)
// ═══════════════════════════════════════════════════════════════════════════

/// Convert a Julian Date (TT) to TDB (Barycentric Dynamical Time).
#[no_mangle]
pub extern "C" fn tempoch_jd_to_tdb(jd: f64) -> f64 {
    JulianDate::new(jd).to::<TDB>().value()
}

/// Convert TDB back to Julian Date (TT).
#[no_mangle]
pub extern "C" fn tempoch_tdb_to_jd(tdb: f64) -> f64 {
    Time::<TDB>::new(tdb).to::<JD>().value()
}

/// Convert a Julian Date (TT) to TT (Terrestrial Time). Identity—included for completeness.
#[no_mangle]
pub extern "C" fn tempoch_jd_to_tt(jd: f64) -> f64 {
    JulianDate::new(jd).to::<TT>().value()
}

/// Convert TT back to Julian Date (TT). Identity.
#[no_mangle]
pub extern "C" fn tempoch_tt_to_jd(tt: f64) -> f64 {
    Time::<TT>::new(tt).to::<JD>().value()
}

/// Convert a Julian Date (TT) to TAI (International Atomic Time).
#[no_mangle]
pub extern "C" fn tempoch_jd_to_tai(jd: f64) -> f64 {
    JulianDate::new(jd).to::<TAI>().value()
}

/// Convert TAI back to Julian Date (TT).
#[no_mangle]
pub extern "C" fn tempoch_tai_to_jd(tai: f64) -> f64 {
    Time::<TAI>::new(tai).to::<JD>().value()
}

/// Convert a Julian Date (TT) to TCG (Geocentric Coordinate Time).
#[no_mangle]
pub extern "C" fn tempoch_jd_to_tcg(jd: f64) -> f64 {
    JulianDate::new(jd).to::<TCG>().value()
}

/// Convert TCG back to Julian Date (TT).
#[no_mangle]
pub extern "C" fn tempoch_tcg_to_jd(tcg: f64) -> f64 {
    Time::<TCG>::new(tcg).to::<JD>().value()
}

/// Convert a Julian Date (TT) to TCB (Barycentric Coordinate Time).
#[no_mangle]
pub extern "C" fn tempoch_jd_to_tcb(jd: f64) -> f64 {
    JulianDate::new(jd).to::<TCB>().value()
}

/// Convert TCB back to Julian Date (TT).
#[no_mangle]
pub extern "C" fn tempoch_tcb_to_jd(tcb: f64) -> f64 {
    Time::<TCB>::new(tcb).to::<JD>().value()
}

/// Convert a Julian Date (TT) to GPS Time.
#[no_mangle]
pub extern "C" fn tempoch_jd_to_gps(jd: f64) -> f64 {
    JulianDate::new(jd).to::<GPS>().value()
}

/// Convert GPS Time back to Julian Date (TT).
#[no_mangle]
pub extern "C" fn tempoch_gps_to_jd(gps: f64) -> f64 {
    Time::<GPS>::new(gps).to::<JD>().value()
}

/// Convert a Julian Date (TT) to UT (Universal Time UT1).
#[no_mangle]
pub extern "C" fn tempoch_jd_to_ut(jd: f64) -> f64 {
    JulianDate::new(jd).to::<UT>().value()
}

/// Convert UT back to Julian Date (TT).
#[no_mangle]
pub extern "C" fn tempoch_ut_to_jd(ut: f64) -> f64 {
    Time::<UT>::new(ut).to::<JD>().value()
}

/// Convert a Julian Date (TT) to JDE (Julian Ephemeris Day — semantic alias of JD(TT)).
#[no_mangle]
pub extern "C" fn tempoch_jd_to_jde(jd: f64) -> f64 {
    JulianDate::new(jd).to::<JDE>().value()
}

/// Convert JDE back to Julian Date (TT).
#[no_mangle]
pub extern "C" fn tempoch_jde_to_jd(jde: f64) -> f64 {
    Time::<JDE>::new(jde).to::<JD>().value()
}

/// Convert a Julian Date (TT) to Unix Time (seconds since 1970-01-01T00:00:00 UTC, ignoring leap seconds).
#[no_mangle]
pub extern "C" fn tempoch_jd_to_unix(jd: f64) -> f64 {
    JulianDate::new(jd).to::<tempoch::UnixTime>().value()
}

/// Convert Unix Time back to Julian Date (TT).
#[no_mangle]
pub extern "C" fn tempoch_unix_to_jd(unix: f64) -> f64 {
    Time::<tempoch::UnixTime>::new(unix).to::<JD>().value()
}

/// Return ΔT = TT − UT1 in seconds for a given Julian Date.
///
/// Uses the piecewise polynomial/tabular model from tempoch-core.
#[no_mangle]
pub extern "C" fn tempoch_delta_t_seconds(jd: f64) -> f64 {
    let ut: UniversalTime = JulianDate::new(jd).to::<UT>();
    ut.delta_t().value()
}

/// Scale label for the `tempoch_jd_to_scale()` / `tempoch_scale_to_jd()` dispatch.
///
/// cbindgen:prefix-with-name
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TempochScale {
    JD = 0,
    MJD = 1,
    TDB = 2,
    TT = 3,
    TAI = 4,
    TCG = 5,
    TCB = 6,
    GPS = 7,
    UT = 8,
    JDE = 9,
    UnixTime = 10,
}

/// Generic JD → any scale dispatch.
///
/// Returns the value in the target time scale. Prefer the individual functions
/// (`tempoch_jd_to_tdb`, etc.) when the target scale is known at compile time.
#[no_mangle]
pub extern "C" fn tempoch_jd_to_scale(jd: f64, scale: TempochScale) -> f64 {
    match scale {
        TempochScale::JD => jd,
        TempochScale::MJD => tempoch_jd_to_mjd(jd),
        TempochScale::TDB => tempoch_jd_to_tdb(jd),
        TempochScale::TT => tempoch_jd_to_tt(jd),
        TempochScale::TAI => tempoch_jd_to_tai(jd),
        TempochScale::TCG => tempoch_jd_to_tcg(jd),
        TempochScale::TCB => tempoch_jd_to_tcb(jd),
        TempochScale::GPS => tempoch_jd_to_gps(jd),
        TempochScale::UT => tempoch_jd_to_ut(jd),
        TempochScale::JDE => tempoch_jd_to_jde(jd),
        TempochScale::UnixTime => tempoch_jd_to_unix(jd),
    }
}

/// Generic any scale → JD dispatch.
#[no_mangle]
pub extern "C" fn tempoch_scale_to_jd(value: f64, scale: TempochScale) -> f64 {
    match scale {
        TempochScale::JD => value,
        TempochScale::MJD => tempoch_mjd_to_jd(value),
        TempochScale::TDB => tempoch_tdb_to_jd(value),
        TempochScale::TT => tempoch_tt_to_jd(value),
        TempochScale::TAI => tempoch_tai_to_jd(value),
        TempochScale::TCG => tempoch_tcg_to_jd(value),
        TempochScale::TCB => tempoch_tcb_to_jd(value),
        TempochScale::GPS => tempoch_gps_to_jd(value),
        TempochScale::UT => tempoch_ut_to_jd(value),
        TempochScale::JDE => tempoch_jde_to_jd(value),
        TempochScale::UnixTime => tempoch_unix_to_jd(value),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::TempochStatus;
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

    // ── TempochUtc::into_chrono ───────────────────────────────────────

    #[test]
    fn into_chrono_invalid_nanoseconds_returns_none() {
        // and_hms_nano_opt returns None when nanosecond >= 1_000_000_000.
        let utc = TempochUtc {
            year: 2000,
            month: 1,
            day: 1,
            hour: 12,
            minute: 0,
            second: 0,
            nanosecond: 1_500_000_000,
        };
        assert!(utc.into_chrono().is_none());
    }

    // ── tempoch_jd_new / tempoch_jd_j2000 ────────────────────────────

    #[test]
    fn jd_new_is_identity() {
        assert_eq!(tempoch_jd_new(2_451_545.0), 2_451_545.0);
        assert_eq!(tempoch_jd_new(0.0), 0.0);
    }

    #[test]
    fn jd_j2000_value() {
        assert_eq!(tempoch_jd_j2000(), 2_451_545.0);
    }

    // ── tempoch_jd_to_mjd ────────────────────────────────────────────

    #[test]
    fn jd_to_mjd_roundtrip() {
        let mjd = tempoch_jd_to_mjd(2_451_545.0);
        assert!((mjd - 51_544.5).abs() < 1e-10);
    }

    // ── tempoch_jd_from_utc ──────────────────────────────────────────

    #[test]
    fn jd_from_utc_null_pointer_returns_error() {
        let status = unsafe { tempoch_jd_from_utc(utc_j2000(), ptr::null_mut()) };
        assert_eq!(status, TempochStatus::NullPointer);
    }

    #[test]
    fn jd_from_utc_invalid_date_returns_error() {
        let bad = TempochUtc {
            year: 2000,
            month: 1,
            day: 1,
            hour: 12,
            minute: 0,
            second: 0,
            nanosecond: 2_000_000_000,
        };
        let mut out: f64 = 0.0;
        let status = unsafe { tempoch_jd_from_utc(bad, &mut out) };
        assert_eq!(status, TempochStatus::UtcConversionFailed);
    }

    #[test]
    fn jd_from_utc_success() {
        let mut out: f64 = 0.0;
        let status = unsafe { tempoch_jd_from_utc(utc_j2000(), &mut out) };
        assert_eq!(status, TempochStatus::Ok);
        // J2000.0 UTC → JD(TT) should be close to 2 451 545
        assert!((out - 2_451_545.0).abs() < 0.01);
    }

    // ── tempoch_jd_to_utc ────────────────────────────────────────────

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

    // ── tempoch_mjd_new ──────────────────────────────────────────────

    #[test]
    fn mjd_new_is_identity() {
        assert_eq!(tempoch_mjd_new(51_544.5), 51_544.5);
    }

    // ── tempoch_mjd_to_jd ────────────────────────────────────────────

    #[test]
    fn mjd_to_jd_roundtrip() {
        let jd = tempoch_mjd_to_jd(51_544.5);
        assert!((jd - 2_451_545.0).abs() < 1e-10);
    }

    // ── tempoch_mjd_from_utc ─────────────────────────────────────────

    #[test]
    fn mjd_from_utc_null_pointer_returns_error() {
        let status = unsafe { tempoch_mjd_from_utc(utc_j2000(), ptr::null_mut()) };
        assert_eq!(status, TempochStatus::NullPointer);
    }

    #[test]
    fn mjd_from_utc_success() {
        let mut out: f64 = 0.0;
        let status = unsafe { tempoch_mjd_from_utc(utc_j2000(), &mut out) };
        assert_eq!(status, TempochStatus::Ok);
        assert!((out - 51_544.5).abs() < 0.01);
    }

    // ── tempoch_mjd_to_utc ───────────────────────────────────────────

    #[test]
    fn mjd_to_utc_null_pointer_returns_error() {
        let status = unsafe { tempoch_mjd_to_utc(51_544.5, ptr::null_mut()) };
        assert_eq!(status, TempochStatus::NullPointer);
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
    }

    // ── arithmetic helpers ───────────────────────────────────────────

    #[test]
    fn jd_difference_returns_days() {
        let diff = tempoch_jd_difference(2_451_546.0, 2_451_545.0);
        assert!((diff - 1.0).abs() < 1e-10);
    }

    #[test]
    fn jd_add_days_advances_epoch() {
        let result = tempoch_jd_add_days(2_451_545.0, 1.5);
        assert!((result - 2_451_546.5).abs() < 1e-10);
    }

    #[test]
    fn mjd_difference_returns_days() {
        let diff = tempoch_mjd_difference(59001.0, 59000.0);
        assert!((diff - 1.0).abs() < 1e-10);
    }

    #[test]
    fn mjd_add_days_advances_epoch() {
        let result = tempoch_mjd_add_days(59000.0, 0.5);
        assert!((result - 59000.5).abs() < 1e-10);
    }

    #[test]
    fn jd_julian_centuries_at_j2000_is_zero() {
        let c = tempoch_jd_julian_centuries(2_451_545.0);
        assert!(c.abs() < 1e-10);
    }
}
