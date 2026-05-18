// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Shared crate foundations used by every domain layer.

pub mod constats;
pub mod error;
pub(crate) mod sealed;

pub use error::{ConversionError, TimeDataError};
