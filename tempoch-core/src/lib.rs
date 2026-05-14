// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Typed astronomical time primitives.
//!
//! The central type is [`Time<S>`], where `S` is a [`Scale`] marker
//! (`TT`, `TAI`, `UTC`, `UT1`, `TDB`, `TCG`, `TCB`).
//!
//! `tempoch` makes a few explicit modeling decisions:
//!
//! - [`Time<S>`] is an instant on a scale-specific axis, not a bare scalar.
//! - Time arithmetic follows affine rules: instant minus instant yields a
//!   duration; shifting an instant by a duration yields another instant.
//! - Internal storage is a compensated `(hi, lo)` pair of J2000-based seconds
//!   so large epoch values can retain small corrections and sub-second detail.
//! - `JD`, `MJD`, `J2000s`, `Unix`, and `GPS` are conversion targets,
//!   not independent storage models.
//! - `UTC` keeps special civil semantics: it is stored as a continuous instant
//!   and interpreted through the active UTC-TAI table when civil labels are
//!   needed.
//!
//! The old format-generic storage model has been replaced with explicit
//! constructors and accessors:
//!
//! - built-in coordinate scales expose J2000-second, JD, and MJD
//!   constructors/accessors
//! - `UTC` exposes both raw instant-axis helpers and civil/transport APIs
//!   (`chrono`, POSIX)
//! - `TAI` exposes GPS transport helpers
//! - unified conversion targets are available through `time.to::<Target>()`,
//!   `time.try_to::<Target>()`, and `time.to_with::<Target>(&ctx)`
//!
//! See [`constats`] for typed epoch and offset constants.

mod civil;
pub mod compat;
pub mod constats;
mod context;
pub mod coord;
mod data;
mod delta_t;
pub(crate) mod encoding;
pub mod eop;
pub mod error;
pub mod format;
pub mod ext;
pub(crate) mod generated;
mod interval;
pub mod representation;
pub mod scalar;
mod scale;
mod sealed;
mod target;
mod time;

#[cfg(feature = "serde")]
#[path = "serde.rs"]
mod serde_impl;
#[cfg(feature = "serde")]
pub mod tagged;

pub use constats::{
    GPS_EPOCH_JD_TAI, GPS_EPOCH_JD_UTC, GPS_EPOCH_TAI_MINUS_UTC, UTC_DEFINED_FROM_MJD,
};
pub use context::TimeContext;
pub use coord::{Coord, Offset};
#[cfg(feature = "runtime-data-fetch")]
pub use data::active::{
    fetch_latest_time_data, refresh_runtime_time_data, update_runtime_time_data,
};
pub use delta_t::{delta_t_seconds, delta_t_seconds_extrapolated, DELTA_T_PREDICTION_HORIZON_MJD};
pub use error::{ConversionError, TimeDataError};
pub use ext::TimeInstant;
pub use generated::{
    EOP_END_MJD, EOP_OBSERVED_END_MJD, EOP_START_MJD, MODERN_DELTA_T_OBSERVED_END_MJD,
};
pub use interval::{Interval, InvalidIntervalError, InvalidPeriodError, Period, PeriodListError};
pub use format::{
    EncodedTime, FormatForScale, GpsTime, InfallibleFormatForScale, J2000Seconds, J2000s,
    JulianDate, ModifiedJulianDate, TimeFormat, Unix, UnixTime, GPS, JD, MJD,
};
/// Compatibility re-export: `RepresentationForScale` is now [`FormatForScale`].
pub use format::FormatForScale as RepresentationForScale;
/// Compatibility re-export: `InfallibleRepresentationForScale` is now [`InfallibleFormatForScale`].
pub use format::InfallibleFormatForScale as InfallibleRepresentationForScale;
/// Compatibility re-export: `TimeRepresentation` is now [`TimeFormat`].
pub use format::TimeFormat as TimeRepresentation;
pub use scalar::{
    scalar_add_days, scalar_difference_in_days, time_tt_from_scalar, time_tt_to_scalar, ScaleKind,
};
pub use scale::{ContinuousScale, CoordinateScale, Scale, TAI, TCB, TCG, TDB, TT, UT1, UTC};
pub use target::{ContextConversionTarget, ConversionTarget, InfallibleConversionTarget};
pub use compat::{complement_within, TimeInstant, J2000_TT, JULIAN_YEAR_DAYS};
pub use time::Time;

#[cfg(test)]
mod size_tests {
    use super::*;
    #[test]
    fn time_uses_compensated_pair_storage() {
        assert_eq!(core::mem::size_of::<Time<TT>>(), 16);
        assert_eq!(core::mem::size_of::<Time<TAI>>(), 16);
        assert_eq!(core::mem::size_of::<Time<TDB>>(), 16);
        assert_eq!(core::mem::size_of::<Time<TCG>>(), 16);
        assert_eq!(core::mem::size_of::<Time<TCB>>(), 16);
        assert_eq!(core::mem::size_of::<Time<UT1>>(), 16);
        assert_eq!(core::mem::size_of::<Time<UTC>>(), 16);
    }
}
