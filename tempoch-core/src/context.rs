// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Conversion context.

/// Explicit, immutable context for conversions that need one.
///
/// Currently a zero-sized token. All converted data (ΔT, UTC-TAI history)
/// is compiled into the crate as static tables. `TimeContext` is the
/// intended future injection point for observed Earth-orientation or
/// time-history data; callers should construct it through [`TimeContext::new`]
/// rather than relying on the `Default` impl so that the API remains stable
/// when the type gains fields.
#[derive(Debug, Clone, Copy, Default)]
pub struct TimeContext {
    _private: (),
}

impl TimeContext {
    /// Construct a default context backed by the compiled data tables.
    #[inline]
    pub const fn new() -> Self {
        Self { _private: () }
    }
}
