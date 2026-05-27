// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Runtime access to bundled and optionally refreshed time-data tables.

pub mod provenance;
pub mod runtime_data;
pub use provenance::{
    assert_fresh, provenance, DataHorizons, FreshnessError, ProvenanceSnapshot, SourceUrls,
};
