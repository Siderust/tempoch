// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Optional or higher-level feature modules layered on top of the core time model.

mod time_instant;

pub use time_instant::TimeInstant;

#[cfg(feature = "serde")]
mod serde_impl;
#[cfg(feature = "serde")]
pub mod tagged;
