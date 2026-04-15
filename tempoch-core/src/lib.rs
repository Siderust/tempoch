// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Typed astronomical time primitives.
//!
//! `tempoch-core` exposes a time model with a single type dimension:
//!
//! - [`Axis`] describes the physical or civil axis (`TT`, `TAI`, `UTC`,
//!   `UT1`, ...).
//!
//! The primary type is [`Time<A>`]. Axis conversions use `.to::<A2>()`
//! (returns `Result`) and `.to_with::<UT1>(&ctx)` for context-dependent
//! routes.
//! - [`constats`] for typed epoch and offset constants

mod axis;
mod civil;
pub mod constats;
mod context;
mod conversion;
mod delta_t;
pub(crate) mod encoding;
pub mod error;
pub(crate) mod generated;
mod interval;
mod sealed;
mod storage;
mod time;

pub use axis::{Axis, TAI, TCB, TCG, TDB, TT, UT1, UTC};
pub use context::TimeContext;
pub use delta_t::DELTA_T_PREDICTION_HORIZON_MJD;
pub use error::ConversionError;
pub use interval::{
    complement_within, intersect_periods, normalize_periods, validate_period_list, Interval,
    InvalidIntervalError, PeriodListError,
};
pub use time::Time;
