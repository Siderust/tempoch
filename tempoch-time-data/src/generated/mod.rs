// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Valles Puig, Ramon

use qtty::Day;

#[allow(dead_code)]
pub mod time_data;

pub const MODERN_DELTA_T_START_MJD: Day = Day::new(time_data::MODERN_DELTA_T_START_MJD);
pub const MODERN_DELTA_T_OBSERVED_END_MJD: Day =
    Day::new(time_data::MODERN_DELTA_T_OBSERVED_END_MJD);
pub const MODERN_DELTA_T_END_MJD: Day = Day::new(time_data::MODERN_DELTA_T_END_MJD);
