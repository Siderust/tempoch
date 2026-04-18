// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Typed astronomical time primitives.
//!
//! The central type is [`Time<S, F>`], where `S` is a [`Scale`] marker
//! (`TT`, `TAI`, `UTC`, `UT1`, `TDB`, `TCG`, `TCB`) and `F` is a
//! [`Format`] marker (`J2000s`, `JD`, `MJD`, `UnixSecs`, `GpsSecs`,
//! `DayCount`).
//!
//! Scale conversions:
//!
//! - `.to_scale::<S2>()` — infallible closed-form routes (TT↔TAI, TT↔TDB,
//!   UTC↔any, etc.). Returns `Time<S2, F>` directly.
//! - `.to_scale_with::<S2>(&ctx)` — context-required routes (UT1, via ΔT).
//!   Returns `Result<Time<S2, F>, ConversionError>`.
//!
//! Format conversions:
//!
//! - `.reformat::<F2>()` — convert to a different format on the same scale.
//!
//! See [`constats`] for typed epoch and offset constants.

mod civil;
pub mod constats;
mod context;
mod delta_t;
pub(crate) mod encoding;
pub mod eop;
pub mod error;
mod format;
mod format_conversion;
pub(crate) mod generated;
mod interval;
#[cfg(feature = "runtime-data")]
pub mod runtime_data;
mod scale;
mod scale_conversion;
mod sealed;
mod time;

pub use context::TimeContext;
pub use delta_t::{delta_t_seconds, delta_t_seconds_extrapolated, DELTA_T_PREDICTION_HORIZON_MJD};
pub use error::ConversionError;
pub use format::{DayCount, Format, GpsSecs, J2000s, JD, MJD, UnixSecs};
pub use generated::{
    EOP_END_MJD, EOP_OBSERVED_END_MJD, EOP_START_MJD, MODERN_DELTA_T_OBSERVED_END_MJD,
};
pub use interval::{
    complement_within, intersect_periods, normalize_periods, validate_period_list, Interval,
    InvalidIntervalError, InvalidPeriodError, Period, PeriodListError,
};
pub use scale::{ContinuousScale, Scale, TAI, TCB, TCG, TDB, TT, UT1, UTC};
pub use time::Time;

#[cfg(test)]
mod size_tests {
    use super::*;
    #[test]
    fn continuous_j2000s_is_eight_bytes() {
        assert_eq!(core::mem::size_of::<Time<TT>>(), 8);
        assert_eq!(core::mem::size_of::<Time<TAI>>(), 8);
        assert_eq!(core::mem::size_of::<Time<TDB>>(), 8);
        assert_eq!(core::mem::size_of::<Time<TCG>>(), 8);
        assert_eq!(core::mem::size_of::<Time<TCB>>(), 8);
        assert_eq!(core::mem::size_of::<Time<UT1>>(), 8);
    }
    #[test]
    fn utc_j2000s_is_eight_bytes() {
        // No longer 16 — leap flag is computed on demand.
        assert_eq!(core::mem::size_of::<Time<UTC>>(), 8);
    }
    #[test]
    fn daycount_is_four_bytes() {
        assert_eq!(core::mem::size_of::<Time<TT, DayCount>>(), 4);
    }
    #[test]
    fn jd_is_eight_bytes() {
        assert_eq!(core::mem::size_of::<Time<TT, JD>>(), 8);
    }
}
