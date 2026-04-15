// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Public façade over `tempoch-core`.
//!
//! The crate root exposes the axis time model:
//!
//! - [`Time<A>`] for typed instants on a given axis
//! - [`Axis`] markers such as [`TT`], [`TAI`], [`UTC`], and [`UT1`]
//! - [`constats`] for typed epoch and offset constants

pub use tempoch_core::{
    complement_within, constats, intersect_periods, normalize_periods, validate_period_list, Axis,
    ConversionError, Interval, InvalidIntervalError, PeriodListError, Time, TimeContext,
    DELTA_T_PREDICTION_HORIZON_MJD, TAI, TCB, TCG, TDB, TT, UT1, UTC,
};
