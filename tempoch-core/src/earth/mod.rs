// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Earth-orientation and Earth-rotation conversion policy.

pub mod context;
pub mod delta_t;
pub mod eop;

pub use context::TimeContext;
pub use delta_t::{delta_t_seconds, delta_t_seconds_extrapolated, DELTA_T_PREDICTION_HORIZON_MJD};
