// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Valles Puig, Ramon

pub mod parse;
pub mod render;

pub use parse::{
    build_modern_delta_t_points, parse_delta_t_observed, parse_delta_t_predictions,
    parse_utc_tai_segments, UtcTaiSegment,
};
pub use render::{render_generated_module, Provenance, Sources};

pub const UTC_TAI_HISTORY_URL: &str = "https://hpiers.obspm.fr/eoppc/bul/bulc/UTC-TAI.history";
pub const DELTA_T_OBSERVED_URL: &str = "https://maia.usno.navy.mil/ser7/deltat.data";
pub const DELTA_T_PREDICTIONS_URL: &str = "https://maia.usno.navy.mil/ser7/deltat.preds";

pub const PRE_1961_TAI_MINUS_UTC_APPROX: f64 = 10.0;
