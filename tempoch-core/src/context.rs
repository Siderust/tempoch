// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Conversion context.

/// Explicit, immutable context for conversions that need one.
///
/// Constructed via `TimeContext::default()`. Contains only compiled data;
/// there is no runtime I/O.
#[derive(Debug, Clone, Copy, Default)]
pub struct TimeContext {
    _private: (),
}

impl TimeContext {
    /// Deterministic, compiled-data context. Cheap to construct.
    #[inline]
    pub const fn new() -> Self {
        Self { _private: () }
    }
}
