// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Named tempoch constants exposed over the C ABI.
//!
//! All values are returned as plain `double`s so that C and C++ consumers
//! can use them without depending on the Rust type system.  Each function
//! wraps a canonical `tempoch` fact or derives a secondary coordinate from
//! canonical typed helpers.

use tempoch::{
    delta_t_seconds, delta_t_seconds_extrapolated, DELTA_T_PREDICTION_HORIZON_MJD,
    GPS_EPOCH_JD_UTC_DAY, GPS_EPOCH_TAI_MINUS_UTC, IAU_TIME_EPOCH_T0_JD_DAY, J2000_JD_TT_DAY,
    MODERN_DELTA_T_OBSERVED_END_MJD, NANOS_PER_SECOND, TDB_TT_MODEL_HIGH_ACCURACY_END_JD_DAY,
    TDB_TT_MODEL_HIGH_ACCURACY_START_JD_DAY, TT_MINUS_TAI, UNIX_EPOCH_JD_DAY,
    UTC_DEFINED_FROM_MJD_DAY,
};

/// J2000.0 epoch as JD(TT) — 2 451 545.0.
#[no_mangle]
pub extern "C" fn tempoch_const_j2000_jd_tt() -> f64 {
    J2000_JD_TT_DAY.value()
}

/// One Julian year in days — 365.25.
#[no_mangle]
pub extern "C" fn tempoch_const_julian_year_days() -> f64 {
    qtty::time::JULIAN_YEAR.to::<qtty::unit::Day>().value()
}

/// First MJD covered by the built-in UTC-TAI segment table (1961-01-01).
#[no_mangle]
pub extern "C" fn tempoch_const_utc_defined_from_mjd() -> f64 {
    UTC_DEFINED_FROM_MJD_DAY.value()
}

/// GPS epoch as a Julian Day on the UTC axis (1980-01-06T00:00:00 UTC).
#[no_mangle]
pub extern "C" fn tempoch_const_gps_epoch_jd_utc() -> f64 {
    GPS_EPOCH_JD_UTC_DAY.value()
}

/// Unix epoch Julian Date on the UTC axis (`1970-01-01T00:00:00 UTC`).
#[no_mangle]
pub extern "C" fn tempoch_const_unix_epoch_jd() -> f64 {
    UNIX_EPOCH_JD_DAY.value()
}

/// Unix epoch Modified Julian Day on the UTC axis.
#[no_mangle]
pub extern "C" fn tempoch_const_unix_epoch_mjd() -> f64 {
    tempoch::unix_epoch_mjd().raw().value()
}

/// GPS epoch expressed as a Julian Day on the TAI axis.
#[no_mangle]
pub extern "C" fn tempoch_const_gps_epoch_jd_tai() -> f64 {
    tempoch::gps_epoch_jd_tai().raw().value()
}

/// Exact TAI − UTC offset at the GPS epoch, in seconds (19.0).
#[no_mangle]
pub extern "C" fn tempoch_const_gps_epoch_tai_minus_utc_seconds() -> f64 {
    GPS_EPOCH_TAI_MINUS_UTC.value()
}

/// MJD of the last date for which a ΔT prediction (not extrapolation) is
/// available from the compiled USNO data.
#[no_mangle]
pub extern "C" fn tempoch_const_delta_t_prediction_horizon_mjd() -> f64 {
    DELTA_T_PREDICTION_HORIZON_MJD.value()
}

/// First MJD covered by the active IERS EOP series, or NaN when no EOP data is loaded.
#[no_mangle]
pub extern "C" fn tempoch_const_eop_start_mjd() -> f64 {
    tempoch_core::eop::eop_start()
        .map(|v| v.value())
        .unwrap_or(f64::NAN)
}

/// Last MJD covered by the active IERS EOP series, or NaN when no EOP data is loaded.
#[no_mangle]
pub extern "C" fn tempoch_const_eop_end_mjd() -> f64 {
    tempoch_core::eop::eop_end()
        .map(|v| v.value())
        .unwrap_or(f64::NAN)
}

/// Last MJD with *observed* (Bulletin C04) EOP data in the active series, or NaN when no data.
#[no_mangle]
pub extern "C" fn tempoch_const_eop_observed_end_mjd() -> f64 {
    tempoch_core::eop::eop_observed_end()
        .map(|v| v.value())
        .unwrap_or(f64::NAN)
}

/// Last MJD with modern observed ΔT data (post-1955 atomic-clock era).
#[no_mangle]
pub extern "C" fn tempoch_const_modern_delta_t_observed_end_mjd() -> f64 {
    MODERN_DELTA_T_OBSERVED_END_MJD.value()
}

/// Constant TT − TAI offset, in seconds (32.184).
#[no_mangle]
pub extern "C" fn tempoch_const_tt_minus_tai_seconds() -> f64 {
    TT_MINUS_TAI.value()
}

/// Number of nanoseconds in one SI second (1e9).
#[no_mangle]
pub extern "C" fn tempoch_const_nanos_per_second() -> f64 {
    NANOS_PER_SECOND as f64
}

/// IAU time-scale epoch T0 as a Julian Day on the TT axis (1977-01-01 TAI).
#[no_mangle]
pub extern "C" fn tempoch_const_iau_time_epoch_t0_jd() -> f64 {
    IAU_TIME_EPOCH_T0_JD_DAY.value()
}

/// First JD(TT) of the high-accuracy TDB−TT model validity window.
#[no_mangle]
pub extern "C" fn tempoch_const_tdb_tt_model_high_accuracy_start_jd() -> f64 {
    TDB_TT_MODEL_HIGH_ACCURACY_START_JD_DAY.value()
}

/// Last JD(TT) of the high-accuracy TDB−TT model validity window.
#[no_mangle]
pub extern "C" fn tempoch_const_tdb_tt_model_high_accuracy_end_jd() -> f64 {
    TDB_TT_MODEL_HIGH_ACCURACY_END_JD_DAY.value()
}

/// ΔT = TT − UT1 in seconds for a UT1 Julian Day, using the compiled USNO
/// model. Returns NaN when the requested epoch is outside the model domain.
#[no_mangle]
pub extern "C" fn tempoch_delta_t_seconds(jd_ut1: f64) -> f64 {
    delta_t_seconds(qtty::Day::new(jd_ut1))
        .map(|s| s.value())
        .unwrap_or(f64::NAN)
}

/// ΔT = TT − UT1 in seconds for a UT1 Julian Day, extrapolating with the
/// long-term parabola beyond the tabulated range (always finite).
#[no_mangle]
pub extern "C" fn tempoch_delta_t_seconds_extrapolated(jd_ut1: f64) -> f64 {
    delta_t_seconds_extrapolated(qtty::Day::new(jd_ut1)).value()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn j2000_jd_tt_is_canonical() {
        assert_eq!(tempoch_const_j2000_jd_tt(), 2_451_545.0);
    }

    #[test]
    fn julian_year_days_is_canonical() {
        assert_eq!(tempoch_const_julian_year_days(), 365.25);
    }

    #[test]
    fn utc_defined_from_mjd_is_canonical() {
        assert_eq!(tempoch_const_utc_defined_from_mjd(), 37_300.0);
    }

    #[test]
    fn unix_epoch_jd_is_canonical() {
        assert_eq!(tempoch_const_unix_epoch_jd(), 2_440_587.5);
    }

    #[test]
    fn unix_epoch_mjd_is_canonical() {
        assert_eq!(tempoch_const_unix_epoch_mjd(), 40_587.0);
    }

    #[test]
    fn gps_epoch_jd_utc_is_canonical() {
        assert_eq!(tempoch_const_gps_epoch_jd_utc(), 2_444_244.5);
    }

    #[test]
    fn gps_epoch_tai_minus_utc_is_canonical() {
        assert_eq!(tempoch_const_gps_epoch_tai_minus_utc_seconds(), 19.0);
    }

    #[test]
    fn gps_epoch_jd_tai_is_consistent_with_utc_and_offset() {
        let expected = tempoch_const_gps_epoch_jd_utc()
            + tempoch_const_gps_epoch_tai_minus_utc_seconds() / 86_400.0;
        assert!((tempoch_const_gps_epoch_jd_tai() - expected).abs() < 1e-9);
    }

    #[test]
    fn eop_range_is_nan_without_loaded_data() {
        // EOP data is not compiled in; without a loaded bundle the FFI
        // functions return NaN.
        assert!(tempoch_const_eop_start_mjd().is_nan());
        assert!(tempoch_const_eop_end_mjd().is_nan());
        assert!(tempoch_const_eop_observed_end_mjd().is_nan());
    }

    #[test]
    fn delta_t_horizon_is_finite_and_positive() {
        let horizon = tempoch_const_delta_t_prediction_horizon_mjd();
        assert!(horizon.is_finite());
        assert!(horizon > 0.0);
    }

    #[test]
    fn modern_delta_t_observed_end_mjd_is_finite_and_positive() {
        let end = tempoch_const_modern_delta_t_observed_end_mjd();
        assert!(end.is_finite());
        assert!(end > 0.0);
    }
}
