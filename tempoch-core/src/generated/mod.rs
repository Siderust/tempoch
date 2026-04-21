// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Valles Puig, Ramon

use qtty::Day;

#[allow(dead_code)]
pub(crate) mod eop_data;
#[allow(dead_code)]
pub(crate) mod time_data;

pub(crate) const MODERN_DELTA_T_START_MJD: Day = Day::new(time_data::MODERN_DELTA_T_START_MJD);
pub const MODERN_DELTA_T_OBSERVED_END_MJD: Day =
    Day::new(time_data::MODERN_DELTA_T_OBSERVED_END_MJD);
pub(crate) const MODERN_DELTA_T_END_MJD: Day = Day::new(time_data::MODERN_DELTA_T_END_MJD);

pub const EOP_START_MJD: Day = Day::new(eop_data::EOP_START_MJD as f64);
pub const EOP_OBSERVED_END_MJD: Day = Day::new(eop_data::EOP_OBSERVED_END_MJD as f64);
pub const EOP_END_MJD: Day = Day::new(eop_data::EOP_END_MJD as f64);
