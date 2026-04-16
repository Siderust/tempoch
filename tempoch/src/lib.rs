// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Public façade over `tempoch-core`.
//!
//! The crate root exposes the scale×format time model:
//!
//! - [`Time<S, F>`] for typed instants on a given scale in a given format
//! - [`Scale`] markers such as [`TT`], [`TAI`], [`UTC`], and [`UT1`]
//! - [`Format`] markers such as [`J2000s`], [`Jd`], [`Mjd`]
//! - [`constats`] for typed epoch and offset constants

pub use tempoch_core::{
    complement_within, constats, delta_t_seconds, delta_t_seconds_extrapolated, intersect_periods,
    normalize_periods, validate_period_list, ContinuousScale, ConversionError, DayCount, Format,
    GpsSecs, Interval, InvalidIntervalError, InvalidPeriodError, J2000s, Jd, Mjd, Period,
    PeriodListError, Scale, Time, TimeContext, UnixSecs, DELTA_T_PREDICTION_HORIZON_MJD,
    MODERN_DELTA_T_OBSERVED_END_MJD, TAI, TCB, TCG, TDB, TT, UT1, UTC,
};
