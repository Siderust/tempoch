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

pub use tempoch_core::{
    complement_within, constats, delta_t_seconds, delta_t_seconds_extrapolated, eop,
    refresh_runtime_time_data,
    intersect_periods, normalize_periods, validate_period_list, ContinuousScale,
    ContextConversionTarget, ConversionError, ConversionTarget, GpsSecs, InfallibleConversionTarget,
    Interval, InvalidIntervalError, InvalidPeriodError, J2000s, JD, MJD, Period,
    PeriodListError, Scale, Time, TimeContext, TimeDataError, UnixSecs,
    update_runtime_time_data, DELTA_T_PREDICTION_HORIZON_MJD, EOP_END_MJD,
    EOP_OBSERVED_END_MJD, EOP_START_MJD, MODERN_DELTA_T_OBSERVED_END_MJD, TAI, TCB, TCG, TDB,
    TT, UT1, UTC,
};
