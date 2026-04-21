// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Conversion context.

use crate::data::active::{active_time_data, time_data_eop_at};
use crate::eop::EopValues;
use std::sync::Arc;
use tempoch_time_data::TimeDataBundle;
use qtty::{Day, Second};

/// Explicit, immutable context for conversions that need one.
///
/// A `TimeContext` snapshots the active time-data bundle at construction time
/// and selects which parts of that snapshot back context-required conversions.
/// The default constructor [`TimeContext::new`] uses the monthly ΔT series from
/// the captured bundle, matching the behaviour of previous versions;
/// [`TimeContext::with_builtin_eop`] selects the daily IERS `finals2000A.all`
/// series from that same snapshot for the highest-fidelity bundled UT1 path
/// inside its coverage window.
///
/// # ΔT / UT1 accuracy
///
/// | Epoch range | Default context (monthly ΔT) | `with_builtin_eop()` |
/// |---|---|---|
/// | Pre-948 CE | ±hundreds of s (Stephenson & Houlden quadratic) | same (outside EOP range) |
/// | 948–1619 | ±15 s (Stephenson & Houlden) | same |
/// | 1620–1973 | ±0.1–1 s (Meeus biennial table) | same |
/// | 1973 – EOP start | ~0.01 s (USNO monthly) | same |
/// | EOP observed range | For the compiled bundle fetched 2026-04-18, < 10 ms from the bundled daily IERS-derived path through 2026-04-16 | preferred highest-fidelity bundled UT1 path |
/// | EOP prediction range | For the compiled bundle fetched 2026-04-18, < 0.2 s from the bundled short-range daily prediction through 2027-04-24 | preferred highest-fidelity bundled UT1 path |
/// | Beyond EOP | monthly ΔT only; prediction uncertainty grows | falls back to monthly ΔT |
///
/// The builtin EOP is only consulted inside the captured bundle's coverage;
/// outside of that range the monthly ΔT path applies unchanged. Construct a
/// fresh context after refreshing the active bundle if you want to use the
/// updated runtime data.
#[derive(Debug, Clone)]
pub struct TimeContext {
    data: Arc<TimeDataBundle>,
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

impl Default for TimeContext {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl TimeContext {
    #[inline]
    fn snapshot(eop: EopSource) -> Self {
        Self {
            data: active_time_data(),
            eop,
        }
    }

    /// Construct a default context backed by the monthly ΔT table.
    #[inline]
    pub fn new() -> Self {
        Self::snapshot(EopSource::None)
    }

    /// Construct a context that prefers the compiled daily IERS
    /// `finals2000A.all` series for UT1 conversions when the epoch is
    /// within its coverage window.
    #[inline]
    pub fn with_builtin_eop() -> Self {
        Self::snapshot(EopSource::Builtin)
    }

    #[inline]
    pub(crate) fn time_data(&self) -> &TimeDataBundle {
        self.data.as_ref()
    }

    /// Interpolated EOP at `mjd_utc`, if this context has an EOP source and
    /// the MJD is in range. Intended for scale-conversion internals and for
    /// callers who want the same values the context uses.
    #[inline]
    pub fn eop_at(&self, mjd_utc: Day) -> Option<EopValues> {
        match self.eop {
            EopSource::None => None,
            EopSource::Builtin => time_data_eop_at(self.time_data(), mjd_utc),
        }
    }

    /// Interpolated UT1 − UTC from the context's EOP source, if available.
    #[inline]
    pub fn ut1_minus_utc(&self, mjd_utc: Day) -> Option<Second> {
        self.eop_at(mjd_utc).map(|v| v.ut1_minus_utc)
    }
}
