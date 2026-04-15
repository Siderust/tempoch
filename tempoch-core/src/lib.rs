// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Typed astronomical time primitives.
//!
//! The central type is [`Time<A>`], where `A` is an [`Axis`] marker
//! (`TT`, `TAI`, `UTC`, `UT1`, `TDB`, `TCG`, `TCB`).
//!
//! Axis conversions:
//!
//! - `.to::<A2>()` — infallible closed-form routes (TT↔TAI, TT↔TDB, UTC↔any,
//!   etc.). Returns `Time<A2>` directly.
//! - `.to_with::<A2>(&ctx)` — context-required routes (UT1, via ΔT).
//!   Returns `Result<Time<A2>, ConversionError>`.
//!
//! See [`constats`] for typed epoch and offset constants.

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
