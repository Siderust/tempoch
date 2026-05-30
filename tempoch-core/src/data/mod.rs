// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Runtime access to bundled and optionally refreshed time-data tables.

pub mod runtime_data;
pub mod status;

pub use status::{
    assert_fresh, time_data_status, ActiveTimeDataSource, DataHorizons, FreshnessError,
    TimeDataStatus,
};
