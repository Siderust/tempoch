// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Internal IERS time-data support crate for tempoch.
//!
//! All types, parsers, and the runtime `TimeDataManager` are re-exported from
//! [`siderust_archive::time`].  This crate contributes:
//!
//! * The compiled-in UTC-TAI and ΔT snapshot (`generated`).
//! * [`bundled_time_data`]: the default offline fallback bundle (UTC-TAI + ΔT
//!   only; EOP data requires a runtime fetch via `TimeDataManager`).
//! * [`TEMPOCH_DATA_DIR_ENV`]: the environment variable used to locate the
//!   runtime data directory.

pub mod generated;

// All public types, parsers, managers, and URL constants come from the archive crate.
pub use siderust_archive::time::*;

/// Environment variable that overrides the default `~/.tempoch/data` directory.
pub const TEMPOCH_DATA_DIR_ENV: &str = "TEMPOCH_DATA_DIR";

/// Build the compiled-in bundled time-data snapshot.
///
/// The bundle contains the UTC-TAI history and ΔT tables compiled into the
/// crate at build time.  **EOP data is not included in the compiled bundle**;
/// callers that need Earth orientation parameters must either load a cached
/// bundle via [`TimeDataManager`] or call
/// [`crate::data::runtime_data::update_runtime_time_data`].
///
/// This is the default offline fallback used when no runtime bundle has been
/// loaded.
pub fn bundled_time_data() -> TimeDataBundle {
    TimeDataBundle::new(
        generated::time_data::UTC_TAI_SEGMENTS
            .iter()
            .map(|s| UtcTaiSegment {
                start_mjd: s.start_mjd,
                end_mjd: s.end_mjd,
                base_seconds: s.base_seconds,
                reference_mjd: s.reference_mjd,
                slope_seconds_per_day: s.slope_seconds_per_day,
            })
            .collect(),
        generated::time_data::MODERN_DELTA_T_POINTS.to_vec(),
        generated::MODERN_DELTA_T_OBSERVED_END_MJD.value(),
        Vec::new(), // EOP data is not compiled-in; requires runtime fetch
        TimeDataProvenance::new("compiled", "compiled", "compiled", "compiled", "compiled"),
    )
}
