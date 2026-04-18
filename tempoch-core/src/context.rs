// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Conversion context.

use crate::eop::{builtin_eop_at, EopValues};
use qtty::{Day, Second};

/// Explicit, immutable context for conversions that need one.
///
/// A `TimeContext` selects which compiled data tables back UT1 conversions.
/// The default constructor [`TimeContext::new`] uses the monthly ΔT series,
/// matching the behaviour of previous versions; [`TimeContext::with_builtin_eop`]
/// selects the daily IERS `finals2000A.all` series for sub-ΔT-quantum UT1
/// accuracy inside its coverage window.
///
/// # ΔT / UT1 accuracy
///
/// | Epoch range | Default context (monthly ΔT) | `with_builtin_eop()` |
/// |---|---|---|
/// | Pre-948 CE | ±hundreds of s (Stephenson & Houlden quadratic) | same (outside EOP range) |
/// | 948–1619 | ±15 s (Stephenson & Houlden) | same |
/// | 1620–1973 | ±0.1–1 s (Meeus biennial table) | same |
/// | 1973 – EOP start | ~0.01 s (USNO monthly) | same |
/// | EOP observed range | ~0.01 s | ≲ 10 ms (daily IERS) |
/// | EOP prediction range | ~0.01 s | short-range Bulletin A prediction |
/// | Beyond EOP | ~0.01–growing | falls back to monthly ΔT |
///
/// The builtin EOP is only consulted inside its compiled coverage; outside of
/// that range the monthly ΔT path applies unchanged.
#[derive(Debug, Clone, Copy, Default)]
pub struct TimeContext {
    eop: EopSource,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum EopSource {
    /// Use the monthly ΔT series only (default, bit-compatible with
    /// pre-EOP tempoch releases).
    #[default]
    None,
    /// Consult the compiled daily IERS `finals2000A.all` series when the
    /// requested epoch is within coverage, and fall back to the monthly ΔT
    /// series otherwise.
    Builtin,
}

impl TimeContext {
    /// Construct a default context backed by the monthly ΔT table.
    #[inline]
    pub const fn new() -> Self {
        Self {
            eop: EopSource::None,
        }
    }

    /// Construct a context that prefers the compiled daily IERS
    /// `finals2000A.all` series for UT1 conversions when the epoch is
    /// within its coverage window.
    #[inline]
    pub const fn with_builtin_eop() -> Self {
        Self {
            eop: EopSource::Builtin,
        }
    }

    /// Interpolated EOP at `mjd_utc`, if this context has an EOP source and
    /// the MJD is in range. Intended for scale-conversion internals and for
    /// callers who want the same values the context uses.
    #[inline]
    pub fn eop_at(&self, mjd_utc: Day) -> Option<EopValues> {
        match self.eop {
            EopSource::None => None,
            EopSource::Builtin => builtin_eop_at(mjd_utc),
        }
    }

    /// Interpolated UT1 − UTC from the context's EOP source, if available.
    #[inline]
    pub fn ut1_minus_utc(&self, mjd_utc: Day) -> Option<Second> {
        self.eop_at(mjd_utc).map(|v| v.ut1_minus_utc)
    }
}
