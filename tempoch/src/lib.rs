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
    complement_within, constats, delta_t_seconds, delta_t_seconds_extrapolated, eop,
    intersect_periods, normalize_periods, try_complement_within, try_intersect_periods,
    validate_period_list, ContextConversionTarget, ContinuousScale, ConversionError,
    ConversionTarget, CoordinateScale, GpsSecs, InfallibleConversionTarget, Interval,
    InvalidIntervalError, InvalidPeriodError, J2000s, Period, PeriodListError, Scale, ScaleKind,
    Time, TimeContext, TimeDataError, UnixSecs, DELTA_T_PREDICTION_HORIZON_MJD, EOP_END_MJD,
    EOP_OBSERVED_END_MJD, EOP_START_MJD, GPS_EPOCH_JD_TAI, GPS_EPOCH_JD_UTC,
    GPS_EPOCH_TAI_MINUS_UTC, JD, MJD, MODERN_DELTA_T_OBSERVED_END_MJD, TAI, TCB, TCG, TDB, TT, UT1,
    UTC, UTC_DEFINED_FROM_MJD,
};
#[cfg(feature = "runtime-data-fetch")]
pub use tempoch_core::{
    fetch_latest_time_data, refresh_runtime_time_data, update_runtime_time_data,
};

#[cfg(feature = "serde")]
pub use tempoch_core::tagged;
