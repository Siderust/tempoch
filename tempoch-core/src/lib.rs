// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Typed astronomical time primitives.
//!
//! The central type is [`Time<S, F>`] (default `F` = [`J2000s`]), where `S` is a [`Scale`]
//! marker (`TT`, `TAI`, `UTC`, `UT1`, `TDB`, `TCG`, `TCB`) and `F` is a [`TimeFormat`]
//! tag (`JD`, `MJD`, [`J2000s`], [`Unix`], [`GPS`]).
//!
//! `tempoch` makes a few explicit modeling decisions:
//!
//! - [`Time<S, F>`] is an instant on a scale-specific axis; `F` only types external
//!   views (`raw()`, conversion targets), not a second storage layout.
//! - Time arithmetic follows affine rules: instant minus instant yields a
//!   duration; shifting an instant by a duration yields another instant.
//! - Internal storage is always a compensated `(hi, lo)` pair of J2000-based seconds
//!   so large epoch values can retain small corrections and sub-second detail.
//! - `JD`, `MJD`, `J2000s`, `Unix`, and `GPS` are conversion targets, not alternate
//!   stored encodings.
//! - `UTC` keeps special civil semantics: it is stored as a continuous instant
//!   and interpreted through the active UTC-TAI table when civil labels are
//!   needed.
//!
//! - [`Time::new`] builds from a raw scalar when `F` is [`InfallibleFormatForScale`](crate::InfallibleFormatForScale) for `S` (**NaN panics**; ±∞ allowed at rest). POSIX [`Unix`] instants still use [`Time::try_new`] / [`Time::try_new_with`] because decoding depends on leap-second tables.
//! - [`Time::try_new`] / [`Time::try_new_with`] surface **domain** decode failures only (UTC policy, leap seconds, …); callers must not pass **NaN**.
//! - `UTC`: civil (`chrono`) and POSIX ([`Unix`]); `TAI`: GPS ([`GPS`])
//! - Unified targets: [`Time::to`], [`Time::try_to`], [`Time::to_with`]. Prefer
//!   [`try_to`](Time::try_to) or [`to_with`](Time::to_with) for [`Unix`] so positive
//!   leap-second instants are rejected when they are not representable as POSIX.
//! - [`Time::to_j2000s`], [`Time::reinterpret`], and aliases
//!   such as [`crate::JulianDate`].
//! - [`JulianDate`], [`ModifiedJulianDate`], [`UnixTime`], and [`GpsTime`] implement [`Into`] into default-tagged
//!   [`Time<S>`] / [`Time<UTC>`] / [`Time<TAI>`], so APIs such as [`Period::try_new`](crate::Period) accept encoded
//!   endpoints without spelling [`Time::to_j2000s`]. [`J2000Seconds<S>`] is already [`Time<S>`]; no conversion needed.
//!
//! See [`constats`] for epoch [`Time`] helpers, day/second scratch constants, and offsets.
//!
//! # Module map
//!
//! - [`foundation`]: shared sealed traits, typed constants, and error types.
//! - [`model`]: [`Time`], scale markers, and conversion targets.
//! - [`format`]: external format markers and format conversion traits.
//! - `encoding`: crate-local JD/MJD/J2000/Unix arithmetic helpers.
//! - [`earth`]: ΔT, EOP, and [`TimeContext`] Earth-rotation policy.
//! - [`data`]: runtime access to bundled and optionally refreshed time-data tables.
//! - [`period`]: interval and period-list algebra.
//! - [`features`]: optional serde/tagged/time-instant integration helpers.
//!
//! Reference modules:
//!
//! - [`earth::delta_t`]: piecewise ΔT (`TT - UT1`) model and modern tabular segment.
//! - [`earth::eop`]: public EOP sampling API over bundled IERS series.
//! - [`earth::context`]: immutable time-data snapshot plus conversion policy.

pub mod data;
pub mod earth;
pub(crate) mod encoding;
pub mod features;
pub mod format;
pub mod foundation;
pub mod model;
pub mod period;

// Compiled tables live in `tempoch-time-data`; these are crate-local shims for tests and helpers.
#[allow(unused_imports)]
pub(crate) use tempoch_time_data::generated::{eop_data, time_data};
#[allow(unused_imports)]
pub(crate) use tempoch_time_data::generated::{MODERN_DELTA_T_END_MJD, MODERN_DELTA_T_START_MJD};

pub use earth::eop;
pub use foundation::{constats, error};

#[cfg(feature = "runtime-data-fetch")]
pub use data::runtime_data::{
    fetch_latest_time_data, refresh_runtime_time_data, update_runtime_time_data,
};
pub use earth::context::TimeContext;
pub use earth::delta_t::{
    delta_t_seconds, delta_t_seconds_extrapolated, DELTA_T_PREDICTION_HORIZON_MJD,
};
pub use features::TimeInstant;
pub use format::{
    FormatForScale, GpsTime, InfallibleFormatForScale, J2000Seconds, J2000s, JulianDate,
    ModifiedJulianDate, TimeFormat, Unix, UnixTime, GPS, JD, MJD,
};
pub use foundation::constats::{
    gps_epoch_jd_tai, gps_epoch_jd_utc, gps_epoch_tai, iau_time_epoch_t0_jd, j2000_jd_tt,
    tdb_tt_model_high_accuracy_end_jd, tdb_tt_model_high_accuracy_start_jd, unix_epoch_jd,
    unix_epoch_mjd, utc_defined_from_mjd, GPS_EPOCH_JD_TAI_DAY, GPS_EPOCH_JD_UTC_DAY,
    GPS_EPOCH_TAI_MINUS_UTC, GPS_EPOCH_TAI_SECONDS, IAU_TIME_EPOCH_T0_JD_DAY, J2000_JD_TT_DAY,
    JULIAN_YEAR_DAYS, TDB_TT_MODEL_HIGH_ACCURACY_END_JD_DAY,
    TDB_TT_MODEL_HIGH_ACCURACY_START_JD_DAY, TT_MINUS_TAI, UNIX_EPOCH_JD_DAY, UNIX_EPOCH_MJD_DAY,
    UTC_DEFINED_FROM_MJD_DAY,
};
pub use foundation::error::{ConversionError, TimeDataError};
pub use model::scale::{ContinuousScale, CoordinateScale, Scale, TAI, TCB, TCG, TDB, TT, UT1, UTC};
pub use model::target::{ContextConversionTarget, ConversionTarget, InfallibleConversionTarget};
pub use model::time::Time;
pub use period::{complement_within, Interval, InvalidIntervalError, Period, PeriodListError};
pub use tempoch_time_data::generated::{
    EOP_END_MJD, EOP_OBSERVED_END_MJD, EOP_START_MJD, MODERN_DELTA_T_OBSERVED_END_MJD,
};

#[cfg(feature = "serde")]
pub use features::tagged;

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
