// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Conversion context.

use crate::data::runtime_data::{active_time_data, time_data_eop_at};
use crate::earth::eop::EopValues;
use qtty::{Day, Second};
use std::sync::Arc;
use tempoch_time_data::TimeDataBundle;

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
/// | EOP observed range | < 15 ms from the bundled daily IERS-derived path over the compiled observed overlap | preferred highest-fidelity bundled UT1 path |
/// | EOP prediction range | < 0.2 s from the bundled short-range daily prediction over the compiled prediction overlap | preferred highest-fidelity bundled UT1 path |
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
    utc_pre_definition: bool,
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
            utc_pre_definition: false,
        }
    }

    /// Construct a default context backed by the monthly ΔT table.
    ///
    /// This is the lightweight, always-available choice. It does not consult
    /// the daily EOP series even when the bundled data contains one.
    #[inline]
    pub fn new() -> Self {
        Self::snapshot(EopSource::None)
    }

    /// Construct a context that prefers the compiled daily IERS
    /// `finals2000A.all` series for UT1 conversions when the epoch is
    /// within its coverage window.
    ///
    /// Outside the bundled EOP coverage, this falls back to the same monthly
    /// ΔT path used by [`TimeContext::new`].
    #[inline]
    pub fn with_builtin_eop() -> Self {
        Self::snapshot(EopSource::Builtin)
    }

    #[inline]
    pub(crate) fn time_data(&self) -> &TimeDataBundle {
        self.data.as_ref()
    }

    /// Allow UTC conversions for dates before 1961-01-01.
    ///
    /// By default, [`Time::<UTC>::try_from_chrono_with`](crate::Time::try_from_chrono_with) and related conversions
    /// return [`crate::ConversionError::UtcBeforeDefinition`] for any date
    /// before MJD 37 300 (1961-01-01), because UTC was not an international
    /// standard before that date and the back-extrapolated offset is
    /// historically fabricated.
    ///
    /// Calling this method on a context opts into the approximate
    /// continuation: the first official UTC-TAI segment is extrapolated
    /// backwards. Round-trips close, but the values are not
    /// standards-defined UTC.
    ///
    /// # Example
    /// ```
    /// use tempoch_core::{TimeContext, Time, UTC};
    /// use chrono::DateTime;
    ///
    /// let dt = DateTime::from_timestamp(-631_152_000, 0).unwrap();
    /// let ctx = TimeContext::new().allow_pre_definition_utc();
    /// let utc = Time::<UTC>::try_from_chrono_with(dt, &ctx).unwrap();
    /// ```
    #[inline]
    pub fn allow_pre_definition_utc(mut self) -> Self {
        self.utc_pre_definition = true;
        self
    }

    #[inline]
    pub(crate) fn allows_pre_definition_utc(&self) -> bool {
        self.utc_pre_definition
    }
    /// Interpolated EOP at `mjd_utc`, if this context has an EOP source and
    /// the MJD is in range.
    ///
    /// This exposes the same interpolated values that context-backed scale
    /// conversions consult internally, so callers can inspect or reuse them
    /// without reimplementing the lookup path.
    #[inline]
    pub fn eop_at(&self, mjd_utc: Day) -> Option<EopValues> {
        match self.eop {
            EopSource::None => None,
            EopSource::Builtin => time_data_eop_at(self.time_data(), mjd_utc),
        }
    }

    /// Interpolated `UT1 - UTC` from the context's EOP source, if available.
    ///
    /// Returns `None` when this context is monthly-ΔT-only or when the epoch is
    /// outside the captured EOP coverage window.
    #[inline]
    pub fn ut1_minus_utc(&self, mjd_utc: Day) -> Option<Second> {
        self.eop_at(mjd_utc).map(|v| v.ut1_minus_utc)
    }
}
