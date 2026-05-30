// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Typed epoch and offset constants.
//!
//! Epoch **instants** are exposed as [`Time<S, F>`] via small `…_jd()`, `…_mjd()`,
//! or `…_tai()` helpers so construction matches the normal [`crate::Time`] pathways.
//!
//! Matching [`qtty::Day`] / [`qtty::Second`] scratch values remain as `*_DAY` /
//! `*_SECONDS` constants for FFI, encoding helpers, and `const` contexts.
//!
//! Pure SI-second offsets between scales (e.g. [`TT_MINUS_TAI`]) stay as bare
//! [`qtty::Second`] values: they are durations, not instants.

use qtty::{Day, Second};

use crate::format::{J2000s, JD, MJD};
use crate::model::scale::{TAI, TT, UTC};
use crate::model::time::Time;
use crate::InfallibleFormatForScale;

/// Days in a Julian year (365.25 d).
pub const JULIAN_YEAR_DAYS: Day = Day::new(365.25);

/// J2000 epoch Julian Day value on the TT axis (`JD 2 451 545.0 TT`).
pub const J2000_JD_TT_DAY: Day = Day::new(2_451_545.0);

/// Offset between Julian Day and Modified Julian Day counts.
///
/// `MJD = JD - JD_MINUS_MJD`. Kept crate-private: external callers should rely
/// on typed [`JD`] / [`MJD`] conversions instead of duplicating this offset.
pub(crate) const JD_MINUS_MJD: Day = Day::new(2_400_000.5);

/// Exact `TT - TAI` offset (32.184 s).
///
/// This is a pure SI-second offset between two coordinate scales, not an
/// instant; it is intentionally kept as a [`qtty::Second`] for algebraic
/// use in scale conversions.
pub const TT_MINUS_TAI: Second = Second::new(32.184);

/// Unix epoch JD value on the UTC axis: `1970-01-01T00:00:00 UTC`.
pub const UNIX_EPOCH_JD_DAY: Day = Day::new(2_440_587.5);

/// Unix epoch MJD value on the UTC axis.
pub const UNIX_EPOCH_MJD_DAY: Day = Day::new(40_587.0);

/// GPS epoch JD value on the UTC axis: `1980-01-06T00:00:00 UTC`.
pub const GPS_EPOCH_JD_UTC_DAY: Day = Day::new(2_444_244.5);

/// Exact `TAI - UTC` offset at the GPS epoch.
///
/// Like [`TT_MINUS_TAI`], this is a pure SI-second offset and stays as a
/// bare [`qtty::Second`].
pub const GPS_EPOCH_TAI_MINUS_UTC: Second = Second::new(19.0);

/// GPS epoch expressed as a JD value on the TAI axis.
///
/// At the GPS epoch, `TAI - UTC = 19 s` exactly, so this is
/// `GPS_EPOCH_JD_UTC_DAY + 19 s` converted to Julian days on the TAI axis.
pub const GPS_EPOCH_JD_TAI_DAY: Day =
    GPS_EPOCH_JD_UTC_DAY.const_add(GPS_EPOCH_TAI_MINUS_UTC.to_const::<qtty::unit::Day>());

/// IAU 2000 B1.9 reference epoch `T0` as a JD value on the TT axis.
pub const IAU_TIME_EPOCH_T0_JD_DAY: Day = Day::new(2_443_144.500_372_5);

/// Start JD (TT axis) for the built-in TT↔TDB truncated-series accuracy band.
///
/// See [`tdb_tt_model_high_accuracy_end_jd`] for the complementary end marker.
pub const TDB_TT_MODEL_HIGH_ACCURACY_START_JD_DAY: Day = Day::new(2_305_447.5);

/// End JD (TT axis) for the built-in TT↔TDB truncated-series accuracy band.
///
/// See [`tdb_tt_model_high_accuracy_start_jd`] for context on the ~10 µs
/// end-to-end budget relative to numerical integration.
pub const TDB_TT_MODEL_HIGH_ACCURACY_END_JD_DAY: Day = Day::new(2_524_598.5);

/// GPS epoch as J2000 TT seconds on the TAI axis for the numeric value
/// `-630_763_181`.
///
/// The storage convention is `(JD_TAI(P) − J2000_JD_TT_DAY) × 86400`. For the GPS
/// epoch, `JD_UTC = GPS_EPOCH_JD_UTC_DAY` and `TAI − UTC = 19 s` (exact), giving:
///
///   `(44_244.0 − 51_544.5) × 86400 + 19 = −630_763_181`.
pub const GPS_EPOCH_TAI_SECONDS: Second = Second::new(-630_763_181.0);

/// First MJD covered by the compiled UTC-TAI segment table, on the UTC axis.
///
/// This corresponds to 1961-01-01. UTC was defined starting from this date.
/// For queries before this boundary, `Time<UTC>` conversions return
/// [`crate::ConversionError::UtcBeforeDefinition`] by default. Back-extrapolation
/// of the first segment can be enabled by building the conversion context with
/// [`crate::TimeContext::allow_pre_definition_utc`]. The extrapolated offset is
/// internally consistent (round-trips close) but is not a historically defined
/// UTC-TAI value; no standard UTC existed before 1961.
pub const UTC_DEFINED_FROM_MJD_DAY: Day = Day::new(37_300.0);

/// One Julian century in days (36 525 d).
pub const DAYS_PER_JULIAN_CENTURY: Day = Day::new(36_525.0);

pub(crate) const DAYS_PER_JC: Day = DAYS_PER_JULIAN_CENTURY;

pub(crate) const UTC_INTERVAL_EPS: Day = Day::new(1e-15);
pub(crate) const L_G: f64 = 6.969_290_134e-10;
pub(crate) const L_B: f64 = 1.550_519_768e-8;
pub(crate) const TDB0: Second = Second::new(-6.55e-5);

/// J2000 epoch as [`Time<TT, JD>`].
#[inline]
pub fn j2000_jd_tt() -> Time<TT, JD> {
    <JD as InfallibleFormatForScale<TT>>::into_time(J2000_JD_TT_DAY)
}

/// Unix epoch as [`Time<UTC, JD>`].
#[inline]
pub fn unix_epoch_jd() -> Time<UTC, JD> {
    <JD as InfallibleFormatForScale<UTC>>::into_time(UNIX_EPOCH_JD_DAY)
}

/// Unix epoch as [`Time<UTC, MJD>`].
#[inline]
pub fn unix_epoch_mjd() -> Time<UTC, MJD> {
    <MJD as InfallibleFormatForScale<UTC>>::into_time(UNIX_EPOCH_MJD_DAY)
}

/// GPS epoch as [`Time<UTC, JD>`].
#[inline]
pub fn gps_epoch_jd_utc() -> Time<UTC, JD> {
    <JD as InfallibleFormatForScale<UTC>>::into_time(GPS_EPOCH_JD_UTC_DAY)
}

/// GPS epoch as [`Time<TAI, JD>`].
#[inline]
pub fn gps_epoch_jd_tai() -> Time<TAI, JD> {
    <JD as InfallibleFormatForScale<TAI>>::into_time(GPS_EPOCH_JD_TAI_DAY)
}

/// IAU time epoch `T0` as [`Time<TT, JD>`].
#[inline]
pub fn iau_time_epoch_t0_jd() -> Time<TT, JD> {
    <JD as InfallibleFormatForScale<TT>>::into_time(IAU_TIME_EPOCH_T0_JD_DAY)
}

/// Start of the TT↔TDB model accuracy interval as [`Time<TT, JD>`].
#[inline]
pub fn tdb_tt_model_high_accuracy_start_jd() -> Time<TT, JD> {
    <JD as InfallibleFormatForScale<TT>>::into_time(TDB_TT_MODEL_HIGH_ACCURACY_START_JD_DAY)
}

/// End of the TT↔TDB model accuracy interval as [`Time<TT, JD>`].
#[inline]
pub fn tdb_tt_model_high_accuracy_end_jd() -> Time<TT, JD> {
    <JD as InfallibleFormatForScale<TT>>::into_time(TDB_TT_MODEL_HIGH_ACCURACY_END_JD_DAY)
}

/// GPS epoch as [`Time<TAI, J2000s>`].
#[inline]
pub fn gps_epoch_tai() -> Time<TAI, J2000s> {
    <J2000s as InfallibleFormatForScale<TAI>>::into_time(GPS_EPOCH_TAI_SECONDS)
}

/// UTC definition boundary as [`Time<UTC, MJD>`].
#[inline]
pub fn utc_defined_from_mjd() -> Time<UTC, MJD> {
    <MJD as InfallibleFormatForScale<UTC>>::into_time(UTC_DEFINED_FROM_MJD_DAY)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::earth::delta_t::DELTA_T_PREDICTION_HORIZON_MJD;

    #[test]
    fn unix_epoch_jd_and_mjd_constants_are_consistent() {
        assert!((UNIX_EPOCH_JD_DAY - JD_MINUS_MJD - UNIX_EPOCH_MJD_DAY).abs() < Day::new(1e-15));
    }

    #[test]
    fn j2000_reference_values_match_known_offsets() {
        assert!((J2000_JD_TT_DAY - JD_MINUS_MJD - Day::new(51_544.5)).abs() < Day::new(1e-12));
        assert!((TT_MINUS_TAI - Second::new(32.184)).abs() < Second::new(1e-12));
        assert!((UTC_DEFINED_FROM_MJD_DAY - Day::new(37_300.0)).abs() < Day::new(1e-12));
        assert!((GPS_EPOCH_JD_UTC_DAY - Day::new(2_444_244.5)).abs() < Day::new(1e-12));
        assert!((GPS_EPOCH_TAI_MINUS_UTC - Second::new(19.0)).abs() < Second::new(1e-12));
        assert!(
            (GPS_EPOCH_JD_TAI_DAY
                - GPS_EPOCH_JD_UTC_DAY
                - GPS_EPOCH_TAI_MINUS_UTC.to::<qtty::unit::Day>())
            .abs()
                < Day::new(1e-9)
        );
    }

    #[test]
    fn high_accuracy_model_interval_is_ordered() {
        assert!(tdb_tt_model_high_accuracy_end_jd() > tdb_tt_model_high_accuracy_start_jd());
        assert!(gps_epoch_tai().raw().is_finite());
        assert!(DELTA_T_PREDICTION_HORIZON_MJD.value().is_finite());
    }

    #[test]
    fn helper_constructors_match_exported_scalar_constants() {
        assert_eq!(j2000_jd_tt().raw(), J2000_JD_TT_DAY);
        assert_eq!(unix_epoch_jd().raw(), UNIX_EPOCH_JD_DAY);
        assert_eq!(unix_epoch_mjd().raw(), UNIX_EPOCH_MJD_DAY);
        assert_eq!(gps_epoch_jd_utc().raw(), GPS_EPOCH_JD_UTC_DAY);
        assert_eq!(gps_epoch_jd_tai().raw(), GPS_EPOCH_JD_TAI_DAY);
        assert_eq!(iau_time_epoch_t0_jd().raw(), IAU_TIME_EPOCH_T0_JD_DAY);
        assert_eq!(
            tdb_tt_model_high_accuracy_start_jd().raw(),
            TDB_TT_MODEL_HIGH_ACCURACY_START_JD_DAY
        );
        assert_eq!(
            tdb_tt_model_high_accuracy_end_jd().raw(),
            TDB_TT_MODEL_HIGH_ACCURACY_END_JD_DAY
        );
        assert_eq!(gps_epoch_tai().raw(), GPS_EPOCH_TAI_SECONDS);
        assert_eq!(utc_defined_from_mjd().raw(), UTC_DEFINED_FROM_MJD_DAY);
    }
}
