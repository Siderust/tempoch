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
///
/// # ΔT / UT1 accuracy
///
/// The compiled ΔT data is sourced from USNO monthly determinations and
/// older polynomial fits. The achievable accuracy for UT1 is:
///
/// | Epoch range | Source | Accuracy |
/// |---|---|---|
/// | Pre-948 CE | Stephenson & Houlden (1986) quadratic | ±hundreds of seconds |
/// | 948–1619 | Stephenson & Houlden (1986) quadratic | ±15 s |
/// | 1620–1973 | Meeus biennial table | ±0.1–1 s |
/// | 1973–observed end | USNO monthly (confirmed) | ~0.01 s |
/// | Observed end–horizon | USNO monthly (prediction) | growing uncertainty |
///
/// For precision work requiring daily IERS EOP (DUT1) data — VLBI, geodesy,
/// pulsar timing — this compiled context is not sufficient. A future version
/// of `TimeContext` may accept an injected EOP series.
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
