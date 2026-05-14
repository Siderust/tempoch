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
//! - [`constats`] for typed epoch and offset constants

pub use tempoch_core::scalar;
pub use tempoch_core::scalar::{
    scalar_add_days, scalar_difference_in_days, time_tt_from_scalar, time_tt_to_scalar,
};
pub use tempoch_core::{
    Coord, Offset,
    constats, delta_t_seconds, delta_t_seconds_extrapolated, eop, ContextConversionTarget,
    ContinuousScale, ConversionError, ConversionTarget, CoordinateScale, EncodedTime, GpsTime,
    FormatForScale, InfallibleConversionTarget, InfallibleFormatForScale, Interval,
    InvalidIntervalError, InvalidPeriodError, J2000Seconds, J2000s, JulianDate, ModifiedJulianDate,
    Period, PeriodListError, Scale, ScaleKind, Time, TimeContext, TimeDataError, TimeFormat,
    TimeInstant, Unix, UnixTime, DELTA_T_PREDICTION_HORIZON_MJD, EOP_END_MJD,
    EOP_OBSERVED_END_MJD, EOP_START_MJD, GPS, GPS_EPOCH_JD_TAI, GPS_EPOCH_JD_UTC,
    GPS_EPOCH_TAI_MINUS_UTC, JD, JULIAN_YEAR_DAYS, MJD, MODERN_DELTA_T_OBSERVED_END_MJD, TAI,
    TCB, TCG, TDB, TT, UT1, UTC, UTC_DEFINED_FROM_MJD, complement_within,
};
#[cfg(feature = "runtime-data-fetch")]
pub use tempoch_core::{
    fetch_latest_time_data, refresh_runtime_time_data, update_runtime_time_data,
};

#[cfg(feature = "serde")]
pub use tempoch_core::tagged;
