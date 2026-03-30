// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! FFI bindings for tempoch time operations.
//!
//! All absolute instant inputs and outputs use the typed carriers defined in
//! [`crate::carriers`] rather than bare `f64`.  Generic scale-dispatch
//! functions accept raw `int32_t` scale IDs (validated before dispatch) rather
//! than `TempochScaleId` enum values in the ABI.

use crate::carriers::{
    jd_to_scale_value, scale_value_to_jd, TempochGps, TempochJd, TempochJde, TempochMjd,
    TempochScaleId, TempochTai, TempochTcb, TempochTcg, TempochTdb, TempochTt, TempochUnixTime,
    TempochUt,
};
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
// UTC calendar breakdown
// ═══════════════════════════════════════════════════════════════════════════

/// UTC date-time breakdown for C interop.
///
/// Calendar fields remain raw integer types.  This struct is NOT a numeric
/// carrier; use the `tempoch_*_t` carriers for absolute instant values.
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
    /// Second of the minute (0–59).
    pub second: u8,
    /// Sub-second component in nanoseconds (0–999_999_999).
    pub nanosecond: u32,
}

impl TempochUtc {
    pub(crate) fn into_chrono(self) -> Option<DateTime<Utc>> {
        let date = NaiveDate::from_ymd_opt(self.year, self.month as u32, self.day as u32)?;
        let time = date.and_hms_nano_opt(
            self.hour.into(),
            self.minute.into(),
            self.second.into(),
            self.nanosecond,
        )?;
        Some(DateTime::<Utc>::from_naive_utc_and_offset(time, Utc))
    }

    pub(crate) fn from_chrono(dt: &DateTime<Utc>) -> Self {
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
// Julian Date (JD / TT)
// ═══════════════════════════════════════════════════════════════════════════

/// Create a `TempochJd` carrier from a raw `double`.
#[no_mangle]
pub extern "C" fn tempoch_jd_new(value: f64) -> TempochJd {
    TempochJd::new(value)
}

/// Return the J2000.0 epoch as a `TempochJd` (2451545.0).
#[no_mangle]
pub extern "C" fn tempoch_jd_j2000() -> TempochJd {
    TempochJd::new(JulianDate::J2000.value())
}

/// Convert a `TempochJd` to a `TempochMjd`.
#[no_mangle]
pub extern "C" fn tempoch_jd_to_mjd(jd: TempochJd) -> TempochMjd {
    TempochMjd::new(JulianDate::new(jd.value).to::<MJD>().value())
}

/// Create a `TempochJd` from a UTC date-time.
///
/// # Safety
/// `out` must be a valid, writable pointer to `TempochJd`.
#[no_mangle]
pub unsafe extern "C" fn tempoch_jd_from_utc(
    utc: TempochUtc,
    out: *mut TempochJd,
) -> TempochStatus {
    catch_panic!(TempochStatus::InternalPanic, {
        if out.is_null() {
            return TempochStatus::NullPointer;
        }
        match utc.into_chrono() {
            Some(dt) => {
                unsafe { *out = TempochJd::new(JulianDate::from_utc(dt).value()) };
                TempochStatus::Ok
            }
            None => TempochStatus::UtcConversionFailed,
        }
    })
}

/// Convert a `TempochJd` to a UTC breakdown.
///
/// # Safety
/// `out` must be a valid, writable pointer to `TempochUtc`.
#[no_mangle]
pub unsafe extern "C" fn tempoch_jd_to_utc(jd: TempochJd, out: *mut TempochUtc) -> TempochStatus {
    catch_panic!(TempochStatus::InternalPanic, {
        if out.is_null() {
            return TempochStatus::NullPointer;
        }
        match JulianDate::new(jd.value).to_utc() {
            Some(dt) => {
                unsafe { *out = TempochUtc::from_chrono(&dt) };
                TempochStatus::Ok
            }
            None => TempochStatus::UtcConversionFailed,
        }
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// Modified Julian Date (MJD / TT)
// ═══════════════════════════════════════════════════════════════════════════

/// Create a `TempochMjd` carrier from a raw `double`.
#[no_mangle]
pub extern "C" fn tempoch_mjd_new(value: f64) -> TempochMjd {
    TempochMjd::new(value)
}

/// Convert a `TempochMjd` to a `TempochJd`.
#[no_mangle]
pub extern "C" fn tempoch_mjd_to_jd(mjd: TempochMjd) -> TempochJd {
    TempochJd::new(ModifiedJulianDate::new(mjd.value).to::<JD>().value())
}

/// Create a `TempochMjd` from a UTC date-time.
///
/// # Safety
/// `out` must be a valid, writable pointer to `TempochMjd`.
#[no_mangle]
pub unsafe extern "C" fn tempoch_mjd_from_utc(
    utc: TempochUtc,
    out: *mut TempochMjd,
) -> TempochStatus {
    catch_panic!(TempochStatus::InternalPanic, {
        if out.is_null() {
            return TempochStatus::NullPointer;
        }
        match utc.into_chrono() {
            Some(dt) => {
                unsafe { *out = TempochMjd::new(ModifiedJulianDate::from_utc(dt).value()) };
                TempochStatus::Ok
            }
            None => TempochStatus::UtcConversionFailed,
        }
    })
}

/// Convert a `TempochMjd` to a UTC breakdown.
///
/// # Safety
/// `out` must be a valid, writable pointer to `TempochUtc`.
#[no_mangle]
pub unsafe extern "C" fn tempoch_mjd_to_utc(
    mjd: TempochMjd,
    out: *mut TempochUtc,
) -> TempochStatus {
    catch_panic!(TempochStatus::InternalPanic, {
        if out.is_null() {
            return TempochStatus::NullPointer;
        }
        match ModifiedJulianDate::new(mjd.value).to_utc() {
            Some(dt) => {
                unsafe { *out = TempochUtc::from_chrono(&dt) };
                TempochStatus::Ok
            }
            None => TempochStatus::UtcConversionFailed,
        }
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// Duration / Difference (raw f64 — for arithmetic convenience)
// ═══════════════════════════════════════════════════════════════════════════

/// Compute the difference between two JD values in days (jd1 − jd2).
#[no_mangle]
pub extern "C" fn tempoch_jd_difference(jd1: TempochJd, jd2: TempochJd) -> f64 {
    JulianDate::new(jd1.value)
        .difference(&JulianDate::new(jd2.value))
        .value()
}

/// Add a duration in days to a `TempochJd`.
#[no_mangle]
pub extern "C" fn tempoch_jd_add_days(jd: TempochJd, days: f64) -> TempochJd {
    TempochJd::new(
        JulianDate::new(jd.value)
            .add_duration(Days::new(days))
            .value(),
    )
}

/// Compute the difference between two MJD values in days (mjd1 − mjd2).
#[no_mangle]
pub extern "C" fn tempoch_mjd_difference(mjd1: TempochMjd, mjd2: TempochMjd) -> f64 {
    ModifiedJulianDate::new(mjd1.value)
        .difference(&ModifiedJulianDate::new(mjd2.value))
        .value()
}

/// Add a duration in days to a `TempochMjd`.
#[no_mangle]
pub extern "C" fn tempoch_mjd_add_days(mjd: TempochMjd, days: f64) -> TempochMjd {
    TempochMjd::new(
        ModifiedJulianDate::new(mjd.value)
            .add_duration(Days::new(days))
            .value(),
    )
}

/// Compute Julian centuries since J2000 for a given `TempochJd`.
#[no_mangle]
pub extern "C" fn tempoch_jd_julian_centuries(jd: TempochJd) -> f64 {
    JulianDate::new(jd.value).julian_centuries().value()
}

// ═══════════════════════════════════════════════════════════════════════════
// Duration / Difference (QttyQuantity — typed)
// ═══════════════════════════════════════════════════════════════════════════

/// Compute the difference between two JDs as a `QttyQuantity` in days.
#[no_mangle]
pub extern "C" fn tempoch_jd_difference_qty(jd1: TempochJd, jd2: TempochJd) -> QttyQuantity {
    let diff = JulianDate::new(jd1.value)
        .difference(&JulianDate::new(jd2.value))
        .value();
    QttyQuantity::new(diff, UnitId::Day)
}

/// Add a `QttyQuantity` duration (time-compatible) to a `TempochJd`.
///
/// Returns `InvalidDurationUnit` if the quantity cannot be converted to days.
///
/// # Safety
/// `out` must be a valid, writable pointer to `TempochJd`.
#[no_mangle]
pub unsafe extern "C" fn tempoch_jd_add_qty(
    jd: TempochJd,
    duration: QttyQuantity,
    out: *mut TempochJd,
) -> TempochStatus {
    catch_panic!(TempochStatus::InternalPanic, {
        if out.is_null() {
            return TempochStatus::NullPointer;
        }
        let days_val = match duration.convert_to(UnitId::Day) {
            Some(q) => q.value,
            None => return TempochStatus::InvalidDurationUnit,
        };
        let result = JulianDate::new(jd.value)
            .add_duration(Days::new(days_val))
            .value();
        unsafe { *out = TempochJd::new(result) };
        TempochStatus::Ok
    })
}

/// Compute the difference between two MJDs as a `QttyQuantity` in days.
#[no_mangle]
pub extern "C" fn tempoch_mjd_difference_qty(mjd1: TempochMjd, mjd2: TempochMjd) -> QttyQuantity {
    let diff = ModifiedJulianDate::new(mjd1.value)
        .difference(&ModifiedJulianDate::new(mjd2.value))
        .value();
    QttyQuantity::new(diff, UnitId::Day)
}

/// Add a `QttyQuantity` duration (time-compatible) to a `TempochMjd`.
///
/// Returns `InvalidDurationUnit` if the quantity cannot be converted to days.
///
/// # Safety
/// `out` must be a valid, writable pointer to `TempochMjd`.
#[no_mangle]
pub unsafe extern "C" fn tempoch_mjd_add_qty(
    mjd: TempochMjd,
    duration: QttyQuantity,
    out: *mut TempochMjd,
) -> TempochStatus {
    catch_panic!(TempochStatus::InternalPanic, {
        if out.is_null() {
            return TempochStatus::NullPointer;
        }
        let days_val = match duration.convert_to(UnitId::Day) {
            Some(q) => q.value,
            None => return TempochStatus::InvalidDurationUnit,
        };
        let result = ModifiedJulianDate::new(mjd.value)
            .add_duration(Days::new(days_val))
            .value();
        unsafe { *out = TempochMjd::new(result) };
        TempochStatus::Ok
    })
}

/// Compute Julian centuries since J2000 as a `QttyQuantity`.
#[no_mangle]
pub extern "C" fn tempoch_jd_julian_centuries_qty(jd: TempochJd) -> QttyQuantity {
    QttyQuantity::new(
        JulianDate::new(jd.value).julian_centuries().value(),
        UnitId::JulianCentury,
    )
}

// ═══════════════════════════════════════════════════════════════════════════
// Typed scale conversions (JD(TT) ↔ each scale)
// ═══════════════════════════════════════════════════════════════════════════

/// Convert a `TempochJd` to `TempochTdb`.
#[no_mangle]
pub extern "C" fn tempoch_jd_to_tdb(jd: TempochJd) -> TempochTdb {
    TempochTdb::new(JulianDate::new(jd.value).to::<TDB>().value())
}

/// Convert a `TempochTdb` to `TempochJd`.
#[no_mangle]
pub extern "C" fn tempoch_tdb_to_jd(tdb: TempochTdb) -> TempochJd {
    TempochJd::new(Time::<TDB>::new(tdb.value).to::<JD>().value())
}

/// Convert a `TempochJd` to `TempochTt` (identity — included for completeness).
#[no_mangle]
pub extern "C" fn tempoch_jd_to_tt(jd: TempochJd) -> TempochTt {
    TempochTt::new(JulianDate::new(jd.value).to::<TT>().value())
}

/// Convert a `TempochTt` to `TempochJd` (identity).
#[no_mangle]
pub extern "C" fn tempoch_tt_to_jd(tt: TempochTt) -> TempochJd {
    TempochJd::new(Time::<TT>::new(tt.value).to::<JD>().value())
}

/// Convert a `TempochJd` to `TempochTai`.
#[no_mangle]
pub extern "C" fn tempoch_jd_to_tai(jd: TempochJd) -> TempochTai {
    TempochTai::new(JulianDate::new(jd.value).to::<TAI>().value())
}

/// Convert a `TempochTai` to `TempochJd`.
#[no_mangle]
pub extern "C" fn tempoch_tai_to_jd(tai: TempochTai) -> TempochJd {
    TempochJd::new(Time::<TAI>::new(tai.value).to::<JD>().value())
}

/// Convert a `TempochJd` to `TempochTcg`.
#[no_mangle]
pub extern "C" fn tempoch_jd_to_tcg(jd: TempochJd) -> TempochTcg {
    TempochTcg::new(JulianDate::new(jd.value).to::<TCG>().value())
}

/// Convert a `TempochTcg` to `TempochJd`.
#[no_mangle]
pub extern "C" fn tempoch_tcg_to_jd(tcg: TempochTcg) -> TempochJd {
    TempochJd::new(Time::<TCG>::new(tcg.value).to::<JD>().value())
}

/// Convert a `TempochJd` to `TempochTcb`.
#[no_mangle]
pub extern "C" fn tempoch_jd_to_tcb(jd: TempochJd) -> TempochTcb {
    TempochTcb::new(JulianDate::new(jd.value).to::<TCB>().value())
}

/// Convert a `TempochTcb` to `TempochJd`.
#[no_mangle]
pub extern "C" fn tempoch_tcb_to_jd(tcb: TempochTcb) -> TempochJd {
    TempochJd::new(Time::<TCB>::new(tcb.value).to::<JD>().value())
}

/// Convert a `TempochJd` to `TempochGps`.
#[no_mangle]
pub extern "C" fn tempoch_jd_to_gps(jd: TempochJd) -> TempochGps {
    TempochGps::new(JulianDate::new(jd.value).to::<GPS>().value())
}

/// Convert a `TempochGps` to `TempochJd`.
#[no_mangle]
pub extern "C" fn tempoch_gps_to_jd(gps: TempochGps) -> TempochJd {
    TempochJd::new(Time::<GPS>::new(gps.value).to::<JD>().value())
}

/// Convert a `TempochJd` to `TempochUt` (UT1).
#[no_mangle]
pub extern "C" fn tempoch_jd_to_ut(jd: TempochJd) -> TempochUt {
    TempochUt::new(JulianDate::new(jd.value).to::<UT>().value())
}

/// Convert a `TempochUt` to `TempochJd`.
#[no_mangle]
pub extern "C" fn tempoch_ut_to_jd(ut: TempochUt) -> TempochJd {
    TempochJd::new(Time::<UT>::new(ut.value).to::<JD>().value())
}

/// Convert a `TempochJd` to `TempochJde` (semantic alias of JD(TT)).
#[no_mangle]
pub extern "C" fn tempoch_jd_to_jde(jd: TempochJd) -> TempochJde {
    TempochJde::new(JulianDate::new(jd.value).to::<JDE>().value())
}

/// Convert a `TempochJde` to `TempochJd`.
#[no_mangle]
pub extern "C" fn tempoch_jde_to_jd(jde: TempochJde) -> TempochJd {
    TempochJd::new(Time::<JDE>::new(jde.value).to::<JD>().value())
}

/// Convert a `TempochJd` to `TempochUnixTime`.
#[no_mangle]
pub extern "C" fn tempoch_jd_to_unix(jd: TempochJd) -> TempochUnixTime {
    TempochUnixTime::new(JulianDate::new(jd.value).to::<tempoch::UnixTime>().value())
}

/// Convert a `TempochUnixTime` to `TempochJd`.
#[no_mangle]
pub extern "C" fn tempoch_unix_to_jd(unix: TempochUnixTime) -> TempochJd {
    TempochJd::new(
        Time::<tempoch::UnixTime>::new(unix.value)
            .to::<JD>()
            .value(),
    )
}

/// Return ΔT = TT − UT1 in seconds for a given `TempochJd`.
#[no_mangle]
pub extern "C" fn tempoch_delta_t_seconds(jd: TempochJd) -> f64 {
    let ut: UniversalTime = JulianDate::new(jd.value).to::<UT>();
    ut.delta_t().value()
}

// ═══════════════════════════════════════════════════════════════════════════
// Generic dispatch (validated raw i32 scale ID)
// ═══════════════════════════════════════════════════════════════════════════

/// Convert a `TempochJd` to any scale, writing the result into `out`.
///
/// `scale_id` must be a valid `TempochScaleId` discriminant (0–10).
/// Returns `InvalidScaleId` if `scale_id` is not recognized.
///
/// # Safety
/// `out` must be a valid, writable pointer to `double`.
#[no_mangle]
pub unsafe extern "C" fn tempoch_jd_to_scale(
    jd: TempochJd,
    scale_id: i32,
    out: *mut f64,
) -> TempochStatus {
    catch_panic!(TempochStatus::InternalPanic, {
        if out.is_null() {
            return TempochStatus::NullPointer;
        }
        match TempochScaleId::from_raw(scale_id) {
            Some(scale) => {
                unsafe { *out = jd_to_scale_value(jd, scale) };
                TempochStatus::Ok
            }
            None => TempochStatus::InvalidScaleId,
        }
    })
}

/// Convert a value in any scale to a `TempochJd`, writing the result into `out`.
///
/// `scale_id` must be a valid `TempochScaleId` discriminant (0–10).
/// Returns `InvalidScaleId` if `scale_id` is not recognized.
///
/// # Safety
/// `out` must be a valid, writable pointer to `TempochJd`.
#[no_mangle]
pub unsafe extern "C" fn tempoch_scale_to_jd(
    value: f64,
    scale_id: i32,
    out: *mut TempochJd,
) -> TempochStatus {
    catch_panic!(TempochStatus::InternalPanic, {
        if out.is_null() {
            return TempochStatus::NullPointer;
        }
        match TempochScaleId::from_raw(scale_id) {
            Some(scale) => {
                unsafe { *out = scale_value_to_jd(value, scale) };
                TempochStatus::Ok
            }
            None => TempochStatus::InvalidScaleId,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::carriers::TempochScaleId;
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

    // ── UTC helpers ───────────────────────────────────────────────────────

    #[test]
    fn into_chrono_invalid_nanoseconds_returns_none() {
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

    // ── tempoch_jd_new / tempoch_jd_j2000 ────────────────────────────────

    #[test]
    fn jd_new_carries_value() {
        let jd = tempoch_jd_new(2_451_545.0);
        assert_eq!(jd.value, 2_451_545.0);
    }

    #[test]
    fn jd_j2000_value() {
        assert_eq!(tempoch_jd_j2000().value, 2_451_545.0);
    }

    // ── JD ↔ MJD ─────────────────────────────────────────────────────────

    #[test]
    fn jd_to_mjd_roundtrip() {
        let jd = TempochJd::new(2_451_545.0);
        let mjd = tempoch_jd_to_mjd(jd);
        assert!((mjd.value - 51_544.5).abs() < 1e-10);
        let back = tempoch_mjd_to_jd(mjd);
        assert!((back.value - jd.value).abs() < 1e-10);
    }

    // ── tempoch_jd_from_utc ───────────────────────────────────────────────

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
        let mut out = TempochJd::new(0.0);
        let status = unsafe { tempoch_jd_from_utc(bad, &mut out) };
        assert_eq!(status, TempochStatus::UtcConversionFailed);
    }

    #[test]
    fn jd_from_utc_success() {
        let mut out = TempochJd::new(0.0);
        let status = unsafe { tempoch_jd_from_utc(utc_j2000(), &mut out) };
        assert_eq!(status, TempochStatus::Ok);
        assert!((out.value - 2_451_545.0).abs() < 0.01);
    }

    // ── tempoch_jd_to_utc ─────────────────────────────────────────────────

    #[test]
    fn jd_to_utc_null_pointer_returns_error() {
        let status = unsafe { tempoch_jd_to_utc(TempochJd::new(2_451_545.0), ptr::null_mut()) };
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
        let status = unsafe { tempoch_jd_to_utc(TempochJd::new(2_451_545.0), &mut out) };
        assert_eq!(status, TempochStatus::Ok);
        assert_eq!(out.year, 2000);
        assert_eq!(out.month, 1);
        assert_eq!(out.day, 1);
    }

    // ── MJD UTC ───────────────────────────────────────────────────────────

    #[test]
    fn mjd_new_carries_value() {
        let mjd = tempoch_mjd_new(51_544.5);
        assert_eq!(mjd.value, 51_544.5);
    }

    #[test]
    fn mjd_from_utc_null_returns_error() {
        let status = unsafe { tempoch_mjd_from_utc(utc_j2000(), ptr::null_mut()) };
        assert_eq!(status, TempochStatus::NullPointer);
    }

    #[test]
    fn mjd_from_utc_success() {
        let mut out = TempochMjd::new(0.0);
        let status = unsafe { tempoch_mjd_from_utc(utc_j2000(), &mut out) };
        assert_eq!(status, TempochStatus::Ok);
        assert!((out.value - 51_544.5).abs() < 0.01);
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
        let status = unsafe { tempoch_mjd_to_utc(TempochMjd::new(51_544.5), &mut out) };
        assert_eq!(status, TempochStatus::Ok);
        assert_eq!(out.year, 2000);
    }

    // ── Arithmetic ────────────────────────────────────────────────────────

    #[test]
    fn jd_difference_is_correct() {
        let jd1 = TempochJd::new(2_451_545.0);
        let jd2 = TempochJd::new(2_451_544.0);
        let diff = tempoch_jd_difference(jd1, jd2);
        assert!((diff - 1.0).abs() < 1e-12);
    }

    #[test]
    fn jd_add_days_is_correct() {
        let jd = TempochJd::new(2_451_545.0);
        let result = tempoch_jd_add_days(jd, 1.0);
        assert!((result.value - 2_451_546.0).abs() < 1e-12);
    }

    #[test]
    fn mjd_difference_is_correct() {
        let mjd1 = TempochMjd::new(51_544.5);
        let mjd2 = TempochMjd::new(51_543.5);
        let diff = tempoch_mjd_difference(mjd1, mjd2);
        assert!((diff - 1.0).abs() < 1e-12);
    }

    #[test]
    fn jd_add_qty_hours() {
        let jd = TempochJd::new(2_451_545.0);
        let duration = QttyQuantity::new(24.0, UnitId::Hour);
        let mut out = TempochJd::new(0.0);
        let status = unsafe { tempoch_jd_add_qty(jd, duration, &mut out) };
        assert_eq!(status, TempochStatus::Ok);
        assert!((out.value - 2_451_546.0).abs() < 1e-10);
    }

    #[test]
    fn jd_add_qty_invalid_unit_returns_error() {
        let jd = TempochJd::new(2_451_545.0);
        let duration = QttyQuantity::new(1.0, UnitId::Meter); // not a time unit
        let mut out = TempochJd::new(0.0);
        let status = unsafe { tempoch_jd_add_qty(jd, duration, &mut out) };
        assert_eq!(status, TempochStatus::InvalidDurationUnit);
    }

    #[test]
    fn jd_add_qty_null_returns_error() {
        let jd = TempochJd::new(2_451_545.0);
        let duration = QttyQuantity::new(1.0, UnitId::Day);
        let status = unsafe { tempoch_jd_add_qty(jd, duration, ptr::null_mut()) };
        assert_eq!(status, TempochStatus::NullPointer);
    }

    // ── Scale round-trips ─────────────────────────────────────────────────

    #[test]
    fn jd_tdb_roundtrip() {
        let jd = TempochJd::new(2_451_545.0);
        let tdb = tempoch_jd_to_tdb(jd);
        let back = tempoch_tdb_to_jd(tdb);
        assert!((back.value - jd.value).abs() < 1e-6);
    }

    #[test]
    fn jd_tai_roundtrip() {
        let jd = TempochJd::new(2_451_545.0);
        let tai = tempoch_jd_to_tai(jd);
        let back = tempoch_tai_to_jd(tai);
        assert!((back.value - jd.value).abs() < 1e-10);
    }

    #[test]
    fn jd_tcg_roundtrip() {
        let jd = TempochJd::new(2_451_545.0);
        let tcg = tempoch_jd_to_tcg(jd);
        let back = tempoch_tcg_to_jd(tcg);
        assert!((back.value - jd.value).abs() < 1e-6);
    }

    #[test]
    fn jd_tcb_roundtrip() {
        let jd = TempochJd::new(2_451_545.0);
        let tcb = tempoch_jd_to_tcb(jd);
        let back = tempoch_tcb_to_jd(tcb);
        assert!((back.value - jd.value).abs() < 1e-6);
    }

    #[test]
    fn jd_gps_roundtrip() {
        let jd = TempochJd::new(2_451_545.0);
        let gps = tempoch_jd_to_gps(jd);
        let back = tempoch_gps_to_jd(gps);
        assert!((back.value - jd.value).abs() < 1e-10);
    }

    #[test]
    fn jd_ut_roundtrip() {
        let jd = TempochJd::new(2_451_545.0);
        let ut = tempoch_jd_to_ut(jd);
        let back = tempoch_ut_to_jd(ut);
        assert!((back.value - jd.value).abs() < 1e-6);
    }

    #[test]
    fn jd_jde_roundtrip() {
        let jd = TempochJd::new(2_451_545.0);
        let jde = tempoch_jd_to_jde(jd);
        let back = tempoch_jde_to_jd(jde);
        assert!((back.value - jd.value).abs() < 1e-12);
    }

    #[test]
    fn jd_unix_roundtrip() {
        let jd = TempochJd::new(2_451_545.0);
        let unix = tempoch_jd_to_unix(jd);
        let back = tempoch_unix_to_jd(unix);
        assert!((back.value - jd.value).abs() < 1e-10);
    }

    // ── Generic dispatch ──────────────────────────────────────────────────

    #[test]
    fn jd_to_scale_valid_ids() {
        let jd = TempochJd::new(2_451_545.0);
        for scale_id in 0..=10i32 {
            let mut out: f64 = 0.0;
            let status = unsafe { tempoch_jd_to_scale(jd, scale_id, &mut out) };
            assert_eq!(
                status,
                TempochStatus::Ok,
                "scale_id {} should succeed",
                scale_id
            );
            assert!(
                out.is_finite(),
                "scale_id {} produced non-finite output",
                scale_id
            );
        }
    }

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
    fn jd_to_scale_null_out() {
        let jd = TempochJd::new(2_451_545.0);
        let status = unsafe { tempoch_jd_to_scale(jd, 0, ptr::null_mut()) };
        assert_eq!(status, TempochStatus::NullPointer);
    }

    #[test]
    fn scale_to_jd_valid_roundtrip() {
        let jd_orig = TempochJd::new(2_451_545.0);
        for scale_id in 0..=10i32 {
            let scale = TempochScaleId::from_raw(scale_id).unwrap();
            let value = jd_to_scale_value(jd_orig, scale);
            let mut out = TempochJd::new(0.0);
            let status = unsafe { tempoch_scale_to_jd(value, scale_id, &mut out) };
            assert_eq!(status, TempochStatus::Ok, "scale_id {} failed", scale_id);
            assert!(
                (out.value - jd_orig.value).abs() < 1e-6,
                "scale_id {} roundtrip error: {} vs {}",
                scale_id,
                out.value,
                jd_orig.value
            );
        }
    }

    #[test]
    fn scale_to_jd_invalid_id() {
        let mut out = TempochJd::new(0.0);
        let status = unsafe { tempoch_scale_to_jd(1.0, -1, &mut out) };
        assert_eq!(status, TempochStatus::InvalidScaleId);
    }

    #[test]
    fn scale_to_jd_null_out() {
        let status = unsafe { tempoch_scale_to_jd(1.0, 0, ptr::null_mut()) };
        assert_eq!(status, TempochStatus::NullPointer);
    }
}
