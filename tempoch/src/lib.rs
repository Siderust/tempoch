// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Public façade over `tempoch-core`.
//!
//! The crate root exposes the axis / representation time model:
//!
//! - [`Time<A, R = Native>`] for typed instants
//! - [`Axis`] markers such as [`TT`], [`TAI`], [`UTC`], and [`UT1`]
//! - [`Representation`] markers such as [`JulianDays`],
//!   [`ModifiedJulianDays`], [`SISeconds`], and [`UnixSeconds`]
//! - compile-time conversion witnesses
//!   ([`InfallibleConvertible`], [`FallibleConvertible`],
//!   [`ContextConvertible`])

pub use tempoch_core::{
    complement_within, intersect_periods, normalize_periods, validate_period_list, Axis,
    ContextConvertible, ConversionError, FallibleConvertible, GpsSeconds, InfallibleConvertible,
    Interval, InvalidIntervalError, JulianDays, ModifiedJulianDays, Native, PeriodListError,
    Representation, SISeconds, Time, TimeContext, UnixSeconds, DELTA_T_PREDICTION_HORIZON_MJD,
    POSIX, TAI, TCB, TCG, TDB, TT, UT1, UTC,
};
