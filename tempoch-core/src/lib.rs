// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Typed astronomical time primitives.
//!
//! `tempoch-core` exposes a time model with two orthogonal type dimensions:
//!
//! - [`Axis`] describes the physical or civil axis (`TT`, `TAI`, `UTC`,
//!   `UT1`, ...).
//! - [`Representation`] describes how an instant on that axis is encoded
//!   (`Native`, [`JulianDays`], [`ModifiedJulianDays`], [`SISeconds`], ...).
//!
//! The primary type is [`Time<A, R = Native>`]. Axis conversions are selected
//! at compile time through witness traits:
//!
//! - [`InfallibleConvertible`] for exact, closed-form conversions
//! - [`FallibleConvertible`] for conversions that depend on compiled civil-time
//!   history
//! - [`ContextConvertible`] for conversions that require an explicit
//!   [`TimeContext`]

mod axis;
mod civil;
mod context;
mod conversion;
mod delta_t;
mod error;
pub(crate) mod generated;
mod interval;
mod representation;
mod sealed;
mod storage;
mod time;

pub use axis::{Axis, TAI, TCB, TCG, TDB, TT, UT1, UTC};
pub use context::TimeContext;
pub use conversion::{ContextConvertible, FallibleConvertible, InfallibleConvertible};
pub use delta_t::DELTA_T_PREDICTION_HORIZON_MJD;
pub use error::ConversionError;
pub use interval::{
    complement_within, intersect_periods, normalize_periods, validate_period_list, Interval,
    InvalidIntervalError, PeriodListError,
};
pub use representation::{
    GpsSeconds, JulianDays, ModifiedJulianDays, Native, Representation, SISeconds, UnixSeconds,
    POSIX,
};
pub use time::Time;
