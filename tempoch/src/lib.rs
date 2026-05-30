// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Public façade over `tempoch-core`.
//!
//! The crate root exposes the redesigned scale-only time model:
//!
//! - [`Time<S>`] for typed instants on a given scale
//! - [`Scale`] markers such as [`TT`], [`TAI`], [`UTC`], and [`UT1`]
//! - unified conversion targets via `time.to::<Target>()`, `try_to`, and
//!   `to_with`
//! - [`constats`] for epoch [`Time`] helpers plus `qtty::Day` / `qtty::Second`
//!   scratch constants

pub use tempoch_core::{
    assert_time_data_fresh, complement_within, constats, delta_t_seconds,
    delta_t_seconds_extrapolated, eop, gps_epoch_jd_tai, gps_epoch_jd_utc, gps_epoch_tai,
    iau_time_epoch_t0_jd, j2000_jd_tt, tdb_tt_model_high_accuracy_end_jd,
    tdb_tt_model_high_accuracy_start_jd, time_data_provenance, unix_epoch_jd, unix_epoch_mjd,
    utc_defined_from_mjd, ContextConversionTarget, ContinuousScale, ConversionError,
    ConversionTarget, CoordinateScale, DataHorizons, DurationError, ExactDuration, FormatForScale,
    FormatOptions, FormatPrecision, FreshnessError, GnssWeek, GnssWeekScale, GpsTime,
    InfallibleConversionTarget, InfallibleFormatForScale, Interval, InvalidIntervalError,
    J2000Seconds, J2000s, JulianDate, ModifiedJulianDate, Period, PeriodListError,
    ProvenanceSnapshot, Scale, SourceUrls, Time, TimeContext, TimeDataError, TimeFormat,
    TimeInstant, TimeSeries, TimeSeriesError, Unix, UnixTime, BDT, DAYS_PER_JULIAN_CENTURY,
    DELTA_T_PREDICTION_HORIZON_MJD, ET, GPS, GPST, GPS_EPOCH_JD_TAI_DAY, GPS_EPOCH_JD_UTC_DAY,
    GPS_EPOCH_TAI_MINUS_UTC, GPS_EPOCH_TAI_SECONDS, GST, IAU_TIME_EPOCH_T0_JD_DAY, J2000_JD_TT_DAY,
    JD, JULIAN_YEAR_DAYS, MJD, MODERN_DELTA_T_OBSERVED_END_MJD, NANOS_PER_SECOND, QZSST, TAI, TCB,
    TCG, TDB, TDB_TT_MODEL_HIGH_ACCURACY_END_JD_DAY, TDB_TT_MODEL_HIGH_ACCURACY_START_JD_DAY, TT,
    TT_MINUS_TAI, UNIX_EPOCH_JD_DAY, UNIX_EPOCH_MJD_DAY, UT1, UTC, UTC_DEFINED_FROM_MJD_DAY,
};

/// Historical name for [`Time<S, F>`] after the format-parameter merge.
pub type EncodedTime<S, F> = Time<S, F>;
#[cfg(feature = "runtime-data-fetch")]
pub use tempoch_core::{
    fetch_latest_time_data, refresh_runtime_time_data, update_runtime_time_data,
};

#[cfg(feature = "serde")]
pub use tempoch_core::tagged;
