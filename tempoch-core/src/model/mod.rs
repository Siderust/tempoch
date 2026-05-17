// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Core time model: instants, scales, and conversion targets.

pub(crate) mod civil;
pub mod scale;
pub mod target;
pub mod time;

pub use scale::{ContinuousScale, CoordinateScale, Scale, TAI, TCB, TCG, TDB, TT, UT1, UTC};
pub use target::{ContextConversionTarget, ConversionTarget, InfallibleConversionTarget};
pub use time::Time;
